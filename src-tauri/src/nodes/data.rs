// nodes/data.rs — 数据处理节点（声明式）
//
// 支持的操作：
//   set       设置变量:       {action: "set", key: "name", value: "hello"}
//   get       读取变量:       {action: "get", key: "name"}
//   length    获取长度:       {action: "length", source: "output.step_xxx.data"}
//   default   设置默认值:     {action: "default", key: "name", value: "fallback"}
//   merge     合并对象:       {action: "merge", target: "a", source: "b"}
//   transform 表达式求值:     {action: "transform", expr: "step_xxx.data.len()"}（向后兼容）

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};

#[derive(Default)]
pub struct DataNode;

#[async_trait]
impl NodeExecutor for DataNode {
    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let action = config.get("action").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("数据节点缺少 action 参数"))?;

        match action {
            "set" => {
                let key = config.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("set 需要 key 参数"))?;
                let value = ctx.resolve_config(
                    config.get("value").unwrap_or(&serde_json::Value::Null)
                );
                ctx.set_var(key.to_string(), value.clone());
                Ok(value)
            }
            "get" => {
                let key = config.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("get 需要 key 参数"))?;
                Ok(ctx.variables.get(key).cloned().unwrap_or(serde_json::Value::Null))
            }
            "length" => {
                let source = config.get("source").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("length 需要 source 参数（如 'output.step_id.data' 或变量名）"))?;

                let value = resolve_source(source, ctx);
                let len = get_length(&value);

                let result = serde_json::json!(len);

                // 可选：保存到变量
                if let Some(key) = config.get("key").and_then(|v| v.as_str()) {
                    ctx.set_var(key.to_string(), result.clone());
                }

                Ok(result)
            }
            "default" => {
                let key = config.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("default 需要 key 参数"))?;
                let default_val = ctx.resolve_config(
                    config.get("value").unwrap_or(&serde_json::Value::Null)
                );

                let current = ctx.variables.get(key).cloned().unwrap_or(serde_json::Value::Null);
                if is_null_or_empty(&current) {
                    ctx.set_var(key.to_string(), default_val.clone());
                    Ok(default_val)
                } else {
                    Ok(current)
                }
            }
            "merge" => {
                let target = config.get("target").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("merge 需要 target 参数"))?;
                let source = config.get("source").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("merge 需要 source 参数"))?;

                let mut base = ctx.variables.get(target).cloned()
                    .unwrap_or(serde_json::json!({}));
                let overlay = ctx.variables.get(source).cloned()
                    .unwrap_or(serde_json::json!({}));

                if let (Some(base_obj), Some(overlay_obj)) = (base.as_object_mut(), overlay.as_object()) {
                    for (k, v) in overlay_obj {
                        base_obj.insert(k.clone(), v.clone());
                    }
                }

                ctx.set_var(target.to_string(), base.clone());
                Ok(base)
            }
            "transform" => {
                // 向后兼容：表达式求值
                let expr = config.get("expr").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("transform 需要 expr 参数"))?;
                ctx.eval_expr(expr).map_err(|e| anyhow!(e))
            }
            _ => Err(anyhow!("未知的数据操作: {}（支持 set/get/length/default/merge/transform）", action)),
        }
    }
}

/// 从 source 路径解析值
/// 支持：
///   "output.step_id"        → 步骤输出
///   "output.step_id.field"  → 步骤输出的嵌套字段
///   "variable_name"         → 上下文变量
fn resolve_source(source: &str, ctx: &ExecutionContext) -> serde_json::Value {
    if let Some(path) = source.strip_prefix("output.") {
        let parts: Vec<&str> = path.splitn(2, '.').collect();
        let step_id = parts[0];
        let field = parts.get(1);

        if let Some(output) = ctx.get_output(step_id) {
            if let Some(f) = field {
                output.get(*f).cloned().unwrap_or(serde_json::Value::Null)
            } else {
                output.clone()
            }
        } else {
            serde_json::Value::Null
        }
    } else {
        ctx.variables.get(source).cloned().unwrap_or(serde_json::Value::Null)
    }
}

/// 获取值的"长度"
fn get_length(val: &serde_json::Value) -> usize {
    match val {
        serde_json::Value::Array(arr) => arr.len(),
        serde_json::Value::String(s) => s.len(),
        serde_json::Value::Object(obj) => obj.len(),
        serde_json::Value::Null => 0,
        _ => 1,
    }
}

/// 判断值是否为 null 或空
fn is_null_or_empty(val: &serde_json::Value) -> bool {
    match val {
        serde_json::Value::Null => true,
        serde_json::Value::String(s) => s.is_empty(),
        _ => false,
    }
}
