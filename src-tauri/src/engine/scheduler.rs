// engine/scheduler.rs — 工作流调度器
use crate::engine::workflow::{Workflow, Step};
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::state::RunState;
use crate::data::db::Database;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use anyhow::Result;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

/// 执行工作流（入口函数，由 commands/run.rs 调用）
///
/// 竞态安全：
/// - cancel_flag (AtomicBool) + cancel_token (CancellationToken) 双重取消机制
/// - AtomicBool 用于循环中的高效非阻塞检查
/// - CancellationToken 用于结构化取消，可在 tokio::select! 中实现即时响应
/// - 所有状态标志均通过 tokio::sync::RwLock 保护的 HashMap 共享
pub async fn run_workflow(
    workflow: &Workflow,
    run_id: &str,
    app_handle: &tauri::AppHandle,
    db: &Arc<Database>,
    browser_channel: &str,
    cancel_flag: Arc<AtomicBool>,
    cancel_token: CancellationToken,
    pause_flag: Arc<AtomicBool>,
    breakpoint_flag: Arc<AtomicBool>,
    step_mode_flag: Arc<AtomicBool>,
    debug_snapshots: Arc<tokio::sync::RwLock<std::collections::HashMap<String, serde_json::Value>>>,
) -> Result<RunState> {
    let mut ctx = ExecutionContext::new(run_id, workflow);
    ctx.browser_channel = browser_channel.to_string();
    let mut state = RunState::new(run_id, ctx.variables.clone());
    let executor = StepExecutor::new();

    let workflow_name = workflow.name.clone();
    info!("工作流启动: {} (run_id: {})", workflow_name, run_id);

    if workflow.steps.is_empty() {
        state.mark_completed();
        let _ = db.update_run_status(run_id, "completed", None);
        emit_run_update(app_handle, run_id, &workflow_name, "completed");
        return Ok(state);
    }

    // 获取第一个步骤
    let mut current_id = workflow.steps[0].id.clone();
    let total_steps = workflow.steps.len();

    // 步骤执行循环
    loop {
        // 检查取消（AtomicBool + CancellationToken 双重机制）
        if cancel_flag.load(Ordering::Relaxed) || cancel_token.is_cancelled() {
            warn!("工作流取消: {} (run_id: {})", workflow_name, run_id);
            state.mark_failed();
            let _ = db.update_run_status(run_id, "cancelled", None);
            emit_run_update(app_handle, run_id, &workflow_name, "cancelled");
            return Err(anyhow::anyhow!("cancelled"));
        }

        // 检查暂停（等待恢复，同时响应取消令牌）
        while pause_flag.load(Ordering::Relaxed) {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    state.mark_failed();
                    let _ = db.update_run_status(run_id, "cancelled", None);
                    emit_run_update(app_handle, run_id, &workflow_name, "cancelled");
                    return Err(anyhow::anyhow!("cancelled"));
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {
                    // 超时后继续循环检查
                }
            }
            // 暂停期间也检查取消
            if cancel_flag.load(Ordering::Relaxed) {
                state.mark_failed();
                let _ = db.update_run_status(run_id, "cancelled", None);
                emit_run_update(app_handle, run_id, &workflow_name, "cancelled");
                return Err(anyhow::anyhow!("cancelled"));
            }
        }

        // 查找当前步骤（引用传递，避免循环中 clone 整个 Step）
        let step = match workflow.steps.iter().find(|s| s.id == current_id) {
            Some(s) => s,
            None => {
                state.mark_failed();
                let _ = db.update_run_status(run_id, "failed", Some(&format!("步骤 '{}' 不存在", current_id)));
                emit_run_update(app_handle, run_id, &workflow_name, "failed");
                return Err(anyhow::anyhow!("步骤 '{}' 不存在", current_id));
            }
        };

        // 更新状态 & 持久化
        info!("步骤执行: {} (类型: {})", step.name, step.step_type);
        state.mark_step_running(&current_id);
        let _ = db.create_step_run(run_id, &current_id);
        emit_step_update(app_handle, run_id, &current_id, &step.name, total_steps, "running", None);

        // ─── 断点 / 单步 检查 ───
        if step.breakpoint || step_mode_flag.load(Ordering::Relaxed) {
            // 更新调试快照
            update_debug_snapshot(&debug_snapshots, run_id, &ctx).await;

            // 通知前端：断点命中
            let _ = app_handle.emit("breakpoint-hit", serde_json::json!({
                "run_id": run_id,
                "step_id": current_id,
                "step_name": step.name,
                "variables": ctx.variables,
                "step_outputs": ctx.step_outputs,
            }));

            // 等待恢复（断点暂停或单步暂停，同时响应取消令牌）
            breakpoint_flag.store(true, Ordering::Relaxed);
            while breakpoint_flag.load(Ordering::Relaxed) {
                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        state.mark_failed();
                        let _ = db.update_run_status(run_id, "cancelled", None);
                        emit_run_update(app_handle, run_id, &workflow_name, "cancelled");
                        return Err(anyhow::anyhow!("cancelled"));
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                        // 超时后继续循环检查
                    }
                }
                // 暂停期间也检查取消（AtomicBool 快速路径）
                if cancel_flag.load(Ordering::Relaxed) {
                    state.mark_failed();
                    let _ = db.update_run_status(run_id, "cancelled", None);
                    emit_run_update(app_handle, run_id, &workflow_name, "cancelled");
                    return Err(anyhow::anyhow!("cancelled"));
                }
            }
        }

        // ─── 步骤延迟 ───
        if let Some(delay_ms) = step.delay {
            if delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
        }

        // 审批节点：先 emit 事件通知前端弹窗
        if step.step_type == "approval" {
            emit_approval_required(app_handle, run_id, &step);
        }

        // 执行步骤（带重试 + 超时）
        let result = execute_with_retry(&executor, &step, &mut ctx).await;

        match result {
            Ok(output) => {
                ctx.set_output(&current_id, output.clone());
                state.mark_step_completed(&current_id);
                let _ = db.complete_step_run(run_id, &current_id, Some(&output), None);
                emit_step_update(app_handle, run_id, &current_id, &step.name, total_steps, "completed", Some(&output));

                // 更新调试快照
                update_debug_snapshot(&debug_snapshots, run_id, &ctx).await;

                // 推送变量快照（实时监视）
                emit_variable_snapshot(app_handle, run_id, &ctx);

                // 单步模式：执行完暂停
                if step_mode_flag.load(Ordering::Relaxed) {
                    breakpoint_flag.store(true, Ordering::Relaxed);
                    let _ = app_handle.emit("breakpoint-hit", serde_json::json!({
                        "run_id": run_id,
                        "step_id": current_id,
                        "step_name": step.name,
                        "reason": "step_mode",
                        "variables": ctx.variables,
                        "step_outputs": ctx.step_outputs,
                    }));
                }
            }
            Err(e) => {
                let err_msg = e.to_string();
                warn!("步骤失败: {} - {}", step.name, err_msg);
                state.mark_step_failed(&current_id);
                let _ = db.complete_step_run(run_id, &current_id, None, Some(&err_msg));
                emit_step_update_with_error(app_handle, run_id, &current_id, &step.name, &err_msg);

                // 更新调试快照（含错误信息）
                update_debug_snapshot(&debug_snapshots, run_id, &ctx).await;

                // ─── 错误恢复策略 ───
                let strategy = step.on_error.clone().unwrap_or_default();
                match strategy {
                    crate::engine::workflow::ErrorStrategy::Fail => {
                        state.mark_failed();
                        let _ = db.update_run_status(run_id, "failed", Some(&err_msg));
                        emit_run_update(app_handle, run_id, &workflow_name, "failed");
                        return Err(e);
                    }
                    crate::engine::workflow::ErrorStrategy::Ignore => {
                        info!("步骤 '{}' 错误已忽略，继续执行", step.name);
                        // 记录错误到上下文，输出 null
                        ctx.set_output(&current_id, serde_json::Value::Null);
                        state.mark_step_completed(&current_id);
                        let _ = emit_step_update_ignored(app_handle, run_id, &current_id, &step.name, total_steps, &err_msg);
                        // 推送变量快照
                        emit_variable_snapshot(app_handle, run_id, &ctx);
                        // 继续到下一步
                    }
                    crate::engine::workflow::ErrorStrategy::Branch { step_id: ref branch_id } => {
                        info!("步骤 '{}' 失败，分支跳转到: {}", step.name, branch_id);
                        // 验证目标步骤存在
                        if !workflow.steps.iter().any(|s| s.id == *branch_id) {
                            warn!("分支目标步骤 '{}' 不存在，回退为 fail", branch_id);
                            state.mark_failed();
                            let _ = db.update_run_status(run_id, "failed", Some(&format!("分支目标不存在: {}", branch_id)));
                            emit_run_update(app_handle, run_id, &workflow_name, "failed");
                            return Err(anyhow::anyhow!("分支目标步骤 '{}' 不存在", branch_id));
                        }
                        // 记录错误输出的同时跳转
                        ctx.set_output(&current_id, serde_json::Value::Null);
                        current_id = branch_id.clone();
                        continue; // 跳过 determine_next_step，直接进入循环
                    }
                }
            }
        }

        // 确定下一个步骤
        current_id = match determine_next_step(&step, workflow, &ctx) {
            Some(next_id) => next_id,
            None => {
                // 没有下一步，工作流完成
                info!("工作流完成: {} (run_id: {})", workflow_name, run_id);
                state.mark_completed();
                let _ = db.update_run_status(run_id, "completed", None);
                emit_run_update(app_handle, run_id, &workflow_name, "completed");
                return Ok(state);
            }
        }
    }
}

