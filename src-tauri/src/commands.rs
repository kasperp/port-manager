use tauri::{AppHandle, Manager, State};

use crate::config::{save_config, scan_ssh_config, Config, Profile, SshHostEntry};
use crate::state::SharedState;
use crate::status::{get_port_owner, probe_port, resolve_status, PortStatusInfo};
use crate::tunnel;

// ---- Config Commands ----

#[tauri::command]
pub fn get_config(state: State<SharedState>) -> Config {
    state.lock().unwrap().config.clone()
}

#[tauri::command]
pub fn save_profile_settings(
    app: AppHandle,
    state: State<SharedState>,
    host: String,
    user: String,
    ssh_port: u16,
    rate_limit_max: u32,
    rate_limit_window_secs: u32,
) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    {
        let profile = s.config.active_profile_mut();
        profile.host = host;
        profile.user = user;
        profile.ssh_port = ssh_port;
        profile.rate_limit_max = rate_limit_max;
        profile.rate_limit_window_secs = rate_limit_window_secs;
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
    s.managed_ports.remove(&port);
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
    {
        let s = &mut *s;
        tunnel::stop_all(&mut s.tunnels, &mut s.managed_ports);
    }
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
        rate_limit_max: 6,
        rate_limit_window_secs: 30,
    };
    s.config.profiles.push(profile);

    // Stop tunnels from the current profile and switch to the new one
    {
        let s = &mut *s;
        tunnel::stop_all(&mut s.tunnels, &mut s.managed_ports);
    }
    s.tunnel_cooldowns.clear();
    s.config.active_profile = name;

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
        {
            let s = &mut *s;
            tunnel::stop_all(&mut s.tunnels, &mut s.managed_ports);
        }
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
        name: profile_name.clone(),
        host: entry.hostname.clone(),
        user: entry.user.clone(),
        ssh_port: entry.port,
        ports: Vec::new(),
        rate_limit_max: 6,
        rate_limit_window_secs: 30,
    };
    s.config.profiles.push(profile);

    // Stop tunnels from the current profile and switch to the imported one
    {
        let s = &mut *s;
        tunnel::stop_all(&mut s.tunnels, &mut s.managed_ports);
    }
    s.tunnel_cooldowns.clear();
    s.config.active_profile = profile_name;

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    save_config(&data_dir, &s.config)?;

    Ok(s.config.clone())
}

// ---- Tunnel Control Commands ----

#[tauri::command]
pub fn start_all(state: State<SharedState>) -> Vec<String> {
    let mut s = state.lock().unwrap();
    let profile = s.config.active_profile().clone();
    let profile_name = profile.name.clone();
    let s = &mut *s;
    let attempts = s.connection_attempts.entry(profile_name).or_default();
    tunnel::start_all(&mut s.tunnels, &mut s.managed_ports, &profile, attempts)
}

#[tauri::command]
pub fn stop_all(state: State<SharedState>) {
    let mut s = state.lock().unwrap();
    let s = &mut *s;
    tunnel::stop_all(&mut s.tunnels, &mut s.managed_ports);
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
    let auto_reconnect = s.auto_reconnect;
    let port_info: Vec<(u16, bool, Option<u32>, bool, bool)> = profile
        .ports
        .iter()
        .map(|&port| {
            let has_tunnel = s.tunnels.contains_key(&port);
            let pid = s.tunnels.get(&port).map(|p| p.pid);
            let is_intended = s.managed_ports.contains(&port);
            // Will reconnect if auto-reconnect is on and port is not in cooldown
            let in_cooldown = s.tunnel_cooldowns.contains_key(&port);
            let will_reconnect = auto_reconnect && !in_cooldown;
            (port, has_tunnel, pid, is_intended, will_reconnect)
        })
        .collect();
    drop(s); // Release lock before probing

    port_info
        .into_iter()
        .map(|(port, has_tunnel, pid, is_intended, will_reconnect)| {
            let raw_status = probe_port(port, has_tunnel);
            let port_status = resolve_status(raw_status, is_intended, will_reconnect);
            let (owner_pid, process_name) = if port_status == crate::status::PortStatus::PortInUse {
                match get_port_owner(port) {
                    Some((op, name)) => (Some(op), Some(name)),
                    None => (None, None),
                }
            } else {
                (None, None)
            };
            PortStatusInfo {
                port,
                status: port_status,
                pid,
                owner_pid,
                process_name,
            }
        })
        .collect()
}

#[tauri::command]
pub fn kill_port_process(port: u16) -> Result<(), String> {
    let (pid, _name) = crate::status::get_port_owner(port)
        .ok_or_else(|| format!("No process found listening on port {}", port))?;
    crate::status::kill_process(pid)
}

#[tauri::command]
pub fn start_port(state: State<SharedState>, port: u16) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    let profile = s.config.active_profile().clone();

    if profile.host.is_empty() || profile.user.is_empty() {
        return Err("Profile has no host/user configured".to_string());
    }
    if !profile.ports.contains(&port) {
        return Err(format!("Port {} is not in the active profile", port));
    }
    if s.tunnels.contains_key(&port) {
        return Err(format!("Port {} is already managed", port));
    }
    if crate::status::is_local_port_bound(port) {
        return Err(format!(
            "Port {} is already in use by another process",
            port
        ));
    }

    let profile_name = profile.name.clone();
    let s = &mut *s;
    let attempts = s.connection_attempts.entry(profile_name).or_default();

    if !tunnel::can_connect(
        attempts,
        profile.rate_limit_max,
        profile.rate_limit_window_secs,
    ) {
        return Err("Rate limit reached — try again shortly".to_string());
    }
    tunnel::record_attempt(attempts);

    match tunnel::spawn_tunnel(port, &profile) {
        Ok(proc) => {
            s.tunnels.insert(port, proc);
            s.managed_ports.insert(port);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub fn stop_port(state: State<SharedState>, port: u16) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    s.managed_ports.remove(&port);
    match s.tunnels.remove(&port) {
        Some(mut proc) => {
            let _ = proc.child.kill();
            Ok(())
        }
        None => Ok(()), // Port wasn't running, but we've unmanaged it
    }
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
pub fn set_startup_enabled(enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;

        fn autostart_entry_path() -> Result<std::path::PathBuf, String> {
            let config_dir = dirs::config_dir()
                .ok_or_else(|| "Cannot resolve Linux config directory".to_string())?;
            Ok(config_dir.join("autostart").join("port-manager.desktop"))
        }

        fn escape_exec_arg(arg: &str) -> String {
            arg.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace(' ', "\\ ")
        }

        let entry_path = autostart_entry_path()?;

        if enabled {
            let autostart_dir = entry_path
                .parent()
                .ok_or_else(|| "Invalid autostart directory".to_string())?;
            fs::create_dir_all(autostart_dir).map_err(|e| e.to_string())?;

            let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
            let exec = escape_exec_arg(&exe_path.to_string_lossy());

            let desktop_entry = format!(
                "[Desktop Entry]\nType=Application\nVersion=1.0\nName=Port Manager\nComment=Keep SSH port forwards alive\nExec={}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
                exec
            );
            fs::write(&entry_path, desktop_entry).map_err(|e| e.to_string())?;
            return Ok(());
        }

        if entry_path.exists() {
            fs::remove_file(&entry_path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = enabled;
        Err("Startup registration is only supported on Windows and Linux".to_string())
    }
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
    #[cfg(target_os = "linux")]
    {
        let Some(config_dir) = dirs::config_dir() else {
            return false;
        };
        return config_dir
            .join("autostart")
            .join("port-manager.desktop")
            .exists();
    }

    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}
