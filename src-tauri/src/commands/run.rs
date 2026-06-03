// commands/run.rs — 执行控制命令
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{State, AppHandle, Emitter};
use crate::App;
use crate::data::models::{RunHistoryItem, RunDetail, StepLogEntry};
use tracing::{info, warn, error};
use anyhow;


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
    app_handle: AppHandle,
    workflow_id: String,
) -> Result<String, String> {
    use crate::engine::scheduler;

    // 共享准备阶段（消除与 managers/run_manager.rs 的重复）
    let prep = scheduler::prepare_run(
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
    .map_err(|e| e.to_string())?;

    // 发射 Tauri 启动事件
    if let Err(e) = app_handle.emit("run-update", serde_json::json!({
        "run_id": prep.run_id,
        "workflow_id": &workflow_id,
        "workflow_name": &prep.workflow_name,
        "status": "running",
    })) {
        warn!("发送 run-update 事件失败: {}", e);
    }

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

    tauri::async_runtime::spawn(async move {
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
                &workflow, &run_id_clone, Some(&app_handle), &db, approval_store, &[], &ctrl, &timeouts, &shell_allowed,
            ),
        ).await;
        let result = match result {
            Ok(r) => r,
            Err(_elapsed) => {
                warn!("Workflow global timeout (30min): {}", run_id_clone);
                Err(anyhow::anyhow!("Workflow execution timeout (exceeded 30 minutes)"))
            }
        };

        cancel_flags.write().await.remove(&run_id_clone);
        cancel_tokens.write().await.remove(&run_id_clone);
        pause_flags.write().await.remove(&run_id_clone);
        breakpoint_flags.write().await.remove(&run_id_clone);
        step_mode_flags.write().await.remove(&run_id_clone);
        debug_snapshots_cleanup.write().await.remove(&run_id_clone);

        match result {
            Ok(_state) => {
                info!("工作流执行完成: {}", run_id_clone);
            }
            Err(e) => {
                let err_msg = e.to_string();
                let status = if err_msg.contains("cancelled") { "cancelled" } else { "failed" };
                error!("工作流{}: {} - {}", if status == "cancelled" { "已取消" } else { "执行失败" }, run_id_clone, err_msg);
            }
        }
    });

    Ok(prep.run_id)
}

/// 取消运行
///
/// 同时设置 AtomicBool 标志（用于循环轮询检查）和触发 CancellationToken
/// （用于结构化取消，在长时间 I/O 或 sleep 中立即响应）
#[tauri::command]
pub async fn run_cancel(
    app: State<'_, App>,
    run_id: String,
) -> Result<(), String> {
    let flags = app.cancel_flags.read().await;
    let tokens = app.cancel_tokens.read().await;

    if let Some(flag) = flags.get(&run_id) {
        flag.store(true, Ordering::Relaxed);
        // 同时触发取消令牌（配合 tokio::select! 实现即时响应）
        if let Some(token) = tokens.get(&run_id) {
            token.cancel();
        }
        Ok(())
    } else {
        Err("运行不存在或已结束".to_string())
    }
}

/// 暂停运行
#[tauri::command]
pub async fn run_pause(
    app: State<'_, App>,
    app_handle: AppHandle,
    run_id: String,
) -> Result<(), String> {
    let flags = app.pause_flags.read().await;
    if let Some(flag) = flags.get(&run_id) {
        flag.store(true, Ordering::Relaxed);
        if let Err(e) = app_handle.emit("run-update", serde_json::json!({
            "run_id": run_id,
            "status": "paused",
        })) {
            warn!("发送 run-update 事件失败: {}", e);
        }
        Ok(())
    } else {
        Err("运行不存在或已结束".to_string())
    }
}

/// 恢复运行
#[tauri::command]
pub async fn run_resume(
    app: State<'_, App>,
    app_handle: AppHandle,
    run_id: String,
) -> Result<(), String> {
    let flags = app.pause_flags.read().await;
    if let Some(flag) = flags.get(&run_id) {
        flag.store(false, Ordering::Relaxed);
        if let Err(e) = app_handle.emit("run-update", serde_json::json!({
            "run_id": run_id,
            "status": "running",
        })) {
            warn!("发送 run-update 事件失败: {}", e);
        }
        Ok(())
    } else {
        Err("运行不存在或已结束".to_string())
    }
}

