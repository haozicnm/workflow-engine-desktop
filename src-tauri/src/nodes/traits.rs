// nodes/traits.rs — 节点执行器 trait (v8 升级：从 FF 移植类型元数据)
//
// v8 新增：
//   - type_def()      — 声明式类型元数据（版本/端口/JSON Schema）
//   - validate_config() — 执行前配置校验
//   - NodeTypeDef / PortDef — 自描述节点定义
//
// 向后兼容：现有 34 个节点只需实现 execute()，新方法有默认值。

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 端口定义（输入或输出）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortDef {
    /// 端口标签，用于变量引用：{{nodeId.portLabel}}
    pub label: String,
    /// 数据类型提示（string / number / object / any）
    #[serde(default = "default_port_type")]
    pub data_type: String,
    /// 是否必须
    #[serde(default)]
    pub required: bool,
}

fn default_port_type() -> String {
    "any".to_string()
}

/// 节点类型定义（自描述元数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTypeDef {
    /// 唯一类型名（如 "http", "shell", "condition"）
    pub type_name: String,
    /// 语义化版本
    #[serde(default = "default_version")]
    pub version: String,
    /// 人类可读的显示名
    pub display_name: String,
    /// 描述
    pub description: String,
    /// 分类（用于节点面板分组）
    pub category: String,
    /// 输入端口定义
    pub inputs: Vec<PortDef>,
    /// 输出端口定义
    pub outputs: Vec<PortDef>,
    /// JSON Schema 描述 config 字段
    pub config_schema: serde_json::Value,
}

fn default_version() -> String {
    "1.0".to_string()
}

/// 配置校验错误
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

#[async_trait]
pub trait NodeExecutor: Send + Sync {
    // ─── v8 新增：类型元数据 ───────────────────────────

    /// 返回节点类型定义（元数据：版本/端口/Schema）
    ///
    /// 默认返回一个最小定义，节点作者应该 override 这个方法。
    fn type_def(&self) -> NodeTypeDef {
        NodeTypeDef {
            type_name: "unknown".to_string(),
            version: "1.0".to_string(),
            display_name: "Unknown Node".to_string(),
            description: String::new(),
            category: "uncategorized".to_string(),
            inputs: vec![],
            outputs: vec![PortDef {
                label: "result".to_string(),
                data_type: "any".to_string(),
                required: false,
            }],
            config_schema: serde_json::json!({"type": "object"}),
        }
    }

    /// 配置预校验（在变量解析后、执行前调用）
    ///
    /// 返回 Ok(()) 或错误列表。默认接受一切。
    fn validate_config(
        &self,
        _config: &serde_json::Value,
    ) -> std::result::Result<(), Vec<ValidationError>> {
        Ok(())
    }

    // ─── v7 原有：执行方法 ─────────────────────────────

    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value>;

    /// 是否由节点自行解析 config 中的模板变量。
    /// 返回 true 时，executor 跳过全局 `ctx.resolve_config(&step.config)`，
    /// 由节点在迭代期间自行处理（如 map 节点的 `{{__item}}` 模板）。
    fn resolve_config_self(&self) -> bool {
        false
    }
}
