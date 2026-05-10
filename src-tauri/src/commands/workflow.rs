// commands/workflow.rs — 工作流 CRUD 命令
use tauri::State;
use serde::Serialize;
use crate::App;
use crate::data::models::WorkflowMeta;
use std::sync::Arc;

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
    app.db.list_workflows().map_err(|e| format!("查询工作流列表失败: {e}"))
}

#[tauri::command]
pub async fn workflow_create(
    app: State<'_, App>,
    name: String,
    description: Option<String>,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    app.db.create_workflow(&id, &name, description.as_deref().unwrap_or(""), &now, &now)
        .map_err(|e| format!("创建工作流失败 (name={name}): {e}"))?;

    Ok(id)
}

#[tauri::command]
pub async fn workflow_get(
    app: State<'_, App>,
    id: String,
) -> Result<Option<WorkflowMeta>, String> {
    app.db.get_workflow(&id).map_err(|e| format!("获取工作流失败 (id={id}): {e}"))
}

#[tauri::command]
pub async fn workflow_update(
    app: State<'_, App>,
    id: String,
    name: Option<String>,
    description: Option<String>,
    enabled: Option<bool>,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    app.db.update_workflow(&id, name.as_deref(), description.as_deref(), enabled, &now)
        .map_err(|e| format!("更新工作流失败 (id={id}): {e}"))
}

