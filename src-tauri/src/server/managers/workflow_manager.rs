// server/managers/workflow_manager.rs — 工作流 CRUD handler
//
// 从 handlers.rs 提取的工作流相关 handler 函数和类型。

use axum::{
    extract::Path,
    http::StatusCode,
    response::{Json, Response},
};
use serde::Deserialize;
use tracing::warn;

use crate::server::events;
use crate::server::handlers::{err_response, map_err_resp, map_not_found_resp, ok_response};
use crate::server::state;

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

// ═══════════════════════════════════════════════════════════
// 工作流 CRUD handler
// ═══════════════════════════════════════════════════════════

pub async fn workflow_list() -> Response {
    let app = state::get();
    map_err_resp(
        app.db
            .list_workflows()
            .map_err(|e| format!("Failed to list workflows: {e}")),
    )
}

pub async fn workflow_create(Json(body): Json<WorkflowCreateBody>) -> Response {
    let app = state::get();
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
            events::emit(
                "workflow-changed",
                serde_json::json!({
                    "action": "create",
                    "workflow_id": &id,
                    "workflow_name": &body.name,
                }),
            );
            ok_response(serde_json::json!({ "id": id }))
        }
        Err(e) => err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create workflow (name={}): {e}", body.name),
        ),
    }
}

pub async fn workflow_get(Path(id): Path<String>) -> Response {
    let app = state::get();
    map_not_found_resp(
        app.db
            .get_workflow(&id)
            .map_err(|e| format!("Failed to get workflow (id={id}): {e}")),
    )
}

pub async fn workflow_update(
    Path(id): Path<String>,
    Json(body): Json<WorkflowUpdateBody>,
) -> Response {
    let app = state::get();
    let now = chrono::Utc::now().to_rfc3339();
    match app.db.update_workflow(
        &id,
        body.name.as_deref(),
        body.description.as_deref(),
        body.enabled,
        &now,
    ) {
        Ok(()) => {
            events::emit(
                "workflow-changed",
                serde_json::json!({
                    "action": "update",
                    "workflow_id": &id,
                }),
            );
            ok_response(serde_json::json!({ "success": true }))
        }
        Err(e) => err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to update workflow (id={id}): {e}"),
        ),
    }
}

pub async fn workflow_delete(Path(id): Path<String>) -> Response {
    let app = state::get();
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
            events::emit(
                "workflow-changed",
                serde_json::json!({
                    "action": "delete",
                    "workflow_id": &id,
                }),
            );
            ok_response(serde_json::json!({ "success": true }))
        }
        Err(e) => err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete workflow (id={id}): {e}"),
        ),
    }
}

pub async fn workflow_lock(Path(id): Path<String>, Json(body): Json<WorkflowLockBody>) -> Response {
    let app = state::get();
    match app.db.set_workflow_locked(&id, body.locked) {
        Ok(()) => {
            events::emit(
                "workflow-changed",
                serde_json::json!({
                    "action": if body.locked { "lock" } else { "unlock" },
                    "workflow_id": &id,
                }),
            );
            ok_response(serde_json::json!({ "success": true }))
        }
        Err(e) => err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "Failed to {} workflow: {e}",
                if body.locked { "lock" } else { "unlock" }
            ),
        ),
    }
}

pub async fn workflow_save_yaml(
    Path(id): Path<String>,
    Json(body): Json<WorkflowSaveYamlBody>,
) -> Response {
    let app = state::get();
    let wf = match crate::engine::parser::parse_workflow(&body.yaml) {
        Ok(wf) => wf,
        Err(e) => {
            return err_response(
                StatusCode::BAD_REQUEST,
                format!("Failed to parse workflow YAML: {e}"),
            )
        }
    };
    let validation = crate::engine::validate::validate_workflow(&wf);
    if !validation.valid {
        return err_response(
            StatusCode::BAD_REQUEST,
            format!(
                "Workflow validation failed:\n{}",
                validation.errors.join("\n")
            ),
        );
    }
    if !validation.warnings.is_empty() {
        for w in &validation.warnings {
            warn!("Workflow validation warning: {}", w);
        }
    }

    match app.db.save_workflow_yaml(&id, &body.yaml) {
        Ok(()) => {
            events::emit(
                "workflow-changed",
                serde_json::json!({
                    "action": "save",
                    "workflow_id": &id,
                }),
            );
            ok_response(serde_json::json!({ "success": true }))
        }
        Err(e) => err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save workflow YAML (id={id}): {e}"),
        ),
    }
}

