use std::sync::Arc;
// server/handlers.rs — HTTP API 请求处理器
//
// 每个 handler 对应原 commands/ 中的一个 Tauri 命令。
// 签名模式: async fn handler(State(app): State<Arc<App>>, ...) -> impl IntoResponse

use axum::{
    extract::{Path, Query},
    response::{IntoResponse, Response, Json, sse::{Event, Sse}},
    http::StatusCode,
};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use tokio_stream::StreamExt;
use tracing::{info, warn, error};

use crate::server::events;

// ═══════════════════════════════════════════════════════════
// Helper: convert Result<T, String> → axum Response
// ═══════════════════════════════════════════════════════════

fn ok_response<T: Serialize>(data: T) -> Response {
    Json(data).into_response()
}

fn err_response(status: StatusCode, msg: impl Into<String>) -> Response {
    let body = serde_json::json!({ "error": msg.into() });
    (status, Json(body)).into_response()
}

fn map_err_resp<T: Serialize>(result: Result<T, String>) -> Response {
    match result {
        Ok(data) => ok_response(data),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

fn map_not_found_resp<T: Serialize>(result: Result<Option<T>, String>) -> Response {
    match result {
        Ok(Some(data)) => ok_response(data),
        Ok(None) => err_response(StatusCode::NOT_FOUND, "Not found"),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

// ═══════════════════════════════════════════════════════════
// Request body types
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct WorkflowCreateBody {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowUpdateBody {
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowLockBody {
    pub locked: bool,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowSaveYamlBody {
    pub yaml: String,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowValidateBody {
    pub yaml: String,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowAutoOrderBody {
    pub yaml: String,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowExportBody {
    pub name: String,
    pub description: Option<String>,
    pub nodes: Vec<serde_json::Value>,
    pub edges: Vec<serde_json::Value>,
    pub variables: Option<serde_json::Value>,
    pub output_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowImportBody {
    pub yaml_content: String,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowCreateFromRecordingBody {
    pub actions: Vec<serde_json::Value>,
    pub workflow_name: String,
    pub source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RunStartBody {
    pub workflow_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ApprovalRespondBody {
    pub approval_id: String,
    pub approved: bool,
    pub comment: Option<String>,
    pub option: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SettingsUpdateBody {
    pub theme: String,
    pub language: String,
    pub auto_start: bool,
    pub log_level: String,
    pub python_path: Option<String>,
    pub browser_channel: String,
    pub browser_executable_path: String,
    pub working_dir: String,
}

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

#[derive(Debug, Deserialize)]
pub struct StepTestBody {
    pub step_type: String,
    pub config: serde_json::Value,
    pub variables: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct DebugSetBreakpointBody {
    pub workflow_id: String,
    pub step_id: String,
}

#[derive(Debug, Deserialize)]
pub struct PluginInstallBody {
    pub wfplug_path: String,
}

#[derive(Debug, Deserialize)]
pub struct PluginUninstallBody {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct PipelineRunBody {
    pub excel_path: Option<String>,
    pub template_path: Option<String>,
    pub output_path: Option<String>,
    pub use_browser: Option<bool>,
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

#[derive(Debug, Deserialize)]
pub struct PreviewExcelBody {
    pub path: String,
    pub sheet: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PreviewWordBody {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct WebScrapePreviewBody {
    pub url: String,
    pub headless: Option<bool>,
    pub viewport_width: Option<u32>,
    pub viewport_height: Option<u32>,
}

// ═══════════════════════════════════════════════════════════
// 工作流 CRUD handler
// ═══════════════════════════════════════════════════════════

pub async fn workflow_list(
) -> Response {
    let app = crate::server::state::get();
    map_err_resp(app.db.list_workflows().map_err(|e| format!("Failed to list workflows: {e}")))
}

pub async fn workflow_create(
    Json(body): Json<WorkflowCreateBody>,
) -> Response {
    let app = crate::server::state::get();
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    match app.db.create_workflow(
        &id,
        &body.name,
        body.description.as_deref().unwrap_or(""),
        &now,
        &now,
    ) {
        Ok(()) => {
            events::emit("workflow-changed", serde_json::json!({
                "action": "create",
                "workflow_id": &id,
                "workflow_name": &body.name,
            }));
            ok_response(serde_json::json!({ "id": id }))
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create workflow (name={}): {e}", body.name)),
    }
}

pub async fn workflow_get(
    Path(id): Path<String>,
) -> Response {
    let app = crate::server::state::get();
    map_not_found_resp(app.db.get_workflow(&id).map_err(|e| format!("Failed to get workflow (id={id}): {e}")))
}

pub async fn workflow_update(
    Path(id): Path<String>,
    Json(body): Json<WorkflowUpdateBody>,
) -> Response {
    let app = crate::server::state::get();
    let now = chrono::Utc::now().to_rfc3339();
    match app.db.update_workflow(&id, body.name.as_deref(), body.description.as_deref(), body.enabled, &now) {
        Ok(()) => {
            events::emit("workflow-changed", serde_json::json!({
                "action": "update",
                "workflow_id": &id,
            }));
            ok_response(serde_json::json!({ "success": true }))
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to update workflow (id={id}): {e}")),
    }
}

pub async fn workflow_delete(
    Path(id): Path<String>,
) -> Response {
    let app = crate::server::state::get();
    let wf = match app.db.get_workflow(&id) {
        Ok(Some(wf)) => wf,
        Ok(None) => return err_response(StatusCode::NOT_FOUND, "Workflow not found"),
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    if wf.locked {
        return err_response(StatusCode::CONFLICT, "Workflow is locked, cannot delete");
    }
    match app.db.delete_workflow(&id) {
        Ok(()) => {
            events::emit("workflow-changed", serde_json::json!({
                "action": "delete",
                "workflow_id": &id,
            }));
            ok_response(serde_json::json!({ "success": true }))
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete workflow (id={id}): {e}")),
    }
}

pub async fn workflow_lock(
    Path(id): Path<String>,
    Json(body): Json<WorkflowLockBody>,
) -> Response {
    let app = crate::server::state::get();
    match app.db.set_workflow_locked(&id, body.locked) {
        Ok(()) => {
            events::emit("workflow-changed", serde_json::json!({
                "action": if body.locked { "lock" } else { "unlock" },
                "workflow_id": &id,
            }));
            ok_response(serde_json::json!({ "success": true }))
        }
        Err(e) => err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to {} workflow: {e}", if body.locked { "lock" } else { "unlock" }),
        ),
    }
}

pub async fn workflow_save_yaml(
    Path(id): Path<String>,
    Json(body): Json<WorkflowSaveYamlBody>,
) -> Response {
    let app = crate::server::state::get();
    let wf = match crate::engine::parser::parse_workflow(&body.yaml) {
        Ok(wf) => wf,
        Err(e) => return err_response(StatusCode::BAD_REQUEST, format!("Failed to parse workflow YAML: {e}")),
    };
    let validation = crate::engine::validate::validate_workflow(&wf);
    if !validation.valid {
        return err_response(
            StatusCode::BAD_REQUEST,
            format!("Workflow validation failed:\n{}", validation.errors.join("\n")),
        );
    }
    if !validation.warnings.is_empty() {
        for w in &validation.warnings {
            warn!("Workflow validation warning: {}", w);
        }
    }

    match app.db.save_workflow_yaml(&id, &body.yaml) {
        Ok(()) => {
            events::emit("workflow-changed", serde_json::json!({
                "action": "save",
                "workflow_id": &id,
            }));
            ok_response(serde_json::json!({ "success": true }))
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save workflow YAML (id={id}): {e}")),
    }
}

pub async fn workflow_validate(
    Json(body): Json<WorkflowValidateBody>,
) -> Response {
    match crate::engine::parser::parse_workflow(&body.yaml) {
        Ok(wf) => ok_response(serde_json::json!({
            "valid": true,
            "workflow": {
                "name": wf.name,
                "step_count": wf.steps.len(),
            }
        })),
        Err(e) => ok_response(serde_json::json!({
            "valid": false,
            "error": e.to_string()
        })),
    }
}

pub async fn workflow_auto_order(
    Json(body): Json<WorkflowAutoOrderBody>,
) -> Response {
    let wf = match crate::engine::parser::parse_workflow(&body.yaml) {
        Ok(wf) => wf,
        Err(e) => return err_response(StatusCode::BAD_REQUEST, format!("解析工作流失败: {e}")),
    };
    let order = crate::engine::parser::auto_order_steps(&wf.steps);
    ok_response(serde_json::json!({
        "order": order,
        "steps": order.iter().map(|&i| &wf.steps[i].id).collect::<Vec<_>>(),
    }))
}

pub async fn workflow_export(
    Json(body): Json<WorkflowExportBody>,
) -> Response {
    use crate::engine::workflow::Step;

    let steps: Vec<Step> = body.nodes.iter().map(|n| {
        let node_type = n.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
        let label = n.get("label").and_then(|v| v.as_str()).unwrap_or("Unnamed");
        let id = n.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
        let config = n.get("config").cloned().unwrap_or(serde_json::json!({}));

        Step {
            id: id.to_string(),
            name: label.to_string(),
            step_type: node_type.to_string(),
            config,
            next: None,
            retry: None,
            timeout: None,
            body_steps: None,
            breakpoint: false,
            delay: None,
            on_error: None,
            actions: None,
            expanded: None,
            condition: None,
            condition_group: None,
            run_condition: None,
        }
    }).collect();

    let vars: Option<std::collections::HashMap<String, serde_json::Value>> =
        body.variables.and_then(|v| serde_json::from_value(v).ok());

    let wf = crate::engine::workflow::Workflow {
        name: body.name.clone(),
        description: body.description.clone(),
        steps,
        variables: vars,
    };

    let mut yaml_value = match serde_yaml::to_value(&wf) {
        Ok(v) => v,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("序列化 YAML 失败: {e}")),
    };

    if !body.edges.is_empty() {
        if let Some(map) = yaml_value.as_mapping_mut() {
            if let Ok(edges_yaml) = serde_yaml::to_value(&body.edges) {
                map.insert(
                    serde_yaml::Value::String("edges".to_string()),
                    edges_yaml,
                );
            }
        }
    }

    let yaml_str = match serde_yaml::to_string(&yaml_value) {
        Ok(s) => s,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("生成 YAML 失败: {e}")),
    };

    let final_path = if let Some(path) = body.output_path {
        std::path::PathBuf::from(&path)
    } else {
        let sanitized: String = body.name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        std::env::current_dir()
            .unwrap_or_default()
            .join(format!("{}.workflow.yaml", sanitized))
    };

    if let Some(parent) = final_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create directory: {e}"));
        }
    }

    if let Err(e) = std::fs::write(&final_path, &yaml_str) {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("写入文件失败: {e}"));
    }

    ok_response(serde_json::json!({
        "success": true,
        "path": final_path.to_string_lossy().to_string(),
        "yaml": yaml_str,
        "step_count": body.nodes.len(),
        "edge_count": body.edges.len(),
    }))
}

pub async fn workflow_import(
    Json(body): Json<WorkflowImportBody>,
) -> Response {
    use crate::engine::workflow::Workflow;

    let wf: Workflow = match serde_yaml::from_str(&body.yaml_content) {
        Ok(wf) => wf,
        Err(e) => return err_response(StatusCode::BAD_REQUEST, format!("YAML 解析失败: {e}")),
    };

    let edges_value: Option<Vec<serde_json::Value>> =
        serde_yaml::from_str::<serde_yaml::Value>(&body.yaml_content)
            .ok()
            .and_then(|v| v.get("edges").cloned())
            .and_then(|e| serde_yaml::from_value(e).ok());

    let nodes_json: Vec<serde_json::Value> = wf.steps.iter().map(|s| {
        serde_json::json!({
            "id": s.id,
            "type": s.step_type,
            "label": s.name,
            "config": s.config,
        })
    }).collect();

    let variables_json = wf.variables.map(|v| serde_json::to_value(v).unwrap_or_default());

    ok_response(serde_json::json!({
        "success": true,
        "name": wf.name,
        "description": wf.description,
        "nodes": nodes_json,
        "edges": edges_value.unwrap_or_default(),
        "variables": variables_json,
        "step_count": wf.steps.len(),
    }))
}

pub async fn workflow_create_from_recording(
    Json(body): Json<WorkflowCreateFromRecordingBody>,
) -> Response {
    let app = crate::server::state::get();
    use crate::engine::recording_converter::{self, RecordedAction, RecordingSource};

    let recorded_actions: Vec<RecordedAction> = body.actions
        .iter()
        .filter_map(|a| serde_json::from_value(a.clone()).ok())
        .collect();

    let src = match body.source.as_deref() {
        Some("desktop") => RecordingSource::Desktop,
        Some("mixed") => RecordingSource::Mixed,
        _ => RecordingSource::Browser,
    };

    let conversion = recording_converter::convert_actions_to_workflow(
        &recorded_actions, &body.workflow_name, src,
    );

    if conversion.yaml.is_empty() {
        return err_response(StatusCode::BAD_REQUEST, "Recording is empty, cannot generate workflow");
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    if let Err(e) = app.db.create_workflow(&id, &body.workflow_name, "由录制操作生成", &now, &now) {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create workflow: {e}"));
    }
    if let Err(e) = app.db.save_workflow_yaml(&id, &conversion.yaml) {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save workflow YAML: {e}"));
    }

    ok_response(serde_json::json!({
        "id": id,
        "name": body.workflow_name,
        "yaml": conversion.yaml,
        "step_count": conversion.step_count,
        "action_count": conversion.action_count,
        "merged_count": conversion.merged_count,
        "step_summary": conversion.step_summary,
    }))
}

// ═══════════════════════════════════════════════════════════
// 执行控制 handler
// ═══════════════════════════════════════════════════════════

pub async fn run_start(
    Json(body): Json<RunStartBody>,
) -> Response {
    let app = crate::server::state::get();
    let workflow_id = body.workflow_id;

    // 1. 获取工作流 YAML
    let yaml = match app.db.get_workflow_yaml(&workflow_id) {
        Ok(Some(y)) => y,
        Ok(None) => return err_response(StatusCode::NOT_FOUND, "工作流不存在"),
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    // 2. 解析工作流
    let workflow = match crate::engine::parser::parse_workflow(&yaml) {
        Ok(wf) => wf,
        Err(e) => return err_response(StatusCode::BAD_REQUEST, format!("YAML 解析失败: {e}")),
    };

    // 3. 创建 run 记录
    let run_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let workflow_name = workflow.name.clone();
    if let Err(e) = app.db.create_run(&run_id, &workflow_id, &workflow_name, &now) {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }

    // 4. 读取浏览器通道设置
    let browser_channel = app.config.read().await.browser_channel.clone();

    // 5. 创建取消/暂停/断点/单步标志
    use std::sync::atomic::AtomicBool;
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let pause_flag = Arc::new(AtomicBool::new(false));
    let breakpoint_flag = Arc::new(AtomicBool::new(false));
    let step_mode_flag = Arc::new(AtomicBool::new(false));

    // 5.5 获取并发信号量
    let semaphore = app.run_semaphore.clone();
    let permit = match semaphore.clone().try_acquire_owned() {
        Ok(p) => p,
        Err(_) => {
            return err_response(
                StatusCode::TOO_MANY_REQUESTS,
                "已达到最大并发工作流数限制，请等待其他工作流完成后再试",
            );
        }
    };

    app.cancel_flags.write().await.insert(run_id.clone(), cancel_flag.clone());
    app.cancel_tokens.write().await.insert(run_id.clone(), cancel_token.clone());
    app.pause_flags.write().await.insert(run_id.clone(), pause_flag.clone());
    app.breakpoint_flags.write().await.insert(run_id.clone(), breakpoint_flag.clone());
    app.step_mode_flags.write().await.insert(run_id.clone(), step_mode_flag.clone());

    // 6. 发射 run 启动事件
    events::emit("run-update", serde_json::json!({
        "run_id": &run_id,
        "workflow_id": &workflow_id,
        "workflow_name": &workflow_name,
        "status": "running",
    }));

    // 7. 后台异步执行
    let db = app.db.clone();
    let approval_store = app.approval_store.clone();
    let run_id_clone = run_id.clone();
    let cancel_flags = app.cancel_flags.clone();
    let cancel_tokens = app.cancel_tokens.clone();
    let pause_flags = app.pause_flags.clone();
    let breakpoint_flags = app.breakpoint_flags.clone();
    let step_mode_flags = app.step_mode_flags.clone();
    let debug_snapshots = app.debug_snapshots.clone();
    let debug_snapshots_cleanup = debug_snapshots.clone();

    tokio::spawn(async move {
        let _permit = permit;
        let ctrl = crate::engine::scheduler::RunControl {
            cancel_flag,
            cancel_token,
            pause_flag,
            breakpoint_flag,
            step_mode_flag,
            debug_snapshots,
        };
        let global_timeout = std::time::Duration::from_secs(30 * 60);
        let result = tokio::time::timeout(
            global_timeout,
            crate::engine::scheduler::run_workflow(
                &workflow, &run_id_clone, None, &db, approval_store, &browser_channel, &[], &ctrl,
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

    ok_response(serde_json::json!({ "run_id": run_id }))
}

pub async fn run_cancel(
    Path(run_id): Path<String>,
) -> Response {
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

pub async fn run_pause(
    Path(run_id): Path<String>,
) -> Response {
    let app = crate::server::state::get();
    let flags = app.pause_flags.read().await;
    match flags.get(&run_id) {
        Some(flag) => {
            use std::sync::atomic::Ordering;
            flag.store(true, Ordering::Relaxed);
            events::emit("run-update", serde_json::json!({
                "run_id": &run_id,
                "status": "paused",
            }));
            ok_response(serde_json::json!({ "success": true }))
        }
        None => err_response(StatusCode::NOT_FOUND, "运行不存在或已结束"),
    }
}

pub async fn run_resume(
    Path(run_id): Path<String>,
) -> Response {
    let app = crate::server::state::get();
    let flags = app.pause_flags.read().await;
    match flags.get(&run_id) {
        Some(flag) => {
            use std::sync::atomic::Ordering;
            flag.store(false, Ordering::Relaxed);
            events::emit("run-update", serde_json::json!({
                "run_id": &run_id,
                "status": "running",
            }));
            ok_response(serde_json::json!({ "success": true }))
        }
        None => err_response(StatusCode::NOT_FOUND, "运行不存在或已结束"),
    }
}

pub async fn run_status(
    Path(run_id): Path<String>,
) -> Response {
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

pub async fn run_list(
    Query(query): Query<RunListQuery>,
) -> Response {
    let app = crate::server::state::get();
    map_err_resp(app.db.list_runs(
        query.workflow_id.as_deref(),
        query.limit.unwrap_or(50),
    ).map_err(|e| e.to_string()))
}

pub async fn run_detail(
    Path(run_id): Path<String>,
) -> Response {
    let app = crate::server::state::get();
    match app.db.get_run_detail(&run_id) {
        Ok(Some(detail)) => ok_response(detail),
        Ok(None) => err_response(StatusCode::NOT_FOUND, "运行记录不存在"),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn run_logs(
    Path(run_id): Path<String>,
    Query(query): Query<RunLogsQuery>,
) -> Response {
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

pub async fn run_step_logs(
    Path(run_id): Path<String>,
) -> Response {
    let app = crate::server::state::get();
    map_err_resp(app.db.get_step_logs(&run_id).map_err(|e| e.to_string()))
}

// ═══════════════════════════════════════════════════════════
// 审批 handler
// ═══════════════════════════════════════════════════════════

pub async fn approval_list_pending(
) -> Response {
    let app = crate::server::state::get();
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
                id: rec.id,
                run_id: rec.run_id,
                step_id: rec.step_id,
                title: rec.title,
                message: rec.message,
                item,
                options,
                recommended: rec.recommended,
                timeout_secs: rec.timeout_secs as u64,
                timeout_action: rec.timeout_action,
                created_at: rec.created_at,
                recommendation_reason: None,
            });
        }
    }

    ok_response(live)
}

pub async fn approval_respond(
    Json(body): Json<ApprovalRespondBody>,
) -> Response {
    let app = crate::server::state::get();
    let option_str = body.option.unwrap_or_else(|| {
        if body.approved { "同意".into() } else { "拒绝".into() }
    });

    if let Err(e) = app.db.update_approval_decision(&body.approval_id, &option_str, body.comment.as_deref()) {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }

    let decision = crate::engine::approval_store::ApprovalDecision {
        option: option_str,
        comment: body.comment,
    };

    match app.approval_store.decide(&body.approval_id, decision).await {
        Ok(()) => ok_response(serde_json::json!({ "success": true })),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

// ═══════════════════════════════════════════════════════════
// SSE handler
// ═══════════════════════════════════════════════════════════

pub async fn events_sse() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = events::get_tx().subscribe();
    let stream = tokio_stream::wrappers::BroadcastStream::new(rx)
        .filter_map(|result| {
            match result {
                Ok((event, data)) => {
                    let data_str = data.to_string();
                    Some(Ok(Event::default().event(event).data(data_str)))
                }
                Err(_) => None,
            }
        });
    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("ping"),
    )
}

// ═══════════════════════════════════════════════════════════
// 系统 handler
// ═══════════════════════════════════════════════════════════

pub async fn system_health() -> Response {
    ok_response(serde_json::json!({
        "status": "ok",
        "version": "7.1.0",
    }))
}

pub async fn node_list_types() -> Response {
    let mut types: Vec<String> = crate::nodes::registry::all_nodes()
        .into_iter().map(|n| n.node_type).collect();
    for t in crate::nodes::mcp_node::get_all_mcp_types() {
        if !types.contains(&t) {
            types.push(t);
        }
    }
    ok_response(types)
}

pub async fn settings_get(
) -> Response {
    let app = crate::server::state::get();
    let config = app.config.read().await;
    ok_response(serde_json::json!({
        "theme": config.theme,
        "language": config.language,
        "auto_start": config.auto_start,
        "log_level": config.log_level,
        "python_path": config.python_path,
        "browser_channel": config.browser_channel,
        "browser_executable_path": config.browser_executable_path,
        "working_dir": config.working_dir,
    }))
}

pub async fn settings_update(
    Json(body): Json<SettingsUpdateBody>,
) -> Response {
    let app = crate::server::state::get();
    let mut config = app.config.write().await;
    config.theme = body.theme;
    config.language = body.language;
    config.auto_start = body.auto_start;
    config.log_level = body.log_level;
    config.python_path = body.python_path;
    config.browser_channel = body.browser_channel;
    config.browser_executable_path = body.browser_executable_path;
    config.working_dir = body.working_dir;
    info!("设置已更新");
    match config.save() {
        Ok(()) => ok_response(serde_json::json!({ "success": true })),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn system_check_browser() -> Response {
    let system_python = which::which("python3")
        .or_else(|_| which::which("python"))
        .ok()
        .map(|p| p.to_string_lossy().to_string());

    #[cfg(target_os = "windows")]
    let scanned_python: Option<String> = {
        use std::path::PathBuf;
        let candidates: [PathBuf; 3] = [
            PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default()).join("Programs").join("Python"),
            PathBuf::from(std::env::var("PROGRAMFILES").unwrap_or_default()).join("Python"),
            PathBuf::from("C:\\Python"),
        ];
        let mut found: Vec<PathBuf> = Vec::new();
        for base in &candidates {
            if !base.exists() { continue }
            if let Ok(entries) = std::fs::read_dir(base) {
                for e in entries.flatten() {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.starts_with("Python3") {
                        let py = e.path().join("python.exe");
                        if py.exists() { found.push(py); }
                    }
                }
            }
        }
        found.sort_by(|a, b| b.cmp(a));
        found.into_iter().next().map(|p| p.to_string_lossy().to_string())
    };
    #[cfg(not(target_os = "windows"))]
    let scanned_python: Option<String> = None;

    let best_python = scanned_python.or(system_python);

    let has_edge = {
        #[cfg(target_os = "windows")]
        {
            let pf_x86 = std::env::var("PROGRAMFILES(X86)").unwrap_or_default();
            let pf = std::env::var("PROGRAMFILES").unwrap_or_default();
            let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
            std::path::PathBuf::from(&pf_x86).join("Microsoft/Edge/Application/msedge.exe").exists()
                || std::path::PathBuf::from(&pf).join("Microsoft/Edge/Application/msedge.exe").exists()
                || std::path::PathBuf::from(&local).join("Microsoft/Edge/Application/msedge.exe").exists()
        }
        #[cfg(not(target_os = "windows"))]
        { which::which("microsoft-edge").is_ok() }
    };

    let has_chrome = {
        #[cfg(target_os = "windows")]
        {
            let pf = std::env::var("PROGRAMFILES").unwrap_or_default();
            let pf_x86 = std::env::var("PROGRAMFILES(X86)").unwrap_or_default();
            let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
            which::which("chrome").is_ok()
                || std::path::PathBuf::from(&pf).join("Google/Chrome/Application/chrome.exe").exists()
                || std::path::PathBuf::from(&pf_x86).join("Google/Chrome/Application/chrome.exe").exists()
                || std::path::PathBuf::from(&local).join("Google/Chrome/Application/chrome.exe").exists()
        }
        #[cfg(not(target_os = "windows"))]
        {
            which::which("google-chrome-stable").is_ok()
                || which::which("google-chrome").is_ok()
                || which::which("chromium-browser").is_ok()
                || which::which("chromium").is_ok()
        }
    };

    let python_available = best_python.is_some();
    let has_system_browser = has_edge || has_chrome;

    let has_playwright_pkg = if let Some(ref py) = best_python {
        let mut cmd = std::process::Command::new(py);
        cmd.args(["-c", "import playwright; print('ok')"]);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        cmd.output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else { false };

    let has_playwright_chromium = if let Ok(exe) = std::env::current_exe() {
        exe.parent()
            .map(|d| d.join("playwright-browsers"))
            .map(|p| p.exists() && p.read_dir().ok()
                .map(|mut entries| entries.any(|e| e.ok()
                    .map(|f| f.file_name().to_string_lossy().starts_with("chromium-"))
                    .unwrap_or(false)))
                .unwrap_or(false))
            .unwrap_or(false)
    } else { false };

    let has_playwright_cache = if let Some(ref py) = best_python {
        let mut cmd = std::process::Command::new(py);
        cmd.args(["-c", r#"
import os, sys
home = os.environ.get('PLAYWRIGHT_BROWSERS_PATH',
    os.path.join(os.environ.get('LOCALAPPDATA', ''), 'ms-playwright') if sys.platform == 'win32'
    else os.path.join(os.path.expanduser('~'), '.cache', 'ms-playwright'))
dirs = [d for d in os.listdir(home) if d.startswith('chromium-')] if os.path.exists(home) else []
print('ok' if dirs else 'missing')
"#]);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        cmd.output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "ok")
            .unwrap_or(false)
    } else { false };

    let has_browser = has_system_browser || has_playwright_chromium || has_playwright_cache;
    let ready = python_available && has_playwright_pkg && has_browser;

    ok_response(serde_json::json!({
        "python_available": python_available,
        "system_python": best_python,
        "has_playwright_pkg": has_playwright_pkg,
        "has_playwright_chromium": has_playwright_chromium,
        "has_playwright_cache": has_playwright_cache,
        "has_edge": has_edge,
        "has_chrome": has_chrome,
        "has_system_browser": has_system_browser,
        "has_browser": has_browser,
        "ready": ready,
    }))
}

pub async fn get_log_path() -> Response {
    let log_dir = crate::data::paths::resolve_log_dir();
    ok_response(serde_json::json!({ "path": log_dir.to_string_lossy().to_string() }))
}

pub async fn open_log_dir() -> Response {
    let log_dir = crate::data::paths::resolve_log_dir();
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }
    // 使用系统命令打开目录
    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open").arg(&log_dir).spawn();
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(&log_dir).spawn();
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("explorer").arg(&log_dir).spawn();
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    let result = std::process::Command::new("xdg-open").arg(&log_dir).spawn();

    match result {
        Ok(_) => ok_response(serde_json::json!({ "success": true })),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("打开日志目录失败: {e}")),
    }
}

pub async fn clear_logs() -> Response {
    let log_dir = crate::data::paths::resolve_log_dir();
    if log_dir.exists() {
        let entries = match std::fs::read_dir(&log_dir) {
            Ok(e) => e,
            Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("log") {
                if let Err(e) = std::fs::remove_file(&path) {
                    return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
                }
            }
        }
    }
    info!("日志已清空");
    ok_response(serde_json::json!({ "success": true }))
}

pub async fn check_ipc() -> Response {
    use tokio::net::TcpStream;
    use tokio::time::{timeout, Duration};
    use std::net::SocketAddr;
    let addr: SocketAddr = match "127.0.0.1:19527".parse() {
        Ok(a) => a,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    match timeout(Duration::from_secs(2), TcpStream::connect(addr)).await {
        Ok(Ok(_)) => ok_response(serde_json::json!({ "alive": true })),
        _ => ok_response(serde_json::json!({ "alive": false })),
    }
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

// ═══════════════════════════════════════════════════════════
// 预览 handler
// ═══════════════════════════════════════════════════════════

pub async fn preview_excel(
    Json(body): Json<PreviewExcelBody>,
) -> Response {
    let config = serde_json::json!({
        "path": &body.path,
        "sheet": body.sheet,
        "action": "read",
    });
    match crate::nodes::excel::excel_read(&body.path, &config).await {
        Ok(v) => ok_response(v),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("Excel 预览失败: {e}")),
    }
}

pub async fn preview_word(
    Json(body): Json<PreviewWordBody>,
) -> Response {
    match crate::nodes::word::word_read(&body.path).await {
        Ok(v) => ok_response(v),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("Word 预览失败: {e}")),
    }
}

pub async fn get_trajectory(
    Path(run_id): Path<String>,
) -> Response {
    let trajectory = crate::engine::preview::read_trajectory(&run_id);
    ok_response(trajectory)
}

pub async fn get_bundle_files(
    Path((run_id, step_id)): Path<(String, String)>,
) -> Response {
    let bundle_dir = crate::engine::preview::preview_dir(&run_id).join("bundles").join(&step_id);
    if !bundle_dir.exists() {
        return ok_response(Vec::<String>::new());
    }
    let mut files = Vec::new();
    match std::fs::read_dir(&bundle_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    files.push(name.to_string());
                }
            }
        }
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("读取 bundle 目录失败: {e}")),
    }
    files.sort();
    ok_response(files)
}

pub async fn read_bundle_file(
    Path((run_id, step_id, filename)): Path<(String, String, String)>,
) -> Response {
    let preview_dir = crate::engine::preview::preview_dir(&run_id);
    let path = preview_dir.join("bundles").join(&step_id).join(&filename);
    if !path.starts_with(&preview_dir) {
        return err_response(StatusCode::FORBIDDEN, "非法文件路径");
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => ok_response(serde_json::json!({ "content": content })),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("读取文件失败: {e}")),
    }
}

// ═══════════════════════════════════════════════════════════
// Step test / debug / recording handler
// ═══════════════════════════════════════════════════════════

pub async fn step_test(
    Json(body): Json<StepTestBody>,
) -> Response {
    let app = crate::server::state::get();
    use crate::engine::context::ExecutionContext;
    use crate::engine::executor::StepExecutor;
    use crate::engine::workflow::{Step, Workflow};

    let step = Step {
        id: "test_step".to_string(),
        name: "测试步骤".to_string(),
        step_type: body.step_type.clone(),
        config: body.config.clone(),
        next: None,
        timeout: None,
        retry: None,
        body_steps: None,
        breakpoint: false,
        delay: None,
        on_error: None,
        actions: None,
        expanded: None,
        condition: None,
        condition_group: None,
        run_condition: None,
    };

    let wf = Workflow {
        name: "test".to_string(),
        description: None,
        steps: vec![],
        variables: body.variables,
    };
    let mut ctx = ExecutionContext::new("test", &wf);

    let executor = StepExecutor::new(app.approval_store.clone(), app.db.clone());
    match executor.execute(&step, &mut ctx).await {
        Ok(output) => ok_response(serde_json::json!({
            "success": true,
            "output": output,
        })),
        Err(e) => ok_response(serde_json::json!({
            "success": false,
            "error": e.to_string(),
        })),
    }
}

pub async fn recording_status() -> Response {
    #[cfg(feature = "gui")]
    {
        let result = crate::nodes::recording::get_recording_status().await;
        ok_response(result)
    }
    #[cfg(not(feature = "gui"))]
    {
        ok_response(serde_json::json!({
            "recording": false,
            "message": "Recording is only available in GUI mode"
        }))
    }
}

// ═══════════════════════════════════════════════════════════
// 调试 handler
// ═══════════════════════════════════════════════════════════

pub async fn debug_step(
    Path(run_id): Path<String>,
) -> Response {
    let app = crate::server::state::get();
    let flags = app.step_mode_flags.read().await;
    match flags.get(&run_id) {
        Some(flag) => {
            use std::sync::atomic::Ordering;
            flag.store(true, Ordering::Relaxed);
            if let Some(bp) = app.breakpoint_flags.read().await.get(&run_id) {
                bp.store(false, Ordering::Relaxed);
            }
            events::emit("run-update", serde_json::json!({
                "run_id": &run_id,
                "status": "running",
            }));
            ok_response(serde_json::json!({ "success": true }))
        }
        None => err_response(StatusCode::NOT_FOUND, "运行不存在或已结束"),
    }
}

pub async fn debug_continue(
    Path(run_id): Path<String>,
) -> Response {
    let app = crate::server::state::get();
    use std::sync::atomic::Ordering;
    if let Some(bp) = app.breakpoint_flags.read().await.get(&run_id) {
        bp.store(false, Ordering::Relaxed);
    }
    if let Some(sm) = app.step_mode_flags.read().await.get(&run_id) {
        sm.store(false, Ordering::Relaxed);
    }
    events::emit("run-update", serde_json::json!({
        "run_id": &run_id,
        "status": "running",
    }));
    ok_response(serde_json::json!({ "success": true }))
}

pub async fn debug_set_breakpoint(
    Json(body): Json<DebugSetBreakpointBody>,
) -> Response {
    let app = crate::server::state::get();
    let key = format!("breakpoints:{}", body.workflow_id);
    let mut bps: Vec<String> = app.config.read().await
        .get_temp(&key)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    if !bps.contains(&body.step_id) {
        bps.push(body.step_id.clone());
        app.config.write().await.set_temp(&key, serde_json::json!(bps));
    }
    ok_response(serde_json::json!({ "success": true }))
}

pub async fn debug_remove_breakpoint(
    Json(body): Json<DebugSetBreakpointBody>,
) -> Response {
    let app = crate::server::state::get();
    let key = format!("breakpoints:{}", body.workflow_id);
    let mut bps: Vec<String> = app.config.read().await
        .get_temp(&key)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    bps.retain(|id| id != &body.step_id);
    app.config.write().await.set_temp(&key, serde_json::json!(bps));
    ok_response(serde_json::json!({ "success": true }))
}

pub async fn debug_get_breakpoints(
    Path(workflow_id): Path<String>,
) -> Response {
    let app = crate::server::state::get();
    let key = format!("breakpoints:{}", workflow_id);
    let bps: Vec<String> = app.config.read().await
        .get_temp(&key)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    ok_response(bps)
}

pub async fn debug_vars(
    Path(run_id): Path<String>,
) -> Response {
    let app = crate::server::state::get();
    let snapshots = app.debug_snapshots.read().await;
    ok_response(snapshots.get(&run_id).cloned().unwrap_or(serde_json::json!({
        "variables": {},
        "step_outputs": {},
    })))
}

// ═══════════════════════════════════════════════════════════
// 插件 handler
// ═══════════════════════════════════════════════════════════

pub async fn plugin_list() -> Response {
    let plugins = match crate::engine::plugin_manager::list_plugins() {
        Ok(p) => p,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("获取插件列表失败: {e}")),
    };

    let items: Vec<serde_json::Value> = plugins.iter().map(|p| {
        serde_json::json!({
            "name": p.name,
            "version": p.version,
            "title": p.title,
            "description": p.description,
            "author": p.author,
            "icon": p.icon,
            "mcp_count": p.mcp_mappings.len(),
            "template_count": p.templates.len(),
        })
    }).collect();

    ok_response(serde_json::json!({ "plugins": items }))
}

pub async fn plugin_install(
    Json(body): Json<PluginInstallBody>,
) -> Response {
    let path = std::path::Path::new(&body.wfplug_path);
    let meta = match crate::engine::plugin_manager::install_plugin(path) {
        Ok(m) => m,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("安装失败: {e}")),
    };

    ok_response(serde_json::json!({
        "success": true,
        "plugin": {
            "name": meta.name,
            "version": meta.version,
            "title": meta.title,
            "description": meta.description,
            "mcp_count": meta.mcp_mappings.len(),
            "template_count": meta.templates.len(),
        }
    }))
}

pub async fn plugin_uninstall(
    Json(body): Json<PluginUninstallBody>,
) -> Response {
    match crate::engine::plugin_manager::uninstall_plugin(&body.name) {
        Ok(()) => ok_response(serde_json::json!({
            "success": true,
            "message": format!("插件 {} 已卸载", body.name),
        })),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("卸载失败: {e}")),
    }
}

pub async fn plugin_pick_file() -> Response {
    err_response(StatusCode::NOT_IMPLEMENTED, "plugin_pick_file is not available in standalone server mode — use the REST API directly")
}

// ═══════════════════════════════════════════════════════════
// Pipeline handler
// ═══════════════════════════════════════════════════════════

pub async fn run_pipeline(
    Json(body): Json<PipelineRunBody>,
) -> Response {
    use std::process::Command as StdCommand;
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;

    let base = crate::data::paths::resolve_data_dir().join("examples");
    let script_path = base.join("run_full_pipeline.py");

    if !script_path.exists() {
        let project_examples = std::path::PathBuf::from("examples");
        if project_examples.exists() {
            if let Err(e) = std::fs::create_dir_all(&base) {
                return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
            }
            for file in &["run_full_pipeline.py", "test_data.xlsx", "report_template.docx"] {
                let src = project_examples.join(file);
                let dst = base.join(file);
                if src.exists() {
                    if let Err(e) = std::fs::copy(&src, &dst) {
                        return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("复制 {file} 失败: {e}"));
                    }
                }
            }
        }
    }

    let script = if script_path.exists() {
        script_path.to_string_lossy().to_string()
    } else {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("管道脚本不存在: {:?}", script_path));
    };

    let mut args = vec![script];
    if let Some(ep) = &body.excel_path {
        args.push("--excel".into());
        args.push(ep.clone());
    }
    if let Some(tp) = &body.template_path {
        args.push("--template".into());
        args.push(tp.clone());
    }
    if let Some(op) = &body.output_path {
        args.push("--output".into());
        args.push(op.clone());
    }
    if body.use_browser.unwrap_or(false) {
        args.push("--browser".into());
        args.push("--headless".into());
    }

    let mut cmd = StdCommand::new("python");
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);
    let output = match cmd.args(&args).output() {
        Ok(o) => o,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("执行 Python 失败: {e}")),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("管道执行失败:\n{}\n{}", stdout, stderr));
    }

    for line in stdout.lines() {
        if let Some(json_str) = line.strip_prefix("__RESULT_JSON__:") {
            if let Ok(result) = serde_json::from_str::<serde_json::Value>(json_str) {
                return ok_response(result);
            }
        }
    }

    ok_response(serde_json::json!({
        "success": true,
        "output": stdout,
    }))
}

// ═══════════════════════════════════════════════════════════
// Web scrape preview handler
// ═══════════════════════════════════════════════════════════

pub async fn web_scrape_preview(
    Json(body): Json<WebScrapePreviewBody>,
) -> Response {
    use crate::nodes::browser;

    let headless = body.headless.unwrap_or(true);

    let mut launch_params = serde_json::json!({
        "headless": headless,
        "channel": "auto",
    });
    if let (Some(w), Some(h)) = (body.viewport_width, body.viewport_height) {
        launch_params["viewport"] = serde_json::json!({"width": w, "height": h});
    }

    if let Err(e) = browser::send_sidecar_action("launch", &launch_params).await {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("启动浏览器失败: {e}. 预览需要浏览器环境"));
    }

    let preview_params = serde_json::json!({
        "url": body.url,
        "wait_until": "networkidle",
    });

    match browser::send_sidecar_action("preview", &preview_params).await {
        Ok(result) => ok_response(result),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("页面预览失败: {e}")),
    }
}
