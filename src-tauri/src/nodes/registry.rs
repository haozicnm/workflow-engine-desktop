// nodes/registry.rs — 节点注册清单（v8.1: 从 node-schema.json 驱动，含端口定义）
//
// 设计原则：node-schema.json 是唯一真相来源。
// Rust 后端用 include_str! 编译期嵌入，前端也可读取同一份 JSON。

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ─── 端口类型系统 ───

/// 端口数据类型枚举
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DataType {
    Text,
    Number,
    Bool,
    Json,
    Array,
}

impl DataType {
    /// 检查实际 JSON 值是否与此类型兼容
    pub fn is_compatible(&self, value: &Value) -> bool {
        match self {
            DataType::Text => value.is_string(),
            DataType::Number => value.is_number(),
            DataType::Bool => value.is_boolean(),
            DataType::Json => value.is_object(),
            DataType::Array => value.is_array(),
        }
    }

    /// 返回类型名称字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            DataType::Text => "text",
            DataType::Number => "number",
            DataType::Bool => "bool",
            DataType::Json => "json",
            DataType::Array => "array",
        }
    }
}

/// 输入端口定义
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InputPortDef {
    pub name: String,
    pub data_type: DataType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub desc: String,
}

/// 输出端口定义
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OutputPortDef {
    pub name: String,
    pub data_type: DataType,
    #[serde(default)]
    pub desc: String,
}

/// 参数验证约束（可选，用于 Agent 知道参数边界）
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ParamValidation {
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default, rename = "minLength")]
    pub min_length: Option<usize>,
    #[serde(default, rename = "maxLength")]
    pub max_length: Option<usize>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default, rename = "patternDesc")]
    pub pattern_desc: Option<String>,
    #[serde(default)]
    pub desc: Option<String>,
}

/// 条件可见性规则（参数仅在条件满足时显示）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VisibleWhen {
    pub param: String,
    /// 精确匹配值（与 op 互斥）
    #[serde(default)]
    pub value: Option<Value>,
    /// 操作符: "empty" / "not_empty" / "eq" / "ne"
    #[serde(default)]
    pub op: Option<String>,
}

/// 参数示例
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ParamExample {
    #[serde(default)]
    pub desc: Option<String>,
    pub value: Value,
}

/// 参数定义（用于节点自描述 params schema）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ParamDef {
    pub name: String,
    /// 字段类型: string, number, boolean, select, json, code, file_path, text
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<Value>,
    #[serde(default)]
    pub desc: Option<String>,
    /// select 类型的可选值
    #[serde(default)]
    pub options: Option<Vec<String>>,
    /// basic 或 advanced
    #[serde(default)]
    pub group: Option<String>,
    /// code 类型的语言标记（如 "rhai"）
    #[serde(default)]
    pub lang: Option<String>,
    /// 验证约束（min/max/pattern 等）
    #[serde(default)]
    pub validation: Option<ParamValidation>,
    /// 条件可见性（仅当其他参数满足条件时显示）
    #[serde(default)]
    pub visible_when: Option<VisibleWhen>,
    /// 示例值（帮助 Agent 理解参数格式）
    #[serde(default)]
    pub examples: Option<Vec<ParamExample>>,
}

// ─── Schema 解析 ───

/// schema 中的节点定义
#[derive(Debug, Clone, Deserialize)]
struct SchemaNode {
    #[serde(rename = "type")]
    node_type: String,
    label: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    is_container: bool,
    #[serde(default, rename = "desc")]
    description: String,
    #[serde(default)]
    inputs: Vec<InputPortDef>,
    #[serde(default)]
    outputs: Vec<OutputPortDef>,
    #[serde(default)]
    params: Vec<ParamDef>,
    /// 节点标签（用于搜索和发现）
    #[serde(default)]
    tags: Vec<String>,
}

/// schema 顶层结构
#[derive(Debug, Clone, Deserialize)]
struct NodeSchema {
    #[serde(default)]
    nodes: Vec<SchemaNode>,
    #[serde(default)]
    container_types: Vec<String>,
}

/// 节点元数据清单（公开 API）
#[derive(Debug, Clone, Serialize)]
pub struct NodeManifest {
    pub node_type: String,
    pub label: String,
    pub category: String,
    pub description: String,
    pub icon: String,
    pub is_container: bool,
    pub default_config: Value,
    /// 输入端口定义列表
    pub inputs: Vec<InputPortDef>,
    /// 输出端口定义列表
    pub outputs: Vec<OutputPortDef>,
    /// 参数定义列表（节点自描述 params schema）
    pub params: Vec<ParamDef>,
    /// 节点标签（用于搜索和发现）
    pub tags: Vec<String>,
}

