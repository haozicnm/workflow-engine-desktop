// commands/run.rs — 执行控制命令
use tauri::State;
use serde::Serialize;
use crate::App;

#[derive(Debug, Serialize)]
pub struct RunStatus {
    pub run_id: String,
    pub workflow_id: String,
    pub status: String,
    pub current_step: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

#[tauri::command]
pub async fn run_start(
    app: State<'_, App>,
    workflow_id: String,
) -> Result<String, String> {
    // TODO: 引擎启动
    Err("引擎尚未实现".to_string())
}

#[tauri::command]
pub async fn run_pause(
    app: State<'_, App>,
    run_id: String,
) -> Result<(), String> {
    Err("引擎尚未实现".to_string())
}

#[tauri::command]
pub async fn run_resume(
    app: State<'_, App>,
    run_id: String,
) -> Result<(), String> {
    Err("引擎尚未实现".to_string())
}

#[tauri::command]
pub async fn run_cancel(
    app: State<'_, App>,
    run_id: String,
) -> Result<(), String> {
    Err("引擎尚未实现".to_string())
}

#[tauri::command]
pub async fn run_status(
    app: State<'_, App>,
    run_id: String,
) -> Result<RunStatus, String> {
    Err("引擎尚未实现".to_string())
}

#[tauri::command]
pub async fn run_logs(
    app: State<'_, App>,
    run_id: String,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Result<serde_json::Value, String> {
    Err("引擎尚未实现".to_string())
}
