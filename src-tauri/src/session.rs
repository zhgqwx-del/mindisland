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

        // --- Clean up stale sessions on startup ---
        // Mark all non-terminal sessions as Completed (they were saved mid-run
        // before a restart). Live sessions will be re-activated by new hook
        // events or transcript discovery.
        {
            let mut sessions = self.sessions.lock().unwrap();
            let now = now_millis();
            let two_hours = 2 * 3600 * 1000;
            for session in sessions.values_mut() {
                if session.phase != SessionPhase::Completed {
                    session.phase = SessionPhase::Completed;
                    session.summary = "MindIsland restarted".to_string();
                    session.current_tool = None;
                    session.pending_permission = None;
                    // Keep original updated_at so stale sessions don't
                    // reappear in the 2-minute visibility window.
                }
            }
            // Remove sessions completed > 2 hours ago
            sessions.retain(|_, s| (now - s.updated_at) < two_hours);
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
                // Process event under lock, then release before side effects
                let list = {
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
                                    last_assistant_message: None,
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
                            // Don't bump timestamp when re-discovering an
                            // already-completed session (transcript discovery).
                            // This prevents stale sessions from reappearing in
                            // the 2-minute visibility window after restart.
                            let is_rediscovery = session.phase == SessionPhase::Completed
                                && *phase == SessionPhase::Completed;
                            session.phase = phase.clone();
                            session.current_tool = tool_name.clone();
                            if !is_rediscovery {
                                session.updated_at = now;
                            }

                            // Capture user prompt from "Prompt: ..." summaries
                            if let Some(prompt) = summary.strip_prefix("Prompt: ") {
                                session.last_user_prompt = Some(prompt.to_string());
                                if session.initial_prompt.is_none() {
                                    session.initial_prompt = Some(prompt.to_string());
                                }
                            }
                            // Capture assistant messages from Notification events
                            // (tool_name is None for non-tool activity)
                            if tool_name.is_none()
                                && !summary.starts_with("Prompt: ")
                                && !summary.starts_with("Compacting")
                                && !summary.starts_with("Started subagent")
                                && !summary.starts_with("Subagent")
                                && !summary.starts_with("Session started")
                            {
                                session.last_assistant_message = Some(summary.clone());
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
                            // Preserve assistant message for display
                            session.last_assistant_message = Some(summary.clone());
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
                list
                }; // ← mutex released here, BEFORE side effects

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

/// Check if agent processes are still running.
/// Mark sessions as completed if their process has exited.
fn check_process_liveness(
    sessions: &Arc<Mutex<HashMap<String, AgentSession>>>,
    app_handle: &AppHandle,
    registry: &Arc<SessionRegistry>,
) {
    // Check which agent processes are alive
    let processes_output = std::process::Command::new("sh")
        .args(["-c", "ps -eo comm="])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let claude_alive = processes_output.lines().any(|l| l.trim() == "claude");
    let opencode_alive = processes_output.lines().any(|l| l.trim() == "opencode");

    let mut sessions = sessions.lock().unwrap();
    let now = now_millis();
    let mut changed = false;

    for session in sessions.values_mut() {
        let is_active = session.phase == SessionPhase::Running
            || session.phase == SessionPhase::WaitingForApproval
            || session.phase == SessionPhase::WaitingForAnswer;
        if !is_active {
            continue;
        }

        let process_dead = match session.agent_id.as_str() {
            "claude-code" => !claude_alive,
            "opencode" => !opencode_alive,
            _ => false,
        };

        if process_dead {
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
        eprintln!("[mindisland] Agent process not found, marked sessions as completed");
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
