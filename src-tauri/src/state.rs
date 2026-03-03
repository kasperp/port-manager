use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::config::Config;
use crate::tunnel::TunnelProcess;

pub struct AppState {
    pub config: Config,
    pub tunnels: HashMap<u16, TunnelProcess>,
    pub auto_reconnect: bool,
    pub tunnel_cooldowns: HashMap<u16, Instant>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            tunnels: HashMap::new(),
            auto_reconnect: true,
            tunnel_cooldowns: HashMap::new(),
        }
    }
}

pub type SharedState = Arc<Mutex<AppState>>;
