// nodes/data.rs — 数据处理节点（v3: 每个操作独立 executor）
//
// data_set     — 设置变量
// data_get     — 读取变量
// data_length  — 获取长度
// data_default — 设置默认值
// data_merge   — 合并对象

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};

fn resolve_source(source: &str, ctx: &ExecutionContext) -> serde_json::Value {
    if let Some(path) = source.strip_prefix("output.") {
        let parts: Vec<&str> = path.splitn(2, '.').collect();
        let step_id = parts[0];
        let field = parts.get(1);
        if let Some(output) = ctx.get_output(step_id) {
            if let Some(f) = field { output.get(*f).cloned().unwrap_or(serde_json::Value::Null) }
            else { output.clone() }
        } else { serde_json::Value::Null }
    } else {
        ctx.variables.get(source).cloned().unwrap_or(serde_json::Value::Null)
    }
}

fn is_null_or_empty(val: &serde_json::Value) -> bool {
    match val {
        serde_json::Value::Null => true,
        serde_json::Value::String(s) => s.is_empty(),
        _ => false,
    }
}

// ═══════════════════════════════════════
// data_set — 设置变量
// ═══════════════════════════════════════

#[derive(Default)]
pub struct DataSetNode;

#[async_trait]
impl NodeExecutor for DataSetNode {
    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let key = config.get("key").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("set 需要 key 参数"))?;
        let value = ctx.resolve_config(config.get("value").unwrap_or(&serde_json::Value::Null));
        ctx.set_var(key.to_string(), value.clone());
        Ok(value)
    }
}

// ═══════════════════════════════════════
// data_get — 读取变量
// ═══════════════════════════════════════

#[derive(Default)]
pub struct DataGetNode;

#[async_trait]
impl NodeExecutor for DataGetNode {
    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let key = config.get("key").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("get 需要 key 参数"))?;
        Ok(ctx.variables.get(key).cloned().unwrap_or(serde_json::Value::Null))
    }
}

// ═══════════════════════════════════════
// data_length — 获取长度
// ═══════════════════════════════════════

#[derive(Default)]
pub struct DataLengthNode;

#[async_trait]
impl NodeExecutor for DataLengthNode {
    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let source = config.get("source").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("length 需要 source 参数"))?;
        let value = resolve_source(source, ctx);
        let len = match &value {
            serde_json::Value::Array(arr) => arr.len(),
            serde_json::Value::String(s) => s.len(),
            serde_json::Value::Object(obj) => obj.len(),
            serde_json::Value::Null => 0,
            _ => 1,
        };
        let result = serde_json::json!(len);
        if let Some(key) = config.get("key").and_then(|v| v.as_str()) {
            ctx.set_var(key.to_string(), result.clone());
        }
        Ok(result)
    }
}

// ═══════════════════════════════════════
// data_default — 设置默认值
// ═══════════════════════════════════════

#[derive(Default)]
pub struct DataDefaultNode;

#[async_trait]
impl NodeExecutor for DataDefaultNode {
    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let key = config.get("key").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("default 需要 key 参数"))?;
        let default_val = ctx.resolve_config(config.get("value").unwrap_or(&serde_json::Value::Null));
        let current = ctx.variables.get(key).cloned().unwrap_or(serde_json::Value::Null);
        if is_null_or_empty(&current) {
            ctx.set_var(key.to_string(), default_val.clone());
            Ok(default_val)
        } else {
            Ok(current)
        }
    }
}

// ═══════════════════════════════════════
// data_merge — 合并对象
// ═══════════════════════════════════════

#[derive(Default)]
pub struct DataMergeNode;

#[async_trait]
impl NodeExecutor for DataMergeNode {
    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let target = config.get("target").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("merge 需要 target 参数"))?;
        let source = config.get("source").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("merge 需要 source 参数"))?;
        let mut base = ctx.variables.get(target).cloned().unwrap_or(serde_json::json!({}));
        let overlay = ctx.variables.get(source).cloned().unwrap_or(serde_json::json!({}));
        if let (Some(base_obj), Some(overlay_obj)) = (base.as_object_mut(), overlay.as_object()) {
            for (k, v) in overlay_obj { base_obj.insert(k.clone(), v.clone()); }
        }
        ctx.set_var(target.to_string(), base.clone());
        Ok(base)
    }
}
