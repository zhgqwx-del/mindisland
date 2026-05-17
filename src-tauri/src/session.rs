use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::agents::claude::ClaudeCodeAdapter;
use crate::agents::claude_discovery::ClaudeTranscriptDiscovery;
use crate::event::{AgentEvent, AgentSession, SessionPhase};

struct AgentMeta {
    name: &'static str,
    color: &'static str,
}

fn agent_meta(agent_id: &str) -> AgentMeta {
    match agent_id {
        "claude-code" => AgentMeta { name: "Claude Code", color: "#d97742" },
        "opencode" => AgentMeta { name: "OpenCode", color: "#ffb547" },
        _ => AgentMeta { name: "Agent", color: "#71717a" },
    }
}

#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, AgentSession>>>,
    app_handle: AppHandle,
}

impl SessionManager {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            app_handle,
        }
    }

    pub fn get_sessions(&self) -> Vec<AgentSession> {
        let sessions = self.sessions.lock().unwrap();
        let mut list: Vec<AgentSession> = sessions.values().cloned().collect();
        list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        list
    }

    pub async fn start_monitoring(&self) -> Result<(), String> {
        let (tx, mut rx) = mpsc::channel::<AgentEvent>(256);

        // --- Discover existing Claude Code sessions from transcripts ---
        let tx_disc = tx.clone();
        tokio::spawn(async move {
            let discovery = ClaudeTranscriptDiscovery::new();
            for event in discovery.discover() {
                let _ = tx_disc.send(event).await;
            }
        });

        // --- Claude Code (Unix Socket hook bridge) ---
        let claude = Arc::new(ClaudeCodeAdapter::new());
        if ClaudeCodeAdapter::is_installed() {
            let tx_cc = tx.clone();
            eprintln!(
                "[mindisland] Claude Code detected, bridge at {:?}",
                claude.socket_path()
            );
            tokio::spawn(async move {
                if let Err(e) = claude.start(tx_cc).await {
                    eprintln!("[mindisland] Claude Code bridge error: {}", e);
                }
            });
        } else {
            eprintln!("[mindisland] Claude Code not installed, skipping");
        }

        // TODO: OpenCode adapter (Phase 2)
        // TODO: UltraWork adapter (Phase 3)

        // --- Event processor ---
        let sessions = self.sessions.clone();
        let app_handle = self.app_handle.clone();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let mut map = sessions.lock().unwrap();
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                match &event {
                    AgentEvent::SessionStarted {
                        agent_id,
                        session_id,
                        title,
                        directory,
                        model,
                    } => {
                        // Only create if not exists (idempotent)
                        if !map.contains_key(session_id) {
                            let meta = agent_meta(agent_id);
                            map.insert(
                                session_id.clone(),
                                AgentSession {
                                    id: session_id.clone(),
                                    agent_id: agent_id.clone(),
                                    agent_name: meta.name.to_string(),
                                    brand_color: meta.color.to_string(),
                                    title: title.clone(),
                                    directory: directory.clone(),
                                    phase: SessionPhase::Completed,
                                    summary: "Session started".to_string(),
                                    updated_at: now,
                                    model: model.clone(),
                                    current_tool: None,
                                    pending_permission: None,
                                },
                            );
                        } else {
                            // Update model if provided
                            if let Some(session) = map.get_mut(session_id) {
                                if model.is_some() {
                                    session.model = model.clone();
                                }
                            }
                        }
                    }

                    AgentEvent::ActivityUpdated {
                        session_id,
                        phase,
                        summary,
                        tool_name,
                    } => {
                        if let Some(session) = map.get_mut(session_id) {
                            session.phase = phase.clone();
                            session.summary = summary.clone();
                            session.current_tool = tool_name.clone();
                            session.updated_at = now;
                        }
                    }

                    AgentEvent::PermissionRequested {
                        session_id,
                        permission,
                    } => {
                        if let Some(session) = map.get_mut(session_id) {
                            session.phase = SessionPhase::WaitingForApproval;
                            session.summary = format!("Permission: {}", permission.title);
                            session.pending_permission = Some(permission.clone());
                            session.updated_at = now;
                        }
                    }

                    AgentEvent::QuestionAsked {
                        session_id,
                        question,
                    } => {
                        if let Some(session) = map.get_mut(session_id) {
                            session.phase = SessionPhase::WaitingForAnswer;
                            session.summary = question.clone();
                            session.updated_at = now;
                        }
                    }

                    AgentEvent::SessionCompleted {
                        session_id,
                        summary,
                    } => {
                        if let Some(session) = map.get_mut(session_id) {
                            session.phase = SessionPhase::Completed;
                            session.summary = summary.clone();
                            session.current_tool = None;
                            session.pending_permission = None;
                            session.updated_at = now;
                        }
                    }

                    AgentEvent::SessionEnded { session_id } => {
                        // Remove ended sessions from the list
                        map.remove(session_id);
                    }
                }

                // Emit to frontend
                let list: Vec<AgentSession> = map.values().cloned().collect();
                let _ = app_handle.emit("sessions-updated", &list);
            }
        });

        Ok(())
    }
}