#[tauri::command]
pub async fn workflow_delete(
    app: State<'_, App>,
    id: String,
) -> Result<(), String> {
    app.db.delete_workflow(&id).map_err(|e| format!("删除工作流失败 (id={id}): {e}"))
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

#[tauri::command]
pub async fn workflow_save_yaml(
    app: State<'_, App>,
    id: String,
    yaml: String,
) -> Result<(), String> {
    app.db.save_workflow_yaml(&id, &yaml)
        .map_err(|e| format!("保存工作流 YAML 失败 (id={id}): {e}"))
}

/// 自动排序步骤（根据 {{step_xxx}} 引用推断依赖）
#[tauri::command]
pub async fn workflow_auto_order(yaml: String) -> Result<serde_json::Value, String> {
    let wf = crate::engine::parser::parse_workflow(&yaml)
        .map_err(|e| format!("解析工作流失败: {e}"))?;
    let order = crate::engine::parser::auto_order_steps(&wf.steps);
    Ok(serde_json::json!({
        "order": order,
        "steps": order.iter().map(|&i| &wf.steps[i].id).collect::<Vec<_>>(),
    }))
}

/// 从录制操作创建新工作流（录制 → YAML → 保存到数据库）
#[tauri::command]
pub async fn workflow_create_from_recording(
    app: State<'_, App>,
    actions: Vec<serde_json::Value>,
    workflow_name: String,
    source: Option<String>,
) -> Result<serde_json::Value, String> {
    use crate::engine::recording_converter::{self, RecordedAction, RecordingSource};

    let recorded_actions: Vec<RecordedAction> = actions
        .iter()
        .filter_map(|a| serde_json::from_value(a.clone()).ok())
        .collect();

    let src = match source.as_deref() {
        Some("desktop") => RecordingSource::Desktop,
        Some("mixed") => RecordingSource::Mixed,
        _ => RecordingSource::Browser,
    };

    let conversion = recording_converter::convert_actions_to_workflow(
        &recorded_actions, &workflow_name, src,
    );

    if conversion.yaml.is_empty() {
        return Err("录制的操作为空，无法生成工作流".to_string());
    }

    // 创建并保存工作流到数据库
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    app.db.create_workflow(&id, &workflow_name, "由录制操作生成", &now, &now)
        .map_err(|e| format!("创建工作流失败: {e}"))?;
    app.db.save_workflow_yaml(&id, &conversion.yaml)
        .map_err(|e| format!("保存工作流 YAML 失败: {e}"))?;

    Ok(serde_json::json!({
        "id": id,
        "name": workflow_name,
        "yaml": conversion.yaml,
        "step_count": conversion.step_count,
        "action_count": conversion.action_count,
        "merged_count": conversion.merged_count,
        "step_summary": conversion.step_summary,
    }))
}

/// 查询录制状态（跨 step_test 调用）
#[tauri::command]
pub async fn recording_status() -> Result<serde_json::Value, String> {
    use crate::nodes::recording;
    let result = recording::get_recording_status().await;
    Ok(result)
}

/// 测试单个步骤（不保存，直接执行一次）
#[tauri::command]
pub async fn step_test(
    step_type: String,
    config: serde_json::Value,
    variables: Option<std::collections::HashMap<String, serde_json::Value>>,
) -> Result<serde_json::Value, String> {
    use crate::engine::context::ExecutionContext;
    use crate::engine::executor::StepExecutor;
    use crate::engine::workflow::{Step, Workflow};

    let step = Step {
        id: "test_step".to_string(),
        name: "测试步骤".to_string(),
        step_type: step_type.clone(),
        config: config.clone(),
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
        variables,
    };
    let mut ctx = ExecutionContext::new("test", &wf);

    let executor = StepExecutor::new(Arc::new(crate::engine::approval_store::ApprovalStore::new()), Arc::new(crate::data::db::Database::open_default().map_err(|e| e.to_string())?));
    match executor.execute(&step, &mut ctx).await {
        Ok(output) => Ok(serde_json::json!({
            "success": true,
            "output": output,
        })),
        Err(e) => Ok(serde_json::json!({
            "success": false,
            "error": e.to_string(),
        })),
    }
}

// ═══════════════════════════════════════════════════════════
// P4: 工作流导入/导出
// ═══════════════════════════════════════════════════════════

/// 将工作流 JSON（节点+连线）导出为 YAML 并保存到指定路径
#[tauri::command]
pub async fn export_workflow(
    name: String,
    description: Option<String>,
    nodes: Vec<serde_json::Value>,
    edges: Vec<serde_json::Value>,
    variables: Option<serde_json::Value>,
    output_path: Option<String>,
) -> Result<serde_json::Value, String> {
    use crate::engine::workflow::Step;

    // 将前端 FlowNode 转换为 Step 列表
    let steps: Vec<Step> = nodes
        .iter()
        .map(|n| {
            let node_type = n.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
            let label = n.get("label").and_then(|v| v.as_str()).unwrap_or("未命名");
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

    // 将 edges 也序列化到 YAML 中（作为自定义字段）
    let mut vars: Option<std::collections::HashMap<String, serde_json::Value>> = None;
    if let Some(v) = variables {
        if let Ok(map) = serde_json::from_value::<
            std::collections::HashMap<String, serde_json::Value>,
        >(v)
        {
            vars = Some(map);
        }
    }

    let wf = crate::engine::workflow::Workflow {
        name: name.clone(),
        description: description.clone(),
        steps,
        variables: vars,
    };

    // 序列化同时包含 edges 元数据
    let mut yaml_value = serde_yaml::to_value(&wf)
        .map_err(|e| format!("序列化 YAML 失败: {}", e))?;

    if !edges.is_empty() {
        if let Some(map) = yaml_value.as_mapping_mut() {
            let edges_yaml = serde_yaml::to_value(&edges)
                .map_err(|e| format!("序列化连线失败: {}", e))?;
            map.insert(
                serde_yaml::Value::String("edges".to_string()),
                edges_yaml,
            );
        }
    }

    let yaml_str = serde_yaml::to_string(&yaml_value)
        .map_err(|e| format!("生成 YAML 失败: {}", e))?;

    // 写入文件
    let final_path = if let Some(path) = output_path {
        std::path::PathBuf::from(&path)
    } else {
        let sanitized = name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect::<String>();
        std::env::current_dir()
            .unwrap_or_default()
            .join(format!("{}.workflow.yaml", sanitized))
    };

    // 确保目录存在
    if let Some(parent) = final_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败: {}", e))?;
    }

    std::fs::write(&final_path, &yaml_str)
        .map_err(|e| format!("写入文件失败: {}", e))?;

    Ok(serde_json::json!({
        "success": true,
        "path": final_path.to_string_lossy().to_string(),
        "yaml": yaml_str,
        "step_count": nodes.len(),
        "edge_count": edges.len(),
    }))
}

/// 从 YAML 文件导入工作流，返回节点、连线、元数据
#[tauri::command]
pub async fn import_workflow(
    yaml_content: String,
) -> Result<serde_json::Value, String> {
    use crate::engine::workflow::Workflow;

    let wf: Workflow = serde_yaml::from_str(&yaml_content)
        .map_err(|e| format!("YAML 解析失败: {}", e))?;

    // 同时尝试解析 edges
    let edges_value: Option<Vec<serde_json::Value>> =
        serde_yaml::from_str::<serde_yaml::Value>(&yaml_content)
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

    let variables_json = wf.variables.map(|v| serde_json::to_value(v).unwrap_or_default());

    Ok(serde_json::json!({
        "success": true,
        "name": wf.name,
        "description": wf.description,
        "nodes": nodes_json,
        "edges": edges_value.unwrap_or_default(),
        "variables": variables_json,
        "step_count": wf.steps.len(),
    }))
}