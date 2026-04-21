// commands/workflow.rs — 工作流 CRUD 命令
use tauri::State;
use serde::{Deserialize, Serialize};
use crate::App;
use crate::engine::workflow::Workflow;
use crate::data::models::WorkflowMeta;

#[derive(Debug, Serialize)]
pub struct WorkflowListItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[tauri::command]
pub async fn workflow_list(
    app: State<'_, App>,
) -> Result<Vec<WorkflowListItem>, String> {
    let db = app.db.read().await;
    db.list_workflows().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn workflow_create(
    app: State<'_, App>,
    name: String,
    description: Option<String>,
) -> Result<String, String> {
    let db = app.db.write().await;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    db.create_workflow(&id, &name, description.as_deref().unwrap_or(""), &now, &now)
        .map_err(|e| e.to_string())?;

    Ok(id)
}

#[tauri::command]
pub async fn workflow_get(
    app: State<'_, App>,
    id: String,
) -> Result<Option<WorkflowMeta>, String> {
    let db = app.db.read().await;
    db.get_workflow(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn workflow_update(
    app: State<'_, App>,
    id: String,
    name: Option<String>,
    description: Option<String>,
    enabled: Option<bool>,
) -> Result<(), String> {
    let db = app.db.write().await;
    let now = chrono::Utc::now().to_rfc3339();
    db.update_workflow(&id, name.as_deref(), description.as_deref(), enabled, &now)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn workflow_delete(
    app: State<'_, App>,
    id: String,
) -> Result<(), String> {
    let db = app.db.write().await;
    db.delete_workflow(&id).map_err(|e|e.to_string())
}

#[tauri::command]
pub async fn workflow_validate(
    yaml: String,
) -> Result<serde_json::Value, String> {
    match crate::engine::parser::parse_workflow(&yaml) {
        Ok(wf) => Ok(serde_json::json!({
            "valid": true,
            "workflow": {
                "name": wf.name,
                "step_count": wf.steps.len(),
            }
        })),
        Err(e) => Ok(serde_json::json!({
            "valid": false,
            "error": e.to_string()
        })),
    }
}
