// data/config.rs — 配置管理
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub theme: String,
    pub language: String,
    pub auto_start: bool,
    pub log_level: String,
    pub python_path: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            theme: "system".to_string(),
            language: "zh-CN".to_string(),
            auto_start: false,
            log_level: "info".to_string(),
            python_path: None,
        }
    }
}

impl AppConfig {
    fn config_path() -> Result<PathBuf> {
        let dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("workflow-engine");
        std::fs::create_dir_all(&dir)?;
        Ok(dir.join("config.json"))
    }

    pub fn load_default() -> Result<Self> {
        let path = Self::config_path()?;
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: AppConfig = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            Ok(AppConfig::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
