// nodes/registry.rs — 节点注册清单（v8.1: 从 node-schema.json 驱动，含端口定义）
//
// 设计原则：node-schema.json 是唯一真相来源。
// Rust 后端用 include_str! 编译期嵌入，前端也可读取同一份 JSON。

use serde::Deserialize;
use serde_json::Value;

// ─── 端口类型系统 ───

/// 端口数据类型枚举
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Deserialize)]
pub struct InputPortDef {
    pub name: String,
    pub data_type: DataType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub desc: String,
}

/// 输出端口定义
#[derive(Debug, Clone, Deserialize)]
pub struct OutputPortDef {
    pub name: String,
    pub data_type: DataType,
    #[serde(default)]
    pub desc: String,
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
#[derive(Debug, Clone)]
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
        }
    }
}

/// 编译期加载 schema
fn load_schema() -> NodeSchema {
    let json_str = include_str!("../../node-schema.json");
    serde_json::from_str(json_str).expect("node-schema.json 格式错误")
}

/// 返回所有已注册的容器类型
pub fn container_types() -> Vec<String> {
    load_schema().container_types
}

/// 返回所有已注册的节点清单
pub fn all_nodes() -> Vec<NodeManifest> {
    load_schema()
        .nodes
        .iter()
        .map(NodeManifest::from_schema)
        .collect()
}

/// 检查某类型是否为容器
pub fn is_container(node_type: &str) -> bool {
    load_schema().container_types.iter().any(|t| t == node_type)
}

/// 检查某类型是否在 schema 中注册
pub fn is_registered(node_type: &str) -> bool {
    load_schema().nodes.iter().any(|n| n.node_type == node_type)
}

/// 按类型获取节点元数据
pub fn get_node(node_type: &str) -> Option<NodeManifest> {
    load_schema()
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
                    !val.is_null() && !(val.is_string() && val.as_str().unwrap_or("").is_empty())
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
