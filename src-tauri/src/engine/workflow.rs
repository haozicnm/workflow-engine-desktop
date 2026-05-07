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


/// 可视化条件（单个条件）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicCondition {
    pub id: String,
    pub left: String,
    pub op: String,
    #[serde(default)]
    pub right: String,
}

/// 可视化条件组（支持 AND/OR 组合）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicConditionGroup {
    #[serde(default = "default_combinator")]
    pub combinator: String,
    #[serde(default)]
    pub conditions: Vec<LogicCondition>,
}

fn default_combinator() -> String {
    "and".to_string()
}

impl Default for LogicConditionGroup {
    fn default() -> Self {
        Self {
            combinator: "and".to_string(),
            conditions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Step {
    pub id: String,
    #[serde(alias = "label", default)]
    pub name: String,
    #[serde(rename = "type")]
    pub step_type: String,
    #[serde(default)]
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
    /// 容器节点的动作列表（新格式）
    #[serde(default)]
    pub actions: Option<Vec<serde_json::Value>>,
    /// 是否展开显示
    #[serde(default)]
    pub expanded: Option<bool>,
    /// 逻辑分支条件
    #[serde(default)]
    pub condition: Option<String>,
    /// 可视化条件组（新格式，支持 AND/OR）
    #[serde(alias = "conditionGroup", default)]
    pub condition_group: Option<LogicConditionGroup>,
    /// 条件为真时的子步骤
    #[serde(default)]
    pub then_steps: Option<Vec<Step>>,
    /// 条件为假时的子步骤
    #[serde(default)]
    pub else_steps: Option<Vec<Step>>,
    /// 条件执行（引用逻辑步骤的 branch）
    #[serde(alias = "runCondition", default)]
    pub run_condition: Option<RunCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunCondition {
    /// 引用的逻辑步骤 ID
    #[serde(alias = "ref")]
    pub ref_step: String,
    /// branch 为该值时执行
    #[serde(default = "default_when")]
    pub when: String,
}

fn default_when() -> String {
    "true".to_string()
}

impl RunCondition {
    pub fn should_run(&self, branch: &str) -> bool {
        self.when == "both" || self.when == branch
    }

    /// 是否为 merge 模式（等待所有分支完成后执行一次）
    pub fn is_merge(&self) -> bool {
        self.when == "merge"
    }
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
