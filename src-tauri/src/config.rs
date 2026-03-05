use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::BufRead;
use std::path::PathBuf;

/// Default: 6 connections (matches ufw LIMIT default).
fn default_rate_limit_max() -> u32 {
    6
}

/// Default: 30-second window (matches ufw LIMIT default).
fn default_rate_limit_window_secs() -> u32 {
    30
}

/// A single connection profile with its own host, user, SSH port, and forwarded ports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub host: String,
    pub user: String,
    pub ssh_port: u16,
    pub ports: Vec<u16>,
    /// Maximum number of SSH connection attempts allowed within the rate limit window.
    #[serde(default = "default_rate_limit_max")]
    pub rate_limit_max: u32,
    /// Duration of the rate limit sliding window in seconds.
    #[serde(default = "default_rate_limit_window_secs")]
    pub rate_limit_window_secs: u32,
}

impl Profile {
    pub fn new(name: String) -> Self {
        Self {
            name,
            host: String::new(),
            user: String::new(),
            ssh_port: 22,
            ports: Vec::new(),
            rate_limit_max: default_rate_limit_max(),
            rate_limit_window_secs: default_rate_limit_window_secs(),
        }
    }
}

/// Top-level application configuration containing multiple profiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub active_profile: String,
    pub profiles: Vec<Profile>,
}

impl Default for Config {
    fn default() -> Self {
        let default_profile = Profile::new("Default".to_string());
        Self {
            active_profile: "Default".to_string(),
            profiles: vec![default_profile],
        }
    }
}

impl Config {
    /// Get a reference to the active profile, falling back to the first profile.
    pub fn active_profile(&self) -> &Profile {
        self.profiles
            .iter()
            .find(|p| p.name == self.active_profile)
            .or_else(|| self.profiles.first())
            .expect("Config must always have at least one profile")
    }

    /// Get a mutable reference to the active profile.
    pub fn active_profile_mut(&mut self) -> &mut Profile {
        let name = self.active_profile.clone();
        self.profiles
            .iter_mut()
            .find(|p| p.name == name)
            .expect("Config must always have at least one profile")
    }
}

/// An entry parsed from ~/.ssh/config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshHostEntry {
    pub name: String,
    pub hostname: String,
    pub user: String,
    pub port: u16,
}

pub fn load_config(app_data_dir: &PathBuf) -> Config {
    let path = app_data_dir.join("config.json");
    if path.exists() {
        let contents = fs::read_to_string(&path).unwrap_or_default();

        // Try loading as the new multi-profile format first
        if let Ok(config) = serde_json::from_str::<Config>(&contents) {
            return config;
        }

        // Fall back to legacy flat format and migrate
        if let Ok(old) = serde_json::from_str::<LegacyConfig>(&contents) {
            let profile = Profile {
                name: "Default".to_string(),
                host: old.host,
                user: old.user,
                ssh_port: old.ssh_port,
                ports: old.ports,
                rate_limit_max: default_rate_limit_max(),
                rate_limit_window_secs: default_rate_limit_window_secs(),
            };
            let config = Config {
                active_profile: "Default".to_string(),
                profiles: vec![profile],
            };
            // Save the migrated config
            let _ = save_config(app_data_dir, &config);
            return config;
        }
    } else {
        // Try to migrate from old PowerShell ports.json
        let old_path = PathBuf::from(
            std::env::current_exe()
                .unwrap_or_default()
                .parent()
                .unwrap_or(std::path::Path::new(""))
                .join("ports.json"),
        );
        if old_path.exists() {
            if let Ok(contents) = fs::read_to_string(&old_path) {
                #[derive(Deserialize)]
                struct OldConfig {
                    #[serde(rename = "Host")]
                    host: Option<String>,
                    #[serde(rename = "User")]
                    user: Option<String>,
                    #[serde(rename = "SshPort")]
                    ssh_port: Option<u16>,
                    #[serde(rename = "Ports")]
                    ports: Option<Vec<u16>>,
                }
                if let Ok(old) = serde_json::from_str::<OldConfig>(&contents) {
                    let profile = Profile {
                        name: "Default".to_string(),
                        host: old.host.unwrap_or_default(),
                        user: old.user.unwrap_or_default(),
                        ssh_port: old.ssh_port.unwrap_or(22),
                        ports: old.ports.unwrap_or_default(),
                        rate_limit_max: default_rate_limit_max(),
                        rate_limit_window_secs: default_rate_limit_window_secs(),
                    };
                    return Config {
                        active_profile: "Default".to_string(),
                        profiles: vec![profile],
                    };
                }
            }
        }
    }
    Config::default()
}

/// The old flat config format, used for migration.
#[derive(Deserialize)]
struct LegacyConfig {
    host: String,
    user: String,
    ssh_port: u16,
    ports: Vec<u16>,
}

pub fn save_config(app_data_dir: &PathBuf, config: &Config) -> Result<(), String> {
    fs::create_dir_all(app_data_dir).map_err(|e| e.to_string())?;
    let path = app_data_dir.join("config.json");
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

/// Parse ~/.ssh/config and return host entries.
/// Skips wildcard patterns (e.g. `Host *`) and hosts containing `?`.
pub fn scan_ssh_config() -> Vec<SshHostEntry> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };
    let ssh_config_path = home.join(".ssh").join("config");
    let file = match fs::File::open(&ssh_config_path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = std::io::BufReader::new(file);
    let mut entries: Vec<SshHostEntry> = Vec::new();

    // Temporary state for the current Host block being parsed
    let mut current_names: Vec<String> = Vec::new();
    let mut current_hostname = String::new();
    let mut current_user = String::new();
    let mut current_port: u16 = 22;

    let mut directives: HashMap<String, String> = HashMap::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Split on first whitespace or '='
        let (key, value) = match trimmed.split_once(|c: char| c.is_whitespace() || c == '=') {
            Some((k, v)) => (k.to_lowercase(), v.trim().to_string()),
            None => continue,
        };

        if key == "host" {
            // Flush the previous block
            flush_host_block(
                &current_names,
                &current_hostname,
                &current_user,
                current_port,
                &directives,
                &mut entries,
            );

            // Start a new block
            current_names = value
                .split_whitespace()
                .filter(|h| !h.contains('*') && !h.contains('?'))
                .map(|s| s.to_string())
                .collect();
            current_hostname.clear();
            current_user.clear();
            current_port = 22;
            directives.clear();
        } else {
            directives.insert(key.clone(), value.clone());
            match key.as_str() {
                "hostname" => current_hostname = value,
                "user" => current_user = value,
                "port" => current_port = value.parse().unwrap_or(22),
                _ => {}
            }
        }
    }

    // Flush the last block
    flush_host_block(
        &current_names,
        &current_hostname,
        &current_user,
        current_port,
        &directives,
        &mut entries,
    );

    entries
}

fn flush_host_block(
    names: &[String],
    hostname: &str,
    user: &str,
    port: u16,
    _directives: &HashMap<String, String>,
    entries: &mut Vec<SshHostEntry>,
) {
    for name in names {
        entries.push(SshHostEntry {
            name: name.clone(),
            hostname: if hostname.is_empty() {
                name.clone()
            } else {
                hostname.to_string()
            },
            user: user.to_string(),
            port,
        });
    }
}
