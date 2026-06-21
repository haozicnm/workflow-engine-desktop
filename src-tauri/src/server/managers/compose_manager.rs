// server/managers/compose_manager.rs — 组合助手 API handler
//
// POST /api/compose/chain — 将多个 block 串联为完整工作流

use axum::{http::StatusCode, response::{Json, Response}};
use serde::{Deserialize, Serialize};

use crate::server::handlers::{err_response, ok_response};

// ═══════════════════════════════════════════════════════════
// 数据结构
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct ChainConnection {
    /// 源路径，如 "http.body" 或 "step1.output.data"
    pub from: String,
    /// 目标路径，如 "file_write.content"
    pub to: String,
}

#[derive(Debug, Deserialize)]
pub struct ComposeChainBody {
    /// block 类型列表，如 ["http", "file_write"]
    pub blocks: Vec<String>,
    /// 可选：手动指定连接
    pub connections: Option<Vec<ChainConnection>>,
    /// 可选：工作流名称
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ComposeResult {
    pub yaml: String,
    pub step_count: usize,
    pub name: String,
}

// ═══════════════════════════════════════════════════════════
// block 类型 → 默认步骤配置
// ═══════════════════════════════════════════════════════════

fn default_step_config(block_type: &str) -> serde_json::Value {
    match block_type {
        "http" => serde_json::json!({
            "method": "GET",
            "url": ""
        }),
        "shell" => serde_json::json!({
            "command": ""
        }),
        "script" => serde_json::json!({
            "script": ""
        }),
        "file" => serde_json::json!({}),
        "file_write" => serde_json::json!({
            "path": "output.txt",
            "content": ""
        }),
        "delay" => serde_json::json!({
            "ms": 1000
        }),
        "notify" => serde_json::json!({
            "notify_type": "system",
            "title": "",
            "body": ""
        }),
        "logic" => serde_json::json!({
            "conditionGroup": {
                "combinator": "and",
                "conditions": []
            }
        }),
        "excel" => serde_json::json!({}),
        "browser" => serde_json::json!({}),
        _ => serde_json::json!({}),
    }
}

fn block_type_label(block_type: &str) -> &'static str {
    match block_type {
        "http" => "HTTP 请求",
        "shell" => "Shell 命令",
        "script" => "脚本执行",
        "file" => "文件操作",
        "file_write" => "写入文件",
        "delay" => "延时等待",
        "notify" => "发送通知",
        "logic" => "逻辑判断",
        "excel" => "Excel 操作",
        "browser" => "浏览器操作",
        _ => "未知节点",
    }
}

// ═══════════════════════════════════════════════════════════
// Handler
// ═══════════════════════════════════════════════════════════

