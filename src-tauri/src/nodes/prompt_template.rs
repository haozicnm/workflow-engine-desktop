// nodes/prompt_template.rs — Prompt 模板节点
// 变量插值生成 Prompt 文本（为 llm_chat 准备输入）
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Default)]
pub struct PromptTemplateNode;

#[async_trait]
impl NodeExecutor for PromptTemplateNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "prompt_template".into(),
            version: "1.0".into(),
            display_name: "Prompt 模板".into(),
            description: "生成 Prompt 文本（变量插值，支持 {{variable}} 语法）".into(),
            category: "ai".into(),
            inputs: vec![
                crate::nodes::traits::PortDef { label: "variables".into(), data_type: "object".into(), required: false },
            ],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "prompt".into(), data_type: "string".into(), required: false },
                crate::nodes::traits::PortDef { label: "messages".into(), data_type: "array".into(), required: false },
            ],
            config_schema: json!({
                "type": "object",
                "properties": {
                    "template": { "type": "string", "description": "模板文本（支持 {{variable}} 语法）" },
                    "system_prompt": { "type": "string", "description": "系统提示词（可选）" },
                    "role": { "type": "string", "description": "消息角色", "default": "user" }
                },
                "required": ["template"]
            }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        let config = &step.config;
        let template = config.get("template")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("prompt_template: 缺少 template 参数"))?;

        let role = config.get("role").and_then(|v| v.as_str()).unwrap_or("user");

        // 使用 context 的 resolve_config 做变量插值
        let resolved = ctx.resolve_config(&Value::String(template.to_string()));
        let prompt = match resolved {
            Value::String(s) => s,
            other => serde_json::to_string(&other).unwrap_or_default(),
        };

        // 构造消息列表
        let mut messages = Vec::new();
        if let Some(sys) = config.get("system_prompt").and_then(|v| v.as_str()) {
            let resolved_sys = ctx.resolve_config(&Value::String(sys.to_string()));
            let sys_text = match resolved_sys {
                Value::String(s) => s,
                other => serde_json::to_string(&other).unwrap_or_default(),
            };
            messages.push(json!({"role": "system", "content": sys_text}));
        }
        messages.push(json!({"role": role, "content": &prompt}));

        Ok(json!({
            "prompt": prompt,
            "messages": messages,
        }))
    }
}
