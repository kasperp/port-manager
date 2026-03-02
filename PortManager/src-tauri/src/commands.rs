use tauri::{AppHandle, Manager, State};

use crate::config::{save_config, Config};
use crate::state::SharedState;
use crate::status::{is_port_active, PortStatus, PortStatusInfo};
use crate::tunnel;

// ---- Config Commands ----

#[tauri::command]
pub fn get_config(state: State<SharedState>) -> Config {
    state.lock().unwrap().config.clone()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<SharedState>,
    host: String,
    user: String,
    ssh_port: u16,
) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    s.config.host = host;
    s.config.user = user;
    s.config.ssh_port = ssh_port;
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    save_config(&data_dir, &s.config)
}

#[tauri::command]
pub fn add_port(
    app: AppHandle,
    state: State<SharedState>,
    port: u16,
) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    if s.config.ports.contains(&port) {
        return Err(format!("Port {port} already exists"));
    }
    s.config.ports.push(port);
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    save_config(&data_dir, &s.config)
}

#[tauri::command]
pub fn remove_port(
    app: AppHandle,
    state: State<SharedState>,
    port: u16,
) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    if let Some(mut proc) = s.tunnels.remove(&port) {
        let _ = proc.child.kill();
    }
    s.config.ports.retain(|&p| p != port);
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    save_config(&data_dir, &s.config)
}

// ---- Tunnel Control Commands ----

#[tauri::command]
pub fn start_all(state: State<SharedState>) -> Vec<String> {
    let mut s = state.lock().unwrap();
    let config = s.config.clone();
    tunnel::start_all(&mut s.tunnels, &config)
}

#[tauri::command]
pub fn stop_all(state: State<SharedState>) {
    let mut s = state.lock().unwrap();
    tunnel::stop_all(&mut s.tunnels);
}

#[tauri::command]
pub fn set_auto_reconnect(state: State<SharedState>, enabled: bool) {
    state.lock().unwrap().auto_reconnect = enabled;
}

// ---- Status Query ----

#[tauri::command]
pub fn get_port_statuses(state: State<SharedState>) -> Vec<PortStatusInfo> {
    let s = state.lock().unwrap();
    s.config
        .ports
        .iter()
        .map(|&port| {
            let active = is_port_active(port);
            let pid = s.tunnels.get(&port).map(|p| p.pid);
            PortStatusInfo {
                port,
                status: if active {
                    PortStatus::Active
                } else {
                    PortStatus::Inactive
                },
                pid,
            }
        })
        .collect()
}

// ---- Windows Startup Registry ----

#[tauri::command]
pub fn set_startup_enabled(enabled: bool) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let run_key = hkcu
        .open_subkey_with_flags(path, KEY_WRITE)
        .map_err(|e| e.to_string())?;

    if enabled {
        let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
        run_key
            .set_value("PortManager", &exe_path.to_string_lossy().as_ref())
            .map_err(|e| e.to_string())?;
    } else {
        let _ = run_key.delete_value("PortManager");
    }
    Ok(())
}

#[tauri::command]
pub fn get_startup_enabled() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"Software\Microsoft\Windows\CurrentVersion\Run";
    if let Ok(run_key) = hkcu.open_subkey_with_flags(path, KEY_READ) {
        run_key.get_value::<String, _>("PortManager").is_ok()
    } else {
        false
    }
}
