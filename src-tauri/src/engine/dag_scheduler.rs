// engine/dag_scheduler.rs — DAG 工作流调度执行器
//
// 提供两种 DAG 执行入口：
//   1. run_dag(plan)         — P2 新增：按 ExecutionPlan 拓扑顺序执行
//   2. run_dag_workflow(dag) — 旧版：DAGWorkflow → 自动构建计划并执行

use crate::engine::dag::{DAGWorkflow, ExecutionPlan, ExecStep};
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::{Step, ErrorStrategy, Workflow};
use crate::engine::state::RunState;
use crate::engine::scheduler::RunControl;
use crate::data::db::Database;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use anyhow::Result;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn, error};

// ═══════════════════════════════════════════════════════════════
// P2: run_dag — 按拓扑排序执行 ExecutionPlan
// ═══════════════════════════════════════════════════════════════

/// 执行 DAG 执行计划
///
/// 流程：
///   1. 按拓扑顺序遍历 ordered_steps
///   2. 并行组内的步骤通过 tokio::join! 并发执行
///   3. 执行结果存入 ctx.step_outputs，供后续步骤解析模板
///   4. 通过 Tauri events 实时推送步骤状态
///   5. 支持取消、暂停、断点
///   6. 支持 on_error 策略（fail / ignore / branch）
///   7. 支持 retry + timeout
pub async fn run_dag(
    plan: &ExecutionPlan,
    run_id: &str,
    app_handle: &tauri::AppHandle,
    db: &Arc<Database>,
    browser_channel: &str,
    workflow_name: &str,
    ctrl: &RunControl,
) -> Result<RunState> {
    let executor = StepExecutor::new();

    let workflow = Workflow {
        name: workflow_name.to_string(),
        description: None,
        steps: vec![],
        variables: None,
    };

    let mut ctx = ExecutionContext::new(run_id, &workflow);
    ctx.browser_channel = browser_channel.to_string();
    let mut state = RunState::new(run_id, ctx.variables.clone());

    let total_steps = plan.ordered_steps.len();
    info!(
        "[DAG] 工作流启动: {} (run_id: {}, {} 节点, {} 并行组)",
        workflow_name, run_id, total_steps, plan.parallel_groups.len()
    );

    if total_steps == 0 {
        state.mark_completed();
        let _ = db.update_run_status(run_id, "completed", None);
        emit_run_update(app_handle, run_id, workflow_name, "completed");
        return Ok(state);
    }

    // 构建并行组索引集合，快速查询
    let parallel_indices: HashMap<usize, usize> = {
        let mut m = HashMap::new();
        for (gid, group) in plan.parallel_groups.iter().enumerate() {
            for &idx in group {
                m.insert(idx, gid);
            }
        }
        m
    };

    let mut step_index = 0;
    while step_index < total_steps {
        // ── 检查取消 ──
        if ctrl.cancel_flag.load(Ordering::Relaxed) || ctrl.cancel_token.is_cancelled() {
            warn!("[DAG] 工作流取消: {} (run_id: {})", workflow_name, run_id);
            state.mark_failed();
            if let Err(e) = db.update_run_status(run_id, "cancelled", None) {
                warn!("[DAG] 更新运行状态失败: {}", e);
            }
            emit_run_update(app_handle, run_id, workflow_name, "cancelled");
            return Err(anyhow::anyhow!("cancelled"));
        }

        // ── 检查暂停 ──
        while ctrl.pause_flag.load(Ordering::Relaxed) {
            tokio::select! {
                _ = ctrl.cancel_token.cancelled() => {
                    state.mark_failed();
                    if let Err(e) = db.update_run_status(run_id, "cancelled", None) {
                warn!("[DAG] 更新运行状态失败: {}", e);
            }
                    emit_run_update(app_handle, run_id, workflow_name, "cancelled");
                    return Err(anyhow::anyhow!("cancelled"));
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {}
            }
            if ctrl.cancel_flag.load(Ordering::Relaxed) {
                state.mark_failed();
                if let Err(e) = db.update_run_status(run_id, "cancelled", None) {
                warn!("[DAG] 更新运行状态失败: {}", e);
            }
                emit_run_update(app_handle, run_id, workflow_name, "cancelled");
                return Err(anyhow::anyhow!("cancelled"));
            }
        }

        // ── 并行组？并发执行组内所有步骤 ──
        if let Some(&group_id) = parallel_indices.get(&step_index) {
            let group = &plan.parallel_groups[group_id];
            info!("[DAG] 并行组 #{}: {:?} ({} 节点)", group_id, group, group.len());

            let parallel_results = run_parallel_group(
                &executor,
                group,
                &plan.ordered_steps,
                &mut ctx,
                app_handle,
                db,
                run_id,
                workflow_name,
                total_steps,
                ctrl,
            ).await;

            // 处理并行组结果
            for (idx, result) in parallel_results {
                let exec_step = &plan.ordered_steps[idx];
                match result {
                    Ok(output) => {
                        ctx.set_output(&exec_step.node_id, output.clone());
                        state.mark_step_completed(&exec_step.node_id);
                        let _ = db.complete_step_run(run_id, &exec_step.node_id, Some(&output), None);
                        emit_step_update(
                            app_handle, run_id, &exec_step.node_id, &exec_step.step.name,
                            total_steps, "completed", Some(&output),
                        );
                        update_debug_snapshot(&ctrl.debug_snapshots, run_id, &ctx).await;
                        emit_variable_snapshot(app_handle, run_id, &ctx);
                    }
                    Err(e) => {
                        let err_msg = e.to_string();
                        warn!("[DAG] 并行节点失败: {} - {}", exec_step.step.name, err_msg);
                        state.mark_step_failed(&exec_step.node_id);
                        let _ = db.complete_step_run(run_id, &exec_step.node_id, None, Some(&err_msg));
                        emit_step_update_with_error(
                            app_handle, run_id, &exec_step.node_id, &exec_step.step.name, &err_msg,
                        );

                        update_debug_snapshot(&ctrl.debug_snapshots, run_id, &ctx).await;

                        let strategy = exec_step.step.on_error.clone().unwrap_or_default();
                        let result = handle_step_error(
                            &strategy, &exec_step.step.name, &err_msg,
                            app_handle, run_id, workflow_name, db, &mut state,
                        );
                        result?
                        // Ignore 或 Branch 已内部处理，继续
                    }
                }
            }

            // 跳过组内已处理的步骤
            step_index = group.iter().max().copied().unwrap_or(step_index) + 1;
            // 单步模式：并行组执行完后暂停
            if ctrl.step_mode_flag.load(Ordering::Relaxed) {
                ctrl.breakpoint_flag.store(true, Ordering::Relaxed);
                if let Err(e) = app_handle.emit("breakpoint-hit", serde_json::json!({
                    "run_id": run_id,
                    "step_id": "dag_parallel_group",
                    "reason": "step_mode",
                    "variables": ctx.variables,
                    "step_outputs": ctx.step_outputs,
                })) {
                    warn!("[DAG] 发送 breakpoint-hit 事件失败: {}", e);
                }
            }
            continue;
        }

        // ── 单步：顺序执行 ──
        let exec_step = &plan.ordered_steps[step_index];
        let step = &exec_step.step;

        // 断点 / 单步检查
        if step.breakpoint || ctrl.step_mode_flag.load(Ordering::Relaxed) {
            update_debug_snapshot(&ctrl.debug_snapshots, run_id, &ctx).await;

            if let Err(e) = app_handle.emit("breakpoint-hit", serde_json::json!({
                "run_id": run_id,
                "step_id": step.id,
                "step_name": step.name,
                "variables": ctx.variables,
                "step_outputs": ctx.step_outputs,
            })) {
                warn!("[DAG] 发送 breakpoint-hit 事件失败: {}", e);
            }

            ctrl.breakpoint_flag.store(true, Ordering::Relaxed);
            while ctrl.breakpoint_flag.load(Ordering::Relaxed) {
                tokio::select! {
                    _ = ctrl.cancel_token.cancelled() => {
                        state.mark_failed();
                        if let Err(e) = db.update_run_status(run_id, "cancelled", None) {
                warn!("[DAG] 更新运行状态失败: {}", e);
            }
                        emit_run_update(app_handle, run_id, workflow_name, "cancelled");
                        return Err(anyhow::anyhow!("cancelled"));
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
                }
                if ctrl.cancel_flag.load(Ordering::Relaxed) {
                    state.mark_failed();
                    if let Err(e) = db.update_run_status(run_id, "cancelled", None) {
                warn!("[DAG] 更新运行状态失败: {}", e);
            }
                    emit_run_update(app_handle, run_id, workflow_name, "cancelled");
                    return Err(anyhow::anyhow!("cancelled"));
                }
            }
        }

        // 延迟
        if let Some(delay_ms) = step.delay {
            if delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
        }

        // 审批节点
        if step.step_type == "approval" {
            emit_approval_required(app_handle, run_id, step);
        }

        // 执行
        info!("[DAG] 步骤执行: {} (类型: {})", step.name, step.step_type);
        state.mark_step_running(&step.id);
        let _ = db.create_step_run(run_id, &step.id);
        emit_step_update(app_handle, run_id, &step.id, &step.name, total_steps, "running", None);

        let result = execute_with_retry(&executor, step, &mut ctx).await;

        match result {
            Ok(output) => {
                ctx.set_output(&step.id, output.clone());
                state.mark_step_completed(&step.id);
                let _ = db.complete_step_run(run_id, &step.id, Some(&output), None);
                emit_step_update(
                    app_handle, run_id, &step.id, &step.name,
                    total_steps, "completed", Some(&output),
                );
                update_debug_snapshot(&ctrl.debug_snapshots, run_id, &ctx).await;
                emit_variable_snapshot(app_handle, run_id, &ctx);

                if ctrl.step_mode_flag.load(Ordering::Relaxed) {
                    ctrl.breakpoint_flag.store(true, Ordering::Relaxed);
                    if let Err(e) = app_handle.emit("breakpoint-hit", serde_json::json!({
                        "run_id": run_id,
                        "step_id": step.id,
                        "step_name": step.name,
                        "reason": "step_mode",
                        "variables": ctx.variables,
                        "step_outputs": ctx.step_outputs,
                    })) {
                        warn!("[DAG] 发送 breakpoint-hit 事件失败: {}", e);
                    }
                }
            }
            Err(e) => {
                let err_msg = e.to_string();
                warn!("[DAG] 步骤失败: {} - {}", step.name, err_msg);
                state.mark_step_failed(&step.id);
                let _ = db.complete_step_run(run_id, &step.id, None, Some(&err_msg));
                emit_step_update_with_error(app_handle, run_id, &step.id, &step.name, &err_msg);
                update_debug_snapshot(&ctrl.debug_snapshots, run_id, &ctx).await;

                let strategy = step.on_error.clone().unwrap_or_default();
                let result = handle_step_error(
                    &strategy, &step.name, &err_msg,
                    app_handle, run_id, workflow_name, db, &mut state,
                );
                result?
                // Ignore/Branch 已处理，继续
            }
        }

        step_index += 1;
    }

    // 全部完成
    info!("[DAG] 工作流完成: {} (run_id: {})", workflow_name, run_id);
    state.mark_completed();
    let _ = db.update_run_status(run_id, "completed", None);
    emit_run_update(app_handle, run_id, workflow_name, "completed");
    Ok(state)
}

/// 并行执行一个步骤组
///
/// 每个子步骤独立执行，互不依赖。通过 tokio::spawn 并发启动，
/// 然后 join_all 收集结果。
#[allow(clippy::too_many_arguments)]
async fn run_parallel_group(
    executor: &Arc<StepExecutor>,
    indices: &[usize],
    ordered_steps: &[ExecStep],
    _ctx: &mut ExecutionContext,
    app_handle: &tauri::AppHandle,
    db: &Arc<Database>,
    run_id: &str,
    workflow_name: &str,
    total_steps: usize,
    ctrl: &RunControl,
) -> Vec<(usize, Result<serde_json::Value>)> {
    // 并发执行组内所有步骤
    let mut handles = Vec::new();

    for &idx in indices {
        let step = &ordered_steps[idx];
        let step_owned = step.step.clone();
        let executor_clone = executor.clone();
        let cancel_flag_c = ctrl.cancel_flag.clone();
        let cancel_token_c = ctrl.cancel_token.clone();
        let pause_flag_c = ctrl.pause_flag.clone();
        let app_handle_c = app_handle.clone();
        let db_c = db.clone();
        let run_id_c = run_id.to_string();
        let workflow_name_c = workflow_name.to_string();

        handles.push(tokio::spawn(async move {
            let mut local_ctx = ExecutionContext::new(
                &run_id_c,
                &Workflow {
                    name: workflow_name_c.clone(),
                    description: None,
                    steps: vec![],
                    variables: None,
                },
            );

            // 检查取消
            if cancel_flag_c.load(Ordering::Relaxed) || cancel_token_c.is_cancelled() {
                return (idx, Err(anyhow::anyhow!("cancelled")));
            }

            // 暂停等待
            while pause_flag_c.load(Ordering::Relaxed) {
                tokio::select! {
                    _ = cancel_token_c.cancelled() => {
                        return (idx, Err(anyhow::anyhow!("cancelled")));
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {}
                }
            }

            info!("[DAG:parallel] 执行: {} (索引 {})", step_owned.name, idx);
            let _ = db_c.create_step_run(&run_id_c, &step_owned.id);
            emit_step_update(
                &app_handle_c, &run_id_c, &step_owned.id, &step_owned.name,
                total_steps, "running", None,
            );

            let result = execute_with_retry(&executor_clone, &step_owned, &mut local_ctx).await;

            // 合并 local_ctx 的结果到主 ctx（由调用方处理）

            (idx, result)
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(r) => results.push(r),
            Err(e) => {
                // tokio 任务失败
                error!("[DAG:parallel] 任务 panicked: {}", e);
                // 找出这个任务的索引（从 indices 中取第一个未使用的）
                let used: std::collections::HashSet<usize> =
                    results.iter().map(|(i, _)| *i).collect();
                for &idx in indices {
                    if !used.contains(&idx) {
                        results.push((idx, Err(anyhow::anyhow!("internal error: {}", e))));
                        break;
                    }
                }
            }
        }
    }

    results
}

/// 处理步骤错误（按 on_error 策略）
#[allow(clippy::too_many_arguments)]
fn handle_step_error(
    strategy: &ErrorStrategy,
    step_name: &str,
    err_msg: &str,
    app_handle: &tauri::AppHandle,
    run_id: &str,
    workflow_name: &str,
    db: &Arc<Database>,
    state: &mut RunState,
) -> Result<()> {
    match strategy {
        ErrorStrategy::Fail => {
            error!("[DAG] 步骤 '{}' 失败，工作流终止", step_name);
            state.mark_failed();
            let _ = db.update_run_status(run_id, "failed", Some(err_msg));
            emit_run_update(app_handle, run_id, workflow_name, "failed");
            Err(anyhow::anyhow!("步骤 '{}' 执行失败: {}", step_name, err_msg))
        }
        ErrorStrategy::Ignore => {
            info!("[DAG] 步骤 '{}' 错误已忽略: {}", step_name, err_msg);
            // 不终止，调用方继续
            Ok(())
        }
        ErrorStrategy::Branch { step_id } => {
            warn!(
                "[DAG] 步骤 '{}' 失败，branch 目标: {}（DAG 模式暂不支持 branch）",
                step_name, step_id
            );
            // DAG 模式下 branch 语义复杂（可能跳转到依赖未满足的节点），
            // 当前回退为 Ignore
            Ok(())
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
            let timeout_dur = std::time::Duration::from_secs(timeout);
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
                    let delay = delay_ms * (attempt + 1) as u64;
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    info!("[DAG] 步骤 '{}' 重试 {}/{}", step.name, attempt + 1, max_retries);
                }
            }
        }
    }

    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("执行失败")))
}

// ═══════════════════════════════════════════════════════════════
// 旧版: run_dag_workflow（向后兼容 dag_run_start 命令）
// ═══════════════════════════════════════════════════════════════

/// DAG 执行返回
#[derive(Debug)]
pub struct DAGRunResult {
    pub completed: bool,
    pub node_outputs: HashMap<String, serde_json::Value>,
}

/// 执行 DAG 工作流（旧版 JSON 格式入口，保留向后兼容）
#[allow(clippy::too_many_arguments)]
pub async fn run_dag_workflow(
    dag: &DAGWorkflow,
    run_id: &str,
    app_handle: &tauri::AppHandle,
    db: &Arc<Database>,
    browser_channel: &str,
    cancel_flag: Arc<AtomicBool>,
    cancel_token: CancellationToken,
    pause_flag: Arc<AtomicBool>,
    step_mode: bool,
) -> Result<DAGRunResult> {
    if dag.nodes.is_empty() {
        let _ = db.update_run_status(run_id, "completed", None);
        return Ok(DAGRunResult {
            completed: true,
            node_outputs: HashMap::new(),
        });
    }

    let plan = dag
        .build_execution_plan()
        .map_err(|e| anyhow::anyhow!(e))?;

    let breakpoint_flag = Arc::new(AtomicBool::new(false));
    let step_mode_flag = Arc::new(AtomicBool::new(step_mode));
    let snapshots = Arc::new(tokio::sync::RwLock::new(HashMap::new()));

    let dag_ctrl = RunControl {
        cancel_flag,
        cancel_token,
        pause_flag,
        breakpoint_flag,
        step_mode_flag,
        debug_snapshots: snapshots,
    };

    let result = run_dag(
        &plan,
        run_id,
        app_handle,
        db,
        browser_channel,
        &dag.name,
        &dag_ctrl,
    )
    .await;

    match result {
        Ok(state) => Ok(DAGRunResult {
            completed: state.status == "completed",
            node_outputs: state
                .steps.keys().map(|k| (k.clone(), serde_json::Value::Null))
                .collect(),
        }),
        Err(e) => {
            let err_msg = e.to_string();
            if err_msg.contains("cancelled") {
                emit_dag_complete(app_handle, run_id, &dag.name, "cancelled", None);
            } else {
                emit_dag_complete(app_handle, run_id, &dag.name, "failed", Some(&err_msg));
            }
            Err(e)
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// 事件发射辅助
// ═══════════════════════════════════════════════════════════════

fn emit_step_update(
    app: &tauri::AppHandle,
    run_id: &str,
    step_id: &str,
    step_name: &str,
    total_steps: usize,
    status: &str,
    output: Option<&serde_json::Value>,
) {
    let event = serde_json::json!({
        "run_id": run_id,
        "step_id": step_id,
        "step_name": step_name,
        "total_steps": total_steps,
        "status": status,
        "output": output,
        "error": null,
    });
    if let Err(e) = app.emit("step-update", event) {
        warn!("[DAG] 发送 step-update 事件失败: {}", e);
    }

    // v3: 同时发射 step-status-update 事件（前端状态可视化用）
    let current_step = if status == "running" { 0 } else { 0 }; // 由前端自行计数
    let status_event = serde_json::json!({
        "step_id": step_id,
        "step_name": step_name,
        "status": status,
        "current_step": current_step,
        "total_steps": total_steps,
    });
    let _ = app.emit("step-status-update", status_event);

    // v3: 成功时发射 step-output 事件（输出内联预览用）
    if status == "completed" || status == "success" {
        if let Some(out) = output {
            let output_event = serde_json::json!({
                "step_id": step_id,
                "output": out,
            });
            let _ = app.emit("step-output", output_event);
        }
    }
}

fn emit_step_update_with_error(
    app: &tauri::AppHandle,
    run_id: &str,
    step_id: &str,
    step_name: &str,
    error: &str,
) {
    let event = serde_json::json!({
        "run_id": run_id,
        "step_id": step_id,
        "step_name": step_name,
        "status": "failed",
        "output": null,
        "error": error,
    });
    if let Err(e) = app.emit("step-update", event) {
        warn!("[DAG] 发送 step-update 事件失败: {}", e);
    }

    // v3: 同时发射 step-status-update + step-error
    let status_event = serde_json::json!({
        "step_id": step_id,
        "step_name": step_name,
        "status": "error",
        "current_step": 0,
        "total_steps": 0,
    });
    let _ = app.emit("step-status-update", status_event);

    let error_event = serde_json::json!({
        "step_id": step_id,
        "step_name": step_name,
        "error": error,
    });
    let _ = app.emit("step-error", error_event);
}

fn emit_run_update(
    app: &tauri::AppHandle,
    run_id: &str,
    workflow_name: &str,
    status: &str,
) {
    let event = serde_json::json!({
        "run_id": run_id,
        "workflow_name": workflow_name,
        "status": status,
    });
    if let Err(e) = app.emit("run-update", event) {
        warn!("[DAG] 发送 run-update 事件失败: {}", e);
    }
}

fn emit_approval_required(app: &tauri::AppHandle, run_id: &str, step: &Step) {
    let event = serde_json::json!({
        "run_id": run_id,
        "step_id": step.id,
        "approval_id": format!("approval:{}", step.id),
        "message": step.config.get("message").and_then(|v| v.as_str()).unwrap_or("请审批此操作"),
        "options": step.config.get("options").cloned().unwrap_or_else(|| serde_json::json!(["approve", "reject"])),
    });
    if let Err(e) = app.emit("approval-required", event) {
        warn!("[DAG] 发送 approval-required 事件失败: {}", e);
    }
}

fn emit_variable_snapshot(app: &tauri::AppHandle, run_id: &str, ctx: &ExecutionContext) {
    let event = serde_json::json!({
        "run_id": run_id,
        "variables": ctx.variables,
        "step_outputs": ctx.step_outputs,
    });
    if let Err(e) = app.emit("variable-update", event) {
        warn!("[DAG] 发送 variable-update 事件失败: {}", e);
    }
}

async fn update_debug_snapshot(
    snapshots: &Arc<tokio::sync::RwLock<HashMap<String, serde_json::Value>>>,
    run_id: &str,
    ctx: &ExecutionContext,
) {
    let snapshot = serde_json::json!({
        "variables": ctx.variables,
        "step_outputs": ctx.step_outputs,
    });
    snapshots.write().await.insert(run_id.to_string(), snapshot);
}

fn emit_dag_complete(
    app_handle: &tauri::AppHandle,
    run_id: &str,
    workflow_name: &str,
    status: &str,
    error: Option<&str>,
) {
    let mut payload = serde_json::json!({
        "run_id": run_id,
        "workflow_name": workflow_name,
        "status": status,
    });
    if let Some(e) = error {
        payload["error"] = serde_json::json!(e);
    }
    if let Err(e) = app_handle.emit("dag-run-complete", payload) {
        warn!("[DAG] 发送 dag-run-complete 事件失败: {}", e);
    }
}