/// 带重试和超时的步骤执行
async fn execute_with_retry(
    executor: &Arc<StepExecutor>,
    step: &Step,
    ctx: &mut ExecutionContext,
) -> Result<serde_json::Value> {
    let max_retries = step.retry.as_ref().map(|r| r.max).unwrap_or(0);
    let mut last_err = None;

    for attempt in 0..=max_retries {
        let result = if let Some(timeout) = step.timeout {
            let timeout_dur = std::time::Duration::from_secs(timeout as u64);
            match tokio::time::timeout(timeout_dur, executor.execute(step, ctx)).await {
                Ok(r) => r,
                Err(_) => Err(anyhow::anyhow!("步骤 '{}' 超时 ({}秒)", step.name, timeout)),
            }
        } else {
            executor.execute(step, ctx).await
        };

        match result {
            Ok(output) => return Ok(output),
            Err(e) => {
                last_err = Some(e);
                if attempt < max_retries {
                    let delay_ms = step.retry.as_ref().map(|r| r.delay_ms).unwrap_or(1000);
                    let delay = delay_ms * (attempt + 1) as u64; // 线性退避
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    info!("步骤 '{}' 重试 {}/{}", step.name, attempt + 1, max_retries);
                }
            }
        }
    }

    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("执行失败")))
}

