// system/scheduler.rs — 后台定时调度器
// 每 30 秒检查一次到期的定时计划，自动触发工作流执行
use crate::data::db::Database;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tracing::{error, info, warn};

/// 启动后台调度器（在 app.setup 中调用）
pub fn start(app_handle: AppHandle, db: Arc<Database>) {
    tauri::async_runtime::spawn(async move {
        info!("定时调度器已启动");
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            check_and_run(&app_handle, &db).await;
        }
    });
}

/// 检查到期的计划并触发执行
async fn check_and_run(app_handle: &AppHandle, db: &Arc<Database>) {
    let now = chrono::Utc::now();

    // 查询所有启用的计划
    let schedules = match db.list_enabled_schedules() {
        Ok(s) => s,
        Err(e) => {
            error!("查询定时计划失败: {}", e);
            return;
        }
    };

    for schedule in schedules {
        // 解析 cron 表达式（5 字段标准 cron → 7 字段 Quartz）
        let cron_expr = normalize_cron(&schedule.cron_expr);
        let cron_schedule = match cron_expr.parse::<cron::Schedule>() {
            Ok(s) => s,
            Err(e) => {
                warn!("计划 '{}' 的 cron 表达式无效: {}", schedule.id, e);
                continue;
            }
        };

        // 计算上次运行之后的下一次触发时间
        let last_run = schedule
            .last_run_at
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|| {
                schedule
                    .created_at
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap_or_else(|_| now - chrono::Duration::hours(1))
            });

        // 如果下次触发时间 <= now，执行
        if let Some(next_fire) = cron_schedule.after(&last_run).next() {
            if next_fire <= now {
                info!(
                    "定时计划 '{}' 到期，触发工作流 '{}'",
                    schedule.id, schedule.workflow_id
                );

                // 更新 last_run_at
                let now_str = now.to_rfc3339();
                if let Err(e) = db.update_schedule_last_run(&schedule.id, &now_str) {
                    error!("更新计划运行时间失败: {}", e);
                    continue;
                }

                // 触发工作流执行（复用 prepare_run 统一入口）
                let wf_id = schedule.workflow_id.clone();
                let handle_clone = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    match start_scheduled_run(&handle_clone, &wf_id).await {
                        Ok(run_id) => {
                            info!("定时任务已触发: workflow={} run={}", wf_id, run_id);
                        }
                        Err(e) => {
                            error!("定时任务触发失败: workflow={} error={}", wf_id, e);
                        }
                    }
                });
            }
        }
    }
}

/// 触发一次工作流执行（通过 prepare_run 统一入口，共享并发/取消控制）
async fn start_scheduled_run(
    app_handle: &tauri::AppHandle,
    workflow_id: &str,
) -> Result<String, String> {
    use tauri::Manager;
    let app = app_handle.state::<crate::App>();

    let prep = crate::engine::scheduler::prepare_run(
        &app.db,
        &app.config,
        &app.run_semaphore,
        &app.cancel_flags,
        &app.cancel_tokens,
        &app.pause_flags,
        &app.breakpoint_flags,
        &app.step_mode_flags,
        workflow_id,
    )
    .await
    .map_err(|e| e.to_string())?;

    // 发射启动事件
    if let Err(e) = app_handle.emit(
        "run-update",
        serde_json::json!({
            "run_id": prep.run_id,
            "workflow_id": workflow_id,
            "workflow_name": prep.workflow_name,
            "status": "running",
            "trigger": "schedule",
        }),
    ) {
        warn!("emit run-update failed: {}", e);
    }

    let run_id = prep.run_id.clone();
    let db = app.db.clone();
    let approval_store = app.approval_store.clone();
    let cancel_flags = app.cancel_flags.clone();
    let cancel_tokens = app.cancel_tokens.clone();
    let pause_flags = app.pause_flags.clone();
    let breakpoint_flags = app.breakpoint_flags.clone();
    let step_mode_flags = app.step_mode_flags.clone();
    let debug_snapshots = app.debug_snapshots.clone();
    let handle_clone = app_handle.clone();

    tauri::async_runtime::spawn(async move {
        let _permit = prep.permit;
        let ctrl = crate::engine::scheduler::RunControl {
            cancel_flag: prep.cancel_flag,
            cancel_token: prep.cancel_token,
            pause_flag: prep.pause_flag,
            breakpoint_flag: prep.breakpoint_flag,
            step_mode_flag: prep.step_mode_flag,
            debug_snapshots,
        };
        match crate::engine::scheduler::run_workflow(
            &prep.workflow,
            &run_id,
            Some(&handle_clone),
            &db,
            approval_store,
            &[],
            &ctrl,
            &prep.timeouts,
            &prep.shell_allowed_commands,
        )
        .await
        {
            Ok(_) => info!("定时工作流执行完成: {}", run_id),
            Err(e) => {
                error!("定时工作流执行失败: {} - {}", run_id, e);
            }
        }
        cancel_flags.write().await.remove(&run_id);
        cancel_tokens.write().await.remove(&run_id);
        pause_flags.write().await.remove(&run_id);
        breakpoint_flags.write().await.remove(&run_id);
        step_mode_flags.write().await.remove(&run_id);
    });

    Ok(run_id)
}

/// 标准 5 字段 cron → Quartz 7 字段
/// 5 字段: min hour dom month dow
/// 7 字段: sec min hour dom month dow year
fn normalize_cron(expr: &str) -> String {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    match fields.len() {
        5 => format!("0 {} *", expr), // 加秒=0, 年=*
        6 => format!("0 {}", expr),   // 加秒=0
        7 => expr.to_string(),
        _ => expr.to_string(), // 让解析器报错
    }
}
