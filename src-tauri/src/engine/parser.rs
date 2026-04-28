// engine/parser.rs — YAML 工作流解析
use crate::engine::workflow::Workflow;
use anyhow::{Result, anyhow};

pub fn parse_workflow(yaml: &str) -> Result<Workflow> {
    let wf: Workflow = serde_yaml::from_str(yaml)
        .map_err(|e| anyhow!("YAML 解析失败: {}", e))?;

    // 基本校验
    if wf.name.trim().is_empty() {
        return Err(anyhow!("工作流名称不能为空"));
    }
    if wf.steps.is_empty() {
        return Err(anyhow!("工作流至少需要一个步骤"));
    }

    // 检查步骤 ID 唯一性
    let mut ids = std::collections::HashSet::new();
    for step in &wf.steps {
        if !ids.insert(&step.id) {
            return Err(anyhow!("步骤 ID 重复: {}", step.id));
        }
    }

    Ok(wf)
}

/// 根据 {{step_xxx}} / output.step_xxx 引用自动推断步骤依赖顺序
/// 解析每个步骤 config 中的引用，构建依赖图，拓扑排序
pub fn auto_order_steps(steps: &[crate::engine::workflow::Step]) -> Vec<usize> {
    use std::collections::HashMap;

    let n = steps.len();
    if n == 0 {
        return Vec::new();
    }

    let id_to_idx: HashMap<&str, usize> = steps.iter().enumerate()
        .map(|(i, s)| (s.id.as_str(), i)).collect();

    // 构建依赖图：deps[i] = 步骤 i 依赖的步骤索引列表
    let mut deps: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, step) in steps.iter().enumerate() {
        let config_str = serde_json::to_string(&step.config).unwrap_or_default();
        for dep_id in extract_step_refs(&config_str) {
            if let Some(&j) = id_to_idx.get(dep_id.as_str()) {
                if j != i && !deps[i].contains(&j) {
                    deps[i].push(j);
                }
            }
        }
    }

    // Kahn 拓扑排序
    // adj[j] = 依赖 j 的步骤列表（j 完成后才能执行这些步骤）
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut in_deg = vec![0usize; n];
    for (i, dep_list) in deps.iter().enumerate() {
        in_deg[i] = dep_list.len();
        for &j in dep_list {
            adj[j].push(i);
        }
    }

    let mut queue: std::collections::VecDeque<usize> = (0..n)
        .filter(|&i| in_deg[i] == 0).collect();
    let mut order = Vec::with_capacity(n);

    while let Some(j) = queue.pop_front() {
        order.push(j);
        for &i in &adj[j] {
            in_deg[i] -= 1;
            if in_deg[i] == 0 {
                queue.push_back(i);
            }
        }
    }

    // 有环时，把剩余步骤按原顺序追加
    if order.len() < n {
        for i in 0..n {
            if !order.contains(&i) {
                order.push(i);
            }
        }
    }

    order
}

/// 从配置字符串中提取 step 引用
/// 匹配 {{step_xxx}} 和 output.step_xxx
fn extract_step_refs(s: &str) -> Vec<String> {
    let mut refs = Vec::new();

    // 匹配 output.step_xxx → step id 是 xxx
    let prefix1 = "output.step_";
    for (pos, _) in s.match_indices(prefix1) {
        let rest = &s[pos + prefix1.len()..];
        let id: String = rest.chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        if !id.is_empty() {
            refs.push(id);
        }
    }

    // 匹配 {{step_xxx → step id 是 xxx
    let prefix2 = "{{step_";
    for (pos, _) in s.match_indices(prefix2) {
        let rest = &s[pos + prefix2.len()..];
        let id: String = rest.chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        if !id.is_empty() {
            refs.push(id);
        }
    }

    refs.sort();
    refs.dedup();
    refs
}
