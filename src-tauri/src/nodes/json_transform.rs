// nodes/json_transform.rs — JSON 数据转换节点
// 支持映射（字段提取）、筛选（条件过滤）、重排（排序）
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
pub struct JsonTransformNode;

#[async_trait]
impl NodeExecutor for JsonTransformNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "json_transform".into(),
            version: "1.0".into(),
            display_name: "JSON 转换".into(),
            description: "JSON 数据映射、筛选、排序（支持 Rhai 表达式）".into(),
            category: "data".into(),
            inputs: vec![
                crate::nodes::traits::PortDef { label: "data".into(), data_type: "any".into(), required: true },
            ],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "result".into(), data_type: "any".into(), required: false },
            ],
            config_schema: json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["pick", "filter", "sort", "map", "merge", "keys", "values"],
                        "description": "操作类型"
                    },
                    "fields": { "type": "array", "items": { "type": "string" }, "description": "pick: 提取的字段列表" },
                    "expression": { "type": "string", "description": "filter/map: Rhai 表达式（__item 为当前元素）" },
                    "sort_key": { "type": "string", "description": "sort: 排序字段" },
                    "sort_desc": { "type": "boolean", "description": "sort: 是否降序", "default": false },
                    "source": { "type": "string", "description": "merge: 第二个数据源的变量名" }
                },
                "required": ["operation"]
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
        let operation = config.get("operation").and_then(|v| v.as_str()).unwrap_or("pick");

        // 从输入端口或变量获取数据
        let data = ctx.input_ports
            .values()
            .next()
            .cloned()
            .or_else(|| ctx.variables.get("__item").cloned())
            .unwrap_or(Value::Null);

        match operation {
            "pick" => {
                let fields: Vec<String> = config.get("fields")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                match data {
                    Value::Object(map) => {
                        let mut result = serde_json::Map::new();
                        for f in &fields {
                            if let Some(v) = map.get(f) {
                                result.insert(f.clone(), v.clone());
                            }
                        }
                        Ok(Value::Object(result))
                    }
                    Value::Array(arr) => {
                        let result: Vec<Value> = arr.iter().map(|item| {
                            if let Some(obj) = item.as_object() {
                                let mut picked = serde_json::Map::new();
                                for f in &fields {
                                    if let Some(v) = obj.get(f) {
                                        picked.insert(f.clone(), v.clone());
                                    }
                                }
                                Value::Object(picked)
                            } else {
                                item.clone()
                            }
                        }).collect();
                        Ok(Value::Array(result))
                    }
                    _ => Ok(data),
                }
            }
            "filter" => {
                let expr = config.get("expression").and_then(|v| v.as_str()).unwrap_or("true");
                match data {
                    Value::Array(arr) => {
                        let mut result = Vec::new();
                        for (i, item) in arr.iter().enumerate() {
                            ctx.set_var("__item".to_string(), item.clone());
                            ctx.set_var("__index".to_string(), json!(i));
                            match ctx.eval_expr(expr) {
                                Ok(val) if val.as_bool().unwrap_or(false) => result.push(item.clone()),
                                Ok(_) => {}
                                Err(e) => {
                                    warn!("filter 表达式求值失败 [{}]: {}", i, e);
                                }
                            }
                        }
                        ctx.variables.remove("__item");
                        ctx.variables.remove("__index");
                        Ok(Value::Array(result))
                    }
                    _ => Ok(data),
                }
            }
            "map" => {
                let expr = config.get("expression").and_then(|v| v.as_str()).unwrap_or("__item");
                match data {
                    Value::Array(arr) => {
                        let mut result = Vec::new();
                        for (i, item) in arr.iter().enumerate() {
                            ctx.set_var("__item".to_string(), item.clone());
                            ctx.set_var("__index".to_string(), json!(i));
                            match ctx.eval_expr(expr) {
                                Ok(val) => result.push(val),
                                Err(e) => {
                                    warn!("map 表达式求值失败 [{}]: {}", i, e);
                                    result.push(Value::Null);
                                }
                            }
                        }
                        ctx.variables.remove("__item");
                        ctx.variables.remove("__index");
                        Ok(Value::Array(result))
                    }
                    _ => Ok(data),
                }
            }
            "sort" => {
                let sort_key = config.get("sort_key").and_then(|v| v.as_str()).unwrap_or("");
                let desc = config.get("sort_desc").and_then(|v| v.as_bool()).unwrap_or(false);
                match data {
                    Value::Array(mut arr) => {
                        arr.sort_by(|a, b| {
                            let va = if sort_key.is_empty() { a.clone() } else { a.get(sort_key).cloned().unwrap_or(Value::Null) };
                            let vb = if sort_key.is_empty() { b.clone() } else { b.get(sort_key).cloned().unwrap_or(Value::Null) };
                            let cmp = compare_json_values(&va, &vb);
                            if desc { cmp.reverse() } else { cmp }
                        });
                        Ok(Value::Array(arr))
                    }
                    _ => Ok(data),
                }
            }
            "merge" => {
                let source_key = config.get("source").and_then(|v| v.as_str()).unwrap_or("");
                let other = ctx.variables.get(source_key).cloned().unwrap_or(Value::Null);
                match (data, other) {
                    (Value::Array(mut a), Value::Array(b)) => {
                        a.extend(b);
                        Ok(Value::Array(a))
                    }
                    (Value::Object(mut a), Value::Object(b)) => {
                        for (k, v) in b { a.insert(k, v); }
                        Ok(Value::Object(a))
                    }
                    _ => Err(anyhow!("merge: 两个数据源类型不匹配")),
                }
            }
            "keys" => {
                match data {
                    Value::Object(map) => Ok(Value::Array(map.keys().map(|k| Value::String(k.clone())).collect())),
                    _ => Err(anyhow!("keys: 数据必须是对象")),
                }
            }
            "values" => {
                match data {
                    Value::Object(map) => Ok(Value::Array(map.values().cloned().collect())),
                    _ => Err(anyhow!("values: 数据必须是对象")),
                }
            }
            _ => Err(anyhow!("json_transform: 未知操作 '{}'", operation)),
        }
    }
}

fn compare_json_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => a.as_f64().unwrap_or(0.0).partial_cmp(&b.as_f64().unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal),
        (Value::String(a), Value::String(b)) => a.cmp(b),
        (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
        (Value::Null, Value::Null) => std::cmp::Ordering::Equal,
        (Value::Null, _) => std::cmp::Ordering::Less,
        (_, Value::Null) => std::cmp::Ordering::Greater,
        _ => std::cmp::Ordering::Equal,
    }
}
