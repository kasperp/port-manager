use serde::Serialize;
use std::net::TcpStream;
use std::time::Duration;

/// Check if a port has something listening on it.
/// Uses TcpStream::connect_timeout which is fast (100ms max) and has no race condition.
pub fn is_port_active(port: u16) -> bool {
    let addr = format!("127.0.0.1:{}", port);
    match addr.parse() {
        Ok(addr) => TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_ok(),
        Err(_) => false,
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum PortStatus {
    Active,
    Inactive,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortStatusInfo {
    pub port: u16,
    pub status: PortStatus,
    pub pid: Option<u32>,
}
