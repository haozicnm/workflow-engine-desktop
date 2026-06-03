// engine/yaml_format.rs — 工作流 YAML 标准化格式导出/导入
//
// 设计目标：
//   1. Agent 可生成：结构清晰，字段语义明确
//   2. 人类可读：有注释、有缩进、有顺序
//   3. 可版本化：version 字段 + 兼容性检查
//
// 格式示例：
//   version: "1.0"
//   name: "数据抓取工作流"
//   description: "每天抓取 XX 数据"
//   meta:
//     author: agent
//     tags: [data, scrape]
//     created_at: "2026-06-03"
//   variables:
//     url: "https://example.com"
//   steps:
//     - id: fetch
//       type: http
//       name: "获取数据"
//       config:
//         method: GET
//         url: "{{url}}"
//     - id: save
//       type: shell
//       name: "保存"
//       config:
//         command: "echo done"

use crate::engine::workflow::{FORMAT_VERSION, Step, Workflow, WorkflowMeta};
use anyhow::{anyhow, Result};

/// 将 Workflow 导出为标准 YAML 格式
pub fn export_workflow_yaml(wf: &Workflow) -> Result<String> {
    let mut doc = serde_yaml::Mapping::new();

    // version
    doc.insert(
        serde_yaml::Value::String("version".to_string()),
        serde_yaml::Value::String(wf.version.clone().unwrap_or_else(|| FORMAT_VERSION.to_string())),
    );

    // name
    doc.insert(
        serde_yaml::Value::String("name".to_string()),
        serde_yaml::Value::String(wf.name.clone()),
    );

    // description
    if let Some(ref desc) = wf.description {
        if !desc.is_empty() {
            doc.insert(
                serde_yaml::Value::String("description".to_string()),
                serde_yaml::Value::String(desc.clone()),
            );
        }
    }

    // meta
    if let Some(ref meta) = wf.meta {
        let meta_yaml = export_meta(meta);
        if meta_yaml.as_mapping().is_some_and(|m| !m.is_empty()) {
            doc.insert(
                serde_yaml::Value::String("meta".to_string()),
                meta_yaml,
            );
        }
    }

    // variables
    if let Some(ref vars) = wf.variables {
        if !vars.is_empty() {
            let vars_yaml = serde_yaml::to_value(vars)?;
            doc.insert(
                serde_yaml::Value::String("variables".to_string()),
                vars_yaml,
            );
        }
    }

    // steps
    let steps_yaml: Vec<serde_yaml::Value> = wf.steps.iter().map(export_step).collect();
    doc.insert(
        serde_yaml::Value::String("steps".to_string()),
        serde_yaml::Value::Sequence(steps_yaml),
    );

    let value = serde_yaml::Value::Mapping(doc);
    let mut output = serde_yaml::to_string(&value)?;

    // 添加文件头注释
    let header = format!(
        "# Workflow Engine YAML Format v{}\n\
         # 此文件由 Workflow Engine 自动生成\n\
         # 文档: https://github.com/haozicnm/workflow-engine-desktop\n",
        FORMAT_VERSION
    );
    output = format!("{}{}", header, output);

    Ok(output)
}

