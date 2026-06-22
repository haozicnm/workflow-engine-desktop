use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{Json, Response},
};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::server::events;
use crate::server::handlers::{err_response, map_err_resp, ok_response};

// ═══════════════════════════════════════════════════════════
// Request body types
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct RunStartBody {
    pub workflow_id: String,
}

#[derive(Debug, Serialize)]
pub struct RunStartResponse {
    pub run_id: String,
}

#[derive(Debug, Deserialize)]
pub struct RunLogsQuery {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct RunListQuery {
    pub workflow_id: Option<String>,
    pub limit: Option<usize>,
}

// ═══════════════════════════════════════════════════════════
// 执行控制 handler
// ═══════════════════════════════════════════════════════════

pub async fn run_start(Json(body): Json<RunStartBody>) -> Response {
    let app = crate::server::state::get();
    let workflow_id = body.workflow_id;

    // 共享准备阶段（消除与 commands/run.rs 的重复）
    use crate::engine::scheduler;
    let prep = match scheduler::prepare_run(
        &app.db,
        &app.config,
        &app.run_semaphore,
        &app.cancel_flags,
        &app.cancel_tokens,
        &app.pause_flags,
        &app.breakpoint_flags,
        &app.step_mode_flags,
        &workflow_id,
    )
    .await
    {
        Ok(p) => p,
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("工作流不存在") {
                return err_response(StatusCode::NOT_FOUND, msg);
            }
            if msg.contains("并发") || msg.contains("限制") {
                return err_response(StatusCode::TOO_MANY_REQUESTS, msg);
            }
            return err_response(StatusCode::INTERNAL_SERVER_ERROR, msg);
        }
    };

    // 发射 SSE 启动事件
    events::emit(
        "run-update",
        serde_json::json!({
            "run_id": prep.run_id,
            "workflow_id": &workflow_id,
            "workflow_name": &prep.workflow_name,
            "status": "running",
        }),
    );

    // 后台异步执行
    let db = app.db.clone();
    let approval_store = app.approval_store.clone();
    let run_id_clone = prep.run_id.clone();
    let cancel_flags = app.cancel_flags.clone();
    let cancel_tokens = app.cancel_tokens.clone();
    let pause_flags = app.pause_flags.clone();
    let breakpoint_flags = app.breakpoint_flags.clone();
    let step_mode_flags = app.step_mode_flags.clone();
    let debug_snapshots = app.debug_snapshots.clone();
    let debug_snapshots_cleanup = debug_snapshots.clone();
    let timeouts = prep.timeouts;
    let shell_allowed = prep.shell_allowed_commands;
    let workflow = prep.workflow;

    tokio::spawn(async move {
        let _permit = prep.permit;
        let ctrl = scheduler::RunControl {
            cancel_flag: prep.cancel_flag,
            cancel_token: prep.cancel_token,
            pause_flag: prep.pause_flag,
            breakpoint_flag: prep.breakpoint_flag,
            step_mode_flag: prep.step_mode_flag,
            debug_snapshots,
        };
        let global_timeout_ms = timeouts.workflow_total_ms;
        let global_timeout = if global_timeout_ms == 0 {
            std::time::Duration::from_secs(365 * 24 * 3600)
        } else {
            std::time::Duration::from_millis(global_timeout_ms)
        };
        let result = tokio::time::timeout(
            global_timeout,
            scheduler::run_workflow(
                &workflow,
                &run_id_clone,
                None,
                &db,
                approval_store,
                &[],
                &ctrl,
                &timeouts,
                &shell_allowed,
            ),
        )
        .await;
        let result = match result {
            Ok(r) => r,
            Err(_elapsed) => {
                warn!("工作流全局超时 ({}ms): {}", global_timeout_ms, run_id_clone);
                Err(anyhow::anyhow!("工作流执行超时 (超过 {}ms)", global_timeout_ms))
            }
        };

        // 确保清理（即使 panic 也能执行，因为 tokio::spawn 的 Future 正常 drop）
        cancel_flags.write().await.remove(&run_id_clone);
        cancel_tokens.write().await.remove(&run_id_clone);
        pause_flags.write().await.remove(&run_id_clone);
        breakpoint_flags.write().await.remove(&run_id_clone);
        step_mode_flags.write().await.remove(&run_id_clone);
        debug_snapshots_cleanup.write().await.remove(&run_id_clone);

        match result {
            Ok(_state) => info!("工作流执行完成: {}", run_id_clone),
            Err(e) => {
                let err_msg = e.to_string();
                let status = if err_msg.contains("cancelled") { "cancelled" } else { "failed" };
                error!("工作流{}: {} - {}", if status == "cancelled" { "已取消" } else { "执行失败" }, run_id_clone, err_msg);
            }
        }
    });

