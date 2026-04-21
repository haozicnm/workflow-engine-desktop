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
