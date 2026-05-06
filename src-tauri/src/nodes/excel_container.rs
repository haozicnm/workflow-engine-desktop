// nodes/excel_container.rs — Excel 容器执行器
//
// v4.0: 在一个 Excel 文件内按顺序执行多个 action，
// 支持通过 DAG 连线从上游容器端口接收数据并产出输出端口数据。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

pub async fn execute_excel_container(
    config: &ExcelContainerConfig,
    input_ports: &HashMap<String, Value>,
) -> Result<ContainerResult> {
    let mut output_ports = HashMap::new();

    for action in &config.actions {
        tracing::info!(
            "ExcelContainer — 执行 action: {} ({})",
            action.label,
            action.action_type
        );

        match action.action_type.as_str() {
            "read" => {
                let read_cfg = serde_json::json!({
                    "sheet": config.sheet,
                });
                match crate::nodes::excel::excel_read(&config.file_path, &read_cfg).await {
                    Ok(data) => {
                        output_ports.insert(action.label.clone(), data);
                    }
                    Err(e) => {
                        return Err(anyhow!("Excel 读取失败: {}", e));
                    }
                }
            }
            "write" => {
                let value = input_ports
                    .get(&format!("{}_in", &action.label))
                    .or_else(|| action.config.get("value"))
                    .cloned()
                    .unwrap_or(Value::Null);

                let write_cfg = serde_json::json!({
                    "sheet": config.sheet,
                    "data": match &value {
                        Value::Array(_arr) => value.clone(),
                        _ => serde_json::json!([[value]]),
                    },
                });

                if let Err(e) =
                    crate::nodes::excel::excel_write(&config.file_path, &write_cfg).await
                {
                    return Err(anyhow!("Excel 写入失败: {}", e));
                }
            }
            "filter" => {
                // 读取→内存筛选→产出结果
                let read_cfg = serde_json::json!({ "sheet": config.sheet });
                let data = crate::nodes::excel::excel_read(&config.file_path, &read_cfg).await
                    .map_err(|e| anyhow!("Excel 筛选读取失败: {}", e))?;

                let rows = data.get("data").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                let column_idx = action.config.get("column").and_then(|v| v.as_str())
                    .map(excel_col_to_idx)
                    .unwrap_or(0);
                let filter_val = input_ports
                    .get(&format!("{}_in", &action.label))
                    .or_else(|| action.config.get("value"))
                    .cloned();
                let op = action.config.get("op").and_then(|v| v.as_str()).unwrap_or("contains");

                let filtered: Vec<Value> = rows.iter().filter(|row| {
                    let cell = row.as_array().and_then(|r| r.get(column_idx)).unwrap_or(&Value::Null);
                    match_filter(op, cell, &filter_val)
                }).cloned().collect();

                let result = serde_json::json!({
                    "sheet": config.sheet,
                    "rows": filtered.len(),
                    "data": filtered,
                });
                output_ports.insert(action.label.clone(), result);
            }
            "sort" => {
                // 读取→内存排序→产出结果
                let read_cfg = serde_json::json!({ "sheet": config.sheet });
                let data = crate::nodes::excel::excel_read(&config.file_path, &read_cfg).await
                    .map_err(|e| anyhow!("Excel 排序读取失败: {}", e))?;

                let mut rows = data.get("data").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                let column_idx = action.config.get("column").and_then(|v| v.as_str())
                    .map(excel_col_to_idx)
                    .unwrap_or(0);
                let order = action.config.get("order").and_then(|v| v.as_str()).unwrap_or("asc");
                let ascending = order != "desc";

                rows.sort_by(|a, b| {
                    let va = a.as_array().and_then(|r| r.get(column_idx));
                    let vb = b.as_array().and_then(|r| r.get(column_idx));
                    let cmp = compare_values(va, vb);
                    if ascending { cmp } else { cmp.reverse() }
                });

                let result = serde_json::json!({
                    "sheet": config.sheet,
                    "rows": rows.len(),
                    "data": rows,
                });
                output_ports.insert(action.label.clone(), result);
            }
            "create" => {
                let headers = action
                    .config
                    .get("headers")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let create_cfg = serde_json::json!({
                    "sheet": config.sheet,
                    "headers": headers,
                });

                if let Err(e) =
                    crate::nodes::excel::excel_create(&config.file_path, &create_cfg).await
                {
                    return Err(anyhow!("Excel 创建失败: {}", e));
                }
            }
            "append" => {
                let value = input_ports
                    .get(&format!("{}_in", &action.label))
                    .or_else(|| action.config.get("value"))
                    .cloned()
                    .unwrap_or(Value::Null);

                let append_cfg = serde_json::json!({
                    "sheet": config.sheet,
                    "data": value,
                });

                if let Err(e) =
                    crate::nodes::excel::excel_append(&config.file_path, &append_cfg).await
                {
                    return Err(anyhow!("Excel 追加失败: {}", e));
                }
            }
            "formula" => {
                let _cell = action
                    .config
                    .get("cell")
                    .and_then(|v| v.as_str())
                    .unwrap_or("A1");
                let formula = action
                    .config
                    .get("formula")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                // formula action: write formula as cell value
                // Use excel_write for single-cell formula
                let write_cfg = serde_json::json!({
                    "sheet": config.sheet,
                    "data": [[Value::String(format!("={}", formula))]],
                });

                if let Err(e) =
                    crate::nodes::excel::excel_write(&config.file_path, &write_cfg).await
                {
                    return Err(anyhow!("Excel 公式设置失败: {}", e));
                }
            }
            _ => {
                tracing::warn!(
                    "ExcelContainer — 未知 action 类型: {}",
                    action.action_type
                );
            }
        }
    }

    Ok(ContainerResult {
        output_ports,
        error: None,
    })
}

/// 列字母→索引 (A=0, B=1, ...)
fn excel_col_to_idx(col: &str) -> usize {
    col.bytes().fold(0usize, |acc, b| {
        acc * 26 + (b.to_ascii_uppercase() as usize).saturating_sub(65)
    })
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
        }
    }
}

// ─── NodeExecutor trait 实现 ───

#[derive(Default)]
pub struct ExcelContainerNode;

#[async_trait]
impl NodeExecutor for ExcelContainerNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        let config: ExcelContainerConfig = serde_json::from_value(step.config.clone())
            .map_err(|e| anyhow!("Excel 容器配置解析失败: {}", e))?;

        let input_ports = ctx.input_ports.clone();
        let result = execute_excel_container(&config, &input_ports).await?;

        if let Some(ref err) = result.error {
            return Err(anyhow!("Excel 容器执行失败: {}", err));
        }

        Ok(serde_json::to_value(&result.output_ports).unwrap_or(Value::Null))
    }
}