    ok_response(serde_json::json!({ "run_id": prep.run_id }))
}

pub async fn run_cancel(Path(run_id): Path<String>) -> Response {
    let app = crate::server::state::get();
    let flags = app.cancel_flags.read().await;
    let tokens = app.cancel_tokens.read().await;

    match flags.get(&run_id) {
        Some(flag) => {
            use std::sync::atomic::Ordering;
            flag.store(true, Ordering::Relaxed);
            if let Some(token) = tokens.get(&run_id) {
                token.cancel();
            }
            ok_response(serde_json::json!({ "success": true }))
        }
        None => err_response(StatusCode::NOT_FOUND, "运行不存在或已结束"),
    }
}

pub async fn run_pause(Path(run_id): Path<String>) -> Response {
    let app = crate::server::state::get();
    let flags = app.pause_flags.read().await;
    match flags.get(&run_id) {
        Some(flag) => {
            use std::sync::atomic::Ordering;
            flag.store(true, Ordering::Relaxed);
            events::emit(
                "run-update",
                serde_json::json!({
                    "run_id": &run_id,
                    "status": "paused",
                }),
            );
            ok_response(serde_json::json!({ "success": true }))
        }
        None => err_response(StatusCode::NOT_FOUND, "运行不存在或已结束"),
    }
}

pub async fn run_resume(Path(run_id): Path<String>) -> Response {
    let app = crate::server::state::get();
    let flags = app.pause_flags.read().await;
    match flags.get(&run_id) {
        Some(flag) => {
            use std::sync::atomic::Ordering;
            flag.store(false, Ordering::Relaxed);
            events::emit(
                "run-update",
                serde_json::json!({
                    "run_id": &run_id,
                    "status": "running",
                }),
            );
            ok_response(serde_json::json!({ "success": true }))
        }
        None => err_response(StatusCode::NOT_FOUND, "运行不存在或已结束"),
    }
}

pub async fn run_status(Path(run_id): Path<String>) -> Response {
    let app = crate::server::state::get();
    let run = match app.db.get_run(&run_id) {
        Ok(Some(r)) => r,
        Ok(None) => return err_response(StatusCode::NOT_FOUND, "运行不存在"),
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    ok_response(serde_json::json!({
        "run_id": run.id,
        "workflow_id": run.workflow_id,
        "status": run.status,
        "current_step": run.current_step,
        "started_at": run.started_at,
        "finished_at": run.finished_at,
    }))
}

pub async fn run_list(Query(query): Query<RunListQuery>) -> Response {
    let app = crate::server::state::get();
    map_err_resp(
        app.db
            .list_runs(query.workflow_id.as_deref(), query.limit.unwrap_or(50))
            .map_err(|e| e.to_string()),
    )
}

pub async fn run_detail(Path(run_id): Path<String>) -> Response {
    let app = crate::server::state::get();
    match app.db.get_run_detail(&run_id) {
        Ok(Some(detail)) => ok_response(detail),
        Ok(None) => err_response(StatusCode::NOT_FOUND, "运行记录不存在"),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn run_logs(Path(run_id): Path<String>, Query(query): Query<RunLogsQuery>) -> Response {
    let app = crate::server::state::get();
    let mut steps = match app.db.get_step_runs(&run_id) {
        Ok(s) => s,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let total = steps.len();
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(100);

    if offset < total {
        let end = std::cmp::min(offset + limit, total);
        steps = steps[offset..end].to_vec();
    } else {
        steps.clear();
    }

    ok_response(serde_json::json!({
        "total": total,
        "offset": offset,
        "steps": steps,
    }))
}

pub async fn run_step_logs(Path(run_id): Path<String>) -> Response {
    let app = crate::server::state::get();
    map_err_resp(app.db.get_step_logs(&run_id).map_err(|e| e.to_string()))
}
