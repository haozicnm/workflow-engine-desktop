// nodes/word_container.rs — Word 容器执行器
//
// v4.0: 在一个 Word 文件内按顺序执行多个 action，
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

/// Word 容器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordContainerConfig {
    #[serde(default)]
    pub file_path: String,
    pub actions: Vec<ContainerAction>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerResult {
    pub output_ports: HashMap<String, Value>,
    pub error: Option<String>,
}

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

pub async fn execute_word_container(
    config: &WordContainerConfig,
    input_ports: &HashMap<String, Value>,
) -> Result<ContainerResult> {
    let mut output_ports = HashMap::new();

    for action in &config.actions {
        tracing::info!("Word action: {} ({})", action.label, action.action_type);

        match action.action_type.as_str() {
            "read" => match crate::nodes::word::word_read(&config.file_path).await {
                Ok(content) => {
                    output_ports.insert(action.id.clone(), content);
                }
                Err(e) => record_error(&mut output_ports, &action.id, "Word read failed", &e),
            },
            "write" => {
                let value = input_ports
                    .get(&format!("{}_in", &action.id))
                    .or_else(|| action.config.get("value"))
                    .cloned()
                    .unwrap_or(Value::Null);
                let paragraphs = match &value {
                    Value::String(s) => json!([s]),
                    Value::Array(_) => value.clone(),
                    _ => json!([value.to_string()]),
                };
                if let Err(e) = crate::nodes::word::word_write(
                    &config.file_path,
                    &json!({"paragraphs": paragraphs}),
                )
                .await
                {
                    record_error(&mut output_ports, &action.id, "Word write failed", &e);
                }
            }
            "replace" => {
                let old_text = action
                    .config
                    .get("old_text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let new_text = input_ports
                    .get(&format!("{}_in", &action.id))
                    .and_then(|v| v.as_str())
                    .or_else(|| action.config.get("new_text").and_then(|v| v.as_str()))
                    .unwrap_or("");
                let mut replacements = serde_json::Map::new();
                replacements.insert(old_text.to_string(), json!(new_text));
                if let Err(e) = crate::nodes::word::word_replace(
                    &config.file_path,
                    &json!({"replacements": replacements}),
                )
                .await
                {
                    record_error(&mut output_ports, &action.id, "Word replace failed", &e);
                }
            }
            "create" => {
                let title = action
                    .config
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let mut paras: Vec<Value> = Vec::new();
                if !title.is_empty() {
                    paras.push(json!({"type": "heading", "level": 1, "runs": [{"text": title, "bold": true, "size": 32}]}));
                }
                if let Err(e) =
                    crate::nodes::word::word_write(&config.file_path, &json!({"paragraphs": paras}))
                        .await
                {
                    record_error(&mut output_ports, &action.id, "Word create failed", &e);
                }
            }
            "merge" => {
                let files_value = input_ports
                    .get(&format!("{}_in", &action.id))
                    .or_else(|| action.config.get("files"))
                    .cloned()
                    .unwrap_or(Value::Null);
                let files: Vec<String> = if let Some(arr) = files_value.as_array() {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                } else {
                    vec![]
                };
                if !files.is_empty() {
                    if let Err(e) =
                        crate::nodes::word::word_merge_files(&files, &config.file_path).await
                    {
                        record_error(&mut output_ports, &action.id, "Word merge failed", &e);
                    }
                }
            }
            "insert_table" => {
                let value = input_ports
                    .get(&format!("{}_in", &action.id))
                    .or_else(|| action.config.get("data"))
                    .cloned()
                    .unwrap_or(Value::Null);
                let rows: Vec<Vec<String>> = if let Some(arr) = value.as_array() {
                    arr.iter()
                        .map(|row| {
                            if let Some(row_arr) = row.as_array() {
                                row_arr
                                    .iter()
                                    .map(|c| c.as_str().unwrap_or("").to_string())
                                    .collect()
                            } else {
                                vec![row.as_str().unwrap_or("").to_string()]
                            }
                        })
                        .collect()
                } else if let Some(s) = value.as_str() {
                    vec![vec![s.to_string()]]
                } else if !value.is_null() {
                    vec![vec![value.to_string()]]
                } else {
                    vec![]
                };
                if rows.is_empty() {
                    record_error(
                        &mut output_ports,
                        &action.id,
                        "Word insert_table failed",
                        &"no data",
                    );
                    continue;
                }
                let table_para = json!({"type": "table", "rows": rows});
                let cfg = json!({"paragraphs": [table_para]});
                let write_result = if std::path::Path::new(&config.file_path).exists() {
                    match crate::nodes::word::word_read(&config.file_path).await {
                        Ok(existing) => {
                            let existing_paras = existing
                                .get("paragraphs")
                                .cloned()
                                .unwrap_or(Value::Array(vec![]));
                            let mut all_paras: Vec<Value> =
                                existing_paras.as_array().cloned().unwrap_or_default();
                            all_paras.push(table_para);
                            crate::nodes::word::word_write(
                                &config.file_path,
                                &json!({"paragraphs": all_paras}),
                            )
                            .await
                        }
                        Err(_) => crate::nodes::word::word_write(&config.file_path, &cfg).await,
                    }
                } else {
                    crate::nodes::word::word_write(&config.file_path, &cfg).await
                };
                if let Err(e) = write_result {
                    record_error(
                        &mut output_ports,
                        &action.id,
                        "Word insert_table failed",
                        &e,
                    );
                }
            }
            _ => tracing::warn!("Unknown Word action: {}", action.action_type),
        }
    }

    Ok(ContainerResult {
        output_ports,
        error: None,
    })
}

// ─── NodeExecutor trait 实现 ───

#[derive(Default)]
pub struct WordContainerNode;

#[async_trait]
impl NodeExecutor for WordContainerNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "word_container".into(),
            version: "1.0".into(),
            display_name: "Word 容器".into(),
            description: "在单个 Word 文件内按顺序执行多个操作（read/write/replace/create/merge/insert_table）".into(),
            category: "Office".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "_container_type".into(), data_type: "string".into(), required: false },
                crate::nodes::traits::PortDef { label: "_step_name".into(), data_type: "string".into(), required: false },
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "required": ["actions"],
                "properties": {
                    "file_path": {"type": "string", "description": "Word 文件路径"},
                    "actions": {"type": "array", "description": "操作列表，每个操作有 id/type/label/config"}
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
        let config: WordContainerConfig = serde_json::from_value(step.config.clone())
            .map_err(|e| anyhow!("Word 容器配置解析失败: {}", e))?;

        // Phase 3: 占位符机制已在 executor 层处理，不需要容器内部 resolve
        // 保留 input_ports 处理（用于 DAG 连线）

        let input_ports = ctx.input_ports.clone();
        let result = execute_word_container(&config, &input_ports).await?;

        let mut output = result.output_ports.clone();
        output.insert(
            "_container_type".to_string(),
            Value::String("word".to_string()),
        );
        output.insert("_step_name".to_string(), Value::String(step.name.clone()));

        Ok(serde_json::to_value(&output).unwrap_or(Value::Null))
    }
}
