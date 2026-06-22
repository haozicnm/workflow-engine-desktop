// nodes/llm_chat.rs — LLM 对话节点（薄封装）
// 调用 OpenAI-compatible API（OpenAI/Claude/DeepSeek/Kimi 等）
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::info;

#[derive(Default)]
pub struct LlmChatNode;

#[async_trait]
impl NodeExecutor for LlmChatNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "llm_chat".into(),
            version: "1.0".into(),
            display_name: "LLM 对话".into(),
            description: "调用 LLM API（OpenAI/Claude/DeepSeek/Kimi 等 OpenAI-compatible 格式）".into(),
            category: "ai".into(),
            inputs: vec![
                crate::nodes::traits::PortDef { label: "messages".into(), data_type: "array".into(), required: false },
                crate::nodes::traits::PortDef { label: "prompt".into(), data_type: "string".into(), required: false },
            ],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "content".into(), data_type: "string".into(), required: false },
                crate::nodes::traits::PortDef { label: "usage".into(), data_type: "object".into(), required: false },
                crate::nodes::traits::PortDef { label: "model".into(), data_type: "string".into(), required: false },
            ],
            config_schema: json!({
                "type": "object",
                "properties": {
                    "api_url": { "type": "string", "description": "API 端点（默认 OpenAI）", "default": "https://api.openai.com/v1/chat/completions" },
                    "api_key": { "type": "string", "description": "API Key（或从环境变量 LLM_API_KEY 读取）" },
                    "model": { "type": "string", "description": "模型名称", "default": "gpt-4o" },
                    "system_prompt": { "type": "string", "description": "系统提示词" },
                    "messages": { "type": "array", "description": "消息数组 [{role, content}]" },
                    "prompt": { "type": "string", "description": "简单提示词（与 messages 二选一）" },
                    "temperature": { "type": "number", "description": "温度 (0-2)", "default": 0.7 },
                    "max_tokens": { "type": "number", "description": "最大 token 数", "default": 1024 }
                },
                "required": ["api_key"]
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

        let api_url = config.get("api_url")
            .and_then(|v| v.as_str())
            .unwrap_or("https://api.openai.com/v1/chat/completions");

        let api_key_owned;
        let api_key = match config.get("api_key").and_then(|v| v.as_str()) {
            Some(k) => k,
            None => {
                api_key_owned = std::env::var("LLM_API_KEY")
                    .map_err(|_| anyhow!("llm_chat: 缺少 api_key 参数或 LLM_API_KEY 环境变量"))?;
                &api_key_owned
            }
        };

        let model = config.get("model").and_then(|v| v.as_str()).unwrap_or("gpt-4o");
        let temperature = config.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7);
        let max_tokens = config.get("max_tokens").and_then(|v| v.as_u64()).unwrap_or(1024);

        // 构造消息列表
        let mut messages = Vec::new();

        // system prompt
        if let Some(sys) = config.get("system_prompt").and_then(|v| v.as_str()) {
            messages.push(json!({"role": "system", "content": sys}));
        }

        // 优先从输入端口获取 messages
        if let Some(input_msgs) = ctx.input_ports.get("messages").and_then(|v| v.as_array()) {
            messages.extend(input_msgs.iter().cloned());
        } else if let Some(config_msgs) = config.get("messages").and_then(|v| v.as_array()) {
            messages.extend(config_msgs.iter().cloned());
        } else if let Some(prompt) = ctx.input_ports.get("prompt").and_then(|v| v.as_str()) {
            messages.push(json!({"role": "user", "content": prompt}));
        } else if let Some(prompt) = config.get("prompt").and_then(|v| v.as_str()) {
            messages.push(json!({"role": "user", "content": prompt}));
        }

        if messages.is_empty() {
            return Err(anyhow!("llm_chat: 没有消息可发送（需要 messages 或 prompt）"));
        }

        let body = json!({
            "model": model,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
        });

        info!("LLM 调用: model={}, messages={}", model, messages.len());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| anyhow!("HTTP client 创建失败: {}", e))?;
        let resp = client.post(api_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("LLM API 请求失败: {}", e))?;

        let status = resp.status();
        if !status.is_success() {
            let error_body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("LLM API 返回错误 {}: {}", status, error_body));
        }

        let result: Value = resp.json().await
            .map_err(|e| anyhow!("LLM API 响应解析失败: {}", e))?;

        let content = result.get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        let usage = result.get("usage").cloned().unwrap_or(json!({}));

        Ok(json!({
            "content": content,
            "usage": usage,
            "model": model,
        }))
    }
}
