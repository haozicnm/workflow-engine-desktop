// nodes/json_schema_extract.rs — 从 LLM 输出中提取结构化 JSON
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::warn;

#[derive(Default)]
pub struct JsonSchemaExtractNode;

#[async_trait]
impl NodeExecutor for JsonSchemaExtractNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "json_schema_extract".into(),
            version: "1.0".into(),
            display_name: "JSON Schema 提取".into(),
            description: "从 LLM 输出文本中提取结构化 JSON（支持 schema 校验）".into(),
            category: "ai".into(),
            inputs: vec![
                crate::nodes::traits::PortDef { label: "text".into(), data_type: "text".into(), required: false },
            ],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "data".into(), data_type: "json".into(), required: false },
                crate::nodes::traits::PortDef { label: "valid".into(), data_type: "bool".into(), required: false },
            ],
            config_schema: json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "LLM 输出文本" },
                    "schema": { "type": "object", "description": "期望的 JSON Schema（可选，用于校验）" },
                    "fields": { "type": "array", "description": "期望的字段名列表（可选，轻量校验）" }
                }
            }),
            params: vec![],
        }
    }

    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<Value> {
        let config = &step.config;
        let text = ctx.input_ports.get("text").and_then(|v| v.as_str()).map(String::from)
            .or_else(|| config.get("text").and_then(|v| v.as_str()).map(String::from))
            .ok_or_else(|| anyhow!("json_schema_extract: 缺少 text"))?;

        // 尝试从文本中提取 JSON
        let data = extract_json(&text);

        // 字段校验
        let mut valid = true;
        if let Some(fields) = config.get("fields").and_then(|v| v.as_array()) {
            if let Some(obj) = data.as_object() {
                for field in fields {
                    if let Some(name) = field.as_str() {
                        if !obj.contains_key(name) {
                            warn!("json_schema_extract: 缺少字段 '{}'", name);
                            valid = false;
                        }
                    }
                }
            } else {
                valid = false;
            }
        }

        // Schema 校验（基本类型检查）
        if let Some(schema) = config.get("schema") {
            if let Some(obj) = data.as_object() {
                if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
                    for (key, prop) in props {
                        if let Some(required) = prop.get("required").and_then(|v| v.as_bool()) {
                            if required && !obj.contains_key(key) {
                                warn!("json_schema_extract: 必需字段 '{}' 缺失", key);
                                valid = false;
                            }
                        }
                    }
                }
            }
        }

        Ok(json!({"data": data, "valid": valid}))
    }
}

/// 从文本中提取 JSON：先尝试整段解析，再尝试 ```json 代码块，最后 {} 和 [] 匹配
fn extract_json(text: &str) -> Value {
    // 1. 直接解析
    if let Ok(v) = serde_json::from_str::<Value>(text.trim()) {
        return v;
    }
    // 2. 提取 ```json ... ``` 代码块
    if let Some(start) = text.find("```json") {
        let rest = &text[start + 7..];
        if let Some(end) = rest.find("```") {
            let json_str = rest[..end].trim();
            if let Ok(v) = serde_json::from_str::<Value>(json_str) {
                return v;
            }
        }
    }
    if let Some(start) = text.find("```") {
        let rest = &text[start + 3..];
        if let Some(end) = rest.find("```") {
            let json_str = rest[..end].trim();
            if let Ok(v) = serde_json::from_str::<Value>(json_str) {
                return v;
            }
        }
    }
    // 3. 查找最外层 {} 或 []
    for (open, close) in [('{', '}'), ('[', ']')] {
        if let Some(start) = text.find(open) {
            if let Some(end) = text.rfind(close) {
                if end > start {
                    let json_str = &text[start..=end];
                    if let Ok(v) = serde_json::from_str::<Value>(json_str) {
                        return v;
                    }
                }
            }
        }
    }
    // 4. 全部失败，返回原文
    Value::String(text.to_string())
}
