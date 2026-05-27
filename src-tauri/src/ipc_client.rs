// ipc_client.rs — wf-cli WebSocket client
//
// 连接到桌面 IPC server，发送请求并流式输出结果。
// 如果 daemon 不可达，回退到本地直接执行（兼容模式）。

use std::collections::HashMap;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const IPC_PORT: u16 = 19527;
const TOKEN_FILE: &str = ".hermes/daemon-token";

/// 读取 IPC token（从 ~/.hermes/daemon-token 或 WF_DAEMON_TOKEN 环境变量）
fn read_token() -> Option<String> {
    if let Ok(t) = std::env::var("WF_DAEMON_TOKEN") {
        if !t.is_empty() {
            return Some(t);
        }
    }
    if let Some(home) = dirs::home_dir() {
        let path = home.join(TOKEN_FILE);
        if let Ok(t) = std::fs::read_to_string(&path) {
            let trimmed = t.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

/// 尝试连接 IPC daemon。返回 client 或回退错误信息。
pub struct IpcClient;

impl IpcClient {
    /// 检查 daemon 是否可用
    pub async fn is_daemon_available() -> bool {
        let url = format!("ws://127.0.0.1:{}", IPC_PORT);
        match tokio::time::timeout(Duration::from_secs(2), connect_async(&url)).await {
            Ok(Ok((_ws, _))) => true,
            _ => false,
        }
    }

    /// 通过 IPC 运行工作流（流式输出到 stdout）
    pub async fn run_remote(
        workflow_id: &str,
        vars: Option<HashMap<String, String>>,
    ) -> Result<(), String> {
        let token = read_token().unwrap_or_default();
        let url = format!("ws://127.0.0.1:{}", IPC_PORT);
        let (ws, _resp) = connect_async(&url)
            .await
            .map_err(|e| format!("Cannot connect to daemon (ws://127.0.0.1:{}): {}\nEnsure the desktop app is running", IPC_PORT, e))?;

        let (mut sender, mut receiver) = ws.split();

        // 发送 run 请求（含 token 认证）
        let request = serde_json::json!({
            "type": "run",
            "id": uuid::Uuid::new_v4().to_string(),
            "token": token,
            "workflow_id": workflow_id,
            "vars": vars.unwrap_or_default(),
        });
        sender
            .send(Message::Text(request.to_string()))
            .await
            .map_err(|e| format!("发送请求失败: {}", e))?;

        // 流式接收响应
        while let Some(msg) = receiver.next().await {
            let msg = msg.map_err(|e| format!("连接断开: {}", e))?;
            let text = match msg {
                Message::Text(t) => t,
                Message::Close(_) => {
                    println!("(连接关闭)");
                    break;
                }
                _ => continue,
            };

            let resp: Value = serde_json::from_str(&text).unwrap_or_default();
            let msg_type = resp.get("type").and_then(|v| v.as_str()).unwrap_or("");

            match msg_type {
                "ack" => {
                    let run_id = resp.get("run_id").and_then(|v| v.as_str()).unwrap_or("");
                    println!("▶ 运行 ID: {}", run_id);
                }
                "step_update" => {
                    let step_name = resp.get("step_name").and_then(|v| v.as_str()).unwrap_or("");
                    let status = resp.get("status").and_then(|v| v.as_str()).unwrap_or("");
                    let icon = match status {
                        "running" => "⏳",
                        "completed" => "✅",
                        "failed" => "❌",
                        "skipped" => "⏭️",
                        _ => "  ",
                    };
                    println!("  {} {} ({})", icon, step_name, status);
                }
                "var_snapshot" => {
                    // CLI 不显示变量快照（可加 --verbose 开启）
                }
                "run_complete" => {
                    let status = resp.get("status").and_then(|v| v.as_str()).unwrap_or("");
                    let elapsed = resp
                        .get("elapsed_secs")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    let error = resp.get("error").and_then(|v| v.as_str());
                    match status {
                        "completed" => println!("\n✓ 完成 ({:.1}s)", elapsed),
                        "failed" => eprintln!(
                            "\n✗ 失败 ({:.1}s): {}",
                            elapsed,
                            error.unwrap_or("未知错误")
                        ),
                        _ => println!("\n状态: {}", status),
                    }
                    if status == "failed" {
                        return Err(error.unwrap_or("执行失败").to_string());
                    }
                    return Ok(());
                }
                "error" => {
                    let message = resp
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("未知错误");
                    eprintln!("错误: {}", message);
                    return Err(message.to_string());
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// 通过 IPC 运行库模板
    pub async fn library_run_remote(
        template: &str,
        params: Option<HashMap<String, String>>,
    ) -> Result<(), String> {
        let token = read_token().unwrap_or_default();
        let url = format!("ws://127.0.0.1:{}", IPC_PORT);
        let (ws, _resp) = connect_async(&url)
            .await
            .map_err(|e| format!("Cannot connect to daemon: {}", e))?;

        let (mut sender, mut receiver) = ws.split();

        let request = serde_json::json!({
            "type": "library_run",
            "id": uuid::Uuid::new_v4().to_string(),
            "token": token,
            "template": template,
            "params": params.unwrap_or_default(),
        });
        sender
            .send(Message::Text(request.to_string()))
            .await
            .map_err(|e| format!("发送请求失败: {}", e))?;

        while let Some(msg) = receiver.next().await {
            let msg = msg.map_err(|e| format!("连接断开: {}", e))?;
            let text = match msg {
                Message::Text(t) => t,
                Message::Close(_) => {
                    println!("(连接关闭)");
                    break;
                }
                _ => continue,
            };

            let resp: Value = serde_json::from_str(&text).unwrap_or_default();
            let msg_type = resp.get("type").and_then(|v| v.as_str()).unwrap_or("");

            match msg_type {
                "step_update" => {
                    let step_name = resp.get("step_name").and_then(|v| v.as_str()).unwrap_or("");
                    let status = resp.get("status").and_then(|v| v.as_str()).unwrap_or("");
                    let icon = match status {
                        "running" => "⏳",
                        "completed" => "✅",
                        "failed" => "❌",
                        "skipped" => "⏭️",
                        _ => "  ",
                    };
                    println!("  {} {} ({})", icon, step_name, status);
                }
                "run_complete" => {
                    let status = resp.get("status").and_then(|v| v.as_str()).unwrap_or("");
                    let elapsed = resp
                        .get("elapsed_secs")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    let error = resp.get("error").and_then(|v| v.as_str());
                    if status == "completed" {
                        println!("\n✓ 完成 ({:.1}s)", elapsed);
                    } else {
                        eprintln!(
                            "\n✗ 失败 ({:.1}s): {}",
                            elapsed,
                            error.unwrap_or("未知错误")
                        );
                        return Err(error.unwrap_or("执行失败").to_string());
                    }
                    return Ok(());
                }
                "error" => {
                    let message = resp
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("未知错误");
                    eprintln!("错误: {}", message);
                    return Err(message.to_string());
                }
                _ => {}
            }
        }
        Ok(())
    }
}
