use std::collections::{HashMap, HashSet, VecDeque};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::config::{PortlessGateway, Profile, ProfileMode};
use crate::status::is_local_port_bound;

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
    spawn_forward(port, port, profile)
}

/// Spawn an SSH tunnel from one local port to one remote port.
pub fn spawn_forward(
    local_port: u16,
    remote_port: u16,
    profile: &Profile,
) -> Result<TunnelProcess, String> {
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
        &format!("127.0.0.1:{local_port}:127.0.0.1:{remote_port}"),
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

/// Spawn one Portless gateway tunnel.
pub fn spawn_portless_gateway(
    gateway: &PortlessGateway,
    profile: &Profile,
) -> Result<TunnelProcess, String> {
    spawn_forward(gateway.local_port, gateway.remote_port, profile)
}

/// Start tunnels for all ports not already active, respecting the rate limit.
/// Returns any spawn errors. Ports that could not be started due to the rate
/// limit will be picked up by subsequent background-loop iterations.
/// All profile ports are added to `managed_ports` to indicate intent.
pub fn start_all(
    tunnels: &mut HashMap<u16, TunnelProcess>,
    managed_ports: &mut HashSet<u16>,
    profile: &Profile,
    attempts: &mut VecDeque<Instant>,
) -> Vec<String> {
    let mut errors = Vec::new();
    if profile.host.is_empty() || profile.user.is_empty() {
        return errors;
    }
    if profile.mode == ProfileMode::Portless {
        for gateway in &profile.portless.gateways {
            let local_port = gateway.local_port;
            managed_ports.insert(local_port);

            if is_local_port_bound(local_port) && !tunnels.contains_key(&local_port) {
                errors.push(format!(
                    "{}: port {local_port} is already in use by another process",
                    gateway.name
                ));
                continue;
            }
            if tunnels.contains_key(&local_port) {
                continue;
            }
            if !can_connect(
                attempts,
                profile.rate_limit_max,
                profile.rate_limit_window_secs,
            ) {
                break;
            }

            record_attempt(attempts);
            match spawn_portless_gateway(gateway, profile) {
                Ok(proc) => {
                    tunnels.insert(local_port, proc);
                }
                Err(e) => errors.push(format!("{}: {e}", gateway.name)),
            }
        }
        return errors;
    }

    // Mark all profile ports as intended-to-forward
    for entry in &profile.ports {
        managed_ports.insert(entry.port);
    }
    for port in profile.ports.iter().map(|entry| entry.port) {
        if !is_local_port_bound(port) && !tunnels.contains_key(&port) {
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

/// Kill all tracked SSH processes and clear managed intent.
pub fn stop_all(tunnels: &mut HashMap<u16, TunnelProcess>, managed_ports: &mut HashSet<u16>) {
    for (_, proc) in tunnels.iter_mut() {
        let _ = proc.child.kill();
    }
    tunnels.clear();
    managed_ports.clear();
}

const RECONNECT_COOLDOWN_SECS: u64 = 60;

/// Remove dead tunnel entries and restart any missing managed ports.
/// Ports that fail are put in a cooldown to avoid rapid respawn loops.
/// Respawns also respect the per-profile rate limit.
/// Only ports in `managed_ports` are eligible for reconnection.
pub fn reconnect_dead(
    tunnels: &mut HashMap<u16, TunnelProcess>,
    cooldowns: &mut HashMap<u16, Instant>,
    managed_ports: &HashSet<u16>,
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

    if profile.mode == ProfileMode::Portless {
        for gateway in &profile.portless.gateways {
            let local_port = gateway.local_port;
            if !managed_ports.contains(&local_port) {
                continue;
            }
            if is_local_port_bound(local_port) || tunnels.contains_key(&local_port) {
                continue;
            }
            if let Some(&failed_at) = cooldowns.get(&local_port) {
                if now.duration_since(failed_at).as_secs() < RECONNECT_COOLDOWN_SECS {
                    continue;
                }
            }
            if !can_connect(
                attempts,
                profile.rate_limit_max,
                profile.rate_limit_window_secs,
            ) {
                break;
            }
            record_attempt(attempts);
            if let Ok(proc) = spawn_portless_gateway(gateway, profile) {
                cooldowns.remove(&local_port);
                tunnels.insert(local_port, proc);
            }
        }
        return;
    }

    // Restart missing ports that the user intended to be forwarded
    for port in profile.ports.iter().map(|entry| entry.port) {
        if !managed_ports.contains(&port) {
            continue;
        }
        if is_local_port_bound(port) || tunnels.contains_key(&port) {
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
