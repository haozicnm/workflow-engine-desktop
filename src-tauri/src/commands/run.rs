// commands/run.rs — 执行控制命令
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{State, AppHandle, Emitter};
use crate::App;
use crate::data::models::{RunHistoryItem, RunDetail, StepLogEntry};
use tracing::{info, warn, error};


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
    // 1. 获取工作流 YAML
    let yaml = app.db.get_workflow_yaml(&workflow_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "工作流不存在".to_string())?;

    // 2. 解析工作流
    let workflow = crate::engine::parser::parse_workflow(&yaml)
        .map_err(|e| format!("YAML 解析失败: {}", e))?;

    // 3. 创建 run 记录
    let run_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let workflow_name = workflow.name.clone();
    app.db.create_run(&run_id, &workflow_id, &workflow_name, &now)
        .map_err(|e| e.to_string())?;

    // 4. 读取浏览器通道设置
    let browser_channel = app.config.read().await.browser_channel.clone();

    // 5. 创建取消令牌（支持结构化取消 + AtomicBool 兼容）
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let pause_flag = Arc::new(AtomicBool::new(false));
    let breakpoint_flag = Arc::new(AtomicBool::new(false));
    let step_mode_flag = Arc::new(AtomicBool::new(false));

    // 5.5 获取并发信号量（限制同时运行的工作流数，默认 10）
    let semaphore = app.run_semaphore.clone();
    let permit = match semaphore.clone().try_acquire_owned() {
        Ok(p) => p,
        Err(_) => {
            return Err("已达到最大并发工作流数限制，请等待其他工作流完成后再试".to_string());
        }
    };
    app.cancel_flags.write().await.insert(run_id.clone(), cancel_flag.clone());
    app.cancel_tokens.write().await.insert(run_id.clone(), cancel_token.clone());
    app.pause_flags.write().await.insert(run_id.clone(), pause_flag.clone());
    app.breakpoint_flags.write().await.insert(run_id.clone(), breakpoint_flag.clone());
    app.step_mode_flags.write().await.insert(run_id.clone(), step_mode_flag.clone());

    // 6. 发射 run 启动事件（workflow_name 已在步骤 3 获取）
    if let Err(e) = app_handle.emit("run-update", serde_json::json!({
        "run_id": run_id,
        "workflow_id": workflow_id,
        "workflow_name": workflow_name,
        "status": "running",
    })) {
        warn!("发送 run-update 事件失败: {}", e);
    }

    // 8. 后台异步执行
    let db = app.db.clone();
    let run_id_clone = run_id.clone();
    let wf_name = workflow_name.clone();
    let cancel_flags = app.cancel_flags.clone();
    let cancel_tokens = app.cancel_tokens.clone();
    let pause_flags = app.pause_flags.clone();
    let breakpoint_flags = app.breakpoint_flags.clone();
    let step_mode_flags = app.step_mode_flags.clone();
    let debug_snapshots = app.debug_snapshots.clone();
    let debug_snapshots_cleanup = debug_snapshots.clone();
    tauri::async_runtime::spawn(async move {
        // permit 在此持有，任务结束时自动释放
        let _permit = permit;
        let ctrl = crate::engine::scheduler::RunControl {
            cancel_flag,
            cancel_token,
            pause_flag,
            breakpoint_flag,
            step_mode_flag,
            debug_snapshots,
        };
        let result = crate::engine::scheduler::run_workflow(
            &workflow, &run_id_clone, &app_handle, &db, &browser_channel, &ctrl,
        ).await;

        // 清理标志和令牌
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
                // 取消不算真正失败
                let status = if err_msg.contains("cancelled") { "cancelled" } else { "failed" };
                error!("工作流{}: {} - {}", if status == "cancelled" { "已取消" } else { "执行失败" }, run_id_clone, err_msg);
                if let Err(e) = app_handle.emit("run-update", serde_json::json!({
                    "run_id": run_id_clone,
                    "workflow_name": wf_name,
                    "status": status,
                    "error": err_msg,
                })) {
                    warn!("发送 run-update 事件失败: {}", e);
                }
            }
        }
    });

    Ok(run_id)
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
    approval_id: String,
    approved: bool,
) -> Result<(), String> {
    crate::nodes::approval::get_approval_manager().await
        .respond(&approval_id, approved).await
        .map_err(|e| e.to_string())
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
    use crate::nodes::browser;

    let headless = headless.unwrap_or(true);

    // 启动浏览器 sidecar（复用全局实例）
    let mut launch_params = serde_json::json!({
        "headless": headless,
        "channel": "auto",
    });
    if let (Some(w), Some(h)) = (viewport_width, viewport_height) {
        launch_params["viewport"] = serde_json::json!({"width": w, "height": h});
    }

    let _launch = browser::send_sidecar_action("launch", &launch_params).await
        .map_err(|e| format!("启动浏览器失败: {}. 预览需要浏览器环境", e))?;

    // 导航到页面并获取预览数据
    let preview_params = serde_json::json!({
        "url": url,
        "wait_until": "networkidle",
    });

    let result = browser::send_sidecar_action("preview", &preview_params).await
        .map_err(|e| format!("页面预览失败: {}", e))?;

    Ok(result)
}

