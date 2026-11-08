use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

pub const DEFAULT_CONFIG_FILE: &str = "wiretap.config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireTapConfig {
    pub listen_port: u16,
    pub target_url: String,
    pub ui_port: u16,
    pub max_history: usize,
}

impl Default for WireTapConfig {
    fn default() -> Self {
        Self {
            listen_port: 8080,
            target_url: "http://localhost:3000".to_string(),
            ui_port: 9090,
            max_history: 1000,
        }
    }
}

impl WireTapConfig {
    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
        let path_ref = path.as_ref();
        if path_ref.exists() {
            if let Ok(content) = fs::read_to_string(path_ref) {
                if let Ok(cfg) = serde_json::from_str::<WireTapConfig>(&content) {
                    return cfg;
                }
            }
        }
        Self::default()
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let json_str = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        fs::write(path, json_str)
    }
}
