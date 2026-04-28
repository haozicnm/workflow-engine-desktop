// lib.rs — 库导出
pub mod commands;
pub mod data;
pub mod engine;
pub mod nodes;
pub mod system;
pub mod platform;

use std::sync::Arc;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use anyhow::Result;

/// 运行控制标志类型
pub type RunFlags = Arc<tokio::sync::RwLock<HashMap<String, Arc<AtomicBool>>>>;

/// 取消令牌类型（用于结构化取消）
pub type CancelTokens = Arc<tokio::sync::RwLock<HashMap<String, tokio_util::sync::CancellationToken>>>;

/// 应用全局状态
pub struct App {
    pub db: Arc<data::db::Database>,
    pub config: Arc<tokio::sync::RwLock<data::config::AppConfig>>,
    /// 取消标志：key=run_id
    pub cancel_flags: RunFlags,
    /// 取消令牌：key=run_id（用于结构化取消）
    pub cancel_tokens: CancelTokens,
    /// 暂停标志：key=run_id，true=暂停中
    pub pause_flags: RunFlags,
    /// 断点命中标志：key=run_id，value=true 表示在断点处暂停
    pub breakpoint_flags: RunFlags,
    /// 单步模式标志：key=run_id，value=true 表示执行完当前步骤后暂停
    pub step_mode_flags: RunFlags,
    /// 调试变量快照：key=run_id，存储当前执行上下文（variables + step_outputs）
    pub debug_snapshots: Arc<tokio::sync::RwLock<HashMap<String, serde_json::Value>>>,
    /// 并发工作流信号量：限制同时运行的工作流数量
    pub run_semaphore: Arc<tokio::sync::Semaphore>,
}

impl App {
    pub fn new() -> Result<Self> {
        let db = data::db::Database::open_default()?;
        let config = data::config::AppConfig::load_default().unwrap_or_default();
        let max_concurrent = std::env::var("MAX_CONCURRENT_WORKFLOWS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(10);

        Ok(App {
            db: Arc::new(db),
            config: Arc::new(tokio::sync::RwLock::new(config)),
            cancel_flags: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            cancel_tokens: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            pause_flags: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            breakpoint_flags: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            step_mode_flags: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            debug_snapshots: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            run_semaphore: Arc::new(tokio::sync::Semaphore::new(max_concurrent)),
        })
    }
}
