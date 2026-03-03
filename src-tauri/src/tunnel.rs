use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::time::Instant;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::config::Config;
use crate::status::is_port_active;

pub struct TunnelProcess {
    pub pid: u32,
    pub child: Child,
}

/// Spawn an SSH tunnel for a single port.
pub fn spawn_tunnel(port: u16, config: &Config) -> Result<TunnelProcess, String> {
    let mut cmd = Command::new("ssh");
    cmd.args([
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
    .stderr(Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let child = cmd.spawn().map_err(|e| format!("Failed to spawn ssh: {e}"))?;

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

const RECONNECT_COOLDOWN_SECS: u64 = 60;

/// Remove dead tunnel entries and restart any missing ports.
/// Ports that fail are put in a cooldown to avoid rapid respawn loops.
pub fn reconnect_dead(
    tunnels: &mut HashMap<u16, TunnelProcess>,
    cooldowns: &mut HashMap<u16, Instant>,
    config: &Config,
) {
    if config.host.is_empty() || config.user.is_empty() {
        return;
    }
    let now = Instant::now();
    // Remove processes that have exited, recording their failure time
    tunnels.retain(|port, proc| {
        let still_running = proc
            .child
            .try_wait()
            .map(|status| status.is_none())
            .unwrap_or(false);
        if !still_running {
            cooldowns.insert(*port, now);
        }
        still_running
    });
    // Restart missing ports, respecting cooldown
    for &port in &config.ports {
        if is_port_active(port) || tunnels.contains_key(&port) {
            continue;
        }
        if let Some(&failed_at) = cooldowns.get(&port) {
            if now.duration_since(failed_at).as_secs() < RECONNECT_COOLDOWN_SECS {
                continue;
            }
        }
        if let Ok(proc) = spawn_tunnel(port, config) {
            cooldowns.remove(&port);
            tunnels.insert(port, proc);
        }
    }
}
