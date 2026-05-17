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

const HOOK_SCRIPT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../hooks/mindisland-claude-hook.sh"
);

const OPENCODE_PLUGIN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../hooks/mindisland-opencode-plugin.js"
);

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

#[tauri::command]
fn get_hook_status() -> HookStatus {
    HookInstaller::new(HOOK_SCRIPT.to_string()).status()
}

#[tauri::command]
fn install_hooks() -> Result<HookStatus, String> {
    HookInstaller::new(HOOK_SCRIPT.to_string()).install()
}

#[tauri::command]
fn uninstall_hooks() -> Result<HookStatus, String> {
    HookInstaller::new(HOOK_SCRIPT.to_string()).uninstall()
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

            // Auto-install Claude Code hooks
            if agents::claude::ClaudeCodeAdapter::is_installed() {
                let installer = HookInstaller::new(HOOK_SCRIPT.to_string());
                let status = installer.status();
                if !status.installed {
                    match installer.install() {
                        Ok(s) => eprintln!("[mindisland] Auto-installed Claude Code hooks for {} events", s.events_registered),
                        Err(e) => eprintln!("[mindisland] Failed to auto-install Claude Code hooks: {}", e),
                    }
                }
            }

            // Auto-install OpenCode plugin
            if agents::opencode::OpenCodeInstaller::is_available() {
                let oc_installer = agents::opencode::OpenCodeInstaller::new(OPENCODE_PLUGIN.to_string());
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
