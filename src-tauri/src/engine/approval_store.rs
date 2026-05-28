// engine/approval_store.rs — 全局审批存储（内存 channel + 定时超时检测）
//
// 核心机制：
//   1. ApprovalNode 调用 register() → 注册审批请求 + 返回 receiver
//   2. ApprovalNode await receiver → 暂停等待决策（tokio::select! 同时处理超时）
//   3. 用户通过 approval_response → 调用 decide() → 通过 sender 发送决策
//   4. ApprovalNode 收到决策 → 返回结果 → scheduler 继续

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalEntry {
    pub id: String,
    pub run_id: String,
    pub step_id: String,
    pub title: String,
    pub message: String,
    pub item: Option<serde_json::Value>,
    pub options: Vec<String>,
    pub recommended: String,
    pub timeout_secs: u64,
    pub timeout_action: String,
    pub created_at: String,
    /// 推荐选项（由上游条件节点通过变量引用提供，如 {{step_check.branch}}）
    #[serde(default)]
    pub recommendation_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ApprovalDecision {
    pub option: String,
    pub comment: Option<String>,
}

pub struct ApprovalStore {
    /// 等待中的审批请求（供前端查询）
    entries: RwLock<HashMap<String, ApprovalEntry>>,
    /// 决策 channel（sender 端，decide() 时发送）
    senders: RwLock<HashMap<String, mpsc::Sender<ApprovalDecision>>>,
}

impl ApprovalStore {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            senders: RwLock::new(HashMap::new()),
        }
    }

    /// 注册新的审批请求，返回 receiver（调用方 await 等待决策）
    pub async fn register(&self, entry: ApprovalEntry) -> mpsc::Receiver<ApprovalDecision> {
        let (tx, rx) = mpsc::channel::<ApprovalDecision>(1);
        let id = entry.id.clone();
        info!("[ApprovalStore] 注册审批请求: {}", id);
        self.entries.write().await.insert(id.clone(), entry);
        self.senders.write().await.insert(id, tx);
        rx
    }

    /// 提交决策（由 approval_response 命令调用）
    pub async fn decide(&self, id: &str, decision: ApprovalDecision) -> Result<(), String> {
        let tx = self
            .senders
            .write()
            .await
            .remove(id)
            .ok_or_else(|| format!("审批请求不存在或已处理: {}", id))?;
        self.entries.write().await.remove(id);
        info!("[ApprovalStore] 审批决策: {} → {}", id, decision.option);
        tx.send(decision)
            .await
            .map_err(|_| "发送决策失败（接收端已关闭）".to_string())
    }

    /// 获取所有待审批（供前端列表展示）
    pub async fn pending(&self) -> Vec<ApprovalEntry> {
        self.entries.read().await.values().cloned().collect()
    }

    /// 检查审批是否存在
    pub async fn exists(&self, id: &str) -> bool {
        self.entries.read().await.contains_key(id)
    }
}

impl Default for ApprovalStore {
    fn default() -> Self {
        Self::new()
    }
}