#[tauri::command]
pub async fn run_status(
    app: State<'_, App>,
    run_id: String,
) -> Result<RunStatus, String> {
    let run = app.db.get_run(&run_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "运行不存在".to_string())?;

    Ok(RunStatus {
        run_id: run.id,
        workflow_id: run.workflow_id,
        status: run.status,
        current_step: run.current_step,
        started_at: Some(run.started_at),
        finished_at: run.finished_at,
    })
}

#[tauri::command]
pub async fn run_logs(
    app: State<'_, App>,
    run_id: String,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Result<serde_json::Value, String> {
    let mut steps = app.db.get_step_runs(&run_id)
        .map_err(|e| e.to_string())?;

    let total = steps.len();
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(100);

    if offset < total {
        let end = std::cmp::min(offset + limit, total);
        steps = steps[offset..end].to_vec();
    } else {
        steps.clear();
    }

    Ok(serde_json::json!({
        "total": total,
        "offset": offset,
        "steps": steps,
    }))
}

/// 审批响应命令 — 前端通过此命令回复审批请求
#[tauri::command]
pub async fn approval_response(
    app: State<'_, App>,
    approval_id: String,
    approved: bool,
    comment: Option<String>,
    option: Option<String>,
) -> Result<(), String> {
    let option_str = option.unwrap_or_else(|| if approved { "同意".into() } else { "拒绝".into() });
    // 更新 SQLite
    app.db.update_approval_decision(&approval_id, &option_str, comment.as_deref())
        .map_err(|e| e.to_string())?;
    let decision = crate::engine::approval_store::ApprovalDecision {
        option: option_str,
        comment,
    };
    app.approval_store.decide(&approval_id, decision).await
        .map_err(|e| e.to_string())
}

/// 查询所有待审批（供前端 ApprovalCenter 使用）
/// 合并内存（实时等待中的）+ SQLite（重启后恢复的）
#[tauri::command]
pub async fn approval_list_pending(
    app: State<'_, App>,
) -> Result<Vec<crate::engine::approval_store::ApprovalEntry>, String> {
    let mut live = app.approval_store.pending().await;
    let live_ids: std::collections::HashSet<String> = live.iter().map(|e| e.id.clone()).collect();
    if let Ok(db_pending) = app.db.get_pending_approvals() {
        for rec in db_pending {
            if live_ids.contains(&rec.id) { continue; }
            let options: Vec<String> = rec.options.as_deref()
                .unwrap_or("同意,拒绝")
                .split(',').map(|s| s.trim().to_string()).collect();
            let item: Option<serde_json::Value> = rec.item.as_deref()
                .and_then(|s| serde_json::from_str(s).ok());
            live.push(crate::engine::approval_store::ApprovalEntry {
                id: rec.id, run_id: rec.run_id, step_id: rec.step_id,
                title: rec.title, message: rec.message, item, options,
                recommended: rec.recommended, timeout_secs: rec.timeout_secs as u64,
                timeout_action: rec.timeout_action, created_at: rec.created_at,
                recommendation_reason: None, // DB 不持久化此字段
            });
        }
    }
    Ok(live)
}

/// 查询运行历史列表
#[tauri::command]
pub async fn run_list(
    app: State<'_, App>,
    workflow_id: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<RunHistoryItem>, String> {
    app.db.list_runs(workflow_id.as_deref(), limit.unwrap_or(50))
        .map_err(|e| e.to_string())
}

/// 查询单次运行详情（含步骤执行记录）
#[tauri::command]
pub async fn run_detail(
    app: State<'_, App>,
    run_id: String,
) -> Result<RunDetail, String> {
    app.db.get_run_detail(&run_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "运行记录不存在".to_string())
}

/// v4.1: 查询某次运行的全部步骤执行日志（持久化日志行）
#[tauri::command]
pub async fn run_step_logs(
    app: State<'_, App>,
    run_id: String,
) -> Result<Vec<StepLogEntry>, String> {
    app.db.get_step_logs(&run_id).map_err(|e| e.to_string())
}

// ═══════════════════════════════════════════
// 调试命令
// ═══════════════════════════════════════════

/// 单步执行：执行完当前步骤后暂停
#[tauri::command]
pub async fn debug_step(
    app: State<'_, App>,
    app_handle: AppHandle,
    run_id: String,
) -> Result<(), String> {
    let flags = app.step_mode_flags.read().await;
    if let Some(flag) = flags.get(&run_id) {
        flag.store(true, Ordering::Relaxed);
        // 同时清除断点暂停，让调度器继续
        if let Some(bp) = app.breakpoint_flags.read().await.get(&run_id) {
            bp.store(false, Ordering::Relaxed);
        }
        if let Err(e) = app_handle.emit("run-update", serde_json::json!({
            "run_id": run_id,
            "status": "running",
        })) {
            warn!("发送 run-update 事件失败: {}", e);
        }
        Ok(())
    } else {
        Err("运行不存在或已结束".to_string())
    }
}

/// 继续执行：从断点恢复
#[tauri::command]
pub async fn debug_continue(
    app: State<'_, App>,
    app_handle: AppHandle,
    run_id: String,
) -> Result<(), String> {
    // 清除断点暂停和单步模式
    if let Some(bp) = app.breakpoint_flags.read().await.get(&run_id) {
        bp.store(false, Ordering::Relaxed);
    }
    if let Some(sm) = app.step_mode_flags.read().await.get(&run_id) {
        sm.store(false, Ordering::Relaxed);
    }
    if let Err(e) = app_handle.emit("run-update", serde_json::json!({
        "run_id": run_id,
        "status": "running",
    })) {
        warn!("发送 run-update 事件失败: {}", e);
    }
    Ok(())
}

/// 设置断点
#[tauri::command]
pub async fn debug_set_breakpoint(
    app: State<'_, App>,
    workflow_id: String,
    step_id: String,
) -> Result<(), String> {
    // 在数据库中存储断点信息（用 workflow YAML 的 metadata）
    // 简单方案：将断点列表存在 config 的临时字段中
    let key = format!("breakpoints:{}", workflow_id);
    let mut bps: Vec<String> = app.config.read().await
        .get_temp(&key)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    if !bps.contains(&step_id) {
        bps.push(step_id.clone());
        app.config.write().await.set_temp(&key, serde_json::json!(bps));
    }
    Ok(())
}

/// 移除断点
#[tauri::command]
pub async fn debug_remove_breakpoint(
    app: State<'_, App>,
    workflow_id: String,
    step_id: String,
) -> Result<(), String> {
    let key = format!("breakpoints:{}", workflow_id);
    let mut bps: Vec<String> = app.config.read().await
        .get_temp(&key)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    bps.retain(|id| id != &step_id);
    app.config.write().await.set_temp(&key, serde_json::json!(bps));
    Ok(())
}

/// 获取断点列表
#[tauri::command]
pub async fn debug_get_breakpoints(
    app: State<'_, App>,
    workflow_id: String,
) -> Result<Vec<String>, String> {
    let key = format!("breakpoints:{}", workflow_id);
    let bps: Vec<String> = app.config.read().await
        .get_temp(&key)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    Ok(bps)
}

/// 获取调试变量快照（当前执行上下文）
#[tauri::command]
pub async fn debug_vars(
    app: State<'_, App>,
    run_id: String,
) -> Result<serde_json::Value, String> {
    let snapshots = app.debug_snapshots.read().await;
    Ok(snapshots.get(&run_id).cloned().unwrap_or(serde_json::json!({
        "variables": {},
        "step_outputs": {},
    })))
}

// ═══════════════════════════════════════════
// DAG 画布执行命令（P2）
// ═══════════════════════════════════════════

// ═══════════════════════════════════════════
// Web Scrape Preview — 点页面自动填选择器
// ═══════════════════════════════════════════

/// 打开网页预览：返回截图 + 所有可见元素的 CSS 选择器 + 边界框
///
/// 前端在截图上点击元素 → 自动分析选择器 → 填充到 extract 规则
#[tauri::command]
pub async fn web_scrape_preview(
    url: String,
    headless: Option<bool>,
    viewport_width: Option<u32>,
    viewport_height: Option<u32>,
) -> Result<serde_json::Value, String> {
    // WebBridge 无需启动浏览器（扩展已在浏览器中运行）

    // 导航到页面并获取预览数据
    let preview_params = serde_json::json!({
        "url": url,
        "wait_until": "networkidle",
    });

    let result = crate::nodes::webbridge::send_command("evaluate", preview_params).await
        .map_err(|e| format!("页面预览失败: {}", e))?;

    Ok(result)
}