/// 导出单个步骤为 YAML mapping
fn export_step(step: &Step) -> serde_yaml::Value {
    let mut map = serde_yaml::Mapping::new();

    // 必需字段
    map.insert(
        serde_yaml::Value::String("id".to_string()),
        serde_yaml::Value::String(step.id.clone()),
    );
    map.insert(
        serde_yaml::Value::String("type".to_string()),
        serde_yaml::Value::String(step.step_type.clone()),
    );

    // name（如果有）
    if !step.name.is_empty() {
        map.insert(
            serde_yaml::Value::String("name".to_string()),
            serde_yaml::Value::String(step.name.clone()),
        );
    }

    // config（如果有内容）
    if let Some(obj) = step.config.as_object() {
        if !obj.is_empty() {
            if let Ok(config_yaml) = serde_yaml::to_value(&step.config) {
                map.insert(
                    serde_yaml::Value::String("config".to_string()),
                    config_yaml,
                );
            }
        }
    }

    // next
    if let Some(ref next) = step.next {
        map.insert(
            serde_yaml::Value::String("next".to_string()),
            serde_yaml::Value::String(next.clone()),
        );
    }

    // timeout
    if let Some(timeout) = step.timeout {
        map.insert(
            serde_yaml::Value::String("timeout".to_string()),
            serde_yaml::Value::Number(timeout.into()),
        );
    }

    // delay
    if let Some(delay) = step.delay {
        map.insert(
            serde_yaml::Value::String("delay".to_string()),
            serde_yaml::Value::Number(delay.into()),
        );
    }

    // on_error
    if let Some(ref on_error) = step.on_error {
        let err_yaml = match on_error {
            crate::engine::workflow::ErrorStrategy::Fail => {
                serde_yaml::Value::String("fail".to_string())
            }
            crate::engine::workflow::ErrorStrategy::Ignore => {
                serde_yaml::Value::String("ignore".to_string())
            }
            crate::engine::workflow::ErrorStrategy::Branch { step_id } => {
                let mut m = serde_yaml::Mapping::new();
                m.insert(
                    serde_yaml::Value::String("branch".to_string()),
                    serde_yaml::Value::String(step_id.clone()),
                );
                serde_yaml::Value::Mapping(m)
            }
        };
        map.insert(
            serde_yaml::Value::String("on_error".to_string()),
            err_yaml,
        );
    }

    // retry
    if let Some(ref retry) = step.retry {
        let mut m = serde_yaml::Mapping::new();
        m.insert(
            serde_yaml::Value::String("max".to_string()),
            serde_yaml::Value::Number(retry.max.into()),
        );
        m.insert(
            serde_yaml::Value::String("delay_ms".to_string()),
            serde_yaml::Value::Number(retry.delay_ms.into()),
        );
        map.insert(
            serde_yaml::Value::String("retry".to_string()),
            serde_yaml::Value::Mapping(m),
        );
    }

    // run_condition
    if let Some(ref rc) = step.run_condition {
        let mut m = serde_yaml::Mapping::new();
        m.insert(
            serde_yaml::Value::String("ref".to_string()),
            serde_yaml::Value::String(rc.ref_step.clone()),
        );
        m.insert(
            serde_yaml::Value::String("when".to_string()),
            serde_yaml::Value::String(rc.when.clone()),
        );
        map.insert(
            serde_yaml::Value::String("run_condition".to_string()),
            serde_yaml::Value::Mapping(m),
        );
    }

    // breakpoint
    if step.breakpoint {
        map.insert(
            serde_yaml::Value::String("breakpoint".to_string()),
            serde_yaml::Value::Bool(true),
        );
    }

    // body_steps（容器/迭代节点）
    if let Some(ref body) = step.body_steps {
        if !body.is_empty() {
            let body_yaml: Vec<serde_yaml::Value> = body.iter().map(export_step).collect();
            map.insert(
                serde_yaml::Value::String("body_steps".to_string()),
                serde_yaml::Value::Sequence(body_yaml),
            );
        }
    }

    // actions（容器节点）
    if let Some(ref actions) = step.actions {
        if !actions.is_empty() {
            if let Ok(actions_yaml) = serde_yaml::to_value(actions) {
                map.insert(
                    serde_yaml::Value::String("actions".to_string()),
                    actions_yaml,
                );
            }
        }
    }

    serde_yaml::Value::Mapping(map)
}

/// 导出 meta 为 YAML
fn export_meta(meta: &WorkflowMeta) -> serde_yaml::Value {
    let mut map = serde_yaml::Mapping::new();

    if let Some(ref author) = meta.author {
        map.insert(
            serde_yaml::Value::String("author".to_string()),
            serde_yaml::Value::String(author.clone()),
        );
    }
    if !meta.tags.is_empty() {
        let tags: Vec<serde_yaml::Value> = meta
            .tags
            .iter()
            .map(|t| serde_yaml::Value::String(t.clone()))
            .collect();
        map.insert(
            serde_yaml::Value::String("tags".to_string()),
            serde_yaml::Value::Sequence(tags),
        );
    }
    if let Some(ref created) = meta.created_at {
        map.insert(
            serde_yaml::Value::String("created_at".to_string()),
            serde_yaml::Value::String(created.clone()),
        );
    }
    if let Some(ref updated) = meta.updated_at {
        map.insert(
            serde_yaml::Value::String("updated_at".to_string()),
            serde_yaml::Value::String(updated.clone()),
        );
    }

    serde_yaml::Value::Mapping(map)
}

