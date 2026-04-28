// nodes/approval.rs — 审批节点
// 等待用户通过/拒绝，通过 Tauri 事件与前端通信
use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use std::collections::HashMap;
use anyhow::{Result, anyhow};
use tracing::info;
use tokio::sync::oneshot;

/// 全局审批管理器
pub struct ApprovalManager {
    pending: tokio::sync::Mutex<HashMap<String, oneshot::Sender<bool>>>,
}

impl Default for ApprovalManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ApprovalManager {
    pub fn new() -> Self {
        ApprovalManager {
            pending: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    /// 创建一个等待审批的 channel，返回 receiver
    pub async fn create_pending(&self, approval_id: String) -> oneshot::Receiver<bool> {
        let (tx, rx) = oneshot::channel();
        let mut pending = self.pending.lock().await;
        pending.insert(approval_id, tx);
        rx
    }

    /// 前端回复审批结果
    pub async fn respond(&self, approval_id: &str, approved: bool) -> Result<()> {
        let mut pending = self.pending.lock().await;
        let tx = pending.remove(approval_id)
            .ok_or_else(|| anyhow!("审批 ID 不存在或已处理: {}", approval_id))?;
        tx.send(approved).map_err(|_| anyhow!("审批 channel 已关闭（可能已超时）"))?;
        Ok(())
    }

    /// 获取待审批 ID 列表
    pub async fn list_pending(&self) -> Vec<String> {
        let pending = self.pending.lock().await;
        pending.keys().cloned().collect()
    }
}

// 全局单例（使用 tokio OnceCell）
pub static APPROVAL_MANAGER: tokio::sync::OnceCell<ApprovalManager> = tokio::sync::OnceCell::const_new();

/// 获取全局审批管理器
pub async fn get_approval_manager() -> &'static ApprovalManager {
    APPROVAL_MANAGER.get_or_init(|| async { ApprovalManager::new() }).await
}

// ─── 审批节点实现 ───

#[derive(Default)]
pub struct ApprovalNode;

#[async_trait]
impl NodeExecutor for ApprovalNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;

        // 使用 step.id 作为 approval_id（scheduler 也用此 ID emit 事件）
        let approval_id = format!("approval:{}", step.id);

        let message = config.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("请审批此操作");

        let timeout_secs = config.get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(300);

        let timeout_action = config.get("timeout_action")
            .and_then(|v| v.as_str())
            .unwrap_or("fail");

        let _options = config.get("options")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(["approve", "reject"]));

        // 创建等待 channel
        let manager = get_approval_manager().await;
        let rx = manager.create_pending(approval_id.clone()).await;

        info!("审批节点 '{}' 等待用户响应（超时: {}s, 超时策略: {}）", step.name, timeout_secs, timeout_action);

        // 等待用户响应（带超时）
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            rx,
        ).await;

        match result {
            Ok(Ok(approved)) => {
                info!("审批节点 '{}' 收到响应: approved={}", step.name, approved);
                Ok(serde_json::json!({
                    "approval_id": approval_id,
                    "approved": approved,
                    "message": message,
                }))
            }
            Ok(Err(_)) => {
                Err(anyhow!("审批 channel 异常关闭"))
            }
            Err(_) => {
                // 超时处理
                let approval_id_clone = approval_id.clone();
                let _ = crate::nodes::approval::APPROVAL_MANAGER.get().map(|mgr| {
                    tokio::spawn(async move {
                        let mut pending = mgr.pending.lock().await;
                        pending.remove(&approval_id_clone);
                    })
                });

                match timeout_action {
                    "approve" => {
                        info!("审批节点 '{}' 超时自动通过", step.name);
                        Ok(serde_json::json!({
                            "approval_id": approval_id,
                            "approved": true,
                            "message": message,
                            "timed_out": true,
                            "timeout_action": "approve",
                        }))
                    }
                    "reject" => {
                        info!("审批节点 '{}' 超时自动拒绝", step.name);
                        Ok(serde_json::json!({
                            "approval_id": approval_id,
                            "approved": false,
                            "message": message,
                            "timed_out": true,
                            "timeout_action": "reject",
                        }))
                    }
                    _ => {
                        Err(anyhow!("审批超时（{} 秒）", timeout_secs))
                    }
                }
            }
        }
    }
}
