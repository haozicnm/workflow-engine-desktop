// engine/workflow.rs — 工作流数据结构
use serde::{Deserialize, Serialize};

/// 步骤失败时的处理策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ErrorStrategy {
    /// 终止工作流（默认）
    #[default]
    Fail,
    /// 忽略错误，继续执行下一个步骤
    Ignore,
    /// 跳转到指定步骤继续执行
    Branch { step_id: String },
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct Workflow {
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<Step>,
    pub variables: Option<std::collections::HashMap<String, serde_json::Value>>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub step_type: String,
    pub config: serde_json::Value,
    pub next: Option<String>,
    pub retry: Option<RetryConfig>,
    pub timeout: Option<u64>,
    /// 循环节点的子步骤
    #[serde(default)]
    pub body_steps: Option<Vec<Step>>,
    /// 断点标记
    #[serde(default)]
    pub breakpoint: bool,
    /// 步骤延迟（毫秒），执行前等待
    #[serde(default)]
    pub delay: Option<u64>,
    /// 错误处理策略（默认 fail）
    #[serde(default)]
    pub on_error: Option<ErrorStrategy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max: u32,
    pub delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        RetryConfig {
            max: 3,
            delay_ms: 1000,
        }
    }
}
