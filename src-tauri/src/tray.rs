use tauri::{
    image::Image,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder,
};

const ICON_IDLE: &[u8] = include_bytes!("../icons/tray-idle.png");
const ICON_ACTIVE: &[u8] = include_bytes!("../icons/tray-active.png");
const ICON_ATTENTION: &[u8] = include_bytes!("../icons/tray-attention.png");

#[derive(Debug, Clone, PartialEq)]
pub enum TrayState {
    Idle,
    Active,
    Attention,
}

pub fn setup_tray(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let icon = Image::from_bytes(ICON_IDLE)?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .icon_as_template(false)
        .tooltip("MindIsland — idle")
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

pub fn update_tray_state(app: &tauri::AppHandle, state: TrayState) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let (icon_bytes, tooltip) = match state {
            TrayState::Idle => (ICON_IDLE, "MindIsland — idle"),
            TrayState::Active => (ICON_ACTIVE, "MindIsland — agents running"),
            TrayState::Attention => (ICON_ATTENTION, "MindIsland — needs attention"),
        };
        if let Ok(icon) = Image::from_bytes(icon_bytes) {
            let _ = tray.set_icon(Some(icon));
            let _ = tray.set_tooltip(Some(tooltip));
        }
    }
}

/// Show the panel without toggling (for auto-popup on permission requests)
pub fn show_panel(app: &tauri::AppHandle) {
    let window = match app.get_webview_window("panel") {
        Some(w) => w,
        None => {
            // Window not created yet — create it now
            let w = WebviewWindowBuilder::new(app, "panel", WebviewUrl::default())
                .title("MindIsland")
                .inner_size(360.0, 480.0)
                .resizable(false)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(false)
                .build();
            match w {
                Ok(w) => {
                    let w_clone = w.clone();
                    w.on_window_event(move |event| {
                        if let tauri::WindowEvent::Focused(false) = event {
                            let _ = w_clone.hide();
                        }
                    });
                    w
                }
                Err(_) => return,
            }
        }
    };
    if !window.is_visible().unwrap_or(false) {
        position_panel_near_tray(&window);
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("panel-opened", ());
    }
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
