use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::config::Config;
use crate::tunnel::TunnelProcess;

pub struct AppState {
    pub config: Config,
    pub tunnels: HashMap<u16, TunnelProcess>,
    pub auto_reconnect: bool,
    pub tunnel_cooldowns: HashMap<u16, Instant>,
    /// Timestamps of recent SSH connection attempts, keyed by profile name.
    /// Used for rate-limiting connection spawns to avoid triggering firewall rules.
    pub connection_attempts: HashMap<String, VecDeque<Instant>>,
    /// Ports the user intends to be forwarded (started via Start All or Start Port).
    /// Used to distinguish "Stopped" (not in set) from "TunnelDown/Reconnecting" (in set but tunnel died).
    pub managed_ports: HashSet<u16>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            tunnels: HashMap::new(),
            auto_reconnect: true,
            tunnel_cooldowns: HashMap::new(),
            connection_attempts: HashMap::new(),
            managed_ports: HashSet::new(),
        }
    }
}

pub type SharedState = Arc<Mutex<AppState>>;
