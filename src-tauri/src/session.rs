use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::agents::claude::{ClaudeCodeAdapter, PermissionResponse};
use crate::agents::claude_discovery::ClaudeTranscriptDiscovery;
use crate::event::{AgentEvent, AgentSession, SessionPhase};
use crate::session_registry::SessionRegistry;
use crate::tray;

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
    /// Track last notification state to avoid duplicate sounds
    last_attention_session: Arc<Mutex<Option<String>>>,
    /// Track if sound is muted
    sound_muted: Arc<Mutex<bool>>,
}

impl SessionManager {
    pub fn new(app_handle: AppHandle) -> Self {
        let registry = Arc::new(SessionRegistry::new());

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
            last_attention_session: Arc::new(Mutex::new(None)),
            sound_muted: Arc::new(Mutex::new(false)),
        }
    }

    pub fn get_sessions(&self) -> Vec<AgentSession> {
        let sessions = self.sessions.lock().unwrap();
        let mut list: Vec<AgentSession> = sessions.values().cloned().collect();
        list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        list
    }

    pub fn toggle_mute(&self) -> bool {
        let mut muted = self.sound_muted.lock().unwrap();
        *muted = !*muted;
        *muted
    }

    pub fn is_muted(&self) -> bool {
        *self.sound_muted.lock().unwrap()
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

            // Clear attention state so sound doesn't replay
            *self.last_attention_session.lock().unwrap() = None;
            // Update tray to reflect resolved state
            self.update_tray(&list);
        }
    }

    pub async fn start_monitoring(&self) -> Result<(), String> {
        let (tx, mut rx) = mpsc::channel::<AgentEvent>(256);

        // --- Clean up stale sessions (completed > 2 hours ago) ---
        {
            let mut sessions = self.sessions.lock().unwrap();
            let now = now_millis();
            let two_hours = 2 * 3600 * 1000;
            sessions.retain(|_, s| {
                s.phase != SessionPhase::Completed || (now - s.updated_at) < two_hours
            });
            self.registry.save(&sessions.values().cloned().collect::<Vec<_>>());
        }

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

        // --- Process liveness monitor (every 15s) ---
        let sessions_for_monitor = self.sessions.clone();
        let app_handle_monitor = self.app_handle.clone();
        let registry_monitor = self.registry.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
                check_process_liveness(
                    &sessions_for_monitor,
                    &app_handle_monitor,
                    &registry_monitor,
                );
            }
        });

        // --- Event processor ---
        let sessions = self.sessions.clone();
        let app_handle = self.app_handle.clone();
        let registry = self.registry.clone();
        let last_attention = self.last_attention_session.clone();
        let sound_muted = self.sound_muted.clone();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let mut map = sessions.lock().unwrap();
                let now = now_millis();

                match &event {
                    AgentEvent::SessionStarted {
                        agent_id, session_id, title, directory, model,
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
                                    initial_prompt: None,
                                    last_user_prompt: None,
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
                            session.current_tool = tool_name.clone();
                            session.updated_at = now;

                            // Capture user prompt from "Prompt: ..." summaries
                            if let Some(prompt) = summary.strip_prefix("Prompt: ") {
                                session.last_user_prompt = Some(prompt.to_string());
                                if session.initial_prompt.is_none() {
                                    session.initial_prompt = Some(prompt.to_string());
                                }
                            }
                            session.summary = summary.clone();
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
                registry.save(&list);

                // --- Tray icon + notification logic ---
                let attention_session = list.iter().find(|s| {
                    s.phase == SessionPhase::WaitingForApproval
                        || s.phase == SessionPhase::WaitingForAnswer
                });
                let has_active = list.iter().any(|s| s.phase == SessionPhase::Running);

                // Update tray icon
                let tray_state = if attention_session.is_some() {
                    tray::TrayState::Attention
                } else if has_active {
                    tray::TrayState::Active
                } else {
                    tray::TrayState::Idle
                };
                tray::update_tray_state(&app_handle, tray_state);

                // Notification: only play sound for NEW attention (deduplicated)
                if let Some(attn) = attention_session {
                    let mut last = last_attention.lock().unwrap();
                    let is_new = last.as_deref() != Some(&attn.id);
                    if is_new {
                        *last = Some(attn.id.clone());
                        drop(last); // release lock before side effects

                        tray::show_panel(&app_handle);
                        if !*sound_muted.lock().unwrap() {
                            play_notification_sound();
                        }
                    }
                } else {
                    // No attention — clear tracking
                    *last_attention.lock().unwrap() = None;
                }
            }
        });

        Ok(())
    }

    fn update_tray(&self, sessions: &[AgentSession]) {
        let has_attention = sessions.iter().any(|s| {
            s.phase == SessionPhase::WaitingForApproval
                || s.phase == SessionPhase::WaitingForAnswer
        });
        let has_active = sessions.iter().any(|s| s.phase == SessionPhase::Running);
        let state = if has_attention {
            tray::TrayState::Attention
        } else if has_active {
            tray::TrayState::Active
        } else {
            tray::TrayState::Idle
        };
        tray::update_tray_state(&self.app_handle, state);
    }
}

/// Check if Claude Code processes are still running.
/// Mark sessions as completed if their process has exited.
fn check_process_liveness(
    sessions: &Arc<Mutex<HashMap<String, AgentSession>>>,
    app_handle: &AppHandle,
    registry: &Arc<SessionRegistry>,
) {
    // Check if any Claude Code CLI process is running.
    // Use ps to find the exact "claude" binary; pgrep -f "claude" is too broad
    // and matches Claude.app, MindIsland paths, hook scripts, etc.
    let claude_alive = std::process::Command::new("sh")
        .args(["-c", "ps -eo comm= | grep -qx claude"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if claude_alive {
        return; // At least one Claude process is running, don't touch sessions
    }

    // No Claude process found — mark active sessions as completed
    let mut sessions = sessions.lock().unwrap();
    let now = now_millis();
    let mut changed = false;

    for session in sessions.values_mut() {
        if session.agent_id == "claude-code"
            && (session.phase == SessionPhase::Running
                || session.phase == SessionPhase::WaitingForApproval
                || session.phase == SessionPhase::WaitingForAnswer)
        {
            session.phase = SessionPhase::Completed;
            session.summary = "Process exited".to_string();
            session.current_tool = None;
            session.pending_permission = None;
            session.updated_at = now;
            changed = true;
        }
    }

    if changed {
        let list: Vec<AgentSession> = sessions.values().cloned().collect();
        let _ = app_handle.emit("sessions-updated", &list);
        registry.save(&list);
        eprintln!("[mindisland] Claude Code process not found, marked sessions as completed");
    }
}

fn play_notification_sound() {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("afplay")
            .arg("/System/Library/Sounds/Glass.aiff")
            .spawn()
            .ok();
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
