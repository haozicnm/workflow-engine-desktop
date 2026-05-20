// nodes/registry.rs — 节点注册清单（v8: 从 node-schema.json 驱动）
//
// 设计原则：node-schema.json 是唯一真相来源。
// Rust 后端用 include_str! 编译期嵌入，前端也可读取同一份 JSON。

use serde::Deserialize;
use serde_json::Value;

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
    pub description: String,
    pub icon: String,
    pub is_container: bool,
    pub default_config: Value,
}

impl NodeManifest {
    fn from_schema(sn: &SchemaNode) -> Self {
        NodeManifest {
            node_type: sn.node_type.clone(),
            label: sn.label.clone(),
            description: sn.description.clone(),
            icon: sn.icon.clone(),
            is_container: sn.is_container,
            default_config: serde_json::json!({}),
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
    load_schema().nodes.iter().map(NodeManifest::from_schema).collect()
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
    load_schema().nodes.iter()
        .find(|n| n.node_type == node_type)
        .map(NodeManifest::from_schema)
}
