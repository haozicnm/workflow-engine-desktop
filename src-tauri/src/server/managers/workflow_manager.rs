// server/managers/workflow_manager.rs — 工作流 CRUD handler
//
// 从 handlers.rs 提取的工作流相关 handler 函数和类型。

use axum::{
    extract::Path,
    http::StatusCode,
    response::{Json, Response},
};
use serde::{Deserialize, Serialize};
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
    // 锁定检查：锁定的工作流不可修改
    match app.db.get_workflow(&id) {
        Ok(Some(wf)) if wf.locked => {
            return err_response(StatusCode::CONFLICT, "工作流已锁定，无法修改。请先解锁。")
        }
        Ok(None) => return err_response(StatusCode::NOT_FOUND, "工作流不存在"),
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        _ => {}
    }
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
    // 锁定检查：锁定的工作流不可修改
    match app.db.get_workflow(&id) {
        Ok(Some(wf)) if wf.locked => {
            return err_response(StatusCode::CONFLICT, "工作流已锁定，无法修改。请先解锁。")
        }
        Ok(None) => return err_response(StatusCode::NOT_FOUND, "工作流不存在"),
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        _ => {}
    }
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

    // 统一存储为 JSON（前端 JSON.parse 解析，YAML 仅用于输入校验）
    let json_str = serde_json::to_string_pretty(&wf).unwrap_or_else(|_| body.yaml.clone());
    match app.db.save_workflow_yaml(&id, &json_str) {
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
        ..Default::default()
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
        let p = std::path::PathBuf::from(&path);
        // 路径遍历防护：规范化后必须在当前工作目录内
        let canonical = p.canonicalize().unwrap_or(p.clone());
        let cwd = std::env::current_dir().unwrap_or_default();
        let canonical_cwd = cwd.canonicalize().unwrap_or(cwd);
        if !canonical.starts_with(&canonical_cwd) {
            return err_response(
                StatusCode::FORBIDDEN,
                format!("导出路径必须在工作目录内: {}", path),
            );
        }
        p
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

// ═══════════════════════════════════════════════════════════
// POST /api/workflows/assemble — 从步骤定义组装工作流
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct WorkflowAssembleBody {
    pub name: String,
    pub steps: Vec<serde_json::Value>,
    pub description: Option<String>,
    pub variables: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize)]
struct AssembleError {
    step: String,
    field: String,
    message: String,
}

/// 校验步骤的 params，返回 required 参数缺失的错误列表
fn validate_step_params(
    step_type: &str,
    step_id: &str,
    config: &serde_json::Value,
) -> Vec<AssembleError> {
    let mut errors = Vec::new();
    let manifest = match crate::nodes::registry::get_node(step_type) {
        Some(m) => m,
        None => return errors, // 类型不存在的错误在外层处理
    };

    for param in &manifest.params {
        if !param.required {
            continue;
        }
        let has_value = match config {
            serde_json::Value::Object(map) => {
                if let Some(val) = map.get(&param.name) {
                    !(val.is_null() || (val.is_string() && val.as_str().unwrap_or("").is_empty()))
                } else {
                    false
                }
            }
            _ => false,
        };
        if !has_value {
            errors.push(AssembleError {
                step: step_id.to_string(),
                field: param.name.clone(),
                message: format!(
                    "必需参数 '{}' 缺失（节点类型: {}）",
                    param.name, step_type
                ),
            });
        }
    }
    errors
}

