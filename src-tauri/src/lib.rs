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
///
/// 首次启动时将内置模板作为普通工作流写入数据库
fn seed_builtin_workflows(db: &data::db::Database) -> Result<()> {
    use tracing::info;

    let existing = db.list_workflows().unwrap_or_default();
    if !existing.is_empty() {
        return Ok(()); // 已有数据，跳过
    }

    info!("首次启动，正在创建 4 个内置示例工作流...");

    let builtins: &[(&str, &str)] = &[
        (include_str!("../../templates/monitor-excel-alert.json"), "网页监控 → Excel异常报告"),
        (include_str!("../../templates/excel-to-word-batch.json"), "Excel数据 → 批量Word通知书"),
        (include_str!("../../templates/api-excel-word-branch.json"), "JSON数据 → 条件分流 Word/Excel"),
        (include_str!("../../templates/word-extract-excel.json"), "Word文档提取 → Excel汇总分析"),
    ];

    for (json_str, default_name) in builtins {
        let data: serde_json::Value = serde_json::from_str(json_str)?;
        let name = data["name"].as_str().unwrap_or(default_name);
        let desc = data["description"].as_str().unwrap_or("");
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        db.create_workflow(&id, name, desc, &now, &now)?;
        db.save_workflow_yaml(&id, json_str)?;
        info!("  ✅ 已创建: {name}");
    }

    info!("内置示例工作流初始化完成");
    Ok(())
}

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
        seed_builtin_workflows(&db)?;
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
