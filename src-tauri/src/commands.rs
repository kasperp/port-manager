use tauri::{AppHandle, Manager, State};

use crate::config::{save_config, scan_ssh_config, Config, Profile, SshHostEntry};
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
    {
        let profile = s.config.active_profile_mut();
        profile.host = host;
        profile.user = user;
        profile.ssh_port = ssh_port;
    }
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    save_config(&data_dir, &s.config)
}

#[tauri::command]
pub fn add_port(app: AppHandle, state: State<SharedState>, port: u16) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    {
        let profile = s.config.active_profile_mut();
        if profile.ports.contains(&port) {
            return Err(format!("Port {port} already exists"));
        }
        profile.ports.push(port);
    }
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    save_config(&data_dir, &s.config)
}

#[tauri::command]
pub fn remove_port(app: AppHandle, state: State<SharedState>, port: u16) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    if let Some(mut proc) = s.tunnels.remove(&port) {
        let _ = proc.child.kill();
    }
    {
        let profile = s.config.active_profile_mut();
        profile.ports.retain(|&p| p != port);
    }
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    save_config(&data_dir, &s.config)
}

// ---- Profile Commands ----

#[tauri::command]
pub fn switch_profile(
    app: AppHandle,
    state: State<SharedState>,
    name: String,
) -> Result<Config, String> {
    let mut s = state.lock().unwrap();

    // Verify profile exists
    if !s.config.profiles.iter().any(|p| p.name == name) {
        return Err(format!("Profile '{}' not found", name));
    }

    // Stop all tunnels for the current profile
    tunnel::stop_all(&mut s.tunnels);
    s.tunnel_cooldowns.clear();

    // Switch active profile
    s.config.active_profile = name;

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    save_config(&data_dir, &s.config)?;

    Ok(s.config.clone())
}

#[tauri::command]
pub fn create_profile(
    app: AppHandle,
    state: State<SharedState>,
    name: String,
    host: String,
    user: String,
    ssh_port: u16,
) -> Result<Config, String> {
    let mut s = state.lock().unwrap();

    // Check for duplicate name
    if s.config.profiles.iter().any(|p| p.name == name) {
        return Err(format!("Profile '{}' already exists", name));
    }

    let profile = Profile {
        name: name.clone(),
        host,
        user,
        ssh_port,
        ports: Vec::new(),
    };
    s.config.profiles.push(profile);

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    save_config(&data_dir, &s.config)?;

    Ok(s.config.clone())
}

#[tauri::command]
pub fn delete_profile(
    app: AppHandle,
    state: State<SharedState>,
    name: String,
) -> Result<Config, String> {
    let mut s = state.lock().unwrap();

    // Prevent deleting the last profile
    if s.config.profiles.len() <= 1 {
        return Err("Cannot delete the last profile".to_string());
    }

    // If deleting the active profile, stop tunnels and switch to another
    if s.config.active_profile == name {
        tunnel::stop_all(&mut s.tunnels);
        s.tunnel_cooldowns.clear();
    }

    s.config.profiles.retain(|p| p.name != name);

    // If we deleted the active profile, switch to the first remaining one
    if !s
        .config
        .profiles
        .iter()
        .any(|p| p.name == s.config.active_profile)
    {
        s.config.active_profile = s.config.profiles[0].name.clone();
    }

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    save_config(&data_dir, &s.config)?;

    Ok(s.config.clone())
}

#[tauri::command]
pub fn get_ssh_hosts() -> Vec<SshHostEntry> {
    scan_ssh_config()
}

#[tauri::command]
pub fn import_ssh_profile(
    app: AppHandle,
    state: State<SharedState>,
    ssh_host_name: String,
) -> Result<Config, String> {
    let hosts = scan_ssh_config();
    let entry = hosts
        .iter()
        .find(|h| h.name == ssh_host_name)
        .ok_or_else(|| format!("SSH host '{}' not found", ssh_host_name))?;

    let mut s = state.lock().unwrap();

    // Use the SSH host alias as the profile name; avoid duplicates
    let mut profile_name = entry.name.clone();
    let mut counter = 1;
    while s.config.profiles.iter().any(|p| p.name == profile_name) {
        counter += 1;
        profile_name = format!("{} ({})", entry.name, counter);
    }

    let profile = Profile {
        name: profile_name,
        host: entry.hostname.clone(),
        user: entry.user.clone(),
        ssh_port: entry.port,
        ports: Vec::new(),
    };
    s.config.profiles.push(profile);

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    save_config(&data_dir, &s.config)?;

    Ok(s.config.clone())
}

// ---- Tunnel Control Commands ----

#[tauri::command]
pub fn start_all(state: State<SharedState>) -> Vec<String> {
    let mut s = state.lock().unwrap();
    let profile = s.config.active_profile().clone();
    tunnel::start_all(&mut s.tunnels, &profile)
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
    let profile = s.config.active_profile();
    profile
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

#[cfg(windows)]
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

#[cfg(not(windows))]
#[tauri::command]
pub fn set_startup_enabled(_enabled: bool) -> Result<(), String> {
    Err("Startup registration is only supported on Windows".to_string())
}

#[cfg(windows)]
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

#[cfg(not(windows))]
#[tauri::command]
pub fn get_startup_enabled() -> bool {
    false
}
