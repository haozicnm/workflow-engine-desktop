// nodes/word_container.rs — Word 容器执行器
//
// v4.0: 在一个 Word 文件内按顺序执行多个 action，
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
    pub label: String,
    #[serde(default)]
    pub config: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerResult {
    pub output_ports: HashMap<String, Value>,
    pub error: Option<String>,
}

pub async fn execute_word_container(
    config: &WordContainerConfig,
    input_ports: &HashMap<String, Value>,
) -> Result<ContainerResult> {
    let mut output_ports = HashMap::new();

    for action in &config.actions {
        tracing::info!(
            "WordContainer — 执行 action: {} ({})",
            action.label,
            action.action_type
        );

        match action.action_type.as_str() {
            "read" => {
                // 读取文档内容
                match crate::nodes::word::word_read(&config.file_path).await {
                    Ok(content) => {
                        output_ports.insert(action.id.clone(), content);
                    }
                    Err(e) => {
                        return Err(anyhow!("Word 读取失败: {}", e));
                    }
                }
            }
            "write" => {
                let value = input_ports
                    .get(&format!("{}_in", &action.id))
                    .or_else(|| action.config.get("value"))
                    .cloned()
                    .unwrap_or(Value::Null);

                // word_write expects config with "paragraphs" array
                let paragraphs = match &value {
                    Value::String(s) => serde_json::json!([s]),
                    Value::Array(_) => value.clone(),
                    _ => serde_json::json!([value.to_string()]),
                };
                let write_cfg = serde_json::json!({ "paragraphs": paragraphs });

                if let Err(e) =
                    crate::nodes::word::word_write(&config.file_path, &write_cfg).await
                {
                    return Err(anyhow!("Word 写入失败: {}", e));
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
                replacements.insert(old_text.to_string(), serde_json::json!(new_text));
                let replace_cfg = serde_json::json!({
                    "replacements": replacements,
                });

                if let Err(e) =
                    crate::nodes::word::word_replace(&config.file_path, &replace_cfg).await
                {
                    return Err(anyhow!("Word 替换失败: {}", e));
                }
            }
            "create" => {
                // Create an empty docx file or with a title
                let title = action
                    .config
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let mut paras: Vec<Value> = Vec::new();
                if !title.is_empty() {
                    paras.push(serde_json::json!({
                        "type": "heading",
                        "level": 1,
                        "runs": [{ "text": title, "bold": true, "size": 32 }]
                    }));
                }
                let create_cfg = serde_json::json!({ "paragraphs": paras });
                if let Err(e) =
                    crate::nodes::word::word_write(&config.file_path, &create_cfg).await
                {
                    return Err(anyhow!("Word 创建失败: {}", e));
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
                    if let Err(e) = crate::nodes::word::word_merge_files(
                        &files,
                        &config.file_path,
                    )
                    .await
                    {
                        return Err(anyhow!("Word 合并失败: {}", e));
                    }
                }
            }
            "insert_table" => {
                let value = input_ports
                    .get(&format!("{}_in", &action.id))
                    .or_else(|| action.config.get("data"))
                    .cloned()
                    .unwrap_or(Value::Null);

                // Build a table paragraph and append
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
                    return Err(anyhow!("Word 插入表格失败: 无数据"));
                }

                let table_para = serde_json::json!({
                    "type": "table",
                    "rows": rows,
                });
                let cfg = serde_json::json!({
                    "paragraphs": [table_para],
                });

                // Check if file exists to decide append vs write
                let file_exists =
                    std::path::Path::new(&config.file_path).exists();
                if file_exists {
                    // Use word_write to append? Actually word_write overwrites.
                    // Read existing, append table, rewrite.
                    match crate::nodes::word::word_read(&config.file_path).await {
                        Ok(existing) => {
                            let existing_paras = existing
                                .get("paragraphs")
                                .cloned()
                                .unwrap_or(Value::Array(vec![]));
                            let mut all_paras: Vec<Value> = if let Some(arr) =
                                existing_paras.as_array()
                            {
                                arr.clone()
                            } else {
                                vec![]
                            };
                            all_paras.push(table_para);
                            let merge_cfg =
                                serde_json::json!({ "paragraphs": all_paras });
                            if let Err(e) = crate::nodes::word::word_write(
                                &config.file_path,
                                &merge_cfg,
                            )
                            .await
                            {
                                return Err(anyhow!("Word 插入表格失败: {}", e));
                            }
                        }
                        Err(_) => {
                            // File doesn't exist or can't read, just write
                            if let Err(e) = crate::nodes::word::word_write(
                                &config.file_path,
                                &cfg,
                            )
                            .await
                            {
                                return Err(anyhow!("Word 插入表格失败: {}", e));
                            }
                        }
                    }
                } else {
                    if let Err(e) =
                        crate::nodes::word::word_write(&config.file_path, &cfg).await
                    {
                        return Err(anyhow!("Word 插入表格失败: {}", e));
                    }
                }
            }
            _ => {
                tracing::warn!(
                    "WordContainer — 未知 action 类型: {}",
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

// ─── NodeExecutor trait 实现 ───

#[derive(Default)]
pub struct WordContainerNode;

#[async_trait]
impl NodeExecutor for WordContainerNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        let config: WordContainerConfig = serde_json::from_value(step.config.clone())
            .map_err(|e| anyhow!("Word 容器配置解析失败: {}", e))?;

        let input_ports = ctx.input_ports.clone();
        let result = execute_word_container(&config, &input_ports).await?;

        if let Some(ref err) = result.error {
            return Err(anyhow!("Word 容器执行失败: {}", err));
        }

        Ok(serde_json::to_value(&result.output_ports).unwrap_or(Value::Null))
    }
}
