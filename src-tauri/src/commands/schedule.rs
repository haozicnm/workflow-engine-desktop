// commands/schedule.rs — 定时计划命令
use tauri::State;
use crate::App;
use crate::data::models::ScheduleInfo;

#[tauri::command]
pub async fn schedule_list(
    app: State<'_, App>,
) -> Result<Vec<ScheduleInfo>, String> {
    app.db.list_schedules().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn schedule_create(
    app: State<'_, App>,
    workflow_id: String,
    cron_expr: String,
) -> Result<String, String> {
    // 验证 cron 表达式
    let fields: Vec<&str> = cron_expr.split_whitespace().collect();
    let quartz = match fields.len() {
        5 => format!("0 {} *", cron_expr),
        7 => cron_expr.clone(),
        _ => return Err("cron 表达式应为 5 字段（分 时 日 月 周）".to_string()),
    };
    quartz.parse::<cron::Schedule>()
        .map_err(|e| format!("无效的 cron 表达式: {}", e))?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    app.db.create_schedule(&id, &workflow_id, &cron_expr, &now)
        .map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub async fn schedule_update(
    app: State<'_, App>,
    id: String,
    cron_expr: Option<String>,
    enabled: Option<bool>,
) -> Result<(), String> {
    // 如果有新 cron 表达式，验证它
    if let Some(ref expr) = cron_expr {
        let fields: Vec<&str> = expr.split_whitespace().collect();
        let quartz = match fields.len() {
            5 => format!("0 {} *", expr),
            7 => expr.clone(),
            _ => return Err("cron 表达式应为 5 字段（分 时 日 月 周）".to_string()),
        };
        quartz.parse::<cron::Schedule>()
            .map_err(|e| format!("无效的 cron 表达式: {}", e))?;
    }

    app.db.update_schedule(&id, cron_expr.as_deref(), enabled)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn schedule_delete(
    app: State<'_, App>,
    id: String,
) -> Result<(), String> {
    app.db.delete_schedule(&id).map_err(|e| e.to_string())
}
