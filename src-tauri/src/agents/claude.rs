use std::path::PathBuf;
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::mpsc;

use crate::event::{AgentEvent, PermissionRequest, SessionPhase};

pub struct ClaudeCodeAdapter {
    socket_path: PathBuf,
}

/// Full Claude Code hook payload — all fields from the wire protocol.
/// Aligned with Open Vibe Island's ClaudeHookPayload.
#[derive(Debug, Deserialize)]
struct Payload {
    session_id: Option<String>,
    hook_event_name: Option<String>,
    cwd: Option<String>,
    tool_name: Option<String>,
    tool_input: Option<serde_json::Value>,
    #[allow(dead_code)]
    tool_use_id: Option<String>,
    #[allow(dead_code)]
    tool_response: Option<serde_json::Value>,
    last_assistant_message: Option<String>,
    #[allow(dead_code)]
    transcript_path: Option<String>,
    permission_mode: Option<String>,
    model: Option<String>,
    prompt: Option<String>,
    message: Option<String>,
    title: Option<String>,
    error: Option<String>,
    #[allow(dead_code)]
    is_interrupt: Option<bool>,
    #[allow(dead_code)]
    agent_id: Option<String>,
    #[allow(dead_code)]
    agent_type: Option<String>,
    #[allow(dead_code)]
    notification_type: Option<String>,
}

impl ClaudeCodeAdapter {
    pub fn new() -> Self {
        Self {
            socket_path: PathBuf::from("/tmp/mindisland-claude.sock"),
        }
    }

    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }

    pub fn is_installed() -> bool {
        dirs::home_dir()
            .map(|h| h.join(".claude").exists())
            .unwrap_or(false)
    }

    pub async fn start(&self, tx: mpsc::Sender<AgentEvent>) -> Result<(), String> {
        let _ = std::fs::remove_file(&self.socket_path);

        let listener = UnixListener::bind(&self.socket_path)
            .map_err(|e| format!("Failed to bind socket: {}", e))?;

        eprintln!(
            "[mindisland] Claude Code bridge listening at {:?}",
            self.socket_path
        );

        loop {
            let (stream, _) = listener
                .accept()
                .await
                .map_err(|e| format!("Accept error: {}", e))?;

            let tx = tx.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stream);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    match serde_json::from_str::<Payload>(&line) {
                        Ok(p) => {
                            // Skip subagent hooks (agent_id set) — parent session
                            // already gets SubagentStart/Stop events
                            if p.agent_id.is_some()
                                && !matches!(
                                    p.hook_event_name.as_deref(),
                                    Some("SubagentStart") | Some("SubagentStop")
                                )
                            {
                                continue;
                            }

                            let events = translate(&p);
                            for event in events {
                                let _ = tx.send(event).await;
                            }
                        }
                        Err(e) => {
                            eprintln!("[mindisland] Claude parse error: {}", e);
                        }
                    }
                }
            });
        }
    }
}

