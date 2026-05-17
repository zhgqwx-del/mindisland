use tauri::{
    image::Image,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder,
};

const ICON_IDLE: &[u8] = include_bytes!("../icons/tray-idle.png");
const ICON_ACTIVE: &[u8] = include_bytes!("../icons/tray-active.png");
const ICON_ATTENTION: &[u8] = include_bytes!("../icons/tray-attention.png");

const PANEL_WIDTH: f64 = 380.0;
const PANEL_HEIGHT: f64 = 140.0; // Small initial height, frontend will resize

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
                toggle_panel(tray.app_handle());
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

pub fn show_panel(app: &tauri::AppHandle) {
    let window = get_or_create_panel(app);
    if let Some(window) = window {
        if !window.is_visible().unwrap_or(false) {
            position_panel(&window);
            let _ = window.show();
            let _ = window.set_focus();
            let _ = window.emit("panel-opened", ());
        }
    }
}

fn toggle_panel(app: &tauri::AppHandle) {
    let window = get_or_create_panel(app);
    if let Some(window) = window {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            position_panel(&window);
            let _ = window.show();
            let _ = window.set_focus();
            let _ = window.emit("panel-opened", ());
        }
    }
}

fn get_or_create_panel(app: &tauri::AppHandle) -> Option<tauri::WebviewWindow> {
    if let Some(w) = app.get_webview_window("panel") {
        return Some(w);
    }

    let w = WebviewWindowBuilder::new(app, "panel", WebviewUrl::default())
        .title("MindIsland")
        .inner_size(PANEL_WIDTH, PANEL_HEIGHT)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .transparent(true)
        .background_color(tauri_utils::config::Color(13, 13, 15, 255))
        .build()
        .ok()?;

    // Auto-hide when focus is lost
    let w_clone = w.clone();
    w.on_window_event(move |event| {
        if let tauri::WindowEvent::Focused(false) = event {
            let _ = w_clone.hide();
        }
    });

    Some(w)
}

fn position_panel(window: &tauri::WebviewWindow) {
    if let Ok(Some(monitor)) = window.primary_monitor() {
        let screen = monitor.size();
        let scale = monitor.scale_factor();

        #[cfg(target_os = "macos")]
        let (x, y) = (
            (screen.width as f64 / scale - PANEL_WIDTH - 16.0) as i32,
            28i32,
        );

        #[cfg(target_os = "windows")]
        let (x, y) = (
            (screen.width as f64 / scale - PANEL_WIDTH - 16.0) as i32,
            (screen.height as f64 / scale - PANEL_HEIGHT - 48.0) as i32,
        );

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let (x, y) = (
            (screen.width as f64 / scale - PANEL_WIDTH - 16.0) as i32,
            28i32,
        );

        let _ = window.set_position(tauri::Position::Physical(
            tauri::PhysicalPosition::new(x, y),
        ));
    }
}
