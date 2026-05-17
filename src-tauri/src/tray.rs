use tauri::{
    image::Image,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder,
};

pub fn setup_tray(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let icon_bytes = include_bytes!("../icons/tray-icon.png");
    let icon = Image::from_bytes(icon_bytes)?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("MindIsland")
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                toggle_panel(app);
            }
        })
        .build(app)?;

    Ok(())
}

fn toggle_panel(app: &tauri::AppHandle) {
    let window = match app.get_webview_window("panel") {
        Some(w) => w,
        None => {
            let w = WebviewWindowBuilder::new(app, "panel", WebviewUrl::default())
                .title("MindIsland")
                .inner_size(360.0, 480.0)
                .resizable(false)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(false)
                .build()
                .expect("failed to create panel window");

            // Auto-hide when focus is lost
            let w_clone = w.clone();
            w.on_window_event(move |event| {
                if let tauri::WindowEvent::Focused(false) = event {
                    let _ = w_clone.hide();
                }
            });

            w
        }
    };

    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        position_panel_near_tray(&window);
        let _ = window.show();
        let _ = window.set_focus();
        // Push latest sessions to frontend when panel opens
        let _ = window.emit("panel-opened", ());
    }
}

fn position_panel_near_tray(window: &tauri::WebviewWindow) {
    if let Ok(Some(monitor)) = window.primary_monitor() {
        let screen_size = monitor.size();
        let scale = monitor.scale_factor();
        let panel_width = 360.0;

        #[cfg(target_os = "macos")]
        {
            let x = (screen_size.width as f64 / scale - panel_width - 16.0) as i32;
            let y = 28i32;
            let _ = window.set_position(tauri::Position::Physical(
                tauri::PhysicalPosition::new(x, y),
            ));
        }

        #[cfg(target_os = "windows")]
        {
            let panel_height = 480.0;
            let x = (screen_size.width as f64 / scale - panel_width - 16.0) as i32;
            let y = (screen_size.height as f64 / scale - panel_height - 48.0) as i32;
            let _ = window.set_position(tauri::Position::Physical(
                tauri::PhysicalPosition::new(x, y),
            ));
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let x = (screen_size.width as f64 / scale - panel_width - 16.0) as i32;
            let _ = window.set_position(tauri::Position::Physical(
                tauri::PhysicalPosition::new(x, 28),
            ));
        }
    }
}