/// Translate a Claude Code hook payload into MindIsland events.
/// May return multiple events (e.g. implicit SessionStarted + ActivityUpdated).
fn translate(p: &Payload) -> Vec<AgentEvent> {
    let session_id = match p.session_id.as_deref() {
        Some(id) => id.to_string(),
        None => return vec![],
    };
    let event = match p.hook_event_name.as_deref() {
        Some(e) => e,
        None => return vec![],
    };
    let cwd = p.cwd.as_deref().unwrap_or("").to_string();

    // For any event, ensure session exists first
    let ensure_session = AgentEvent::SessionStarted {
        agent_id: "claude-code".to_string(),
        session_id: session_id.clone(),
        title: p
            .title
            .as_deref()
            .map(|t| t.to_string())
            .unwrap_or_else(|| project_name(&cwd)),
        directory: cwd.clone(),
        model: p.model.clone(),
    };

    match event {
        "SessionStart" => {
            vec![ensure_session]
        }

        "UserPromptSubmit" => {
            let prompt_preview = p
                .prompt
                .as_deref()
                .or(p.message.as_deref())
                .map(|s| clip(s, 80))
                .unwrap_or_else(|| "Processing prompt...".to_string());

            vec![
                ensure_session,
                AgentEvent::ActivityUpdated {
                    session_id,
                    phase: SessionPhase::Running,
                    summary: format!("Prompt: {}", prompt_preview),
                    tool_name: None,
                },
            ]
        }

        "PreToolUse" => {
            let tool = p.tool_name.as_deref().unwrap_or("tool");
            let detail = tool_detail(tool, &p.tool_input);
            vec![
                ensure_session,
                AgentEvent::ActivityUpdated {
                    session_id,
                    phase: SessionPhase::Running,
                    summary: detail,
                    tool_name: Some(tool.to_string()),
                },
            ]
        }

        "PostToolUse" => {
            let tool = p.tool_name.as_deref().unwrap_or("tool");
            vec![
                ensure_session,
                AgentEvent::ActivityUpdated {
                    session_id,
                    phase: SessionPhase::Running,
                    summary: format!("{} done", tool),
                    tool_name: None,
                },
            ]
        }

        "PostToolUseFailure" => {
            let msg = p.error.as_deref().unwrap_or("Tool failed");
            vec![
                ensure_session,
                AgentEvent::ActivityUpdated {
                    session_id,
                    phase: SessionPhase::Running,
                    summary: clip(msg, 80),
                    tool_name: None,
                },
            ]
        }

        "PermissionRequest" => {
            let tool = p.tool_name.as_deref().unwrap_or("tool");
            let title = format!("Allow {}?", tool);
            let desc = tool_detail(tool, &p.tool_input);
            vec![
                ensure_session,
                AgentEvent::PermissionRequested {
                    session_id,
                    permission: PermissionRequest {
                        id: p.session_id.clone().unwrap_or_default(),
                        title,
                        description: desc,
                        tool_name: Some(tool.to_string()),
                    },
                },
            ]
        }

        "PermissionDenied" => {
            let msg = p.error.as_deref().unwrap_or("Permission denied");
            vec![
                ensure_session,
                AgentEvent::SessionCompleted {
                    session_id,
                    summary: clip(msg, 80),
                },
            ]
        }

        "Notification" => {
            let msg = p
                .last_assistant_message
                .as_deref()
                .or(p.message.as_deref())
                .unwrap_or("Notification");
            vec![
                ensure_session,
                AgentEvent::ActivityUpdated {
                    session_id,
                    phase: SessionPhase::Running,
                    summary: clip(msg, 80),
                    tool_name: None,
                },
            ]
        }

        "PreCompact" => {
            vec![
                ensure_session,
                AgentEvent::ActivityUpdated {
                    session_id,
                    phase: SessionPhase::Running,
                    summary: "Compacting context...".to_string(),
                    tool_name: None,
                },
            ]
        }

        "Stop" | "StopFailure" => {
            let summary = p
                .last_assistant_message
                .as_deref()
                .map(|m| clip(m, 80))
                .or_else(|| p.error.as_deref().map(|e| clip(e, 80)))
                .unwrap_or_else(|| "Completed".to_string());
            vec![
                ensure_session,
                AgentEvent::SessionCompleted {
                    session_id,
                    summary,
                },
            ]
        }

        "SessionEnd" => {
            vec![ensure_session, AgentEvent::SessionEnded { session_id }]
        }

        "SubagentStart" => {
            vec![
                ensure_session,
                AgentEvent::ActivityUpdated {
                    session_id,
                    phase: SessionPhase::Running,
                    summary: "Started subagent".to_string(),
                    tool_name: Some("Agent".to_string()),
                },
            ]
        }

        "SubagentStop" => {
            let msg = p
                .last_assistant_message
                .as_deref()
                .map(|m| clip(m, 60))
                .unwrap_or_else(|| "Subagent finished".to_string());
            vec![
                ensure_session,
                AgentEvent::ActivityUpdated {
                    session_id,
                    phase: SessionPhase::Running,
                    summary: msg,
                    tool_name: None,
                },
            ]
        }

        _ => vec![],
    }
}

fn project_name(cwd: &str) -> String {
    let parts: Vec<&str> = cwd.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() >= 2 {
        format!("{}/{}", parts[parts.len() - 2], parts[parts.len() - 1])
    } else {
        parts.last().unwrap_or(&"Claude Code").to_string()
    }
}

fn clip(s: &str, max: usize) -> String {
    let first_line = s.lines().next().unwrap_or(s);
    if first_line.len() > max {
        format!("{}...", &first_line[..max])
    } else {
        first_line.to_string()
    }
}

fn tool_detail(tool: &str, input: &Option<serde_json::Value>) -> String {
    let input = match input {
        Some(v) => v,
        None => return format!("Running {}", tool),
    };

    match tool {
        "Bash" => {
            let cmd = input.get("command").and_then(|v| v.as_str()).unwrap_or("");
            let first = cmd.lines().next().unwrap_or(cmd);
            let preview: String = first.chars().take(60).collect();
            if preview.is_empty() {
                "Running shell command".to_string()
            } else {
                format!("$ {}", preview)
            }
        }
        "Read" => {
            let path = input.get("file_path").and_then(|v| v.as_str()).unwrap_or("");
            format!("Reading {}", path.split('/').last().unwrap_or(path))
        }
        "Edit" => {
            let path = input.get("file_path").and_then(|v| v.as_str()).unwrap_or("");
            format!("Editing {}", path.split('/').last().unwrap_or(path))
        }
        "Write" => {
            let path = input.get("file_path").and_then(|v| v.as_str()).unwrap_or("");
            format!("Writing {}", path.split('/').last().unwrap_or(path))
        }
        "Grep" => {
            let pat = input.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
            format!("Searching: {}", pat.chars().take(40).collect::<String>())
        }
        "Glob" => {
            let pat = input.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
            format!("Finding: {}", pat)
        }
        "Agent" => {
            let desc = input.get("description").and_then(|v| v.as_str()).unwrap_or("subagent");
            format!("Agent: {}", desc.chars().take(50).collect::<String>())
        }
        "WebFetch" | "WebSearch" => {
            let url_or_q = input
                .get("url")
                .or(input.get("query"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!("{}: {}", tool, url_or_q.chars().take(50).collect::<String>())
        }
        _ => format!("Running {}", tool),
    }
}
