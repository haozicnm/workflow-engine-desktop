// engine/dag.rs — DAG 工作流数据结构和拓扑排序
//
// 支持两种输入路径：
//   1. DAGWorkflow (DAGNode + DAGEdge) — 旧版 JSON 工作流
//   2. FlowNode + FlowEdge — 前端画布直接传入的节点/连线

use crate::engine::workflow::{Step, ErrorStrategy};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

// ═══════════════════════════════════════════════════════════════
// 前端画布直接传入的节点/连线（P2 新增）
// ═══════════════════════════════════════════════════════════════

/// 前端传来的画布节点（匹配 FlowEditor → pinTypes.ts FlowNode）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,   // "http", "data", "script", "http" 等
    pub label: String,        // 节点显示名 → Step.name
    pub position: FlowPosition,
    pub config: serde_json::Value,
    /// 错误处理策略（可选）
    #[serde(default)]
    pub on_error: Option<ErrorStrategy>,
    /// 断点标记（可选）
    #[serde(default)]
    pub breakpoint: bool,
    /// 步骤延迟（可选，毫秒）
    #[serde(default)]
    pub delay: Option<u64>,
    /// 超时（可选，秒）
    #[serde(default)]
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowPosition {
    pub x: f64,
    pub y: f64,
}

/// 前端传来的画布连线（匹配 FlowEditor → pinTypes.ts FlowEdge）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEdge {
    pub id: String,
    pub source: String,              // 源节点 ID
    pub target: String,              // 目标节点 ID
    #[serde(rename = "sourceHandle", default)]
    pub source_handle: String,       // 源针脚名
    #[serde(rename = "targetHandle", default)]
    pub target_handle: String,       // 目标针脚名
}

// ═══════════════════════════════════════════════════════════════
// DAG 执行计划
// ═══════════════════════════════════════════════════════════════

/// 拓扑排序后单个步骤的完整信息
#[derive(Debug, Clone)]
pub struct ExecStep {
    pub node_id: String,            // 节点原始 ID
    pub step: Step,                 // 转换为现有的 Step 结构
    pub dependencies: Vec<String>,  // 依赖的节点 ID 列表
}

/// DAG 执行计划
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    /// 拓扑排序后的执行步骤列表
    pub ordered_steps: Vec<ExecStep>,
    /// 可并行的步骤组（每组内的步骤无相互依赖，按 ordered_steps 的索引）
    pub parallel_groups: Vec<Vec<usize>>,
    /// 每个节点的入度（向后兼容）
    pub in_degree: HashMap<String, usize>,
    /// 每个节点的后继
    pub successors: HashMap<String, Vec<String>>,
    /// 每个节点的前驱
    pub predecessors: HashMap<String, Vec<String>>,
    /// 拓扑排序顺序（仅节点 ID，向后兼容）
    pub order: Vec<String>,
}

// ═══════════════════════════════════════════════════════════════
// build_dag — P2 核心入口：FlowNode + FlowEdge → ExecutionPlan
// ═══════════════════════════════════════════════════════════════

