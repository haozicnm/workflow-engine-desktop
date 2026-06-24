// nodes/http.rs — HTTP 请求节点
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::error_utils;
use crate::nodes::traits::NodeExecutor;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{error, info, warn};

#[derive(Default)]
pub struct HttpNode;

#[async_trait]
impl NodeExecutor for HttpNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "http".into(),
            version: "1.0".into(),
            display_name: "HTTP 请求".into(),
            description: "发送 HTTP 请求，支持 GET/POST/PUT/DELETE 等方法".into(),
            category: "网络".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "body".into(), data_type: "object".into(), required: false },
                crate::nodes::traits::PortDef { label: "status".into(), data_type: "number".into(), required: false },
                crate::nodes::traits::PortDef { label: "headers".into(), data_type: "object".into(), required: false },
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "required": ["url"],
                "properties": {
                    "url": {"type": "string", "description": "请求 URL"},
                    "method": {"type": "string", "enum": ["GET", "POST", "PUT", "DELETE", "PATCH"]},
                    "headers": {"type": "object"},
                    "body": {"type": "string"}
                }
            }),
            params: vec![],
        }
    }

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
            .ok_or_else(|| error_utils::missing_parameter("url", "http").to_error())?;

        // 重试配置
        let max_retries = config.get("retry").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let retry_delay_ms = config
            .get("retry_delay_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(1000);

        info!("HTTP 请求: {} {} (retry={})", method, url, max_retries);

        // SSRF 防护：拒绝私有地址和 file:// 协议
        if url.starts_with("file://") || url.starts_with("data:") {
            return Err(anyhow::anyhow!("HTTP 节点禁止 file:// 和 data: 协议"));
        }
        // 简单的 URL 主机解析（不依赖 url crate）
        if let Some(host_start) = url.find("://").and_then(|i| url.get(i + 3..)) {
            let host = host_start.split('/').next().unwrap_or("")
                .split(':').next().unwrap_or("")
                .split('@').next_back().unwrap_or("");
            let is_private = host == "localhost"
                || host == "127.0.0.1"
                || host == "::1"
                || host == "0.0.0.0"
                || host.starts_with("10.")
                || host.starts_with("192.168.")
                || host.starts_with("172.16.") || host.starts_with("172.17.")
                || host.starts_with("172.18.") || host.starts_with("172.19.")
                || host.starts_with("172.2") || host.starts_with("172.3")
                || host == "169.254.169.254"; // 云 metadata
            if is_private {
                return Err(anyhow::anyhow!("HTTP 节点 SSRF 防护：禁止访问私有地址 '{}'", host));
            }
        }

        let connect_timeout = config
            .get("connect_timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(5);
        let read_timeout = config
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(_ctx.default_timeouts.http_request_ms / 1000);

        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(connect_timeout))
            .timeout(std::time::Duration::from_secs(read_timeout))
            .build()
            .map_err(|e| error_utils::execution_failed("创建 HTTP 客户端", &e.to_string()).to_error())?;

        let max_attempts = 1 + max_retries;
        let mut last_err = None;

        for attempt in 1..=max_attempts {
            let mut req = match method.to_uppercase().as_str() {
                "GET" => client.get(url),
                "POST" => client.post(url),
                "PUT" => client.put(url),
                "DELETE" => client.delete(url),
                "PATCH" => client.patch(url),
                _ => return Err(error_utils::invalid_parameter("method", "GET/POST/PUT/DELETE/PATCH", method).to_error()),
            };

            // 添加 headers（兼容字符串和对象两种格式）
            if let Some(headers) = config.get("headers") {
                if let Some(obj) = headers.as_object() {
                    for (k, v) in obj {
                        if let Some(val) = v.as_str() {
                            req = req.header(k, val);
                        }
                    }
                } else if let Some(s) = headers.as_str() {
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

            match req.send().await {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    // 限制响应体大小（默认 100MB，防止 OOM）
                    let max_bytes = 100 * 1024 * 1024usize;
                    let bytes = resp.bytes().await.map_err(|e| {
                        error!("读取 HTTP 响应失败 ({} {}): {}", method, url, e);
                        error_utils::execution_failed("读取 HTTP 响应", &e.to_string()).to_error()
                    })?;
                    if bytes.len() > max_bytes {
                        return Err(anyhow::anyhow!(
                            "HTTP 响应体过大 ({} bytes > {} bytes limit) [{}]",
                            bytes.len(), max_bytes, url
                        ));
                    }
                    let text = String::from_utf8_lossy(&bytes).to_string();

                    info!("HTTP 响应: {} {} → {}", method, url, status);

                    let body = serde_json::from_str::<serde_json::Value>(&text)
                        .unwrap_or(serde_json::Value::String(text));

                    return Ok(serde_json::json!({
                        "status": status,
                        "body": body,
                    }));
                }
                Err(e) => {
                    last_err = Some(e);
                    if attempt < max_attempts {
                        warn!(
                            "HTTP 请求失败 ({} {}) 第 {}/{} 次: {}，{}ms 后重试",
                            method,
                            url,
                            attempt,
                            max_attempts,
                            last_err.as_ref().unwrap(),
                            retry_delay_ms
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay_ms))
                            .await;
                    }
                }
            }
        }

        let e = last_err.unwrap();
        error!(
            "HTTP 请求失败 ({} {}): {} (已重试 {} 次)",
            method, url, e, max_retries
        );
        Err(error_utils::network_error(&format!("{} (已重试 {} 次)", e, max_retries)).to_error())
    }
}
