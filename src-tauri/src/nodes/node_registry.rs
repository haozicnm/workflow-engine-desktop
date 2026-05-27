// nodes/node_registry.rs — 运行时节点注册表
//
// 设计原则：
//   1. 内置节点元数据来自 node-schema.json（由 registry.rs include_str! 加载）
//   2. 运行时可通过 register_node() 动态注册插件节点
//   3. 查询时优先查运行时注册表，回退到 JSON schema
//
// 这样新增节点类型只需：
//   - 内置节点 → 修改 node-schema.json
//   - 插件节点 → 调用 register_node()

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::registry;

/// 运行时节点元数据（仅用于插件注册，内置节点用 JSON schema）
#[derive(Debug, Clone)]
pub struct NodeMeta {
    pub type_name: String,
    pub label: String,
    pub category: String,
    pub icon: String,
    pub is_container: bool,
    pub description: String,
}

/// 运行时注册表（仅存储插件节点，内置节点从 JSON 查询）
pub struct NodeRegistry {
    /// 插件节点：type_name → metadata
    plugin_nodes: HashMap<String, NodeMeta>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {
            plugin_nodes: HashMap::new(),
        }
    }

    /// 注册一个插件节点类型
    pub fn register(&mut self, meta: NodeMeta) {
        self.plugin_nodes.insert(meta.type_name.clone(), meta);
    }

    /// 查询节点分类（优先插件，回退 JSON schema）
    pub fn get_category(&self, type_name: &str) -> String {
        // 1. 查插件注册表
        if let Some(meta) = self.plugin_nodes.get(type_name) {
            return meta.category.clone();
        }
        // 2. 回退到 JSON schema
        registry::get_node(type_name)
            .map(|n| n.category.clone())
            .unwrap_or_else(|| "core".to_string())
    }

    /// 是否为容器节点（优先插件，回退 JSON schema）
    pub fn is_container(&self, type_name: &str) -> bool {
        if let Some(meta) = self.plugin_nodes.get(type_name) {
            return meta.is_container;
        }
        registry::is_container(type_name)
    }

    /// 查询节点元数据
    pub fn get(&self, type_name: &str) -> Option<&NodeMeta> {
        self.plugin_nodes.get(type_name)
    }

    /// 列出所有节点类型（JSON schema + 插件），含端口定义
    pub fn list_all_types(&self) -> Vec<serde_json::Value> {
        // 内置节点从 JSON schema 获取
        let mut result: Vec<serde_json::Value> = registry::all_nodes()
            .iter()
            .map(|n| {
                let inputs: Vec<serde_json::Value> = n
                    .inputs
                    .iter()
                    .map(|p| {
                        serde_json::json!({
                            "name": p.name,
                            "data_type": p.data_type.as_str(),
                            "required": p.required,
                            "desc": p.desc,
                        })
                    })
                    .collect();
                let outputs: Vec<serde_json::Value> = n
                    .outputs
                    .iter()
                    .map(|p| {
                        serde_json::json!({
                            "name": p.name,
                            "data_type": p.data_type.as_str(),
                            "desc": p.desc,
                        })
                    })
                    .collect();
                serde_json::json!({
                    "type": n.node_type,
                    "label": n.label,
                    "icon": n.icon,
                    "desc": n.description,
                    "is_container": n.is_container,
                    "category": n.category,
                    "inputs": inputs,
                    "outputs": outputs,
                    "source": "builtin",
                })
            })
            .collect();

        // 插件节点追加（无端口定义）
        for meta in self.plugin_nodes.values() {
            result.push(serde_json::json!({
                "type": meta.type_name,
                "label": meta.label,
                "icon": meta.icon,
                "desc": meta.description,
                "is_container": meta.is_container,
                "category": meta.category,
                "inputs": [],
                "outputs": [],
                "source": "plugin",
            }));
        }

        result
    }

    /// 插件节点数量
    pub fn plugin_count(&self) -> usize {
        self.plugin_nodes.len()
    }
}

/// 全局注册表实例
static REGISTRY: OnceLock<Mutex<NodeRegistry>> = OnceLock::new();

/// 获取全局注册表
pub fn global_registry() -> &'static Mutex<NodeRegistry> {
    REGISTRY.get_or_init(|| Mutex::new(NodeRegistry::new()))
}