/// 从前端画布的节点与连线构建 DAG 执行计划
///
/// 1. 验证连线引用有效
/// 2. 构建邻接表
/// 3. Kahn 算法拓扑排序（检测环）
/// 4. 识别可并行执行的步骤组
/// 5. FlowNode → Step 转换（含 on_error, breakpoint, delay, timeout）
pub fn build_dag(
    nodes: &[FlowNode],
    edges: &[FlowEdge],
) -> Result<ExecutionPlan, String> {
    if nodes.is_empty() {
        return Err("工作流至少需要一个节点".to_string());
    }

    let node_ids: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();

    // ── 1. 构建邻接表 ──
    let mut successors: HashMap<String, Vec<String>> = HashMap::new();
    let mut predecessors: HashMap<String, Vec<String>> = HashMap::new();
    let mut in_degree: HashMap<String, usize> = HashMap::new();

    for node in nodes {
        successors.entry(node.id.clone()).or_default();
        predecessors.entry(node.id.clone()).or_default();
        in_degree.entry(node.id.clone()).or_insert(0);
    }

    for edge in edges {
        // 验证
        if !node_ids.contains(edge.source.as_str()) {
            return Err(format!(
                "连线 '{}' 引用了不存在的源节点: {}",
                edge.id, edge.source
            ));
        }
        if !node_ids.contains(edge.target.as_str()) {
            return Err(format!(
                "连线 '{}' 引用了不存在的目标节点: {}",
                edge.id, edge.target
            ));
        }
        // 去重（同一对 source→target 只计数一次）
        let succs = successors.entry(edge.source.clone()).or_default();
        if !succs.contains(&edge.target) {
            succs.push(edge.target.clone());
            predecessors
                .entry(edge.target.clone())
                .or_default()
                .push(edge.source.clone());
            *in_degree.entry(edge.target.clone()).or_insert(0) += 1;
        }
    }

    // ── 2. Kahn 算法拓扑排序 ──
    let mut queue: VecDeque<String> = VecDeque::new();
    for (id, &deg) in &in_degree {
        if deg == 0 {
            queue.push_back(id.clone());
        }
    }

    let mut order: Vec<String> = Vec::new();
    let mut level_map: HashMap<String, usize> = HashMap::new();

    while let Some(node_id) = queue.pop_front() {
        // 计算层级（拓扑层级 = max(前驱层级) + 1）
        let level = predecessors
            .get(&node_id)
            .map(|preds| {
                preds
                    .iter()
                    .filter_map(|p| level_map.get(p))
                    .max()
                    .copied()
                    .unwrap_or(0)
                    + 1
            })
            .unwrap_or(0);
        level_map.insert(node_id.clone(), level);

        order.push(node_id.clone());

        if let Some(succs) = successors.get(&node_id) {
            for succ in succs {
                if let Some(deg) = in_degree.get_mut(succ) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(succ.clone());
                    }
                }
            }
        }
    }

    // 环检测
    if order.len() != nodes.len() {
        let unvisited: Vec<String> = nodes
            .iter()
            .map(|n| n.id.clone())
            .filter(|id| !order.contains(id))
            .collect();
        return Err(format!(
            "工作流包含环或无法到达的节点 ({} / {} 已排序): {:?}",
            order.len(),
            nodes.len(),
            unvisited
        ));
    }

    // ── 3. 识别并行组 ──
    // 相同层级且无互依赖的节点可并行
    let mut by_level: HashMap<usize, Vec<usize>> = HashMap::new();
    for (idx, node_id) in order.iter().enumerate() {
        let level = level_map.get(node_id).copied().unwrap_or(0);
        by_level.entry(level).or_default().push(idx);
    }

    let mut parallel_groups: Vec<Vec<usize>> = Vec::new();
    for indices in by_level.values() {
        if indices.len() > 1 {
            parallel_groups.push(indices.clone());
        }
    }

    // ── 4. FlowNode → ExecStep 转换 ──
    let node_map: HashMap<&str, &FlowNode> =
        nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    let mut ordered_steps: Vec<ExecStep> = Vec::new();

    for node_id in &order {
        let node = node_map.get(node_id.as_str()).unwrap();

        // 收集依赖
        let deps: Vec<String> = predecessors
            .get(node_id)
            .cloned()
            .unwrap_or_default();

        let step = flow_node_to_step(node);

        ordered_steps.push(ExecStep {
            node_id: node_id.clone(),
            step,
            dependencies: deps,
        });
    }

    Ok(ExecutionPlan {
        ordered_steps,
        parallel_groups,
        in_degree: nodes.iter().map(|n| (n.id.clone(), 0usize)).collect(), // 重建（已被修改）
        successors,
        predecessors,
        order,
    })
}

/// FlowNode → Step 转换
///
/// 字段映射：
///   FlowNode.id          → Step.id
///   FlowNode.label       → Step.name
///   FlowNode.node_type   → Step.step_type
///   FlowNode.config      → Step.config（模板替换在执行期由 context 完成）
///   FlowNode.on_error    → Step.on_error
///   FlowNode.breakpoint  → Step.breakpoint
///   FlowNode.delay       → Step.delay
///   FlowNode.timeout     → Step.timeout
fn flow_node_to_step(node: &FlowNode) -> Step {
    Step {
        id: node.id.clone(),
        name: node.label.clone(),
        step_type: node.node_type.clone(),
        config: node.config.clone(),
        next: None,        // DAG 中 next 由拓扑顺序决定
        retry: None,       // 后续可在 FlowNode 中添加
        timeout: node.timeout,
        body_steps: None,
        breakpoint: node.breakpoint,
        delay: node.delay,
        on_error: node.on_error.clone(),
    }
}

/// 从边关系构建节点输出映射（用于执行期注入依赖输出）
pub fn build_input_mapping(edges: &[FlowEdge]) -> HashMap<String, Vec<(String, String)>> {
    // key = target_node_id, value = [(source_node_id, source_handle), ...]
    let mut mapping: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for edge in edges {
        mapping
            .entry(edge.target.clone())
            .or_default()
            .push((edge.source.clone(), edge.source_handle.clone()));
    }
    mapping
}

