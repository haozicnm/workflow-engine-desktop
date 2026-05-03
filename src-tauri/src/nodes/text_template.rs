// nodes/text_template.rs — 文本拼接节点
//
// 使用 {{variable}} 占位符模板替换上下文变量

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use regex::Regex;

#[derive(Default)]
pub struct TextTemplateNode;

#[async_trait]
impl NodeExecutor for TextTemplateNode {
    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let template = config.get("template").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少 template 参数"))?;
        let output_key = config.get("output_key").and_then(|v| v.as_str());

        let result = resolve_template(template, ctx);

        if let Some(key) = output_key {
            ctx.set_var(key.to_string(), serde_json::Value::String(result.clone()));
        }

        Ok(serde_json::json!({ "template": template, "result": result }))
    }
}

fn resolve_template(template: &str, ctx: &ExecutionContext) -> String {
    let re = Regex::new(r"\{\{(\w+(?:\.\w+)*)\}\}").expect("template regex");
    re.replace_all(template, |caps: &regex::Captures| {
        let path = caps.get(1).expect("capture").as_str();
        resolve_var(path, ctx)
    }).to_string()
}

fn resolve_var(path: &str, ctx: &ExecutionContext) -> String {
    // output.step_id.field 引用
    if let Some(rest) = path.strip_prefix("output.") {
        let parts: Vec<&str> = rest.splitn(2, '.').collect();
        if let Some(output) = ctx.get_output(parts[0]) {
            return if let Some(field) = parts.get(1) {
                let mut val = output;
                for part in field.split('.') {
                    val = match val.get(part) { Some(v) => v, None => return format!("{{{{{}}}}}", path) };
                }
                val_to_str(val)
            } else {
                val_to_str(output)
            };
        }
        return format!("{{{{{}}}}}", path);
    }

    // 上下文变量
    if let Some(val) = ctx.variables.get(path) {
        return val_to_str(val);
    }

    format!("{{{{{}}}}}", path)
}

fn val_to_str(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}
