use serde::Serialize;
use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum PortStatus {
    /// Tunnel is up and remote service is accepting connections.
    Forwarding,
    /// Tunnel is up but the remote service is not running.
    RemoteDown,
    /// Tunnel died and the app is actively trying to reconnect.
    Reconnecting,
    /// Tunnel died and the app has stopped trying (auto-reconnect off or cooldown).
    TunnelDown,
    /// Not managed by Port Manager, but something else is listening on this port.
    PortInUse,
    /// Port is not being forwarded (user hasn't started it or explicitly stopped it).
    Stopped,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortStatusInfo {
    pub port: u16,
    pub status: PortStatus,
    pub pid: Option<u32>,
    /// PID of the process that owns the local listening socket (for PortInUse).
    pub owner_pid: Option<u32>,
    /// Name of the process that owns the local listening socket (for PortInUse).
    pub process_name: Option<String>,
}

/// Quick check: is anything listening on the local port?
/// Used by tunnel spawn logic to decide whether to skip a port.
pub fn is_local_port_bound(port: u16) -> bool {
    let addr = format!("127.0.0.1:{}", port);
    match addr.parse() {
        Ok(addr) => TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_ok(),
        Err(_) => false,
    }
}

/// Perform a deep probe of a port to determine its full status.
///
/// - `is_managed`: whether Port Manager has a tracked SSH tunnel process for this port.
///
/// When the port is managed and a local listener is found, this function attempts
/// a short read on the connection to distinguish between a healthy remote service
/// (connection stays open) and a dead remote service (SSH forwards the connect
/// but the remote sshd gets ECONNREFUSED and immediately closes the channel,
/// causing a reset/EOF on the local side).
///
/// Returns one of: Forwarding, RemoteDown, PortInUse, or Stopped.
/// The caller should use `resolve_status` to upgrade Stopped to
/// Reconnecting/TunnelDown based on intent.
pub fn probe_port(port: u16, is_managed: bool) -> PortStatus {
    let addr = match format!("127.0.0.1:{}", port).parse() {
        Ok(a) => a,
        Err(_) => {
            return PortStatus::Stopped;
        }
    };

    // Step 1: Try to connect locally (100ms timeout)
    let mut stream = match TcpStream::connect_timeout(&addr, Duration::from_millis(100)) {
        Ok(s) => s,
        Err(_) => {
            // Nothing listening on the local port
            return PortStatus::Stopped;
        }
    };

    // Something is listening locally
    if !is_managed {
        // We don't own this tunnel — something else is using the port
        return PortStatus::PortInUse;
    }

    // Step 2: For managed ports, probe whether the remote service is alive.
    let _ = stream.set_read_timeout(Some(Duration::from_millis(1500)));
    let mut buf = [0u8; 1];
    match stream.read(&mut buf) {
        Ok(0) => PortStatus::RemoteDown,
        Ok(_) => PortStatus::Forwarding,
        Err(ref e)
            if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut =>
        {
            PortStatus::Forwarding
        }
        Err(_) => PortStatus::RemoteDown,
    }
}

/// Resolve the final display status by combining the raw probe result with
/// the user's intent (is the port in managed_ports?) and reconnect state.
///
/// - `probe_result`: output of `probe_port()`
/// - `is_intended`: port is in `managed_ports` (user wants it forwarded)
/// - `will_reconnect`: auto-reconnect is on AND the port is not in cooldown
pub fn resolve_status(
    probe_result: PortStatus,
    is_intended: bool,
    will_reconnect: bool,
) -> PortStatus {
    match probe_result {
        // Tunnel is alive — these are definitive regardless of intent
        PortStatus::Forwarding | PortStatus::RemoteDown | PortStatus::PortInUse => probe_result,
        // Nothing listening / no tunnel process
        PortStatus::Stopped => {
            if !is_intended {
                PortStatus::Stopped
            } else if will_reconnect {
                PortStatus::Reconnecting
            } else {
                PortStatus::TunnelDown
            }
        }
        // These shouldn't come from probe_port, but handle gracefully
        PortStatus::Reconnecting | PortStatus::TunnelDown => probe_result,
    }
}

/// Look up which process is listening on a given local TCP port.
/// Returns `(pid, process_name)` if found.
///
/// Uses `netstat -ano` to find the listening PID, then `tasklist` to
/// resolve the PID to a process name.
#[cfg(windows)]
pub fn get_port_owner(port: u16) -> Option<(u32, String)> {
    use std::process::Command;

    // Run netstat to find the PID listening on this port
    let mut cmd = Command::new("netstat");
    cmd.args(["-ano", "-p", "TCP"]);
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let output = cmd.output().ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let target = format!("127.0.0.1:{}", port);
    let target_any = format!("0.0.0.0:{}", port);

    let mut owner_pid: Option<u32> = None;
    for line in stdout.lines() {
        // Lines look like:
        //   TCP    127.0.0.1:5432     0.0.0.0:0     LISTENING     1234
        //   TCP    0.0.0.0:5432       0.0.0.0:0     LISTENING     1234
        if !line.contains("LISTENING") {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }
        let local_addr = parts[1];
        if local_addr == target || local_addr == target_any {
            if let Ok(pid) = parts[4].parse::<u32>() {
                owner_pid = Some(pid);
                break;
            }
        }
    }

    let pid = owner_pid?;

    // Resolve PID to process name using tasklist
    let mut tl_cmd = Command::new("tasklist");
    tl_cmd.args(["/FI", &format!("PID eq {}", pid), "/FO", "CSV", "/NH"]);
    tl_cmd.creation_flags(0x08000000);
    let tl_output = tl_cmd.output().ok()?;

    let tl_stdout = String::from_utf8_lossy(&tl_output.stdout);
    // Output format: "process_name.exe","1234","Console","1","12,345 K"
    let first_line = tl_stdout.lines().next().unwrap_or("");
    let name = first_line
        .split(',')
        .next()
        .unwrap_or("")
        .trim_matches('"')
        .to_string();

    if name.is_empty() || name.starts_with("INFO:") {
        // tasklist returns "INFO: No tasks..." when process not found
        Some((pid, format!("PID {}", pid)))
    } else {
        Some((pid, name))
    }
}

#[cfg(not(windows))]
pub fn get_port_owner(_port: u16) -> Option<(u32, String)> {
    None
}

/// Kill a process by PID using `taskkill /F /PID`.
#[cfg(windows)]
pub fn kill_process(pid: u32) -> Result<(), String> {
    use std::process::Command;

    let mut cmd = Command::new("taskkill");
    cmd.args(["/F", "/PID", &pid.to_string()]);
    cmd.creation_flags(0x08000000);
    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run taskkill: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("taskkill failed: {}", stderr.trim()))
    }
}

#[cfg(not(windows))]
pub fn kill_process(_pid: u32) -> Result<(), String> {
    Err("kill_process is only supported on Windows".to_string())
}
