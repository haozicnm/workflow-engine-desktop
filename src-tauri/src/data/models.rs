// data/models.rs — 数据模型
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMeta {
    pub id: String,
    pub name: String,
    pub description: String,
    pub yaml: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunInfo {
    pub id: String,
    pub workflow_id: String,
    pub status: String,
    pub current_step: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StepRunInfo {
    pub id: String,
    pub run_id: String,
    pub step_id: String,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// 运行历史列表项（带工作流名称）
#[derive(Debug, Clone, Serialize)]
pub struct RunHistoryItem {
    pub id: String,
    pub workflow_id: String,
    pub workflow_name: String,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub error: Option<String>,
}

/// 运行详情（运行信息 + 步骤执行记录）
#[derive(Debug, Clone, Serialize)]
pub struct RunDetail {
    pub run: RunInfo,
    pub workflow_name: String,
    pub steps: Vec<StepRunInfo>,
}

/// v4.1: 步骤日志条目（持久化执行日志）
#[derive(Debug, Clone, Serialize)]
pub struct StepLogEntry {
    pub id: i64,
    pub step_run_id: String,
    pub run_id: String,
    pub step_id: String,
    pub level: String,
    pub message: String,
    pub timestamp: String,
}

/// 定时计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleInfo {
    pub id: String,
    pub workflow_id: String,
    pub workflow_name: String,
    pub cron_expr: String,
    pub enabled: bool,
    pub last_run_at: Option<String>,
    pub created_at: String,
}