/// 注册一个插件节点到全局注册表
pub fn register_node(meta: NodeMeta) {
    let reg = global_registry();
    let mut guard = reg.lock().unwrap();
    tracing::info!("注册插件节点: {} ({})", meta.type_name, meta.category);
    guard.register(meta);
}

/// 查询节点分类
pub fn get_category(type_name: &str) -> String {
    let reg = global_registry();
    let guard = reg.lock().unwrap();
    guard.get_category(type_name)
}

/// 查询是否为容器节点
pub fn is_container_node(type_name: &str) -> bool {
    let reg = global_registry();
    let guard = reg.lock().unwrap();
    guard.is_container(type_name)
}

/// 列出所有节点类型（JSON 格式）
pub fn list_all_types() -> Vec<serde_json::Value> {
    let reg = global_registry();
    let guard = reg.lock().unwrap();
    guard.list_all_types()
}

/// 获取指定节点的输入端口定义
pub fn get_input_ports(type_name: &str) -> Vec<registry::InputPortDef> {
    registry::get_input_ports(type_name)
}

/// 获取指定节点的输出端口定义
pub fn get_output_ports(type_name: &str) -> Vec<registry::OutputPortDef> {
    registry::get_output_ports(type_name)
}

/// 校验步骤端口约束（返回警告列表，空 = 无问题）
pub fn validate_step_ports(
    step_id: &str,
    node_type: &str,
    config: &serde_json::Value,
) -> Vec<registry::PortValidationWarning> {
    registry::validate_step_ports(step_id, node_type, config)
}

/// 初始化（无需硬编码，JSON schema 已由 registry.rs 在编译期加载）
/// 保留此函数作为未来运行时初始化钩子（如加载插件目录）
pub fn init_runtime() {
    tracing::info!(
        "节点注册表就绪：内置 {} 个节点 (JSON schema)，插件 0 个",
        registry::all_nodes().len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_registration() {
        let mut reg = NodeRegistry::new();
        reg.register(NodeMeta {
            type_name: "custom_node".into(),
            label: "自定义节点".into(),
            category: "plugin".into(),
            icon: "star".into(),
            is_container: false,
            description: "A custom plugin node".into(),
        });

        assert_eq!(reg.plugin_count(), 1);
        assert_eq!(reg.get_category("custom_node"), "plugin");
        assert!(!reg.is_container("custom_node"));
    }

    #[test]
    fn test_fallback_to_json_schema() {
        let reg = NodeRegistry::new();
        // http 是内置节点，应从 JSON schema 回退
        assert!(!reg.is_container("http"));
    }

    #[test]
    fn test_port_definitions_loaded() {
        // loop 节点应有 1 个必需输入端口 (array)
        let inputs = get_input_ports("loop");
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].name, "array");
        assert!(inputs[0].required);

        // loop 节点应有 2 个输出端口
        let outputs = get_output_ports("loop");
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].name, "results");
        assert_eq!(outputs[1].name, "count");
    }

    #[test]
    fn test_no_ports_for_simple_nodes() {
        // http 节点无必需输入
        let inputs = get_input_ports("http");
        assert!(inputs.is_empty());

        // 但有 3 个输出
        let outputs = get_output_ports("http");
        assert_eq!(outputs.len(), 3);
    }

    #[test]
    fn test_port_validation_missing_required() {
        // loop 节点必需 array 输入，空 config 应产生警告
        let config = serde_json::json!({});
        let warnings = validate_step_ports("s1", "loop", &config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].port_name, "array");
    }

    #[test]
    fn test_port_validation_passes_with_value() {
        // 提供了 array 值，不应有警告
        let config = serde_json::json!({ "array": "{{items}}" });
        let warnings = validate_step_ports("s1", "loop", &config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_list_all_types_includes_ports() {
        let all = list_all_types();
        let loop_node = all.iter().find(|n| n["type"] == "loop").unwrap();
        assert!(loop_node["inputs"].is_array());
        assert!(loop_node["outputs"].is_array());
        assert_eq!(loop_node["inputs"][0]["name"], "array");
        assert_eq!(loop_node["inputs"][0]["required"], true);
    }
}