/// POST /api/compose/chain — 将多个 block 串联为工作流
pub async fn compose_chain(Json(body): Json<ComposeChainBody>) -> Response {
    if body.blocks.is_empty() {
        return err_response(StatusCode::BAD_REQUEST, "blocks 列表不能为空");
    }

    // 验证 block 类型是否已注册
    for block in &body.blocks {
        if !crate::nodes::registry::is_registered(block) {
            // file_write 特殊处理：它是 file 容器的 action，不是独立节点
            if block == "file_write" {
                continue;
            }
            return err_response(
                StatusCode::BAD_REQUEST,
                format!("未知的 block 类型: '{block}'"),
            );
        }
    }

    let wf_name = body
        .name
        .unwrap_or_else(|| "chain-workflow".to_string());

    // 为每个 block 生成 step
    let mut steps = Vec::new();
    for (i, block_type) in body.blocks.iter().enumerate() {
        let step_id = format!("step_{}", i + 1);
        let label = block_type_label(block_type);

        let step = if block_type == "file_write" {
            // file_write 是 file 容器的 action
            serde_json::json!({
                "id": step_id,
                "type": "file",
                "label": format!("{}.{}", i + 1, label),
                "actions": [{
                    "id": format!("act_{}", i + 1),
                    "type": "write",
                    "params": default_step_config(block_type)
                }]
            })
        } else {
            serde_json::json!({
                "id": step_id,
                "type": block_type,
                "label": format!("{}.{}", i + 1, label),
                "config": default_step_config(block_type)
            })
        };

        steps.push(step);
    }

    // 自动串联：设置 next 字段（最后一个步骤不设置）
    for i in 0..steps.len() - 1 {
        let next_id = format!("step_{}", i + 2);
        if let Some(obj) = steps[i].as_object_mut() {
            obj.insert("next".to_string(), serde_json::Value::String(next_id));
        }
    }

    // 处理 connections：将 from → to 转换为 config 中的变量引用
    if let Some(connections) = &body.connections {
        for conn in connections {
            // 解析 from: "step_id.field" 或 "step_id.action_id.field"
            let from_parts: Vec<&str> = conn.from.split('.').collect();
            let to_parts: Vec<&str> = conn.to.split('.').collect();

            if from_parts.len() < 2 || to_parts.len() < 2 {
                continue;
            }

            // 构建变量引用 {{step_xxx.field}} 或 {{step_xxx.action.field}}
            let var_ref = if from_parts.len() == 2 {
                format!("{{{{step_{}.{}.}}}}", from_parts[0], from_parts[1])
            } else {
                format!(
                    "{{{{step_{}.{}.{}.}}}}",
                    from_parts[0], from_parts[1], from_parts[2]
                )
            };

            // 找到目标 step 并注入变量引用
            let target_step_id = to_parts[0];
            let target_field = to_parts[1];

            for step in &mut steps {
                if step.get("id").and_then(|v| v.as_str()) == Some(target_step_id) {
                    // 在 config 中设置字段
                    if let Some(config) = step.get_mut("config") {
                        if let Some(obj) = config.as_object_mut() {
                            obj.insert(
                                target_field.to_string(),
                                serde_json::Value::String(var_ref.clone()),
                            );
                        }
                    }
                    // 也在 actions 的 params 中查找
                    if let Some(actions) = step.get_mut("actions") {
                        if let Some(arr) = actions.as_array_mut() {
                            for action in arr {
                                if let Some(params) = action.get_mut("params") {
                                    if let Some(obj) = params.as_object_mut() {
                                        obj.insert(
                                            target_field.to_string(),
                                            serde_json::Value::String(var_ref.clone()),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 组装完整 workflow
    let workflow = serde_json::json!({
        "name": wf_name,
        "steps": steps,
    });

    let yaml = serde_yaml::to_string(&workflow).unwrap_or_default();

    ok_response(ComposeResult {
        yaml,
        step_count: steps.len(),
        name: wf_name,
    })
}

// ═══════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_configs_exist() {
        let http_cfg = default_step_config("http");
        assert!(http_cfg.get("method").is_some());
        assert!(http_cfg.get("url").is_some());

        let shell_cfg = default_step_config("shell");
        assert!(shell_cfg.get("command").is_some());
    }

    #[test]
    fn unknown_block_type_returns_empty_config() {
        let cfg = default_step_config("nonexistent");
        assert!(cfg.is_object());
    }

    #[test]
    fn chain_builds_steps_with_next() {
        let blocks = ["http".to_string(), "shell".to_string()];
        let mut steps = Vec::new();
        for (i, block_type) in blocks.iter().enumerate() {
            let step_id = format!("step_{}", i + 1);
            let step = serde_json::json!({
                "id": step_id,
                "type": block_type,
                "config": default_step_config(block_type)
            });
            steps.push(step);
        }
        // Set next
        for i in 0..steps.len() - 1 {
            let next_id = format!("step_{}", i + 2);
            if let Some(obj) = steps[i].as_object_mut() {
                obj.insert("next".to_string(), serde_json::Value::String(next_id));
            }
        }
        assert_eq!(steps.len(), 2);
        assert_eq!(
            steps[0].get("next").unwrap().as_str().unwrap(),
            "step_2"
        );
        assert!(steps[1].get("next").is_none());
    }

    #[test]
    fn connection_wiring() {
        // Simulate wiring http.body → file_write.content
        let from = "http.body";
        let to = "file_write.content";
        let from_parts: Vec<&str> = from.split('.').collect();
        let to_parts: Vec<&str> = to.split('.').collect();
        assert_eq!(from_parts, vec!["http", "body"]);
        assert_eq!(to_parts, vec!["file_write", "content"]);
    }
}
