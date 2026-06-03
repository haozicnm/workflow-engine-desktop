// server/handlers.rs — HTTP API 请求处理器
//
// 每个 handler 对应原 commands/ 中的一个 Tauri 命令。
// 签名模式: async fn handler(State(app): State<Arc<App>>, ...) -> impl IntoResponse

use axum::{
    extract::Path,
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        IntoResponse, Json, Response,
    },
};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use tokio_stream::StreamExt;

use crate::server::events;

// ═══════════════════════════════════════════════════════════
// Helper: convert Result<T, String> → axum Response
// ═══════════════════════════════════════════════════════════

pub(crate) fn ok_response<T: Serialize>(data: T) -> Response {
    Json(data).into_response()
}

pub(crate) fn err_response(status: StatusCode, msg: impl Into<String>) -> Response {
    let body = serde_json::json!({ "error": msg.into() });
    (status, Json(body)).into_response()
}

pub(crate) fn map_err_resp<T: Serialize>(result: Result<T, String>) -> Response {
    match result {
        Ok(data) => ok_response(data),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

pub(crate) fn map_not_found_resp<T: Serialize>(result: Result<Option<T>, String>) -> Response {
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
pub struct PipelineRunBody {
    pub excel_path: Option<String>,
    pub template_path: Option<String>,
    pub output_path: Option<String>,
    pub use_browser: Option<bool>,
}

// ═══════════════════════════════════════════════════════════
// SSE handler
// ═══════════════════════════════════════════════════════════

pub async fn events_sse() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = events::get_tx().subscribe();
    let stream =
        tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(|result| match result {
            Ok((event, data)) => {
                let data_str = data.to_string();
                Some(Ok(Event::default().event(event).data(data_str)))
            }
            Err(_) => None,
        });
    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("ping"),
    )
}

// ═══════════════════════════════════════════════════════════
// Step test / debug handler
// ═══════════════════════════════════════════════════════════

pub async fn step_test(Json(body): Json<StepTestBody>) -> Response {
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
        ..Default::default()
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

// ═══════════════════════════════════════════════════════════
// 调试 handler
// ═══════════════════════════════════════════════════════════

pub async fn debug_step(Path(run_id): Path<String>) -> Response {
    let app = crate::server::state::get();
    let flags = app.step_mode_flags.read().await;
    match flags.get(&run_id) {
        Some(flag) => {
            use std::sync::atomic::Ordering;
            flag.store(true, Ordering::Relaxed);
            if let Some(bp) = app.breakpoint_flags.read().await.get(&run_id) {
                bp.store(false, Ordering::Relaxed);
            }
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

pub async fn debug_continue(Path(run_id): Path<String>) -> Response {
    let app = crate::server::state::get();
    use std::sync::atomic::Ordering;
    if let Some(bp) = app.breakpoint_flags.read().await.get(&run_id) {
        bp.store(false, Ordering::Relaxed);
    }
    if let Some(sm) = app.step_mode_flags.read().await.get(&run_id) {
        sm.store(false, Ordering::Relaxed);
    }
    events::emit(
        "run-update",
        serde_json::json!({
            "run_id": &run_id,
            "status": "running",
        }),
    );
    ok_response(serde_json::json!({ "success": true }))
}

pub async fn debug_set_breakpoint(Json(body): Json<DebugSetBreakpointBody>) -> Response {
    let app = crate::server::state::get();
    let key = format!("breakpoints:{}", body.workflow_id);
    let mut bps: Vec<String> = app
        .config
        .read()
        .await
        .get_temp(&key)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    if !bps.contains(&body.step_id) {
        bps.push(body.step_id.clone());
        app.config
            .write()
            .await
            .set_temp(&key, serde_json::json!(bps));
    }
    ok_response(serde_json::json!({ "success": true }))
}

pub async fn debug_remove_breakpoint(Json(body): Json<DebugSetBreakpointBody>) -> Response {
    let app = crate::server::state::get();
    let key = format!("breakpoints:{}", body.workflow_id);
    let mut bps: Vec<String> = app
        .config
        .read()
        .await
        .get_temp(&key)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    bps.retain(|id| id != &body.step_id);
    app.config
        .write()
        .await
        .set_temp(&key, serde_json::json!(bps));
    ok_response(serde_json::json!({ "success": true }))
}

pub async fn debug_get_breakpoints(Path(workflow_id): Path<String>) -> Response {
    let app = crate::server::state::get();
    let key = format!("breakpoints:{}", workflow_id);
    let bps: Vec<String> = app
        .config
        .read()
        .await
        .get_temp(&key)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    ok_response(bps)
}

pub async fn debug_vars(Path(run_id): Path<String>) -> Response {
    let app = crate::server::state::get();
    let snapshots = app.debug_snapshots.read().await;
    ok_response(
        snapshots
            .get(&run_id)
            .cloned()
            .unwrap_or(serde_json::json!({
                "variables": {},
                "step_outputs": {},
            })),
    )
}

// ═══════════════════════════════════════════════════════════
// Pipeline handler
// ═══════════════════════════════════════════════════════════

pub async fn run_pipeline(Json(body): Json<PipelineRunBody>) -> Response {
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;
    use std::process::Command as StdCommand;

    let base = crate::data::paths::resolve_data_dir().join("examples");
    let script_path = base.join("run_full_pipeline.py");

    if !script_path.exists() {
        let project_examples = std::path::PathBuf::from("examples");
        if project_examples.exists() {
            if let Err(e) = std::fs::create_dir_all(&base) {
                return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
            }
            for file in &[
                "run_full_pipeline.py",
                "test_data.xlsx",
                "report_template.docx",
            ] {
                let src = project_examples.join(file);
                let dst = base.join(file);
                if src.exists() {
                    if let Err(e) = std::fs::copy(&src, &dst) {
                        return err_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("复制 {file} 失败: {e}"),
                        );
                    }
                }
            }
        }
    }

    let script = if script_path.exists() {
        script_path.to_string_lossy().to_string()
    } else {
        return err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("管道脚本不存在: {:?}", script_path),
        );
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
        Err(e) => {
            return err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("执行 Python 失败: {e}"),
            )
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("管道执行失败:\n{}\n{}", stdout, stderr),
        );
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

// Re-export from managers
pub use crate::server::managers::preview_manager::{
    get_bundle_files, get_trajectory, preview_excel, preview_word, read_bundle_file,
    web_scrape_preview, PreviewExcelBody, PreviewWordBody, WebScrapePreviewBody,
};

pub use crate::server::managers::workflow_manager::{
    workflow_assemble, workflow_auto_order, workflow_create, workflow_delete, workflow_export,
    workflow_export_yaml, workflow_get, workflow_import, workflow_list, workflow_lock,
    workflow_save_yaml, workflow_update, workflow_validate, WorkflowAssembleBody,
    WorkflowAutoOrderBody, WorkflowCreateBody, WorkflowExportBody, WorkflowImportBody,
    WorkflowLockBody, WorkflowSaveYamlBody, WorkflowUpdateBody, WorkflowValidateBody,
};

pub use crate::server::managers::run_manager::{
    run_cancel, run_detail, run_list, run_logs, run_pause, run_resume, run_start, run_status,
    run_step_logs, RunStartBody, RunStartResponse,
};

pub use crate::server::managers::schedule_manager::{
    schedule_create, schedule_delete, schedule_list, schedule_update, ScheduleCreateBody,
    ScheduleUpdateBody,
};

pub use crate::server::managers::approval_manager::{
    approval_list_pending, approval_respond, ApprovalRespondBody,
};

pub use crate::server::managers::system_manager::{
    blocks_categories, blocks_get, blocks_list, browser_pick_next, browser_pick_start,
    browser_pick_stop, check_ipc, clear_logs, get_log_path, node_list_types, node_schema,
    open_log_dir, plugin_install, plugin_list, plugin_pick_file, plugin_uninstall, plugin_upload,
    settings_get, settings_update, sidecar_health, system_check_browser, system_health,
    webbridge_health, PickStartBody, PluginInstallBody, PluginUninstallBody, SettingsUpdateBody,
};

pub use crate::server::managers::template_manager::{
    template_categories, template_get, template_import, template_instantiate, template_list,
    workflow_save_as_template, InstantiateBody, SaveAsTemplateBody,
};

pub use crate::server::managers::compose_manager::{compose_chain, ComposeChainBody};
