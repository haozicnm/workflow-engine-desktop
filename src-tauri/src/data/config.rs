// data/config.rs — 配置管理
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub theme: String,
    pub language: String,
    pub auto_start: bool,
    pub log_level: String,
    pub python_path: Option<String>,
    /// 浏览器通道: auto / msedge / chrome / chromium
    pub browser_channel: String,
    /// 浏览器可执行文件路径（留空=自动检测）
    pub browser_executable_path: String,
    /// 软件工作目录（模板、导出文件等存储位置）
    pub working_dir: String,
    /// 临时存储（断点等运行时数据，不持久化）
    #[serde(skip)]
    pub temp: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            theme: "system".to_string(),
            language: "zh-CN".to_string(),
            auto_start: false,
            log_level: "info".to_string(),
            python_path: None,
            browser_channel: "auto".to_string(),
            browser_executable_path: String::new(),
            working_dir: String::new(),
            temp: std::collections::HashMap::new(),
        }
    }
}

impl AppConfig {
    fn config_path() -> Result<PathBuf> {
        let dir = crate::data::paths::resolve_data_dir();
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

    /// 获取临时值（运行时数据，不持久化）
    pub fn get_temp(&self, key: &str) -> Option<&serde_json::Value> {
        self.temp.get(key)
    }

    /// 设置临时值（运行时数据，不持久化）
    pub fn set_temp(&mut self, key: &str, value: serde_json::Value) {
        self.temp.insert(key.to_string(), value);
    }
}
