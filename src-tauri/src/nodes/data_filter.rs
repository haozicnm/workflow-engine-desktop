// nodes/data_filter.rs — 数据过滤节点
// 对数组数据进行条件过滤，支持多种比较操作
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
pub struct DataFilterNode;

#[async_trait]
impl NodeExecutor for DataFilterNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "data_filter".into(),
            version: "1.0".into(),
            display_name: "数据过滤".into(),
            description: "按条件过滤数组数据（支持表达式、字段值匹配）".into(),
            category: "data".into(),
            inputs: vec![
                crate::nodes::traits::PortDef { label: "data".into(), data_type: "array".into(), required: true },
            ],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "filtered".into(), data_type: "array".into(), required: false },
                crate::nodes::traits::PortDef { label: "count".into(), data_type: "number".into(), required: false },
            ],
            config_schema: json!({
                "type": "object",
                "properties": {
                    "field": { "type": "string", "description": "要过滤的字段名（对象数组用）" },
                    "op": {
                        "type": "string",
                        "enum": ["equals", "not_equals", "contains", "not_contains", "gt", "lt", "gte", "lte", "is_empty", "not_empty", "regex", "in"],
                        "description": "比较操作"
                    },
                    "value": { "type": "any", "description": "比较值" },
                    "expression": { "type": "string", "description": "Rhai 表达式（优先级高于 field/op/value）" }
                }
            }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        let config = &step.config;

        // 获取输入数据
        let data = ctx.input_ports
            .get("data")
            .cloned()
            .or_else(|| ctx.input_ports.values().next().cloned())
            .or_else(|| ctx.variables.get("__item").cloned())
            .unwrap_or(Value::Null);

        let arr = match data {
            Value::Array(a) => a,
            _ => return Err(anyhow!("data_filter: 输入必须是数组")),
        };

        // 优先使用表达式
        if let Some(expr) = config.get("expression").and_then(|v| v.as_str()) {
            let mut result = Vec::new();
            for (i, item) in arr.iter().enumerate() {
                ctx.set_var("__item".to_string(), item.clone());
                ctx.set_var("__index".to_string(), json!(i));
                match ctx.eval_expr(expr) {
                    Ok(val) if val.as_bool().unwrap_or(false) => result.push(item.clone()),
                    Ok(_) => {}
                    Err(e) => warn!("data_filter 表达式求值失败 [{}]: {}", i, e),
                }
            }
            ctx.variables.remove("__item");
            ctx.variables.remove("__index");
            let count = result.len();
            return Ok(json!({ "filtered": result, "count": count }));
        }

        // 使用 field/op/value 模式
        let field = config.get("field").and_then(|v| v.as_str()).unwrap_or("");
        let op = config.get("op").and_then(|v| v.as_str()).unwrap_or("equals");
        let cmp_value = config.get("value").cloned().unwrap_or(Value::Null);

        let mut result = Vec::new();
        for item in &arr {
            let item_value = if field.is_empty() {
                item.clone()
            } else {
                item.get(field).cloned().unwrap_or(Value::Null)
            };

            let matches = match op {
                "equals" => item_value == cmp_value,
                "not_equals" => item_value != cmp_value,
                "contains" => match (&item_value, &cmp_value) {
                    (Value::String(s), Value::String(sub)) => s.contains(sub.as_str()),
                    (Value::Array(arr), _) => arr.contains(&cmp_value),
                    _ => false,
                },
                "not_contains" => match (&item_value, &cmp_value) {
                    (Value::String(s), Value::String(sub)) => !s.contains(sub.as_str()),
                    (Value::Array(arr), _) => !arr.contains(&cmp_value),
                    _ => true,
                },
                "gt" => compare_gt(&item_value, &cmp_value),
                "lt" => compare_lt(&item_value, &cmp_value),
                "gte" => !compare_lt(&item_value, &cmp_value),
                "lte" => !compare_gt(&item_value, &cmp_value),
                "is_empty" => matches!(item_value, Value::Null) || item_value.as_str().map(|s| s.is_empty()).unwrap_or(false) || item_value.as_array().map(|a| a.is_empty()).unwrap_or(false),
                "not_empty" => !matches!(item_value, Value::Null)
                    && item_value != Value::String(String::new())
                    && item_value != Value::Array(vec![])
                    && item_value != Value::Object(serde_json::Map::new()),
                "regex" => {
                    if let (Some(text), Some(pattern)) = (item_value.as_str(), cmp_value.as_str()) {
                        // 缓存正则编译（避免循环中重复编译）
                        use std::sync::Mutex;
                        static REGEX_CACHE: std::sync::LazyLock<Mutex<std::collections::HashMap<String, regex::Regex>>> =
                            std::sync::LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));
                        let mut cache = REGEX_CACHE.lock().unwrap();
                        if !cache.contains_key(pattern) {
                            if let Ok(re) = regex::Regex::new(pattern) {
                                cache.insert(pattern.to_string(), re);
                            }
                        }
                        cache.get(pattern).map(|re| re.is_match(text)).unwrap_or(false)
                    } else {
                        false
                    }
                }
                "in" => match &cmp_value {
                    Value::Array(arr) => arr.contains(&item_value),
                    _ => false,
                },
                _ => {
                    warn!("data_filter: 未知操作 '{}'", op);
                    false
                }
            };

            if matches {
                result.push(item.clone());
            }
        }

        let count = result.len();
        Ok(json!({ "filtered": result, "count": count }))
    }
}

fn compare_gt(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => a.as_f64().unwrap_or(0.0) > b.as_f64().unwrap_or(0.0),
        (Value::String(a), Value::String(b)) => a > b,
        _ => false,
    }
}

fn compare_lt(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => a.as_f64().unwrap_or(0.0) < b.as_f64().unwrap_or(0.0),
        (Value::String(a), Value::String(b)) => a < b,
        _ => false,
    }
}
