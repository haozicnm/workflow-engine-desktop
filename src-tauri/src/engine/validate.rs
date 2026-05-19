// engine/validate.rs — 工作流语义校验
//
// 在保存工作流前校验：
//   1. runCondition.ref 引用的步骤是否存在
//   2. 必填字段检查
//   3. 变量格式检查

use crate::engine::workflow::{Workflow, Step};
use std::collections::HashSet;

/// 校验结果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self { valid: true, errors: Vec::new(), warnings: Vec::new() }
    }

    fn error(&mut self, msg: String) {
        self.valid = false;
        self.errors.push(msg);
    }

    fn warn(&mut self, msg: String) {
        self.warnings.push(msg);
    }

    pub fn merge(&mut self, other: ValidationResult) {
        self.valid = self.valid && other.valid;
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

/// 校验整个工作流
pub fn validate_workflow(workflow: &Workflow) -> ValidationResult {
    let mut result = ValidationResult::new();

    // 收集所有步骤 ID
    let step_ids: HashSet<&str> = workflow.steps.iter()
        .map(|s| s.id.as_str())
        .collect();

    if workflow.steps.is_empty() {
        result.warn("Workflow has no steps".into());
    }

    for step in &workflow.steps {
        result.merge(validate_step(step, &step_ids));
    }

    result
}

/// 校验单个步骤
fn validate_step(step: &Step, step_ids: &HashSet<&str>) -> ValidationResult {
    let mut result = ValidationResult::new();
    let ctx = format!("step '{}' ({})", step.name, step.id);

    // 1. 校验 runCondition.ref
    if let Some(ref rc) = step.run_condition {
        if !step_ids.contains(rc.ref_step.as_str()) {
            result.error(format!(
                "{}: runCondition.ref '{}' does not exist in workflow steps",
                ctx, rc.ref_step
            ));
        }
    }

    // 2. 校验必填字段
    result.merge(validate_required_fields(step, &ctx));

    // 3. 校验变量格式
    result.merge(validate_variable_refs(step, &ctx));

    // 4. 递归校验 body_steps（loop/cursor 的子步骤）
    if let Some(ref body) = step.body_steps {
        for body_step in body {
            result.merge(validate_step(body_step, step_ids));
        }
    }

    result
}

/// 校验必填字段
fn validate_required_fields(step: &Step, ctx: &str) -> ValidationResult {
    let mut result = ValidationResult::new();
    let config = &step.config;

    match step.step_type.as_str() {
        "approval" => {
            if config.get("options").is_none() {
                result.error(format!("{}: approval step missing 'options' field", ctx));
            }
        }
        "http" => {
            if config.get("url").is_none() {
                result.error(format!("{}: http step missing 'url' field", ctx));
            }
        }
        "shell" => {
            if config.get("command").is_none() {
                result.error(format!("{}: shell step missing 'command' field", ctx));
            }
        }
        "loop" | "cursor" => {
            if config.get("items").is_none() {
                result.error(format!("{}: loop/cursor step missing 'items' field", ctx));
            }
        }
        "script" => {
            if config.get("script").is_none() {
                result.error(format!("{}: script step missing 'script' field", ctx));
            }
        }
        "notify" => {
            if config.get("title").is_none() {
                result.warn(format!("{}: notify step missing 'title' field", ctx));
            }
        }
        _ => {}
    }

    result
}

/// 校验变量引用格式（step_X.field, params.X 等）
fn validate_variable_refs(step: &Step, _ctx: &str) -> ValidationResult {
    let mut result = ValidationResult::new();

    // 递归检查 config 中包含 {{...}} 的字符串值
    let config_str = serde_json::to_string(&step.config).unwrap_or_default();
    check_placeholders(&config_str, &mut result);

    result
}

/// 检查字符串中的 {{...}} 占位符格式
fn check_placeholders(s: &str, result: &mut ValidationResult) {
    let mut pos = 0;
    while let Some(start) = s[pos..].find("{{") {
        let abs_start = pos + start;
        if let Some(end) = s[abs_start + 2..].find("}}") {
            let inner = &s[abs_start + 2..abs_start + 2 + end];
            let trimmed = inner.trim();

            // 空占位符 {{}}
            if trimmed.is_empty() {
                // 跳过（可能是转义）
            } else if !is_valid_variable_path(trimmed) {
                result.warn(format!(
                    "Suspicious variable reference: {{{{ {} }}}} (contains invalid characters)",
                    trimmed
                ));
            }

            pos = abs_start + 2 + end + 2;
        } else {
            result.warn("Unclosed {{ placeholder found".into());
            break;
        }
    }
}

/// 检查变量路径是否合法：字母数字下划线点
fn is_valid_variable_path(path: &str) -> bool {
    path.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::workflow::{RunCondition};

    fn make_step(id: &str, step_type: &str) -> Step {
        Step {
            id: id.to_string(),
            name: format!("Step {}", id),
            step_type: step_type.to_string(),
            config: serde_json::json!({}),
            ..Default::default()
        }
    }

    #[test]
    fn empty_workflow_warns() {
        let wf = Workflow { name: "test".into(), steps: vec![], ..Default::default() };
        let r = validate_workflow(&wf);
        assert!(r.valid);
        assert!(r.warnings.iter().any(|w| w.contains("no steps")));
    }

    #[test]
    fn missing_runcondition_ref_is_error() {
        let mut step = make_step("step_2", "notify");
        step.run_condition = Some(RunCondition {
            ref_step: "step_nonexistent".into(),
            when: "true".into(),
        });
        let wf = Workflow {
            name: "test".into(),
            steps: vec![make_step("step_1", "shell"), step],
            ..Default::default()
        };
        let r = validate_workflow(&wf);
        assert!(!r.valid);
        assert!(r.errors.iter().any(|e| e.contains("runCondition.ref")));
    }

    #[test]
    fn valid_runcondition_passes() {
        let mut step = make_step("step_2", "notify");
        step.run_condition = Some(RunCondition {
            ref_step: "step_1".into(),
            when: "true".into(),
        });
        let wf = Workflow {
            name: "test".into(),
            steps: vec![make_step("step_1", "logic"), step],
            ..Default::default()
        };
        let r = validate_workflow(&wf);
        assert!(r.valid);
    }

    #[test]
    fn missing_required_fields_detected() {
        let wf = Workflow {
            name: "test".into(),
            steps: vec![
                make_step("step_1", "approval"),
                make_step("step_2", "http"),
                make_step("step_3", "shell"),
                make_step("step_4", "script"),
            ],
            ..Default::default()
        };
        let r = validate_workflow(&wf);
        assert!(!r.valid);
        assert!(r.errors.iter().any(|e| e.contains("options")));
        assert!(r.errors.iter().any(|e| e.contains("url")));
        assert!(r.errors.iter().any(|e| e.contains("command")));
        assert!(r.errors.iter().any(|e| e.contains("script")));
    }

    #[test]
    fn suspicious_variable_warns() {
        let mut step = make_step("step_1", "shell");
        step.config = serde_json::json!({
            "command": "echo {{step_1.output!@#}}"
        });
        let wf = Workflow {
            name: "test".into(),
            steps: vec![step],
            ..Default::default()
        };
        let r = validate_workflow(&wf);
        assert!(r.valid); // only warnings for suspicious vars
        assert!(r.warnings.iter().any(|w| w.contains("Suspicious")));
    }
}
