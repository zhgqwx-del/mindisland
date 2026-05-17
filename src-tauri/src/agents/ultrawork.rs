use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::event::{AgentEvent, PermissionRequest, SessionStatus};

pub struct UltraWorkAdapter {
    base_url: String,
    auth_header: String,
}

#[derive(Debug, Deserialize)]
struct SseEvent {
    #[serde(rename = "type")]
    event_type: String,
    properties: serde_json::Value,
}

impl UltraWorkAdapter {
    pub fn new(base_url: String, credentials: String) -> Self {
        use base64::Engine;
        let auth_header = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes())
        );
        Self {
            base_url,
            auth_header,
        }
    }

    pub async fn is_available(&self) -> bool {
        let client = reqwest::Client::new();
        client
            .get(format!("{}/global/health", self.base_url))
            .header(AUTHORIZATION, &self.auth_header)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn start(&self, tx: mpsc::Sender<AgentEvent>) -> Result<(), String> {
        let client = reqwest::Client::new();

        // Fetch existing sessions first
        self.fetch_existing_sessions(&client, &tx).await;

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&self.auth_header).unwrap(),
        );
        headers.insert(ACCEPT, HeaderValue::from_static("text/event-stream"));

        let response = client
            .get(format!("{}/event", self.base_url))
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("SSE connect failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("SSE returned status: {}", response.status()));
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("SSE read error: {}", e))?;
            let text = String::from_utf8_lossy(&chunk);

            buffer.push_str(&text);

            // Process complete SSE lines
            while let Some(pos) = buffer.find("\n\n") {
                let message = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                if let Some(data) = message.strip_prefix("data: ") {
                    if let Ok(event) = serde_json::from_str::<SseEvent>(data) {
                        if let Some(agent_event) = self.translate_event(&event) {
                            let _ = tx.send(agent_event).await;
                        }
                    }
                }
            }
        }

        Err("SSE stream ended".to_string())
    }

    async fn fetch_existing_sessions(
        &self,
        client: &reqwest::Client,
        tx: &mpsc::Sender<AgentEvent>,
    ) {
        let resp = client
            .get(format!("{}/session", self.base_url))
            .header(AUTHORIZATION, &self.auth_header)
            .send()
            .await;

        if let Ok(resp) = resp {
            if let Ok(sessions) = resp.json::<Vec<serde_json::Value>>().await {
                let count = sessions.len();
                for session in sessions {
                    let session_id = session
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if session_id.is_empty() {
                        continue;
                    }
                    let title = session
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Session")
                        .to_string();
                    let directory = session
                        .get("directory")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let updated_at = session
                        .get("time")
                        .and_then(|t| t.get("updated"))
                        .and_then(|v| v.as_u64());

                    let _ = tx
                        .send(AgentEvent::SessionCreated {
                            agent_id: "ultrawork".to_string(),
                            session_id: session_id.clone(),
                            title,
                            directory,
                            updated_at,
                        })
                        .await;

                    // Fetch last message to show as activity
                    let last_activity = self
                        .fetch_last_activity(client, &session_id)
                        .await
                        .unwrap_or_default();
                    if !last_activity.is_empty() {
                        let _ = tx
                            .send(AgentEvent::SessionStatusChanged {
                                session_id,
                                status: SessionStatus::Completed,
                                summary: Some(last_activity),
                            })
                            .await;
                    }
                }
                eprintln!(
                    "[mindisland] Loaded {} existing UltraWork sessions",
                    count
                );
            }
        }
    }

    async fn fetch_last_activity(
        &self,
        client: &reqwest::Client,
        session_id: &str,
    ) -> Option<String> {
        let resp = client
            .get(format!("{}/session/{}/message", self.base_url, session_id))
            .header(AUTHORIZATION, &self.auth_header)
            .send()
            .await
            .ok()?;
        let messages = resp.json::<Vec<serde_json::Value>>().await.ok()?;

        // Find last text part from any message (walk backwards)
        for msg in messages.iter().rev() {
            if let Some(parts) = msg.get("parts").and_then(|v| v.as_array()) {
                for part in parts.iter().rev() {
                    if part.get("type").and_then(|v| v.as_str()) == Some("text") {
                        let text = part
                            .get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .trim();
                        if !text.is_empty() {
                            let first_line = text.lines().next().unwrap_or(text);
                            let preview = if first_line.len() > 80 {
                                format!("{}...", &first_line[..80])
                            } else {
                                first_line.to_string()
                            };
                            return Some(preview);
                        }
                    }
                }
            }
        }
        None
    }

    pub async fn resolve_permission(&self, permission_id: &str, approved: bool) -> Result<(), String> {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "status": if approved { "allow" } else { "deny" }
        });

        client
            .post(format!("{}/permission/{}/reply", self.base_url, permission_id))
            .header(AUTHORIZATION, &self.auth_header)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Permission reply failed: {}", e))?;

        Ok(())
    }

    fn translate_event(&self, event: &SseEvent) -> Option<AgentEvent> {
        let props = &event.properties;

        match event.event_type.as_str() {
            "session.created" => {
                let session_id = props.get("sessionID")?.as_str()?.to_string();
                let info = props.get("info")?;
                let title = info
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("New session")
                    .to_string();
                let directory = info
                    .get("directory")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                Some(AgentEvent::SessionCreated {
                    agent_id: "ultrawork".to_string(),
                    session_id,
                    title,
                    directory,
                    updated_at: None,
                })
            }

            "session.status" => {
                let session_id = props.get("sessionID")?.as_str()?.to_string();
                let status_obj = props.get("status")?;
                let status_type = status_obj.get("type")?.as_str()?;

                let status = match status_type {
                    "busy" => SessionStatus::Busy,
                    "idle" => SessionStatus::Idle,
                    "retry" => SessionStatus::Error,
                    _ => return None,
                };

                Some(AgentEvent::SessionStatusChanged {
                    session_id,
                    status,
                    summary: None,
                })
            }

            "session.idle" => {
                let session_id = props.get("sessionID")?.as_str()?.to_string();
                Some(AgentEvent::SessionStatusChanged {
                    session_id,
                    status: SessionStatus::Idle,
                    summary: Some("Completed".to_string()),
                })
            }

            "permission.asked" => {
                let session_id = props.get("sessionID")?.as_str()?.to_string();
                let id = props.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let title = props
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Permission requested")
                    .to_string();
                let description = props
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let tool_name = props
                    .get("tool")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                Some(AgentEvent::PermissionRequested {
                    session_id,
                    permission: PermissionRequest {
                        id,
                        title,
                        description,
                        tool_name,
                    },
                })
            }

            "message.part.updated" => {
                let session_id = props.get("sessionID")?.as_str()?.to_string();
                let part = props.get("part")?;
                let part_type = part.get("type")?.as_str()?;

                // Only track text completions (step-finish means turn done)
                if part_type == "step-finish" {
                    return Some(AgentEvent::SessionStatusChanged {
                        session_id,
                        status: SessionStatus::Busy,
                        summary: Some("Processing...".to_string()),
                    });
                }

                if part_type == "text" {
                    let text = part.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    if !text.is_empty() {
                        let preview = if text.len() > 80 {
                            format!("{}...", &text[..80])
                        } else {
                            text.to_string()
                        };
                        return Some(AgentEvent::SessionStatusChanged {
                            session_id,
                            status: SessionStatus::Busy,
                            summary: Some(preview),
                        });
                    }
                }

                None
            }

            "message.updated" => {
                // Track user messages as activity
                let session_id = props.get("sessionID")?.as_str()?.to_string();
                let info = props.get("info")?;
                let role = info.get("role")?.as_str()?;

                if role == "user" {
                    Some(AgentEvent::SessionStatusChanged {
                        session_id,
                        status: SessionStatus::Busy,
                        summary: Some("Processing user message...".to_string()),
                    })
                } else {
                    None
                }
            }

            _ => None,
        }
    }
}
