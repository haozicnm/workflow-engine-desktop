// nodes/http.rs — HTTP 请求节点
use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use tracing::{info, error};

#[derive(Default)]
pub struct HttpNode;

#[async_trait]
impl NodeExecutor for HttpNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        // 兼容旧格式 method 和新格式 action
        let method = config.get("action")
            .or_else(|| config.get("method"))
            .and_then(|v| v.as_str())
            .unwrap_or("GET");
        let url = config.get("url").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("HTTP 节点缺少 url 参数"))?;

        info!("HTTP 请求: {} {}", method, url);

        let client = reqwest::Client::new();
        let mut req = match method.to_uppercase().as_str() {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "DELETE" => client.delete(url),
            "PATCH" => client.patch(url),
            _ => return Err(anyhow!("不支持的 HTTP 方法: {}", method)),
        };

        // 添加 headers
        if let Some(headers) = config.get("headers").and_then(|v| v.as_object()) {
            for (k, v) in headers {
                if let Some(val) = v.as_str() {
                    req = req.header(k, val);
                }
            }
        }

        // 添加 body (POST/PUT/PATCH)
        if let Some(body) = config.get("body") {
            if matches!(method.to_uppercase().as_str(), "POST" | "PUT" | "PATCH") {
                req = req.json(body);
            }
        }

        let resp = req.send().await.map_err(|e| {
            error!("HTTP 请求失败 ({} {}): {}", method, url, e);
            anyhow!("HTTP 请求失败: {}", e)
        })?;
        let status = resp.status().as_u16();
        let text = resp.text().await.map_err(|e| {
            error!("读取 HTTP 响应失败 ({} {}): {}", method, url, e);
            anyhow!("读取响应失败: {}", e)
        })?;

        info!("HTTP 响应: {} {} → {}", method, url, status);

        // 尝试解析 JSON
        let body = serde_json::from_str::<serde_json::Value>(&text)
            .unwrap_or_else(|_| serde_json::Value::String(text));

        Ok(serde_json::json!({
            "status": status,
            "body": body,
        }))
    }
}