impl NodeManifest {
    fn from_schema(sn: &SchemaNode) -> Self {
        NodeManifest {
            node_type: sn.node_type.clone(),
            label: sn.label.clone(),
            category: sn.category.clone(),
            description: sn.description.clone(),
            icon: sn.icon.clone(),
            is_container: sn.is_container,
            default_config: serde_json::json!({}),
            inputs: sn.inputs.clone(),
            outputs: sn.outputs.clone(),
            params: sn.params.clone(),
            tags: sn.tags.clone(),
        }
    }
}

/// 编译期嵌入 + 运行时懒解析（只解析一次）
static SCHEMA: std::sync::LazyLock<NodeSchema> = std::sync::LazyLock::new(|| {
    let json_str = include_str!("../../node-schema.json");
    serde_json::from_str(json_str).expect("node-schema.json 格式错误")
});

/// 返回所有已注册的容器类型
pub fn container_types() -> Vec<String> {
    SCHEMA.container_types.clone()
}

/// 返回所有已注册的节点清单
pub fn all_nodes() -> Vec<NodeManifest> {
    SCHEMA.nodes.iter().map(NodeManifest::from_schema).collect()
}

/// 检查某类型是否为容器
pub fn is_container(node_type: &str) -> bool {
    SCHEMA.container_types.iter().any(|t| t == node_type)
}

/// 检查某类型是否在 schema 中注册
pub fn is_registered(node_type: &str) -> bool {
    SCHEMA.nodes.iter().any(|n| n.node_type == node_type)
}

/// 按类型获取节点元数据
pub fn get_node(node_type: &str) -> Option<NodeManifest> {
    SCHEMA
        .nodes
        .iter()
        .find(|n| n.node_type == node_type)
        .map(NodeManifest::from_schema)
}

/// 获取指定节点类型的输入端口定义
pub fn get_input_ports(node_type: &str) -> Vec<InputPortDef> {
    get_node(node_type).map(|n| n.inputs).unwrap_or_default()
}

/// 获取指定节点类型的输出端口定义
pub fn get_output_ports(node_type: &str) -> Vec<OutputPortDef> {
    get_node(node_type).map(|n| n.outputs).unwrap_or_default()
}

/// 获取指定节点类型的必需输入端口名列表
pub fn get_required_inputs(node_type: &str) -> Vec<String> {
    get_input_ports(node_type)
        .into_iter()
        .filter(|p| p.required)
        .map(|p| p.name)
        .collect()
}

/// 按标签搜索节点（任意标签匹配即返回）
pub fn search_by_tags(query: &str) -> Vec<NodeManifest> {
    let q = query.to_lowercase();
    SCHEMA
        .nodes
        .iter()
        .filter(|n| {
            n.tags.iter().any(|t| t.to_lowercase().contains(&q))
                || n.label.to_lowercase().contains(&q)
                || n.description.to_lowercase().contains(&q)
                || n.node_type.to_lowercase().contains(&q)
        })
        .map(NodeManifest::from_schema)
        .collect()
}

/// 获取所有分类及其节点数量
pub fn categories() -> Vec<(String, usize)> {
    let mut map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for n in SCHEMA.nodes.iter() {
        *map.entry(n.category.clone()).or_insert(0) += 1;
    }
    let mut result: Vec<(String, usize)> = map.into_iter().collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

// ─── 端口校验 ───

/// 端口校验警告（非致命，不阻断执行）
#[derive(Debug, Clone)]
pub struct PortValidationWarning {
    pub step_id: String,
    pub port_name: String,
    pub message: String,
}

/// 校验单个步骤的端口约束
///
/// 检查规则：
/// 1. 必需输入端口必须有 config 中对应的值（直接值或模板引用）
/// 2. 输出端口提示（仅文档性质）
///
/// 返回警告列表（空 = 无问题）
pub fn validate_step_ports(
    step_id: &str,
    node_type: &str,
    config: &Value,
) -> Vec<PortValidationWarning> {
    let mut warnings = Vec::new();

    let required_inputs = get_required_inputs(node_type);
    for port_name in &required_inputs {
        // 检查 config 中是否有该端口的值
        let has_value = match config {
            Value::Object(map) => {
                if let Some(val) = map.get(port_name.as_str()) {
                    // 非 null 非空字符串 = 有值
                    !(val.is_null() || val.is_string() && val.as_str().unwrap_or("").is_empty())
                } else {
                    false
                }
            }
            _ => false,
        };

        if !has_value {
            warnings.push(PortValidationWarning {
                step_id: step_id.to_string(),
                port_name: port_name.clone(),
                message: format!("必需输入端口 '{}' 未配置值", port_name),
            });
        }
    }

    warnings
}