pub async fn workflow_assemble(Json(body): Json<WorkflowAssembleBody>) -> Response {
    let mut errors: Vec<AssembleError> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let step_ids: std::collections::HashSet<String> = body
        .steps
        .iter()
        .filter_map(|s| s.get("id").and_then(|v| v.as_str()).map(String::from))
        .collect();

    for (i, step_val) in body.steps.iter().enumerate() {
        let step_id = step_val
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let step_type = step_val
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // 1. 检查 type 是否存在
        if step_type.is_empty() {
            errors.push(AssembleError {
                step: step_id.to_owned(),
                field: "type".into(),
                message: format!("步骤 #{} 缺少 type 字段", i),
            });
            continue;
        }
        if !crate::nodes::registry::is_registered(step_type) {
            errors.push(AssembleError {
                step: step_id.to_owned(),
                field: "type".into(),
                message: format!("节点类型 '{}' 未注册", step_type),
            });
            continue;
        }

        // 2. 检查 id 是否唯一
        if step_id == "unknown" {
            errors.push(AssembleError {
                step: step_id.to_owned(),
                field: "id".into(),
                message: format!("步骤 #{} 缺少 id 字段", i),
            });
        }

        // 3. 校验 required params
        let config = step_val.get("config").cloned().unwrap_or(serde_json::json!({}));
        let param_errors = validate_step_params(step_type, step_id, &config);
        errors.extend(param_errors);

        // 4. 检查 next 引用是否有效
        if let Some(next) = step_val.get("next").and_then(|v| v.as_str()) {
            if !next.is_empty() && !step_ids.contains(next) {
                errors.push(AssembleError {
                    step: step_id.to_owned(),
                    field: "next".into(),
                    message: format!("引用的下一步骤 '{}' 不存在", next),
                });
            }
        }

        // 5. 检查 runCondition.ref 引用
        if let Some(rc_ref) = step_val
            .get("runCondition")
            .and_then(|rc| rc.get("ref").or_else(|| rc.get("ref_step")))
            .and_then(|v| v.as_str())
        {
            if !step_ids.contains(rc_ref) {
                errors.push(AssembleError {
                    step: step_id.to_owned(),
                    field: "runCondition.ref".into(),
                    message: format!("引用的条件步骤 '{}' 不存在", rc_ref),
                });
            }
        }

        // 6. 容器类型但无 body_steps 的警告
        if crate::nodes::registry::is_container(step_type) {
            let has_body = step_val
                .get("body_steps")
                .or_else(|| step_val.get("bodySteps"))
                .map(|v| v.is_array() && !v.as_array().unwrap().is_empty())
                .unwrap_or(false);
            let has_actions = step_val
                .get("actions")
                .map(|v| v.is_array() && !v.as_array().unwrap().is_empty())
                .unwrap_or(false);
            if !has_body && !has_actions {
                warnings.push(format!(
                    "容器节点 '{}' ({}) 没有子步骤或动作",
                    step_id, step_type
                ));
            }
        }
    }

    if !errors.is_empty() {
        return err_response(
            StatusCode::BAD_REQUEST,
            serde_json::json!({ "errors": errors }).to_string(),
        );
    }

    // 组装成功：创建工作流
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
                    "action": "assemble",
                    "workflow_id": &id,
                    "workflow_name": &body.name,
                }),
            );
            ok_response(serde_json::json!({
                "workflow_id": id,
                "warnings": warnings,
            }))
        }
        Err(e) => err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("创建工作流失败: {e}"),
        ),
    }
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

// ═══════════════════════════════════════════════════════════
// GET /api/workflows/{id}/yaml — 导出标准 YAML 格式
// ═══════════════════════════════════════════════════════════

/// GET /api/workflows/{id}/yaml
/// 导出工作流为标准 YAML 格式（带版本号、metadata、干净格式）
pub async fn workflow_export_yaml(Path(id): Path<String>) -> Response {
    let app = state::get();

    // 从 DB 加载工作流
    let wf = match app.db.get_workflow(&id) {
        Ok(Some(wf)) => wf,
        Ok(None) => return err_response(StatusCode::NOT_FOUND, format!("工作流 '{}' 不存在", id)),
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("加载工作流失败: {e}")),
    };

    // 解析为 Workflow 结构
    let yaml_str = &wf.yaml;

    let parsed_wf = match crate::engine::parser::parse_workflow(yaml_str) {
        Ok(wf) => wf,
        Err(_) => {
            // 解析失败，返回错误
            return err_response(
                StatusCode::BAD_REQUEST,
                "工作流 YAML 解析失败".to_string(),
            );
        }
    };

    match crate::engine::yaml_format::export_workflow_yaml(&parsed_wf) {
        Ok(yaml) => ok_response(serde_json::json!({
            "success": true,
            "yaml": yaml,
            "format_version": crate::engine::workflow::FORMAT_VERSION,
        })),
        Err(e) => err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("YAML 导出失败: {e}"),
        ),
    }
}