// ═══════════════════════════════════════════════════════════════
// 旧版 DAG 类型（向后兼容 dag_run_start 命令）
// ═══════════════════════════════════════════════════════════════

/// DAG 工作流：节点 + 连线（替代线性 Step 列表）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAGWorkflow {
    pub name: String,
    pub description: Option<String>,
    pub nodes: Vec<DAGNode>,
    pub edges: Vec<DAGEdge>,
    pub variables: Option<HashMap<String, serde_json::Value>>,
}

/// DAG 节点（旧版，对应前端的 DAGNode）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAGNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub label: String,
    pub position: DAGPosition,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAGPosition {
    pub x: f64,
    pub y: f64,
}

/// DAG 连线（旧版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAGEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(rename = "sourceHandle")]
    pub source_handle: String,
    #[serde(rename = "targetHandle")]
    pub target_handle: String,
}

impl DAGWorkflow {
    /// 构建执行计划：拓扑排序，检测环（旧版方法，向后兼容）
    pub fn build_execution_plan(&self) -> Result<ExecutionPlan, String> {
        let node_ids: HashSet<&str> = self.nodes.iter().map(|n| n.id.as_str()).collect();

        let mut successors: HashMap<String, Vec<String>> = HashMap::new();
        let mut predecessors: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        for node in &self.nodes {
            successors.entry(node.id.clone()).or_default();
            predecessors.entry(node.id.clone()).or_default();
            in_degree.entry(node.id.clone()).or_insert(0);
        }

        for edge in &self.edges {
            if !node_ids.contains(edge.source.as_str()) {
                return Err(format!("连线引用不存在的源节点: {}", edge.source));
            }
            if !node_ids.contains(edge.target.as_str()) {
                return Err(format!("连线引用不存在的目标节点: {}", edge.target));
            }

            successors
                .entry(edge.source.clone())
                .or_default()
                .push(edge.target.clone());
            predecessors
                .entry(edge.target.clone())
                .or_default()
                .push(edge.source.clone());
            *in_degree.entry(edge.target.clone()).or_insert(0) += 1;
        }

        // Kahn 算法
        let mut queue: VecDeque<String> = VecDeque::new();
        for (id, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(id.clone());
            }
        }

