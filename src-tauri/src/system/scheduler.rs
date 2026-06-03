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

                // 触发工作流执行（复用 run_start 逻辑）
                let wf_id = schedule.workflow_id.clone();
                let db_clone = db.clone();
                let handle_clone = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    match start_scheduled_run(&db_clone, &handle_clone, &wf_id).await {
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

/// 触发一次工作流执行（定时调度专用，不经过 Tauri command）
async fn start_scheduled_run(
    db: &Arc<Database>,
    app_handle: &AppHandle,
    workflow_id: &str,
) -> Result<String, String> {
    let yaml = db
        .get_workflow_yaml(workflow_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "工作流不存在".to_string())?;

    let workflow = crate::engine::parser::parse_workflow(&yaml)
        .map_err(|e| format!("YAML 解析失败: {}", e))?;

    let run_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let workflow_name = workflow.name.clone();
    db.create_run(&run_id, workflow_id, &workflow_name, &now)
        .map_err(|e| e.to_string())?;

    if let Err(e) = app_handle.emit(
        "run-update",
        serde_json::json!({
            "run_id": run_id,
            "workflow_id": workflow_id,
            "workflow_name": workflow_name,
            "status": "running",
            "trigger": "schedule",
        }),
    ) {
        warn!("emit run-update failed: {}", e);
    }

    // 读取超时配置
    use tauri::Manager;
    let config_guard = app_handle.state::<crate::App>().config.read().await;
    let timeouts = config_guard.timeouts.clone();
    let shell_allowed = config_guard.execution.shell_allowed_commands.clone();
    drop(config_guard);

    let run_id_clone = run_id.clone();
    let db_clone = db.clone();
    let handle_clone = app_handle.clone();
    let approval_store = app_handle.state::<crate::App>().approval_store.clone();
    tauri::async_runtime::spawn(async move {
        let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancel_token = tokio_util::sync::CancellationToken::new();
        let pause_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let breakpoint_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let step_mode_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let debug_snapshots =
            std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
        let ctrl = crate::engine::scheduler::RunControl {
            cancel_flag,
            cancel_token,
            pause_flag,
            breakpoint_flag,
            step_mode_flag,
            debug_snapshots,
        };
        match crate::engine::scheduler::run_workflow(
            &workflow,
            &run_id_clone,
            Some(&handle_clone),
            &db_clone,
            approval_store,
            &[],
            &ctrl,
            &timeouts,
            &shell_allowed,
        )
        .await
        {
            Ok(_) => info!("定时工作流执行完成: {}", run_id_clone),
            Err(e) => {
                error!("定时工作流执行失败: {} - {}", run_id_clone, e);
                // run_workflow 内部已发射 run-update 事件，此处不再重复
            }
        }
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
