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

            // Start SSH tunnels on launch using the active profile
            {
                let mut s = shared.lock().unwrap();
                let profile = s.config.active_profile().clone();
                let profile_name = profile.name.clone();
                let s = &mut *s;
                let attempts = s
                    .connection_attempts
                    .entry(profile_name)
                    .or_default();
                tunnel::start_all(&mut s.tunnels, &mut s.managed_ports, &profile, attempts);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_profile_settings,
            commands::add_port,
            commands::remove_port,
            commands::start_all,
            commands::stop_all,
            commands::start_port,
            commands::stop_port,
            commands::set_auto_reconnect,
            commands::get_port_statuses,
            commands::kill_port_process,
            commands::set_startup_enabled,
            commands::get_startup_enabled,
            commands::switch_profile,
            commands::create_profile,
            commands::delete_profile,
            commands::get_ssh_hosts,
            commands::import_ssh_profile,
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
    profile_name: &str,
) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(app, "show", "Show Port Manager", true, None::<&str>)?;
    let profile_label = MenuItem::with_id(
        app,
        "profile-label",
        format!("Profile: {}", profile_name),
        false,
        None::<&str>,
    )?;
    let sep1 = PredefinedMenuItem::separator(app)?;

    let port_items: Vec<MenuItem<tauri::Wry>> = statuses
        .iter()
        .map(|s| {
            let (dot, label_text) = match s.status {
                PortStatus::Forwarding => ("●", "Forwarding"),
                PortStatus::RemoteDown => ("●", "Not Listening"),
                PortStatus::Reconnecting => ("●", "Reconnecting"),
                PortStatus::TunnelDown => ("●", "Tunnel Down"),
                PortStatus::PortInUse => ("●", "Port In Use"),
                PortStatus::Stopped => ("○", "Stopped"),
            };
            let label = format!("{}  :{}  —  {}", dot, s.port, label_text);
            MenuItem::with_id(app, format!("ps-{}", s.port), label, false, None::<&str>)
        })
        .collect::<tauri::Result<_>>()?;

    let sep_ports = PredefinedMenuItem::separator(app)?;
    let start = MenuItem::with_id(app, "start", "▶  Start All", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "■  Stop All", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let mut all: Vec<&dyn IsMenuItem<tauri::Wry>> = vec![&show, &profile_label, &sep1];
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
    let menu = build_tray_menu(app, &[], "Default")?;

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
                let profile = s.config.active_profile().clone();
                let profile_name = profile.name.clone();
                let s = &mut *s;
                let attempts = s
                    .connection_attempts
                    .entry(profile_name)
                    .or_default();
                tunnel::start_all(&mut s.tunnels, &mut s.managed_ports, &profile, attempts);
            }
            "stop" => {
                let mut s = state_menu.lock().unwrap();
                let s = &mut *s;
                tunnel::stop_all(&mut s.tunnels, &mut s.managed_ports);
            }
            "quit" => {
                {
                    let mut s = state_menu.lock().unwrap();
                    let s = &mut *s;
                    tunnel::stop_all(&mut s.tunnels, &mut s.managed_ports);
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

        // Phase 1: Lock state, reconnect dead tunnels, collect port info, release lock
        let (port_info, profile_name) = {
            let mut s = state.lock().unwrap();
            let auto = s.auto_reconnect;
            let profile = s.config.active_profile().clone();

            if auto {
                let s = &mut *s;
                let profile_name = profile.name.clone();
                let attempts = s
                    .connection_attempts
                    .entry(profile_name)
                    .or_default();
                tunnel::reconnect_dead(
                    &mut s.tunnels,
                    &mut s.tunnel_cooldowns,
                    &s.managed_ports,
                    &profile,
                    attempts,
                );
            }

            // Collect port info tuples while we hold the lock
            // (port, has_tunnel, pid, is_intended, will_reconnect)
            let info: Vec<(u16, bool, Option<u32>, bool, bool)> = profile
                .ports
                .iter()
                .map(|&port| {
                    let has_tunnel = s.tunnels.contains_key(&port);
                    let pid = s.tunnels.get(&port).map(|p| p.pid);
                    let is_intended = s.managed_ports.contains(&port);
                    let in_cooldown = s.tunnel_cooldowns.contains_key(&port);
                    let will_reconnect = auto && !in_cooldown;
                    (port, has_tunnel, pid, is_intended, will_reconnect)
                })
                .collect();

            let name = profile.name.clone();
            (info, name)
        };
        // Lock is released here

        // Phase 2: Probe all ports in parallel OUTSIDE the mutex
        let probe_handles: Vec<_> = port_info
            .into_iter()
            .map(|(port, has_tunnel, pid, is_intended, will_reconnect)| {
                let handle = tokio::task::spawn_blocking(move || {
                    let raw_status = status::probe_port(port, has_tunnel);
                    let port_status = status::resolve_status(raw_status, is_intended, will_reconnect);
                    let (owner_pid, process_name) = if port_status == status::PortStatus::PortInUse {
                        match status::get_port_owner(port) {
                            Some((op, name)) => (Some(op), Some(name)),
                            None => (None, None),
                        }
                    } else {
                        (None, None)
                    };
                    status::PortStatusInfo { port, status: port_status, pid, owner_pid, process_name }
                });
                handle
            })
            .collect();

        let mut statuses = Vec::with_capacity(probe_handles.len());
        for handle in probe_handles {
            if let Ok(info) = handle.await {
                statuses.push(info);
            }
        }

        let _ = app.emit("port-status-update", &statuses);
        update_tray_icon(&app, &statuses);
        if let Ok(menu) = build_tray_menu(&app, &statuses, &profile_name) {
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
    let forwarding = statuses
        .iter()
        .filter(|s| matches!(s.status, PortStatus::Forwarding))
        .count();

    let icon_bytes: &[u8] = if forwarding == statuses.len() {
        include_bytes!("../icons/tray-green.png")
    } else if forwarding > 0 {
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
