// nodes/kimi_browser.rs — Kimi WebBridge 浏览器操作节点
//
// 通过 Kimi WebBridge Chrome 扩展控制真实浏览器（有登录态）
// API: POST http://127.0.0.1:10086/command
// 文档: https://github.com/haozicnm/kimi-webbridge/blob/main/docs/api.md

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tracing::{error, info};

/// Kimi WebBridge 默认端口
const DEFAULT_PORT: u16 = 10086;

/// Kimi WebBridge 默认超时（秒）
const DEFAULT_TIMEOUT: u64 = 30;

#[derive(Default)]
pub struct KimiBrowserNode;

impl KimiBrowserNode {
    /// 构建 Kimi WebBridge API URL
    fn build_url(port: u16) -> String {
        format!("http://127.0.0.1:{}/command", port)
    }

    /// 发送命令到 Kimi WebBridge
    async fn send_command(
        &self,
        command: &str,
        args: Value,
        port: u16,
        timeout: u64,
    ) -> Result<Value> {
        let url = Self::build_url(port);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout))
            .build()
            .map_err(|e| anyhow!("创建 HTTP 客户端失败: {}", e))?;

        let body = serde_json::json!({
            "action": command,
            "args": args,
        });

        info!("Kimi WebBridge: {} {:?}", command, args);

        let resp = client.post(&url).json(&body).send().await.map_err(|e| {
            error!("Kimi WebBridge 请求失败: {}", e);
            anyhow!("Kimi WebBridge 请求失败: {}", e)
        })?;

        let text = resp.text().await.map_err(|e| {
            error!("读取 Kimi WebBridge 响应失败: {}", e);
            anyhow!("读取响应失败: {}", e)
        })?;

        let result: Value = serde_json::from_str(&text).map_err(|e| {
            error!("解析 Kimi WebBridge 响应失败: {}", e);
            anyhow!("解析响应失败: {}", e)
        })?;

        // 检查 ok 字段（不是 success）
        let ok = result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);

        if !ok {
            let error_msg = result
                .get("error")
                .and_then(|v| v.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("未知错误");
            return Err(anyhow!("Kimi WebBridge 错误: {}", error_msg));
        }

        info!("Kimi WebBridge 响应: {:?}", result);
        Ok(result)
    }

    /// 解析 action 参数
    fn parse_action(config: &Value) -> Result<&str> {
        config
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Kimi Browser 节点缺少 action 参数"))
    }

    /// 解析端口参数
    fn parse_port(config: &Value) -> u16 {
        config
            .get("port")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_PORT as u64) as u16
    }

    /// 解析超时参数
    fn parse_timeout(config: &Value) -> u64 {
        config
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_TIMEOUT)
    }
}

#[async_trait]
impl NodeExecutor for KimiBrowserNode {
    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        let config = &step.config;
        let action = Self::parse_action(config)?;
        let port = Self::parse_port(config);
        let timeout = Self::parse_timeout(config);

        // 构建 args
        let args = config.get("args").cloned().unwrap_or(serde_json::json!({}));

        // 执行命令
        let result = self.send_command(action, args, port, timeout).await?;

        // 提取 data 字段作为输出
        let data = result
            .get("data")
            .cloned()
            .unwrap_or(serde_json::json!(null));

        Ok(serde_json::json!({
            "success": true,
            "data": data,
        }))
    }
}