// ═══════════════════════════════════════════
// DAG 执行命令
// ═══════════════════════════════════════════

/// DAG 执行入口：接收前端画布节点和连线，构建 DAG 执行计划并后台执行
///
/// 接收 FlowEditor 画布上的全部节点和连线：
///   - nodes: Vec<FlowNode>  — 画布节点列表
///   - edges: Vec<FlowEdge>  — 连线列表
///   - workflow_name: String — 工作流名称
///
/// 返回 run_id，前端通过监听 step-update / run-update 事件追踪执行进度
#[tauri::command]
pub async fn run_dag_start(
    app: State<'_, App>,
    app_handle: AppHandle,
    nodes: Vec<crate::engine::dag::FlowNode>,
    edges: Vec<crate::engine::dag::FlowEdge>,
    workflow_name: String,
) -> Result<String, String> {
    use crate::engine::dag::build_dag;

    // 1. 构建 DAG 执行计划（验证 + 拓扑排序）
    let plan = build_dag(&nodes, &edges)
        .map_err(|e| format!("DAG 构建失败: {}", e))?;

    info!("DAG 执行计划构建完成: {} 节点, {} 连线, {} 并行组",
        plan.ordered_steps.len(), edges.len(), plan.parallel_groups.len());

    // 2. 创建 run 记录
    let run_id = uuid::Uuid::new_v4().to_string();
    let workflow_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    app.db.create_run(&run_id, &workflow_id, &workflow_name, &now)
        .map_err(|e| e.to_string())?;

    // 3. 读取浏览器通道设置
    let browser_channel = app.config.read().await.browser_channel.clone();

    // 4. 创建控制标志
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let pause_flag = Arc::new(AtomicBool::new(false));
    let breakpoint_flag = Arc::new(AtomicBool::new(false));
    let step_mode_flag = Arc::new(AtomicBool::new(false));

    // 5. 并发控制
    let semaphore = app.run_semaphore.clone();
    let permit = match semaphore.clone().try_acquire_owned() {
        Ok(p) => p,
        Err(_) => {
            return Err("已达到最大并发工作流数限制，请等待其他工作流完成后再试".to_string());
        }
    };
    app.cancel_flags.write().await.insert(run_id.clone(), cancel_flag.clone());
    app.cancel_tokens.write().await.insert(run_id.clone(), cancel_token.clone());
    app.pause_flags.write().await.insert(run_id.clone(), pause_flag.clone());
    app.breakpoint_flags.write().await.insert(run_id.clone(), breakpoint_flag.clone());
    app.step_mode_flags.write().await.insert(run_id.clone(), step_mode_flag.clone());

    // 6. 发射开始事件
    if let Err(e) = app_handle.emit("dag-run-start", serde_json::json!({
        "run_id": run_id,
        "workflow_name": workflow_name,
        "node_count": plan.ordered_steps.len(),
        "edge_count": edges.len(),
    })) {
        warn!("发送 dag-run-start 事件失败: {}", e);
    }

    // 7. 后台异步执行
    let db = app.db.clone();
    let run_id_clone = run_id.clone();
    let wf_name = workflow_name.clone();
    let cancel_flags = app.cancel_flags.clone();
    let cancel_tokens = app.cancel_tokens.clone();
    let pause_flags = app.pause_flags.clone();
    let breakpoint_flags = app.breakpoint_flags.clone();
    let step_mode_flags = app.step_mode_flags.clone();
    let debug_snapshots = app.debug_snapshots.clone();
    let debug_snapshots_cleanup = debug_snapshots.clone();

    tauri::async_runtime::spawn(async move {
        let _permit = permit;

        let ctrl = crate::engine::scheduler::RunControl {
            cancel_flag,
            cancel_token,
            pause_flag,
            breakpoint_flag,
            step_mode_flag,
            debug_snapshots,
        };

        let result = crate::engine::dag_scheduler::run_dag(
            &plan,
            &run_id_clone,
            &app_handle,
            &db,
            &browser_channel,
            &wf_name,
            &ctrl,
        ).await;

        // 清理
        cancel_flags.write().await.remove(&run_id_clone);
        cancel_tokens.write().await.remove(&run_id_clone);
        pause_flags.write().await.remove(&run_id_clone);
        breakpoint_flags.write().await.remove(&run_id_clone);
        step_mode_flags.write().await.remove(&run_id_clone);
        debug_snapshots_cleanup.write().await.remove(&run_id_clone);

        match result {
            Ok(_state) => {
                info!("[DAG] 执行完成: {}", run_id_clone);
            }
            Err(e) => {
                let err_msg = e.to_string();
                let status = if err_msg.contains("cancelled") { "cancelled" } else { "failed" };
                error!("[DAG] 工作流{}: {} - {}", if status == "cancelled" { "已取消" } else { "执行失败" }, run_id_clone, err_msg);
                if let Err(e) = app_handle.emit("run-update", serde_json::json!({
                    "run_id": run_id_clone,
                    "workflow_name": wf_name,
                    "status": status,
                    "error": err_msg,
                })) {
                    warn!("发送 run-update 事件失败: {}", e);
                }
            }
        }
    });

    Ok(run_id)
}
