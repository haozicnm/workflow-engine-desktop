// engine/dag.rs — DAG 工作流数据结构和拓扑排序
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// DAG 工作流：节点 + 连线（替代线性 Step 列表）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAGWorkflow {
    pub name: String,
    pub description: Option<String>,
    pub nodes: Vec<DAGNode>,
    pub edges: Vec<DAGEdge>,
    pub variables: Option<HashMap<String, serde_json::Value>>,
}

/// DAG 节点（对应前端的 DAGNode）
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

/// DAG 连线
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

/// 拓扑排序结果
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    /// 按拓扑排序的节点执行顺序
    pub order: Vec<String>,
    /// 每个节点的入度（依赖计数）
    pub in_degree: HashMap<String, usize>,
    /// 每个节点的后继节点
    pub successors: HashMap<String, Vec<String>>,
    /// 每个节点的前驱节点
    pub predecessors: HashMap<String, Vec<String>>,
}

impl DAGWorkflow {
    /// 构建执行计划：拓扑排序，检测环
    pub fn build_execution_plan(&self) -> Result<ExecutionPlan, String> {
        let node_ids: HashSet<&str> = self.nodes.iter().map(|n| n.id.as_str()).collect();

        // 构建邻接表
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

        // Kahn 算法拓扑排序
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

        // 环检测
        if order.len() != self.nodes.len() {
            return Err(format!(
                "工作流包含环: {} 个节点未排序 (图中存在循环依赖)",
                self.nodes.len() - order.len()
            ));
        }

        Ok(ExecutionPlan {
            order,
            in_degree,
            successors,
            predecessors,
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

        // 找到所有连到当前节点的边
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
