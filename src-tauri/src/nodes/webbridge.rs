// nodes/webbridge.rs — Workflow WebBridge 客户端
//
// 通过 WebSocket 与浏览器扩展通信，替代 Playwright sidecar。
// 扩展连接到 ws://localhost:19527/ws/browser，workflow-engine 是服务器。

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use tracing::{info, warn};

// ═══════════════════════════════════════════════
// 类型定义
// ═══════════════════════════════════════════════

/// 发送给扩展的命令
#[derive(Debug, Serialize)]
pub struct BridgeCommand {
    pub id: String,
    pub action: String,
    pub params: Value,
}

/// 扩展返回的响应
#[derive(Debug, Deserialize)]
pub struct BridgeResponse {
    pub id: String,
    pub success: bool,
    pub data: Option<Value>,
    pub error: Option<String>,
}

/// 扩展注册消息
#[derive(Debug, Deserialize)]
pub struct BridgeRegister {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub client: String,
    pub version: String,
    pub capabilities: Vec<String>,
}

// ═══════════════════════════════════════════════
// 全局状态
// ═══════════════════════════════════════════════

/// 待响应的命令
struct PendingCommand {
    sender: oneshot::Sender<Result<Value>>,
}

/// WebBridge 连接状态
pub struct WebBridgeState {
    /// 已连接的扩展 WebSocket
    connected: RwLock<bool>,
    /// 扩展信息
    info: RwLock<Option<BridgeRegister>>,
    /// 发送通道（给扩展发消息）
    tx: RwLock<Option<mpsc::UnboundedSender<String>>>,
    /// 待响应的命令
    pending: Mutex<HashMap<String, PendingCommand>>,
    /// 命令 ID 计数器
    counter: Mutex<u64>,
}

impl WebBridgeState {
    pub fn new() -> Self {
        Self {
            connected: RwLock::new(false),
            info: RwLock::new(None),
            tx: RwLock::new(None),
            pending: Mutex::new(HashMap::new()),
            counter: Mutex::new(0),
        }
    }

    /// 是否已连接
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// 获取扩展信息
    pub async fn get_info(&self) -> Option<BridgeRegister> {
        self.info.read().await.clone()
    }

    /// 生成命令 ID
    async fn next_id(&self) -> String {
        let mut counter = self.counter.lock().await;
        *counter += 1;
        format!("cmd-{}", *counter)
    }

    /// 发送命令并等待响应
    pub async fn send_command(&self, action: &str, params: Value) -> Result<Value> {
        let tx = self.tx.read().await;
        let tx = tx.as_ref().ok_or_else(|| anyhow!("WebBridge 未连接"))?;

        let id = self.next_id().await;
        let cmd = BridgeCommand {
            id: id.clone(),
            action: action.to_string(),
            params,
        };

        let (resp_tx, resp_rx) = oneshot::channel();
        self.pending.lock().await.insert(id.clone(), PendingCommand { sender: resp_tx });

        let msg = serde_json::to_string(&cmd)?;
        tx.send(msg).map_err(|_| anyhow!("发送失败"))?;

        // 等待响应（30 秒超时）
        match tokio::time::timeout(std::time::Duration::from_secs(30), resp_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => {
                self.pending.lock().await.remove(&id);
                Err(anyhow!("命令被取消"))
            }
            Err(_) => {
                self.pending.lock().await.remove(&id);
                Err(anyhow!("命令超时 (action: {})", action))
            }
        }
    }

    /// 处理扩展发来的消息
    pub async fn handle_message(&self, msg: &str) {
        // 尝试解析为响应
        if let Ok(resp) = serde_json::from_str::<BridgeResponse>(msg) {
            let mut pending = self.pending.lock().await;
            if let Some(cmd) = pending.remove(&resp.id) {
                let result = if resp.success {
                    Ok(resp.data.unwrap_or(Value::Null))
                } else {
                    Err(anyhow!(resp.error.unwrap_or_else(|| "未知错误".to_string())))
                };
                let _ = cmd.sender.send(result);
                return;
            }
        }

        // 尝试解析为注册消息
        if let Ok(reg) = serde_json::from_str::<BridgeRegister>(msg) {
            if reg.msg_type == "register" {
                info!("WebBridge 扩展已连接: {} v{}", reg.client, reg.version);
                *self.info.write().await = Some(reg);
                return;
            }
        }

        warn!("未知消息: {}", msg);
    }

    /// 设置发送通道（扩展连接时调用）
    pub async fn set_connected(&self, tx: mpsc::UnboundedSender<String>) {
        *self.tx.write().await = Some(tx);
        *self.connected.write().await = true;
    }

    /// 断开连接
    pub async fn set_disconnected(&self) {
        *self.tx.write().await = None;
        *self.connected.write().await = false;
        *self.info.write().await = None;

        // 取消所有待响应的命令
        let mut pending = self.pending.lock().await;
        for (_, cmd) in pending.drain() {
            let _ = cmd.sender.send(Err(anyhow!("连接断开")));
        }
    }
}

// ═══════════════════════════════════════════════
// 全局实例
// ═══════════════════════════════════════════════

use std::sync::OnceLock;

static WEBBRIDGE: OnceLock<Arc<WebBridgeState>> = OnceLock::new();

/// 获取全局 WebBridge 状态
pub fn get_state() -> Arc<WebBridgeState> {
    WEBBRIDGE.get_or_init(|| Arc::new(WebBridgeState::new())).clone()
}

// ═══════════════════════════════════════════════
// 公共 API（供 browser_container 调用）
// ═══════════════════════════════════════════════

/// 发送命令到 WebBridge 扩展
pub async fn send_command(action: &str, params: Value) -> Result<Value> {
    WEBBRIDGE.send_command(action, params).await
}

/// 检查 WebBridge 是否可用
pub async fn is_available() -> bool {
    WEBBRIDGE.is_connected().await
}

/// 获取连接信息
pub async fn get_info() -> Option<BridgeRegister> {
    WEBBRIDGE.get_info().await
}
