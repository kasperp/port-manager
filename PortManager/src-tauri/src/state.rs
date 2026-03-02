use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::tunnel::TunnelProcess;

pub struct AppState {
    pub config: Config,
    pub tunnels: HashMap<u16, TunnelProcess>,
    pub auto_reconnect: bool,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            tunnels: HashMap::new(),
            auto_reconnect: true,
        }
    }
}

pub type SharedState = Arc<Mutex<AppState>>;
