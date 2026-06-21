// nodes/data.rs — 数据处理节点（v3: 每个操作独立 executor）
//
// data_set     — 设置变量
// data_get     — 读取变量
// data_length  — 获取长度
// data_default — 设置默认值
// data_merge   — 合并对象

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;

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
        ctx.variables
            .get(source)
            .cloned()
            .unwrap_or(serde_json::Value::Null)
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
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "data_set".into(), version: "1.0".into(),
            display_name: "设置变量".into(), description: "设置工作流变量键值对".into(),
            category: "数据".into(), inputs: vec![],
            outputs: vec![crate::nodes::traits::PortDef { label: "result".into(), data_type: "any".into(), required: false }],
            config_schema: serde_json::json!({"type": "object", "required": ["key"], "properties": {"key": {"type": "string"}, "value": {"type": "string"}}}),
            params: vec![],
        }
    }


    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let key = config
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("set 需要 key 参数"))?;
        let value = ctx.resolve_config(config.get("value").unwrap_or(&serde_json::Value::Null));
        ctx.set_var(key.to_string(), value.clone());
        // 返回 {key: value} 对象，支持 {{stepId.key}} 字段访问
        Ok(serde_json::json!({ key: value }))
    }
}

// ═══════════════════════════════════════
// data_get — 读取变量
// ═══════════════════════════════════════

#[derive(Default)]
pub struct DataGetNode;

#[async_trait]
impl NodeExecutor for DataGetNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "data_get".into(), version: "1.0".into(),
            display_name: "读取变量".into(), description: "读取工作流变量的值".into(),
            category: "数据".into(), inputs: vec![],
            outputs: vec![crate::nodes::traits::PortDef { label: "result".into(), data_type: "any".into(), required: false }],
            config_schema: serde_json::json!({"type": "object", "required": ["key"], "properties": {"key": {"type": "string"}}}),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let key = config
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("get 需要 key 参数"))?;
        Ok(ctx
            .variables
            .get(key)
            .cloned()
            .unwrap_or(serde_json::Value::Null))
    }
}

// ═══════════════════════════════════════
// data_length — 获取长度
// ═══════════════════════════════════════

#[derive(Default)]
pub struct DataLengthNode;

#[async_trait]
impl NodeExecutor for DataLengthNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "data_length".into(), version: "1.0".into(),
            display_name: "数据长度".into(), description: "获取字符串或数组的长度".into(),
            category: "数据".into(), inputs: vec![],
            outputs: vec![crate::nodes::traits::PortDef { label: "result".into(), data_type: "number".into(), required: false }],
            config_schema: serde_json::json!({"type": "object"}),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let source = config
            .get("source")
            .and_then(|v| v.as_str())
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
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "data_default".into(), version: "1.0".into(),
            display_name: "默认值".into(), description: "变量不存在时设置默认值".into(),
            category: "数据".into(), inputs: vec![],
            outputs: vec![crate::nodes::traits::PortDef { label: "result".into(), data_type: "any".into(), required: false }],
            config_schema: serde_json::json!({"type": "object"}),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let key = config
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("default 需要 key 参数"))?;
        let default_val =
            ctx.resolve_config(config.get("value").unwrap_or(&serde_json::Value::Null));
        let current = ctx
            .variables
            .get(key)
            .cloned()
            .unwrap_or(serde_json::Value::Null);
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
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "data_merge".into(), version: "1.0".into(),
            display_name: "合并数据".into(), description: "合并多个数据源".into(),
            category: "数据".into(), inputs: vec![],
            outputs: vec![crate::nodes::traits::PortDef { label: "result".into(), data_type: "object".into(), required: false }],
            config_schema: serde_json::json!({"type": "object"}),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let target = config
            .get("target")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("merge 需要 target 参数"))?;
        let source = config
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("merge 需要 source 参数"))?;
        let mut base = ctx
            .variables
            .get(target)
            .cloned()
            .unwrap_or(serde_json::json!({}));
        let overlay = ctx
            .variables
            .get(source)
            .cloned()
            .unwrap_or(serde_json::json!({}));
        if let (Some(base_obj), Some(overlay_obj)) = (base.as_object_mut(), overlay.as_object()) {
            for (k, v) in overlay_obj {
                base_obj.insert(k.clone(), v.clone());
            }
        }
        ctx.set_var(target.to_string(), base.clone());
        Ok(base)
    }
}
