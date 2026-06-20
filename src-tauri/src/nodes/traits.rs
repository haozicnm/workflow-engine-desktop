// nodes/traits.rs — 节点执行器 trait (v8 升级：从 FF 移植类型元数据)
//
// v8 新增：
//   - type_def()      — 声明式类型元数据（版本/端口/JSON Schema）
//   - validate_config() — 执行前配置校验
//   - NodeTypeDef / PortDef — 自描述节点定义
//   - DisplayOptions / ConditionOp / ParamDef — 声明式条件显示（参考 n8n）
//
// 向后兼容：现有 34 个节点只需实现 execute()，新方法有默认值。

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// ─── DisplayOptions: 声明式条件显示（参考 n8n） ───────────────────

/// 条件运算符
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", content = "value")]
pub enum ConditionOp {
    #[serde(rename = "eq")] Eq(serde_json::Value),
    #[serde(rename = "not")] Not(serde_json::Value),
    #[serde(rename = "gte")] Gte(f64),
    #[serde(rename = "lte")] Lte(f64),
    #[serde(rename = "gt")] Gt(f64),
    #[serde(rename = "lt")] Lt(f64),
    #[serde(rename = "between")] Between { from: f64, to: f64 },
    #[serde(rename = "startsWith")] StartsWith(String),
    #[serde(rename = "endsWith")] EndsWith(String),
    #[serde(rename = "includes")] Includes(String),
    #[serde(rename = "regex")] Regex(String),
    #[serde(rename = "exists")] Exists,
}

/// 单个条件值：简单值匹配 或 高级条件运算
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConditionValue {
    Simple(serde_json::Value),
    Advanced { _cnd: ConditionOp },
}

/// 参数级条件显示规则
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DisplayOptions {
    /// show: 所有条件必须同时满足（AND 逻辑）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show: Option<HashMap<String, Vec<ConditionValue>>>,
    /// hide: 任一条件满足即隐藏（OR 逻辑）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hide: Option<HashMap<String, Vec<ConditionValue>>>,
}

/// 端口定义（输入或输出）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortDef {
    pub label: String,
    #[serde(default = "default_port_type")]
    pub data_type: String,
    #[serde(default)]
    pub required: bool,
}

fn default_port_type() -> String {
    "any".to_string()
}

/// 参数定义（schema-driven，支持条件显示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    #[serde(rename = "field_type")]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// 条件显示规则（n8n displayOptions 风格）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_options: Option<DisplayOptions>,
}

/// 节点类型定义（自描述元数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTypeDef {
    pub type_name: String,
    #[serde(default = "default_version")]
    pub version: String,
    pub display_name: String,
    pub description: String,
    pub category: String,
    pub inputs: Vec<PortDef>,
    pub outputs: Vec<PortDef>,
    pub config_schema: serde_json::Value,
    /// 声明式参数定义（支持条件显示，优先级高于 config_schema）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<ParamDef>,
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
            params: vec![],
        }
    }

    fn validate_config(
        &self,
        _config: &serde_json::Value,
    ) -> std::result::Result<(), Vec<ValidationError>> {
        Ok(())
    }

    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value>;

    fn resolve_config_self(&self) -> bool {
        false
    }
}