        let mut order: Vec<String> = Vec::new();
        while let Some(node_id) = queue.pop_front() {
            order.push(node_id.clone());
            if let Some(succs) = successors.get(&node_id) {
                for succ in succs {
                    if let Some(deg) = in_degree.get_mut(succ) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(succ.clone());
                        }
                    }
                }
            }
        }

        if order.len() != self.nodes.len() {
            return Err(format!(
                "工作流包含环: {} 个节点未排序 (图中存在循环依赖)",
                self.nodes.len() - order.len()
            ));
        }

        // 转换为 ExecStep
        let node_map: HashMap<&str, &DAGNode> =
            self.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

        let ordered_steps: Vec<ExecStep> = order
            .iter()
            .map(|node_id| {
                let node = node_map.get(node_id.as_str()).unwrap();
                let deps = predecessors.get(node_id).cloned().unwrap_or_default();
                ExecStep {
                    node_id: node_id.clone(),
                    step: Step {
                        id: node.id.clone(),
                        name: node.label.clone(),
                        step_type: node.node_type.clone(),
                        config: node.config.clone(),
                        next: None,
                        retry: None,
                        timeout: None,
                        body_steps: None,
                        breakpoint: false,
                        delay: None,
                        on_error: None,
                    },
                    dependencies: deps,
                }
            })
            .collect();

        let mut by_index: HashMap<String, usize> = HashMap::new();
        for (i, id) in order.iter().enumerate() {
            by_index.insert(id.clone(), i);
        }

        let mut level_map_inner: HashMap<String, usize> = HashMap::new();
        for id in &order {
            let level = predecessors
                .get(id)
                .map(|ps| {
                    ps.iter()
                        .filter_map(|p| level_map_inner.get(p))
                        .max()
                        .copied()
                        .unwrap_or(0)
                        + 1
                })
                .unwrap_or(0);
            level_map_inner.insert(id.clone(), level);
        }

        let mut by_level: HashMap<usize, Vec<usize>> = HashMap::new();
        for (idx, node_id) in order.iter().enumerate() {
            let level = level_map_inner.get(node_id).copied().unwrap_or(0);
            by_level.entry(level).or_default().push(idx);
        }

        let mut parallel_groups: Vec<Vec<usize>> = Vec::new();
        for indices in by_level.values() {
            if indices.len() > 1 {
                parallel_groups.push(indices.clone());
            }
        }

        Ok(ExecutionPlan {
            ordered_steps,
            parallel_groups,
            in_degree,
            successors,
            predecessors,
            order,
        })
    }

    /// 按节点 ID 查找节点
    pub fn find_node(&self, id: &str) -> Option<&DAGNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// 收集某个节点的所有输入数据（从连线来源的 output 中获取）
    pub fn collect_inputs(
        &self,
        node_id: &str,
        node_outputs: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, serde_json::Value> {
        let mut inputs = HashMap::new();

        for edge in &self.edges {
            if edge.target == node_id {
                if let Some(source_output) = node_outputs.get(&edge.source) {
                    inputs.insert(edge.target_handle.clone(), source_output.clone());
                }
            }
        }

        inputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str, label: &str, node_type: &str) -> FlowNode {
        FlowNode {
            id: id.to_string(),
            node_type: node_type.to_string(),
            label: label.to_string(),
            position: FlowPosition { x: 0.0, y: 0.0 },
            config: serde_json::json!({}),
            on_error: None,
            breakpoint: false,
            delay: None,
            timeout: None,
        }
    }

    fn make_edge(id: &str, source: &str, target: &str) -> FlowEdge {
        FlowEdge {
            id: id.to_string(),
            source: source.to_string(),
            target: target.to_string(),
            source_handle: "output".to_string(),
            target_handle: "input".to_string(),
        }
    }

    #[test]
    fn test_linear_dag() {
        let nodes = vec![
            make_node("A", "Node A", "http"),
            make_node("B", "Node B", "script"),
            make_node("C", "Node C", "data"),
        ];
        let edges = vec![
            make_edge("e1", "A", "B"),
            make_edge("e2", "B", "C"),
        ];

        let plan = build_dag(&nodes, &edges).unwrap();
        assert_eq!(plan.ordered_steps.len(), 3);
        assert_eq!(plan.order, vec!["A", "B", "C"]);
        assert_eq!(plan.ordered_steps[1].dependencies, vec!["A"]);
        assert_eq!(plan.ordered_steps[2].dependencies, vec!["B"]);
    }

    #[test]
    fn test_parallel_branches() {
        let nodes = vec![
            make_node("A", "Start", "http"),
            make_node("B", "Left", "script"),
            make_node("C", "Right", "script"),
            make_node("D", "End", "data"),
        ];
        let edges = vec![
            make_edge("e1", "A", "B"),
            make_edge("e2", "A", "C"),
            make_edge("e3", "B", "D"),
            make_edge("e4", "C", "D"),
        ];

        let plan = build_dag(&nodes, &edges).unwrap();
        assert_eq!(plan.ordered_steps.len(), 4);
        // A must be first, D must be last; B and C in middle (order can vary)
        let first = &plan.order[0];
        let last = &plan.order[3];
        assert_eq!(first, "A");
        assert_eq!(last, "D");
        // Should have a parallel group: B and C at same level
        assert!(!plan.parallel_groups.is_empty());
    }

    #[test]
    fn test_cycle_detection() {
        let nodes = vec![
            make_node("A", "Node A", "http"),
            make_node("B", "Node B", "http"),
        ];
        let edges = vec![
            make_edge("e1", "A", "B"),
            make_edge("e2", "B", "A"), // back edge → cycle
        ];

        let result = build_dag(&nodes, &edges);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("环"));
    }

    #[test]
    fn test_empty_nodes() {
        let result = build_dag(&[], &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_flow_node_to_step_mapping() {
        let node = FlowNode {
            id: "n1".into(),
            node_type: "http".into(),
            label: "测试节点".into(),
            position: FlowPosition { x: 100.0, y: 200.0 },
            config: serde_json::json!({"url": "https://example.com"}),
            on_error: Some(ErrorStrategy::Ignore),
            breakpoint: true,
            delay: Some(500),
            timeout: Some(30),
        };

        let step = flow_node_to_step(&node);
        assert_eq!(step.id, "n1");
        assert_eq!(step.name, "测试节点");
        assert_eq!(step.step_type, "http");
        assert_eq!(step.breakpoint, true);
        assert_eq!(step.delay, Some(500));
        assert_eq!(step.timeout, Some(30));
        assert!(matches!(step.on_error, Some(ErrorStrategy::Ignore)));
    }
}
