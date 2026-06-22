// engine/workflow.rs — 工作流数据结构
use serde::{Deserialize, Serialize};

/// 工作流格式版本
pub const FORMAT_VERSION: &str = "2.0";

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

/// 工作流元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowMeta {
    /// 创建者（agent / user / import）
    #[serde(default)]
    pub author: Option<String>,
    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 创建时间
    #[serde(default)]
    pub created_at: Option<String>,
    /// 最后修改时间
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// 节点位置（用于 Canvas 编辑器）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// 图中的边：连接两个节点的端口
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct Edge {
    /// 源节点 ID
    pub from: String,
    /// 源端口标签
    #[serde(alias = "fromPort", default)]
    pub from_port: String,
    /// 目标节点 ID
    pub to: String,
    /// 目标端口标签
    #[serde(alias = "toPort", default)]
    pub to_port: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Workflow {
    /// 工作流格式版本（用于兼容性检查）
    #[serde(default)]
    pub version: Option<String>,
    pub name: String,
    pub description: Option<String>,
    /// 元数据
    #[serde(default)]
    pub meta: Option<WorkflowMeta>,
    pub steps: Vec<Step>,
    #[serde(alias = "params")]
    pub variables: Option<std::collections::HashMap<String, serde_json::Value>>,
    /// v8: 图的边（可选，兼容线性模式）
    #[serde(default)]
    pub edges: Vec<Edge>,
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
    /// 下一步 ID（⚠️ 仅线性模式用。DAG 模式下由 edges 决定执行顺序，此字段被忽略）
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

    /// 条件执行（⚠️ 仅线性模式用。DAG 模式下由 edges 的 fromPort 决定条件路由，此字段被忽略）
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
