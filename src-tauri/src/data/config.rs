// data/config.rs — 配置管理
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ═══════════════════════════════════════════════════
// Sub-config structs (P1: 超时/日志/并发)
// ═══════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// HTTP 请求默认超时 (ms)
    pub http_request_ms: u64,
    /// 浏览器页面加载超时 (ms)
    pub browser_page_ms: u64,
    /// 工作流整体超时 (ms)，0=不限
    pub workflow_total_ms: u64,
    /// 单节点执行超时 (ms)
    pub node_exec_ms: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        TimeoutConfig {
            http_request_ms: 30_000,
            browser_page_ms: 60_000,
            workflow_total_ms: 600_000,
            node_exec_ms: 120_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// 单文件最大大小 (MB)
    pub max_size_mb: u32,
    /// 保留文件数
    pub max_files: u32,
    /// 自动清理天数
    pub auto_clean_days: u32,
}

impl Default for LogConfig {
    fn default() -> Self {
        LogConfig {
            max_size_mb: 50,
            max_files: 10,
            auto_clean_days: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// 最大并发工作流数
    pub max_concurrent_runs: u32,
    /// 默认重试次数
    pub default_retries: u32,
    /// 默认重试间隔 (ms)
    pub retry_delay_ms: u64,
    /// Shell 节点允许的命令白名单（glob 模式，空=允许所有）
    #[serde(default)]
    pub shell_allowed_commands: Vec<String>,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        ExecutionConfig {
            max_concurrent_runs: 3,
            default_retries: 0,
            retry_delay_ms: 1_000,
            shell_allowed_commands: Vec::new(),
        }
    }
}

// ═══════════════════════════════════════════════════
// AppConfig
// ═══════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub theme: String,
    pub language: String,
    pub auto_start: bool,
    pub log_level: String,
    pub python_path: Option<String>,
    /// 软件工作目录（模板、导出文件等存储位置）
    pub working_dir: String,
    // ── P1 新增 ──
    pub timeouts: TimeoutConfig,
    pub logging: LogConfig,
    pub execution: ExecutionConfig,
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
            working_dir: String::new(),
            timeouts: TimeoutConfig::default(),
            logging: LogConfig::default(),
            execution: ExecutionConfig::default(),
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
        // 原子写入：先写临时文件，再 rename（防止崩溃导致配置丢失）
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &content)?;
        std::fs::rename(&tmp_path, &path)?;
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
