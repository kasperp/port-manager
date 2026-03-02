use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub host: String,
    pub user: String,
    pub ssh_port: u16,
    pub ports: Vec<u16>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: String::new(),
            user: String::new(),
            ssh_port: 22,
            ports: Vec::new(),
        }
    }
}

pub fn load_config(app_data_dir: &PathBuf) -> Config {
    let path = app_data_dir.join("config.json");
    if path.exists() {
        let contents = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&contents).unwrap_or_default()
    } else {
        // Try to migrate from old PowerShell ports.json
        let old_path = PathBuf::from(std::env::current_exe()
            .unwrap_or_default()
            .parent()
            .unwrap_or(std::path::Path::new(""))
            .join("ports.json"));
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
                    return Config {
                        host: old.host.unwrap_or_default(),
                        user: old.user.unwrap_or_default(),
                        ssh_port: old.ssh_port.unwrap_or(22),
                        ports: old.ports.unwrap_or_default(),
                    };
                }
            }
        }
        Config::default()
    }
}

pub fn save_config(app_data_dir: &PathBuf, config: &Config) -> Result<(), String> {
    fs::create_dir_all(app_data_dir).map_err(|e| e.to_string())?;
    let path = app_data_dir.join("config.json");
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}
