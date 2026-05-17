use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::agents::claude::{ClaudeCodeAdapter, PermissionResponse};
use crate::agents::claude_discovery::ClaudeTranscriptDiscovery;
use crate::event::{AgentEvent, AgentSession, SessionPhase};
use crate::session_registry::SessionRegistry;

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
    registry: Arc<SessionRegistry>,
    app_handle: AppHandle,
    claude: Arc<ClaudeCodeAdapter>,
}

impl SessionManager {
    pub fn new(app_handle: AppHandle) -> Self {
        let registry = Arc::new(SessionRegistry::new());

        // Restore persisted sessions
        let mut initial = HashMap::new();
        for session in registry.load() {
            initial.insert(session.id.clone(), session);
        }
        if !initial.is_empty() {
            eprintln!("[mindisland] Restored {} sessions from registry", initial.len());
        }

        Self {
            sessions: Arc::new(Mutex::new(initial)),
            registry,
            app_handle,
            claude: Arc::new(ClaudeCodeAdapter::new()),
        }
    }

    pub fn get_sessions(&self) -> Vec<AgentSession> {
        let sessions = self.sessions.lock().unwrap();
        let mut list: Vec<AgentSession> = sessions.values().cloned().collect();
        list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        list
    }

    pub fn resolve_permission(&self, session_id: &str, approved: bool) {
        let resolved = self.claude.resolve_permission(
            session_id,
            PermissionResponse {
                approved,
                message: if approved { None } else { Some("Denied in MindIsland".to_string()) },
            },
        );

        if resolved {
            let mut sessions = self.sessions.lock().unwrap();
            if let Some(session) = sessions.get_mut(session_id) {
                session.pending_permission = None;
                session.phase = if approved {
                    SessionPhase::Running
                } else {
                    SessionPhase::Completed
                };
                session.summary = if approved {
                    "Permission approved".to_string()
                } else {
                    "Permission denied".to_string()
                };
                session.updated_at = now_millis();
            }
            let list: Vec<AgentSession> = sessions.values().cloned().collect();
            let _ = self.app_handle.emit("sessions-updated", &list);
            self.registry.save(&list);
        }
    }

    pub async fn start_monitoring(&self) -> Result<(), String> {
        let (tx, mut rx) = mpsc::channel::<AgentEvent>(256);

        // --- Discover existing sessions from transcripts ---
        let tx_disc = tx.clone();
        tokio::spawn(async move {
            let discovery = ClaudeTranscriptDiscovery::new();
            for event in discovery.discover() {
                let _ = tx_disc.send(event).await;
            }
        });

        // --- Claude Code hook bridge ---
        let claude = self.claude.clone();
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
        }

        // --- Event processor ---
        let sessions = self.sessions.clone();
        let app_handle = self.app_handle.clone();
        let registry = self.registry.clone();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let mut map = sessions.lock().unwrap();
                let now = now_millis();

                match &event {
                    AgentEvent::SessionStarted {
                        agent_id,
                        session_id,
                        title,
                        directory,
                        model,
                    } => {
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
                        } else if let Some(session) = map.get_mut(session_id) {
                            if model.is_some() {
                                session.model = model.clone();
                            }
                        }
                    }

                    AgentEvent::ActivityUpdated {
                        session_id, phase, summary, tool_name,
                    } => {
                        if let Some(session) = map.get_mut(session_id) {
                            session.phase = phase.clone();
                            session.summary = summary.clone();
                            session.current_tool = tool_name.clone();
                            session.updated_at = now;
                        }
                    }

                    AgentEvent::PermissionRequested {
                        session_id, permission,
                    } => {
                        if let Some(session) = map.get_mut(session_id) {
                            session.phase = SessionPhase::WaitingForApproval;
                            session.summary = format!("Permission: {}", permission.title);
                            session.pending_permission = Some(permission.clone());
                            session.updated_at = now;
                        }
                    }

                    AgentEvent::QuestionAsked {
                        session_id, question,
                    } => {
                        if let Some(session) = map.get_mut(session_id) {
                            session.phase = SessionPhase::WaitingForAnswer;
                            session.summary = question.clone();
                            session.updated_at = now;
                        }
                    }

                    AgentEvent::SessionCompleted {
                        session_id, summary,
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
                        map.remove(session_id);
                    }
                }

                let list: Vec<AgentSession> = map.values().cloned().collect();
                let _ = app_handle.emit("sessions-updated", &list);

                // Persist to disk (debounce-friendly — just overwrite)
                registry.save(&list);
            }
        });

        Ok(())
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
