mod event;
mod session;
mod agents;
mod tray;

use session::SessionManager;
use tauri::Manager;

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

            tray::setup_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_sessions, resolve_permission])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
