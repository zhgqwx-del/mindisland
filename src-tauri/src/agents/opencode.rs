use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::event::{AgentEvent, SessionStatus};

/// OpenCode adapter — same SSE mechanism as UltraWork but with
/// configurable URL and different display name.
pub struct OpenCodeAdapter {
    base_url: String,
    auth_header: String,
    display_name: String,
    agent_id: String,
    brand_color: String,
}

#[derive(Debug, Deserialize)]
struct SseEvent {
    #[serde(rename = "type")]
    event_type: String,
    properties: serde_json::Value,
}

impl OpenCodeAdapter {
    pub fn new(base_url: String, credentials: String) -> Self {
        use base64::Engine;
        let auth_header = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes())
        );
        Self {
            base_url,
            auth_header,
            display_name: "OpenCode".to_string(),
            agent_id: "opencode".to_string(),
            brand_color: "#10b981".to_string(), // green
        }
    }

    pub fn with_identity(mut self, agent_id: &str, display_name: &str, brand_color: &str) -> Self {
        self.agent_id = agent_id.to_string();
        self.display_name = display_name.to_string();
        self.brand_color = brand_color.to_string();
        self
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
                    agent_id: self.agent_id.clone(),
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
                let title = props
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Permission requested")
                    .to_string();

                Some(AgentEvent::PermissionRequested {
                    session_id,
                    permission: crate::event::PermissionRequest {
                        id: props.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        title,
                        description: props.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        tool_name: props.get("tool").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    },
                })
            }

            "message.part.updated" => {
                let session_id = props.get("sessionID")?.as_str()?.to_string();
                let part = props.get("part")?;
                let part_type = part.get("type")?.as_str()?;

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

            _ => None,
        }
    }
}
