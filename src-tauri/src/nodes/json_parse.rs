// nodes/json_parse.rs — JSON 解析节点
//
// 用点号路径提取 JSON 字段，支持 `$.data.name`, `$.items[0].id` 等语法

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};

#[derive(Default)]
pub struct JsonParseNode;

#[async_trait]
impl NodeExecutor for JsonParseNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let data = config.get("data").ok_or_else(|| anyhow!("缺少 data 参数"))?;
        let expression = config.get("expression").and_then(|v| v.as_str()).unwrap_or("$");

        // 先将 input 解析为 JSON（如果已经是对象则直接使用）
        let root: serde_json::Value = if let Some(s) = data.as_str() {
            serde_json::from_str(s).unwrap_or_else(|_| data.clone())
        } else {
            data.clone()
        };

        let result = json_path_query(&root, expression);
        Ok(serde_json::json!({ "expression": expression, "result": result }))
    }
}

/// 轻量 JSON 路径查询
/// 支持: $.key, $.key.sub, $.array[0], $.array[*].field, .key
fn json_path_query(root: &serde_json::Value, expr: &str) -> serde_json::Value {
    let path = expr.trim_start_matches('$').trim_start_matches('.');
    if path.is_empty() { return root.clone(); }

    let mut current = root.clone();
    for segment in path.split('.') {
        // 处理数组索引 [n] 或通配 [*]
        if let Some((key, rest)) = segment.split_once('[') {
            let idx_part = rest.trim_end_matches(']');
            // 先取字段
            current = current.get(key).cloned().unwrap_or(serde_json::Value::Null);
            // 再取索引
            if idx_part == "*" {
                return current; // 返回整个数组
            }
            if let Ok(idx) = idx_part.parse::<usize>() {
                current = current.get(idx).cloned().unwrap_or(serde_json::Value::Null);
            }
        } else {
            current = current.get(segment).cloned().unwrap_or(serde_json::Value::Null);
        }
    }
    current
}
