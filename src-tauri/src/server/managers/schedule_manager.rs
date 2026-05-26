// server/managers/schedule_manager.rs — 定时任务 handler
//
// 从 handlers.rs 提取的定时任务相关 handler 函数和类型。

use axum::{
    extract::Path,
    response::{Response, Json},
    http::StatusCode,
};
use serde::Deserialize;

use crate::server::handlers::{ok_response, err_response, map_err_resp};
use crate::server::events;

// ═══════════════════════════════════════════════════════════
// Request body types
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct ScheduleCreateBody {
    pub workflow_id: String,
    pub cron_expr: String,
}

#[derive(Debug, Deserialize)]
pub struct ScheduleUpdateBody {
    pub cron_expr: Option<String>,
    pub enabled: Option<bool>,
}

// ═══════════════════════════════════════════════════════════
// 定时任务 handler
// ═══════════════════════════════════════════════════════════

pub async fn schedule_list(
) -> Response {
    let app = crate::server::state::get();
    map_err_resp(app.db.list_schedules().map_err(|e| e.to_string()))
}

pub async fn schedule_create(
    Json(body): Json<ScheduleCreateBody>,
) -> Response {
    let app = crate::server::state::get();
    let fields: Vec<&str> = body.cron_expr.split_whitespace().collect();
    let quartz = match fields.len() {
        5 => format!("0 {} *", body.cron_expr),
        7 => body.cron_expr.clone(),
        _ => return err_response(StatusCode::BAD_REQUEST, "cron 表达式应为 5 字段（分 时 日 月 周）"),
    };
    if let Err(e) = quartz.parse::<cron::Schedule>() {
        return err_response(StatusCode::BAD_REQUEST, format!("无效的 cron 表达式: {e}"));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    match app.db.create_schedule(&id, &body.workflow_id, &body.cron_expr, &now) {
        Ok(()) => {
            events::emit("schedule-changed", serde_json::json!({
                "action": "create",
                "schedule_id": &id,
                "workflow_id": &body.workflow_id,
            }));
            ok_response(serde_json::json!({ "id": id }))
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn schedule_update(
    Path(id): Path<String>,
    Json(body): Json<ScheduleUpdateBody>,
) -> Response {
    let app = crate::server::state::get();
    if let Some(ref expr) = body.cron_expr {
        let fields: Vec<&str> = expr.split_whitespace().collect();
        let quartz = match fields.len() {
            5 => format!("0 {} *", expr),
            7 => expr.clone(),
            _ => return err_response(StatusCode::BAD_REQUEST, "cron 表达式应为 5 字段（分 时 日 月 周）"),
        };
        if let Err(e) = quartz.parse::<cron::Schedule>() {
            return err_response(StatusCode::BAD_REQUEST, format!("无效的 cron 表达式: {e}"));
        }
    }

    match app.db.update_schedule(&id, body.cron_expr.as_deref(), body.enabled) {
        Ok(()) => {
            events::emit("schedule-changed", serde_json::json!({
                "action": "update",
                "schedule_id": &id,
            }));
            ok_response(serde_json::json!({ "success": true }))
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn schedule_delete(
    Path(id): Path<String>,
) -> Response {
    let app = crate::server::state::get();
    match app.db.delete_schedule(&id) {
        Ok(()) => {
            events::emit("schedule-changed", serde_json::json!({
                "action": "delete",
                "schedule_id": &id,
            }));
            ok_response(serde_json::json!({ "success": true }))
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}
