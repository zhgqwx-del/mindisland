mod event;
mod session;
mod session_registry;
mod hook_installer;
mod agents;
mod tray;

use hook_installer::{HookInstaller, HookStatus};
use session::SessionManager;
use tauri::Manager;
use tauri::LogicalSize;
use std::fs;
use std::os::unix::fs::PermissionsExt;

/// Hook/plugin content embedded at compile time so the binary is self-contained.
const HOOK_SCRIPT_CONTENT: &str = include_str!("../../hooks/mindisland-claude-hook.sh");
const OPENCODE_PLUGIN_CONTENT: &str = include_str!("../../hooks/mindisland-opencode-plugin.js");
const HOOK_RUNTIME_PATH: &str = "~/.mindisland/hooks/mindisland-claude-hook.sh";

/// Write embedded content to a runtime path. Returns the absolute path on success.
fn write_embedded_file(tilde_path: &str, content: &str, executable: bool) -> Option<String> {
    let home = dirs::home_dir()?;
    let path = tilde_path.replacen("~", &home.to_string_lossy(), 1);
    let path = std::path::Path::new(&path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok()?;
    }
    // Only write if content changed
    if path.exists() {
        if let Ok(existing) = fs::read_to_string(path) {
            if existing == content {
                return Some(path.to_string_lossy().to_string());
            }
        }
    }
    fs::write(path, content).ok()?;
    if executable {
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).ok()?;
    }
    eprintln!("[mindisland] Wrote embedded file to {:?}", path);
    Some(path.to_string_lossy().to_string())
}

#[tauri::command]
fn get_sessions(state: tauri::State<'_, SessionManager>) -> Vec<event::AgentSession> {
    state.get_sessions()
}

#[tauri::command]
fn resolve_permission(
    state: tauri::State<'_, SessionManager>,
    session_id: String,
    approved: bool,
) {
    state.resolve_permission(&session_id, approved);
}

#[tauri::command]
fn toggle_mute(state: tauri::State<'_, SessionManager>) -> bool {
    state.toggle_mute()
}

#[tauri::command]
fn is_muted(state: tauri::State<'_, SessionManager>) -> bool {
    state.is_muted()
}

#[tauri::command]
fn resize_panel(window: tauri::WebviewWindow, height: f64) {
    let h = height.max(100.0).min(560.0);
    let _ = window.set_size(LogicalSize::new(380.0, h));
}

fn hook_path() -> String {
    let home = dirs::home_dir().unwrap_or_default();
    HOOK_RUNTIME_PATH.replacen("~", &home.to_string_lossy(), 1)
}

#[tauri::command]
fn get_hook_status() -> HookStatus {
    HookInstaller::new(hook_path()).status()
}

#[tauri::command]
fn install_hooks() -> Result<HookStatus, String> {
    HookInstaller::new(hook_path()).install()
}

#[tauri::command]
fn uninstall_hooks() -> Result<HookStatus, String> {
    HookInstaller::new(hook_path()).uninstall()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }

            let session_manager = SessionManager::new(app.handle().clone());
            app.manage(session_manager.clone());

            tauri::async_runtime::spawn(async move {
                if let Err(e) = session_manager.start_monitoring().await {
                    eprintln!("Failed to start monitoring: {}", e);
                }
            });

            // Write embedded hook/plugin to runtime paths
            let hook_path = write_embedded_file(
                "~/.mindisland/hooks/mindisland-claude-hook.sh",
                HOOK_SCRIPT_CONTENT,
                true, // executable
            );
            let plugin_content = OPENCODE_PLUGIN_CONTENT;

            // Auto-install Claude Code hooks
            if agents::claude::ClaudeCodeAdapter::is_installed() {
                if let Some(ref path) = hook_path {
                    let installer = HookInstaller::new(path.clone());
                    let status = installer.status();
                    if !status.installed {
                        match installer.install() {
                            Ok(s) => eprintln!("[mindisland] Auto-installed Claude Code hooks for {} events", s.events_registered),
                            Err(e) => eprintln!("[mindisland] Failed to auto-install Claude Code hooks: {}", e),
                        }
                    }
                }
            }

            // Auto-install OpenCode plugin
            if agents::opencode::OpenCodeInstaller::is_available() {
                let oc_installer = agents::opencode::OpenCodeInstaller::new_from_content(plugin_content.to_string());
                if !oc_installer.is_installed() {
                    match oc_installer.install() {
                        Ok(()) => eprintln!("[mindisland] Auto-installed OpenCode plugin"),
                        Err(e) => eprintln!("[mindisland] Failed to auto-install OpenCode plugin: {}", e),
                    }
                }
            }

            tray::setup_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_sessions,
            resolve_permission,
            resize_panel,
            toggle_mute,
            is_muted,
            get_hook_status,
            install_hooks,
            uninstall_hooks
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