/// 确定下一个步骤 ID
fn determine_next_step(step: &Step, workflow: &Workflow, ctx: &ExecutionContext) -> Option<String> {
    // 条件节点：根据输出选择 true_next / false_next
    if step.step_type == "condition" {
        if let Some(output) = ctx.get_output(&step.id) {
            // 条件节点输出 {"result": bool, "branch": "true"/"false"}
            let is_true = output.get("result").and_then(|v| v.as_bool()).unwrap_or(false);
            if is_true {
                if let Some(next) = step.config.get("true_next").and_then(|v| v.as_str()) {
                    return Some(next.to_string());
                }
            } else {
                if let Some(next) = step.config.get("false_next").and_then(|v| v.as_str()) {
                    return Some(next.to_string());
                }
            }
            return None;
        }
    }

    // 循环/并行节点：结束（不自动流转）
    if step.step_type == "loop" || step.step_type == "parallel" || step.step_type == "while" {
        return None;
    }

    // 默认：next 字段或列表中下一个步骤
    if let Some(next) = &step.next {
        Some(next.clone())
    } else {
        let pos = workflow.steps.iter().position(|s| s.id == step.id)?;
        workflow.steps.get(pos + 1).map(|s| s.id.clone())
    }
}

// ─── 事件推送 ───

