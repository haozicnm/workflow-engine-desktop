// nodes/llm_agent.rs — 简单 AI Agent 节点（工具调用循环）
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Default)]
pub struct LlmAgentNode;

#[async_trait]
impl NodeExecutor for LlmAgentNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "llm_agent".into(),
            version: "1.0".into(),
            display_name: "AI Agent".into(),
            description: "简单 AI Agent — 循环调用 LLM 直到得到最终答案（工具调用循环）".into(),
            category: "ai".into(),
            inputs: vec![
                crate::nodes::traits::PortDef { label: "prompt".into(), data_type: "text".into(), required: false },
            ],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "answer".into(), data_type: "text".into(), required: false },
                crate::nodes::traits::PortDef { label: "steps".into(), data_type: "json".into(), required: false },
                crate::nodes::traits::PortDef { label: "iterations".into(), data_type: "number".into(), required: false },
            ],
            config_schema: json!({
                "type": "object",
                "properties": {
                    "api_key": { "type": "string", "required": true },
                    "api_url": { "type": "string", "default": "https://api.openai.com/v1/chat/completions" },
                    "model": { "type": "string", "default": "gpt-4o" },
                    "system_prompt": { "type": "string", "default": "You are a helpful assistant." },
                    "prompt": { "type": "string" },
                    "max_iterations": { "type": "number", "default": 5 },
                    "temperature": { "type": "number", "default": 0.7 }
                },
                "required": ["api_key"]
            }),
            params: vec![],
        }
    }

    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<Value> {
        let config = &step.config;
        let api_key_owned;
        let api_key = match config.get("api_key").and_then(|v| v.as_str()) {
            Some(k) => k,
            None => {
                api_key_owned = std::env::var("LLM_API_KEY").map_err(|_| anyhow!("llm_agent: 缺少 api_key"))?;
                &api_key_owned
            }
        };
        let api_url = config.get("api_url").and_then(|v| v.as_str())
            .unwrap_or("https://api.openai.com/v1/chat/completions");
        let model = config.get("model").and_then(|v| v.as_str()).unwrap_or("gpt-4o");
        let system_prompt = config.get("system_prompt").and_then(|v| v.as_str())
            .unwrap_or("You are a helpful assistant.");
        let max_iterations = config.get("max_iterations").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
        let temperature = config.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7);

        let user_prompt = ctx.input_ports.get("prompt").and_then(|v| v.as_str()).map(String::from)
            .or_else(|| config.get("prompt").and_then(|v| v.as_str()).map(String::from))
            .ok_or_else(|| anyhow!("llm_agent: 缺少 prompt"))?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;

        let mut messages = vec![
            json!({"role": "system", "content": system_prompt}),
            json!({"role": "user", "content": user_prompt}),
        ];
        let mut steps_log: Vec<Value> = Vec::new();

        for i in 0..max_iterations {
            let body = json!({
                "model": model,
                "messages": messages,
                "temperature": temperature,
                "max_tokens": 2048,
            });

            let resp = client.post(api_url)
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&body)
                .send().await.map_err(|e| anyhow!("LLM API 请求失败: {}", e))?;

            let status = resp.status();
            if !status.is_success() {
                let err = resp.text().await.unwrap_or_default();
                return Err(anyhow!("LLM API 返回 {}: {}", status, err));
            }

            let result: Value = resp.json().await.map_err(|e| anyhow!("解析失败: {}", e))?;
            let assistant_msg = result["choices"][0]["message"].clone();
            let content = assistant_msg["content"].as_str().unwrap_or("").to_string();

            steps_log.push(json!({"iteration": i + 1, "response": content}));

            // 如果没有 tool_calls，Agent 完成
            if assistant_msg.get("tool_calls").is_none() {
                info!("Agent 完成: {} 轮迭代", i + 1);
                return Ok(json!({
                    "answer": content,
                    "steps": steps_log,
                    "iterations": i + 1,
                }));
            }

            // 有 tool_calls — 在当前系统中我们简化为直接返回最终答案
            messages.push(assistant_msg);
            messages.push(json!({"role": "user", "content": "Please provide a final answer based on your analysis."}));
        }

        warn!("Agent 达到最大迭代次数 {}", max_iterations);
        let last_answer = messages.last()
            .and_then(|m| m["content"].as_str())
            .unwrap_or("")
            .to_string();
        Ok(json!({"answer": last_answer, "steps": steps_log, "iterations": max_iterations}))
    }
}