/// 版本兼容性检查
///
/// 返回 Ok(()) 如果版本兼容，Err 如果不兼容
pub fn check_version_compatibility(wf: &Workflow) -> Result<()> {
    match &wf.version {
        None => Ok(()), // 无版本号 = 旧格式，兼容
        Some(v) => {
            let major: u32 = v
                .split('.')
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let current_major: u32 = FORMAT_VERSION
                .split('.')
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);

            if major > current_major {
                Err(anyhow!(
                    "工作流版本 {} 高于当前支持的版本 {}，请升级 Workflow Engine",
                    v,
                    FORMAT_VERSION
                ))
            } else {
                Ok(()) // 向后兼容
            }
        }
    }
}

/// 为新创建的工作流填充默认 metadata
pub fn ensure_defaults(wf: &mut Workflow) {
    if wf.version.is_none() {
        wf.version = Some(FORMAT_VERSION.to_string());
    }
    if wf.meta.is_none() {
        wf.meta = Some(WorkflowMeta {
            author: Some("user".to_string()),
            ..Default::default()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::workflow::{RetryConfig, RunCondition, Step, Workflow, WorkflowMeta};
    use std::collections::HashMap;

    fn make_test_workflow() -> Workflow {
        let mut vars = HashMap::new();
        vars.insert(
            "url".to_string(),
            serde_json::json!("https://example.com"),
        );

        Workflow {
            version: Some("1.0".to_string()),
            name: "测试工作流".to_string(),
            description: Some("一个测试工作流".to_string()),
            meta: Some(WorkflowMeta {
                author: Some("agent".to_string()),
                tags: vec!["test".to_string(), "demo".to_string()],
                created_at: Some("2026-06-03".to_string()),
                updated_at: None,
            }),
            steps: vec![
                Step {
                    id: "fetch".to_string(),
                    name: "获取数据".to_string(),
                    step_type: "http".to_string(),
                    config: serde_json::json!({
                        "method": "GET",
                        "url": "{{url}}"
                    }),
                    ..Default::default()
                },
                Step {
                    id: "save".to_string(),
                    name: "保存结果".to_string(),
                    step_type: "shell".to_string(),
                    config: serde_json::json!({
                        "command": "echo done"
                    }),
                    timeout: Some(60),
                    on_error: Some(crate::engine::workflow::ErrorStrategy::Ignore),
                    retry: Some(RetryConfig {
                        max: 3,
                        delay_ms: 1000,
                    }),
                    run_condition: Some(RunCondition {
                        ref_step: "fetch".to_string(),
                        when: "true".to_string(),
                    }),
                    ..Default::default()
                },
            ],
            variables: Some(vars),
        }
    }

    #[test]
    fn export_produces_valid_yaml() {
        let wf = make_test_workflow();
        let yaml = export_workflow_yaml(&wf).unwrap();
        assert!(yaml.contains("version: '1.0'"));
        assert!(yaml.contains("name: 测试工作流"));
        assert!(yaml.contains("type: http"));
        assert!(yaml.contains("type: shell"));
        assert!(yaml.contains("ref: fetch"));
        assert!(yaml.contains("tags:"));
        // 可以被重新解析
        let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        assert!(parsed.get("steps").is_some());
    }

    #[test]
    fn version_compatibility_passes() {
        let wf = make_test_workflow();
        assert!(check_version_compatibility(&wf).is_ok());
    }

    #[test]
    fn version_compatibility_fails_for_future() {
        let mut wf = make_test_workflow();
        wf.version = Some("99.0".to_string());
        assert!(check_version_compatibility(&wf).is_err());
    }

    #[test]
    fn no_version_is_compatible() {
        let mut wf = make_test_workflow();
        wf.version = None;
        assert!(check_version_compatibility(&wf).is_ok());
    }

    #[test]
    fn ensure_defaults_fills_version() {
        let mut wf = Workflow {
            name: "test".to_string(),
            steps: vec![],
            ..Default::default()
        };
        ensure_defaults(&mut wf);
        assert_eq!(wf.version, Some("1.0".to_string()));
        assert!(wf.meta.is_some());
    }

    #[test]
    fn roundtrip_yaml_parse() {
        let wf = make_test_workflow();
        let yaml = export_workflow_yaml(&wf).unwrap();
        // 重新解析应该能工作（parser 支持 YAML）
        let parsed = crate::engine::parser::parse_workflow(&yaml);
        assert!(parsed.is_ok(), "roundtrip parse failed: {:?}", parsed.err());
        let wf2 = parsed.unwrap();
        assert_eq!(wf2.name, "测试工作流");
        assert_eq!(wf2.steps.len(), 2);
    }
}
