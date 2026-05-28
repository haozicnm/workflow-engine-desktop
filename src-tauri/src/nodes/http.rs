// nodes/http.rs — HTTP 请求节点
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{error, info};

#[derive(Default)]
pub struct HttpNode;

#[async_trait]
impl NodeExecutor for HttpNode {
    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        // 兼容旧格式 method 和新格式 action
        let method = config
            .get("action")
            .or_else(|| config.get("method"))
            .and_then(|v| v.as_str())
            .unwrap_or("GET");
        let url = config
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("HTTP 节点缺少 url 参数"))?;

        info!("HTTP 请求: {} {}", method, url);

        let connect_timeout = config
            .get("connect_timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(5);
        let read_timeout = config
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| _ctx.default_timeouts.http_request_ms / 1000);

        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(connect_timeout))
            .timeout(std::time::Duration::from_secs(read_timeout))
            .build()
            .map_err(|e| anyhow!("创建 HTTP 客户端失败: {}", e))?;
        let mut req = match method.to_uppercase().as_str() {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "DELETE" => client.delete(url),
            "PATCH" => client.patch(url),
            _ => return Err(anyhow!("不支持的 HTTP 方法: {}", method)),
        };

        // 添加 headers（兼容字符串和对象两种格式）
        if let Some(headers) = config.get("headers") {
            // 1. 对象格式：{"Accept": "application/json"}
            if let Some(obj) = headers.as_object() {
                for (k, v) in obj {
                    if let Some(val) = v.as_str() {
                        req = req.header(k, val);
                    }
                }
            }
            // 2. 字符串格式：解析为 JSON
            else if let Some(s) = headers.as_str() {
                if let Ok(obj) = serde_json::from_str::<serde_json::Value>(s) {
                    if let Some(map) = obj.as_object() {
                        for (k, v) in map {
                            if let Some(val) = v.as_str() {
                                req = req.header(k, val);
                            }
                        }
                    }
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
            .unwrap_or(serde_json::Value::String(text));

        Ok(serde_json::json!({
            "status": status,
            "body": body,
        }))
    }
}
