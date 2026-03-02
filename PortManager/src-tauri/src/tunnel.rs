use std::collections::HashMap;
use std::process::{Child, Command, Stdio};

use crate::config::Config;
use crate::status::is_port_active;

pub struct TunnelProcess {
    pub pid: u32,
    pub child: Child,
}

/// Spawn an SSH tunnel for a single port.
pub fn spawn_tunnel(port: u16, config: &Config) -> Result<TunnelProcess, String> {
    let child = Command::new("ssh")
        .args([
            "-N",
            "-p",
            &config.ssh_port.to_string(),
            "-o",
            "ServerAliveInterval=30",
            "-o",
            "ServerAliveCountMax=3",
            "-o",
            "ExitOnForwardFailure=yes",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-L",
            &format!("127.0.0.1:{port}:127.0.0.1:{port}"),
            &format!("{}@{}", config.user, config.host),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn ssh: {e}"))?;

    let pid = child.id();
    Ok(TunnelProcess { pid, child })
}

/// Start tunnels for all ports not already active.
pub fn start_all(tunnels: &mut HashMap<u16, TunnelProcess>, config: &Config) -> Vec<String> {
    let mut errors = Vec::new();
    if config.host.is_empty() || config.user.is_empty() {
        return errors;
    }
    for &port in &config.ports {
        if !is_port_active(port) && !tunnels.contains_key(&port) {
            match spawn_tunnel(port, config) {
                Ok(proc) => {
                    tunnels.insert(port, proc);
                }
                Err(e) => errors.push(e),
            }
        }
    }
    errors
}

/// Kill all tracked SSH processes.
pub fn stop_all(tunnels: &mut HashMap<u16, TunnelProcess>) {
    for (_, proc) in tunnels.iter_mut() {
        let _ = proc.child.kill();
    }
    tunnels.clear();
}

/// Remove dead tunnel entries and restart any missing ports.
pub fn reconnect_dead(tunnels: &mut HashMap<u16, TunnelProcess>, config: &Config) {
    if config.host.is_empty() || config.user.is_empty() {
        return;
    }
    // Remove processes that have exited
    tunnels.retain(|_, proc| {
        proc.child
            .try_wait()
            .map(|status| status.is_none())
            .unwrap_or(false)
    });
    // Restart missing ports
    for &port in &config.ports {
        if !is_port_active(port) && !tunnels.contains_key(&port) {
            if let Ok(proc) = spawn_tunnel(port, config) {
                tunnels.insert(port, proc);
            }
        }
    }
}