pub async fn workflow_validate(Json(body): Json<WorkflowValidateBody>) -> Response {
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

pub async fn workflow_auto_order(Json(body): Json<WorkflowAutoOrderBody>) -> Response {
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

pub async fn workflow_export(Json(body): Json<WorkflowExportBody>) -> Response {
    use crate::engine::workflow::Step;

    let steps: Vec<Step> = body
        .nodes
        .iter()
        .map(|n| {
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
        })
        .collect();

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
        Err(e) => {
            return err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("序列化 YAML 失败: {e}"),
            )
        }
    };

    if !body.edges.is_empty() {
        if let Some(map) = yaml_value.as_mapping_mut() {
            if let Ok(edges_yaml) = serde_yaml::to_value(&body.edges) {
                map.insert(serde_yaml::Value::String("edges".to_string()), edges_yaml);
            }
        }
    }

    let yaml_str = match serde_yaml::to_string(&yaml_value) {
        Ok(s) => s,
        Err(e) => {
            return err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("生成 YAML 失败: {e}"),
            )
        }
    };

    let final_path = if let Some(path) = body.output_path {
        std::path::PathBuf::from(&path)
    } else {
        let sanitized: String = body
            .name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        std::env::current_dir()
            .unwrap_or_default()
            .join(format!("{}.workflow.yaml", sanitized))
    };

    if let Some(parent) = final_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create directory: {e}"),
            );
        }
    }

    if let Err(e) = std::fs::write(&final_path, &yaml_str) {
        return err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("写入文件失败: {e}"),
        );
    }

    ok_response(serde_json::json!({
        "success": true,
        "path": final_path.to_string_lossy().to_string(),
        "yaml": yaml_str,
        "step_count": body.nodes.len(),
        "edge_count": body.edges.len(),
    }))
}

pub async fn workflow_import(Json(body): Json<WorkflowImportBody>) -> Response {
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

    let nodes_json: Vec<serde_json::Value> = wf
        .steps
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "type": s.step_type,
                "label": s.name,
                "config": s.config,
            })
        })
        .collect();

    let variables_json = wf
        .variables
        .map(|v| serde_json::to_value(v).unwrap_or_default());

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
    let app = state::get();
    use crate::engine::recording_converter::{self, RecordedAction, RecordingSource};

    let recorded_actions: Vec<RecordedAction> = body
        .actions
        .iter()
        .filter_map(|a| serde_json::from_value(a.clone()).ok())
        .collect();

    let src = match body.source.as_deref() {
        Some("desktop") => RecordingSource::Desktop,
        Some("mixed") => RecordingSource::Mixed,
        _ => RecordingSource::Browser,
    };

    let conversion = recording_converter::convert_actions_to_workflow(
        &recorded_actions,
        &body.workflow_name,
        src,
    );

    if conversion.yaml.is_empty() {
        return err_response(
            StatusCode::BAD_REQUEST,
            "Recording is empty, cannot generate workflow",
        );
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    if let Err(e) = app
        .db
        .create_workflow(&id, &body.workflow_name, "由录制操作生成", &now, &now)
    {
        return err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create workflow: {e}"),
        );
    }
    if let Err(e) = app.db.save_workflow_yaml(&id, &conversion.yaml) {
        return err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save workflow YAML: {e}"),
        );
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
