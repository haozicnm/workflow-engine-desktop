// nodes/excel_container.rs — Excel 容器执行器
//
// v4.0: 在一个 Excel 文件内按顺序执行多个 action，
// 支持通过 DAG 连线从上游容器端口接收数据并产出输出端口数据。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tracing;

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};

/// Excel 容器内的单个 action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerAction {
    pub id: String,
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub config: HashMap<String, Value>,
}

/// Excel 容器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcelContainerConfig {
    #[serde(default)]
    pub file_path: String,
    #[serde(default = "default_sheet")]
    pub sheet: String,
    pub actions: Vec<ContainerAction>,
}

fn default_sheet() -> String {
    "Sheet1".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerResult {
    pub output_ports: HashMap<String, Value>,
    pub error: Option<String>,
}

/// 记录 action 错误到 output_ports，不中断容器执行
fn record_error(
    ports: &mut HashMap<String, Value>,
    action_id: &str,
    msg: &str,
    e: &dyn std::fmt::Display,
) {
    let err = format!("{}: {}", msg, e);
    tracing::warn!("{}", err);
    ports.insert(action_id.to_string(), json!({"error": err}));
}

pub async fn execute_excel_container(
    config: &ExcelContainerConfig,
    input_ports: &HashMap<String, Value>,
) -> Result<ContainerResult> {
    let mut output_ports = HashMap::new();

    for action in &config.actions {
        tracing::info!("Excel action: {} ({})", action.label, action.action_type);

        match action.action_type.as_str() {
            "sheets" => match crate::nodes::excel::excel_sheets(&config.file_path).await {
                Ok(data) => {
                    output_ports.insert(action.id.clone(), data);
                }
                Err(e) => record_error(&mut output_ports, &action.id, "Excel sheets failed", &e),
            },
            "read" => {
                let read_cfg = serde_json::json!({ "sheet": config.sheet });
                match crate::nodes::excel::excel_read(&config.file_path, &read_cfg).await {
                    Ok(data) => {
                        output_ports.insert(action.id.clone(), data);
                    }
                    Err(e) => record_error(&mut output_ports, &action.id, "Excel read failed", &e),
                }
            }
            "write" => {
                let value = input_ports
                    .get(&format!("{}_in", &action.id))
                    .or_else(|| action.config.get("value"))
                    .cloned()
                    .unwrap_or(Value::Null);
                let write_cfg = serde_json::json!({
                    "sheet": config.sheet,
                    "data": match &value { Value::Array(_) => value.clone(), _ => json!([[value]]) },
                });
                match crate::nodes::excel::excel_write(&config.file_path, &write_cfg).await {
                    Ok(data) => {
                        output_ports.insert(action.id.clone(), data);
                    }
                    Err(e) => record_error(&mut output_ports, &action.id, "Excel write failed", &e),
                }
            }
            "filter" => {
                let read_cfg = serde_json::json!({ "sheet": config.sheet });
                match crate::nodes::excel::excel_read(&config.file_path, &read_cfg).await {
                    Ok(data) => {
                        let rows = data
                            .get("data")
                            .and_then(|v| v.as_array())
                            .cloned()
                            .unwrap_or_default();
                        let column_idx = action
                            .config
                            .get("column")
                            .and_then(|v| v.as_str())
                            .map(|c| resolve_column(c, &rows))
                            .unwrap_or(0);
                        let filter_val = input_ports
                            .get(&format!("{}_in", &action.id))
                            .or_else(|| action.config.get("value"))
                            .cloned();
                        let op = action
                            .config
                            .get("op")
                            .and_then(|v| v.as_str())
                            .unwrap_or("contains");
                        let filtered: Vec<Value> = rows
                            .iter()
                            .filter(|row| {
                                let cell = row
                                    .as_array()
                                    .and_then(|r| r.get(column_idx))
                                    .unwrap_or(&Value::Null);
                                match_filter(op, cell, &filter_val)
                            })
                            .cloned()
                            .collect();
                        output_ports.insert(action.id.clone(), json!({"sheet": config.sheet, "rows": filtered.len(), "data": filtered}));
                    }
                    Err(e) => {
                        record_error(&mut output_ports, &action.id, "Excel filter failed", &e)
                    }
                }
            }
            "sort" => {
                let read_cfg = serde_json::json!({ "sheet": config.sheet });
                match crate::nodes::excel::excel_read(&config.file_path, &read_cfg).await {
                    Ok(data) => {
                        let mut rows = data
                            .get("data")
                            .and_then(|v| v.as_array())
                            .cloned()
                            .unwrap_or_default();
                        let column_idx = action
                            .config
                            .get("column")
                            .and_then(|v| v.as_str())
                            .map(|c| resolve_column(c, &rows))
                            .unwrap_or(0);
                        let order = action
                            .config
                            .get("order")
                            .and_then(|v| v.as_str())
                            .unwrap_or("asc");
                        let ascending = order != "desc";
                        rows.sort_by(|a, b| {
                            let va = a.as_array().and_then(|r| r.get(column_idx));
                            let vb = b.as_array().and_then(|r| r.get(column_idx));
                            let cmp = compare_values(va, vb);
                            if ascending {
                                cmp
                            } else {
                                cmp.reverse()
                            }
                        });
                        output_ports.insert(
                            action.id.clone(),
                            json!({"sheet": config.sheet, "rows": rows.len(), "data": rows}),
                        );
                    }
                    Err(e) => record_error(&mut output_ports, &action.id, "Excel sort failed", &e),
                }
            }
            "create" => {
                let headers = action
                    .config
                    .get("headers")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let create_cfg = serde_json::json!({ "sheet": config.sheet, "headers": headers });
                if let Err(e) =
                    crate::nodes::excel::excel_create(&config.file_path, &create_cfg).await
                {
                    record_error(&mut output_ports, &action.id, "Excel create failed", &e);
                }
            }
            "append" => {
                let value = input_ports
                    .get(&format!("{}_in", &action.id))
                    .or_else(|| action.config.get("value"))
                    .cloned()
                    .unwrap_or(Value::Null);
                let append_cfg = serde_json::json!({ "sheet": config.sheet, "data": value });
                if let Err(e) =
                    crate::nodes::excel::excel_append(&config.file_path, &append_cfg).await
                {
                    record_error(&mut output_ports, &action.id, "Excel append failed", &e);
                }
            }
            "update" => {
                let updates = input_ports
                    .get(&format!("{}_in", &action.id))
                    .or_else(|| action.config.get("updates"))
                    .cloned()
                    .unwrap_or(Value::Null);
                let update_cfg = serde_json::json!({ "sheet": config.sheet, "updates": updates });
                if let Err(e) =
                    crate::nodes::excel::excel_update(&config.file_path, &update_cfg).await
                {
                    record_error(&mut output_ports, &action.id, "Excel update failed", &e);
                }
            }
            "formula" => {
                let formula = action
                    .config
                    .get("formula")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let write_cfg = serde_json::json!({ "sheet": config.sheet, "data": [[format!("={}", formula)]] });
                if let Err(e) =
                    crate::nodes::excel::excel_write(&config.file_path, &write_cfg).await
                {
                    record_error(&mut output_ports, &action.id, "Excel formula failed", &e);
                }
            }
            _ => {
                tracing::warn!("Unknown Excel action: {}", action.action_type);
            }
        }
    }

    Ok(ContainerResult {
        output_ports,
        error: None,
    })
}

/// 列字母→索引 (A=0, B=1, ...)；如果不是纯字母，返回 0 让调用者用列名回退
fn excel_col_to_idx(col: &str) -> Option<usize> {
    if col.is_empty() || !col.bytes().all(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    Some(col.bytes().fold(0usize, |acc, b| {
        acc * 26 + (b.to_ascii_uppercase() as usize).saturating_sub(65)
    }))
}

/// 解析列引用：先试字母 (A=0)，再试列名（查表头行）
fn resolve_column(col: &str, rows: &[Value]) -> usize {
    if let Some(idx) = excel_col_to_idx(col) {
        return idx;
    }
    // 回退：查第一行的列名
    if let Some(header_row) = rows.first().and_then(|r| r.as_array()) {
        if let Some(pos) = header_row.iter().position(|c| c.as_str() == Some(col)) {
            return pos;
        }
    }
    0
}

/// 简易过滤匹配
fn match_filter(op: &str, cell: &Value, filter_val: &Option<Value>) -> bool {
    let cell_s = cell.as_str().unwrap_or("");
    let filter_s = filter_val.as_ref().and_then(|v| v.as_str()).unwrap_or("");
    let cell_n = cell.as_f64().unwrap_or(0.0);
    let filter_n = filter_val.as_ref().and_then(|v| v.as_f64()).unwrap_or(0.0);

    match op {
        "contains" => cell_s.contains(filter_s),
        "equals" => cell_s == filter_s,
        "not_equals" => cell_s != filter_s,
        "gt" => cell_n > filter_n,
        "gte" => cell_n >= filter_n,
        "lt" => cell_n < filter_n,
        "lte" => cell_n <= filter_n,
        "is_empty" => cell_s.is_empty(),
        "not_empty" => !cell_s.is_empty(),
        _ => true,
    }
}

/// 比较两个值（用于排序）
fn compare_values(a: Option<&Value>, b: Option<&Value>) -> std::cmp::Ordering {
    match (a.and_then(|v| v.as_str()), b.and_then(|v| v.as_str())) {
        (Some(sa), Some(sb)) => sa.cmp(sb),
        _ => match (a.and_then(|v| v.as_f64()), b.and_then(|v| v.as_f64())) {
            (Some(na), Some(nb)) => na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal),
            _ => std::cmp::Ordering::Equal,
        },
    }
}

// ─── NodeExecutor trait 实现 ───

#[derive(Default)]
pub struct ExcelContainerNode;

#[async_trait]
impl NodeExecutor for ExcelContainerNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "excel_container".into(),
            version: "1.0".into(),
            display_name: "Excel 容器".into(),
            description: "在一个 Excel 文件内按顺序执行多个操作（读、写、筛选、排序等），支持 DAG 连线".into(),
            category: "Office".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "output_ports".into(), data_type: "object".into(), required: false },
                crate::nodes::traits::PortDef { label: "_container_type".into(), data_type: "string".into(), required: false },
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Excel 文件路径"},
                    "sheet": {"type": "string", "description": "工作表名称"},
                    "actions": {"type": "array", "description": "操作列表"}
                }
            }),
        }
    }

    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        // 预处理：file_path 可能是对象（如 cursor.current glob match），提取 .path
        let mut raw_config = step.config.clone();
        if let Some(obj) = raw_config.get("file_path").and_then(|v| v.as_object()) {
            let extracted = obj
                .get("path")
                .or_else(|| obj.get("name"))
                .and_then(|v| v.as_str())
                .map(|s| Value::String(s.to_string()))
                .unwrap_or_else(|| Value::String(serde_json::to_string(&obj).unwrap_or_default()));
            if let Value::Object(ref mut map) = raw_config {
                map.insert("file_path".to_string(), extracted);
            }
        }

        let config: ExcelContainerConfig = serde_json::from_value(raw_config)
            .map_err(|e| anyhow!("Excel 容器配置解析失败: {}", e))?;

        // Phase 3: 占位符机制已在 executor 层处理，不需要容器内部 resolve
        // 保留 input_ports 处理（用于 DAG 连线）

        let input_ports = ctx.input_ports.clone();
        let result = execute_excel_container(&config, &input_ports).await?;

        // 添加元数据
        let mut output = result.output_ports.clone();
        output.insert(
            "_container_type".to_string(),
            Value::String("excel".to_string()),
        );
        output.insert("_step_name".to_string(), Value::String(step.name.clone()));

        Ok(serde_json::to_value(&output).unwrap_or(Value::Null))
    }
}
