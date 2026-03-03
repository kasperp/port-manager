mod commands;
mod config;
mod state;
mod status;
mod tunnel;

use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{
    menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

use crate::state::{AppState, SharedState};
use crate::status::PortStatus;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("Cannot resolve AppData dir");
            let cfg = config::load_config(&data_dir);

            let shared: SharedState = Arc::new(Mutex::new(AppState::new(cfg)));
            app.manage(shared.clone());

            setup_tray(app.handle(), shared.clone())?;

            // Start background auto-reconnect + status emission task
            let app_handle = app.handle().clone();
            let state_bg = shared.clone();
            tauri::async_runtime::spawn(async move {
                background_task(app_handle, state_bg).await;
            });

            // Ensure the window grabs focus so WebView2 accepts keyboard input
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_focus();
            }

            // Start SSH tunnels on launch
            {
                let mut s = shared.lock().unwrap();
                let config = s.config.clone();
                tunnel::start_all(&mut s.tunnels, &config);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_settings,
            commands::add_port,
            commands::remove_port,
            commands::start_all,
            commands::stop_all,
            commands::set_auto_reconnect,
            commands::get_port_statuses,
            commands::set_startup_enabled,
            commands::get_startup_enabled,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn build_tray_menu(
    app: &AppHandle,
    statuses: &[status::PortStatusInfo],
) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(app, "show", "Show Port Manager", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;

    let port_items: Vec<MenuItem<tauri::Wry>> = statuses
        .iter()
        .map(|s| {
            let active = matches!(s.status, PortStatus::Active);
            let label = format!(
                "{}  :{}  —  {}",
                if active { "●" } else { "○" },
                s.port,
                if active { "Active" } else { "Inactive" }
            );
            MenuItem::with_id(app, format!("ps-{}", s.port), label, false, None::<&str>)
        })
        .collect::<tauri::Result<_>>()?;

    let sep_ports = PredefinedMenuItem::separator(app)?;
    let start = MenuItem::with_id(app, "start", "▶  Start All", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "■  Stop All", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let mut all: Vec<&dyn IsMenuItem<tauri::Wry>> = vec![&show, &sep1];
    for item in &port_items {
        all.push(item);
    }
    if !port_items.is_empty() {
        all.push(&sep_ports);
    }
    all.extend([
        &start as &dyn IsMenuItem<_>,
        &stop,
        &sep2,
        &quit,
    ]);
    Menu::with_items(app, &all)
}

fn setup_tray(app: &AppHandle, state: SharedState) -> tauri::Result<()> {
    let menu = build_tray_menu(app, &[])?;

    let state_menu = state.clone();

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("Port Manager")
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "start" => {
                let mut s = state_menu.lock().unwrap();
                let cfg = s.config.clone();
                tunnel::start_all(&mut s.tunnels, &cfg);
            }
            "stop" => {
                let mut s = state_menu.lock().unwrap();
                tunnel::stop_all(&mut s.tunnels);
            }
            "quit" => {
                {
                    let mut s = state_menu.lock().unwrap();
                    tunnel::stop_all(&mut s.tunnels);
                }
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

async fn background_task(app: AppHandle, state: SharedState) {
    loop {
        tokio::time::sleep(Duration::from_secs(10)).await;

        let statuses = {
            let mut s = state.lock().unwrap();
            let auto = s.auto_reconnect;
            let config = s.config.clone();

            if auto {
                let s = &mut *s;
                tunnel::reconnect_dead(&mut s.tunnels, &mut s.tunnel_cooldowns, &config);
            }

            config
                .ports
                .iter()
                .map(|&port| {
                    let active = status::is_port_active(port);
                    let pid = s.tunnels.get(&port).map(|p| p.pid);
                    status::PortStatusInfo {
                        port,
                        status: if active {
                            PortStatus::Active
                        } else {
                            PortStatus::Inactive
                        },
                        pid,
                    }
                })
                .collect::<Vec<_>>()
        };

        let _ = app.emit("port-status-update", &statuses);
        update_tray_icon(&app, &statuses);
        if let Ok(menu) = build_tray_menu(&app, &statuses) {
            if let Some(tray) = app.tray_by_id("main") {
                let _ = tray.set_menu(Some(menu));
            }
        }
    }
}

fn update_tray_icon(app: &AppHandle, statuses: &[status::PortStatusInfo]) {
    if statuses.is_empty() {
        return;
    }
    let active = statuses
        .iter()
        .filter(|s| matches!(s.status, PortStatus::Active))
        .count();

    let icon_bytes: &[u8] = if active == statuses.len() {
        include_bytes!("../icons/tray-green.png")
    } else if active > 0 {
        include_bytes!("../icons/tray-orange.png")
    } else {
        include_bytes!("../icons/tray-red.png")
    };

    if let Some(tray) = app.tray_by_id("main") {
        if let Ok(image) = tauri::image::Image::from_bytes(icon_bytes) {
            let _ = tray.set_icon(Some(image));
        }
    }
}