fn emit_step_update(app: &tauri::AppHandle, run_id: &str, step_id: &str, step_name: &str, total_steps: usize, status: &str, output: Option<&serde_json::Value>) {
    let event = serde_json::json!({
        "run_id": run_id,
        "step_id": step_id,
        "step_name": step_name,
        "total_steps": total_steps,
        "status": status,
        "output": output,
        "error": null,
    });
    let _ = app.emit("step-update", event);
}

/// 执行后推送变量快照，供前端实时监视
fn emit_variable_snapshot(app: &tauri::AppHandle, run_id: &str, ctx: &ExecutionContext) {
    let event = serde_json::json!({
        "run_id": run_id,
        "variables": ctx.variables,
        "step_outputs": ctx.step_outputs,
    });
    let _ = app.emit("variable-update", event);
}

fn emit_step_update_with_error(app: &tauri::AppHandle, run_id: &str, step_id: &str, step_name: &str, error: &str) {
    let event = serde_json::json!({
        "run_id": run_id,
        "step_id": step_id,
        "step_name": step_name,
        "status": "failed",
        "output": null,
        "error": error,
    });
    let _ = app.emit("step-update", event);
}

/// 错误被忽略时的事件（status = ignored, 区别于 failed）
fn emit_step_update_ignored(app: &tauri::AppHandle, run_id: &str, step_id: &str, step_name: &str, total_steps: usize, error: &str) {
    let event = serde_json::json!({
        "run_id": run_id,
        "step_id": step_id,
        "step_name": step_name,
        "total_steps": total_steps,
        "status": "ignored",
        "output": null,
        "error": error,
    });
    let _ = app.emit("step-update", event);
}

fn emit_run_update(app: &tauri::AppHandle, run_id: &str, workflow_name: &str, status: &str) {
    let event = serde_json::json!({
        "run_id": run_id,
        "workflow_name": workflow_name,
        "status": status,
    });
    let _ = app.emit("run-update", event);
}

fn emit_approval_required(app: &tauri::AppHandle, run_id: &str, step: &Step) {
    let approval_id = format!("approval:{}", step.id);
    let event = serde_json::json!({
        "run_id": run_id,
        "step_id": step.id,
        "approval_id": approval_id,
        "message": step.config.get("message").and_then(|v| v.as_str()).unwrap_or("请审批此操作"),
        "options": step.config.get("options").cloned().unwrap_or_else(|| serde_json::json!(["approve", "reject"])),
    });
    let _ = app.emit("approval-required", event);

    // 审批通知增强：窗口隐藏时自动弹出
    use tauri::Manager;
    if let Some(window) = app.get_webview_window("main") {
        let is_visible = window.is_visible().unwrap_or(false);
        if !is_visible {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

/// 更新调试快照：将当前执行上下文存入共享状态
async fn update_debug_snapshot(
    snapshots: &Arc<tokio::sync::RwLock<std::collections::HashMap<String, serde_json::Value>>>,
    run_id: &str,
    ctx: &ExecutionContext,
) {
    let snapshot = serde_json::json!({
        "variables": ctx.variables,
        "step_outputs": ctx.step_outputs,
    });
    snapshots.write().await.insert(run_id.to_string(), snapshot);
}
