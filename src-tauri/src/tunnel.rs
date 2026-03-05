use std::collections::{HashMap, VecDeque};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::config::Profile;
use crate::status::is_port_active;

pub struct TunnelProcess {
    pub pid: u32,
    pub child: Child,
}

/// Prune connection attempts older than the rate limit window, then check
/// whether a new connection is allowed.
pub fn can_connect(attempts: &mut VecDeque<Instant>, max: u32, window_secs: u32) -> bool {
    let now = Instant::now();
    let window = Duration::from_secs(window_secs as u64);
    // Remove entries that have fallen outside the sliding window
    while let Some(&front) = attempts.front() {
        if now.duration_since(front) >= window {
            attempts.pop_front();
        } else {
            break;
        }
    }
    attempts.len() < max as usize
}

/// Record a connection attempt timestamp.
pub fn record_attempt(attempts: &mut VecDeque<Instant>) {
    attempts.push_back(Instant::now());
}

/// Spawn an SSH tunnel for a single port.
pub fn spawn_tunnel(port: u16, profile: &Profile) -> Result<TunnelProcess, String> {
    let mut cmd = Command::new("ssh");
    cmd.args([
        "-N",
        "-p",
        &profile.ssh_port.to_string(),
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
        &format!("{}@{}", profile.user, profile.host),
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn ssh: {e}"))?;

    let pid = child.id();
    Ok(TunnelProcess { pid, child })
}

/// Start tunnels for all ports not already active, respecting the rate limit.
/// Returns any spawn errors. Ports that could not be started due to the rate
/// limit will be picked up by subsequent background-loop iterations.
pub fn start_all(
    tunnels: &mut HashMap<u16, TunnelProcess>,
    profile: &Profile,
    attempts: &mut VecDeque<Instant>,
) -> Vec<String> {
    let mut errors = Vec::new();
    if profile.host.is_empty() || profile.user.is_empty() {
        return errors;
    }
    for &port in &profile.ports {
        if !is_port_active(port) && !tunnels.contains_key(&port) {
            if !can_connect(
                attempts,
                profile.rate_limit_max,
                profile.rate_limit_window_secs,
            ) {
                // Rate limit reached — remaining ports will be retried on the next tick
                break;
            }
            record_attempt(attempts);
            match spawn_tunnel(port, profile) {
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
/// Respawns also respect the per-profile rate limit.
pub fn reconnect_dead(
    tunnels: &mut HashMap<u16, TunnelProcess>,
    cooldowns: &mut HashMap<u16, Instant>,
    profile: &Profile,
    attempts: &mut VecDeque<Instant>,
) {
    if profile.host.is_empty() || profile.user.is_empty() {
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
    for &port in &profile.ports {
        if is_port_active(port) || tunnels.contains_key(&port) {
            continue;
        }
        if let Some(&failed_at) = cooldowns.get(&port) {
            if now.duration_since(failed_at).as_secs() < RECONNECT_COOLDOWN_SECS {
                continue;
            }
        }
        if !can_connect(
            attempts,
            profile.rate_limit_max,
            profile.rate_limit_window_secs,
        ) {
            // Rate limit reached — remaining ports will be retried on the next tick
            break;
        }
        record_attempt(attempts);
        if let Ok(proc) = spawn_tunnel(port, profile) {
            cooldowns.remove(&port);
            tunnels.insert(port, proc);
        }
    }
}
