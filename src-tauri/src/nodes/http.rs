// nodes/http.rs — HTTP 请求节点
use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use anyhow::{Result, anyhow};

pub struct HttpNode;

#[async_trait]
impl NodeExecutor for HttpNode {
    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext) -> Result<serde_json::Value> {
        let config = &step.config;
        let method = config.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
        let url = config.get("url").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("HTTP 节点缺少 url 参数"))?;

        let client = reqwest::Client::new();
        let mut req = match method.to_uppercase().as_str() {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "DELETE" => client.delete(url),
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

        // 添加 body (POST/PUT)
        if let Some(body) = config.get("body") {
            if method == "POST" || method == "PUT" {
                req = req.json(body);
            }
        }

        let resp = req.send().await.map_err(|e| anyhow!("HTTP 请求失败: {}", e))?;
        let status = resp.status().as_u16();
        let text = resp.text().await.map_err(|e| anyhow!("读取响应失败: {}", e))?;

        Ok(serde_json::json!({
            "status": status,
            "body": text,
        }))
    }
}
