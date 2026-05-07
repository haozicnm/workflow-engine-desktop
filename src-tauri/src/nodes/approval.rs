// nodes/approval.rs — 审批节点（文件持久化版）
// 第1次 run_start：保存审批请求到文件 → 返回 awaiting_approval → 暂停
// 用户决策后：approval_response 命令写决策到文件
// 第2次 run_start：读决策 → 返回结果 → 继续后续步骤
//
// 输出（待审批）:
//   { status: "awaiting_approval", title: "...", message: "..." }
// 输出（已决策）:
//   { decision: "approved"/"rejected", comment: "...", item: {...} }

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::path::PathBuf;
use tracing::info;

/// 审批状态文件结构
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ApprovalState {
    pub run_id: String,
    pub step_id: String,
    pub title: String,
    pub message: String,
    pub status: String,         // "pending" | "decided"
    pub decision: Option<String>, // "approved" | "rejected"
    pub comment: Option<String>,
    pub created_at: String,
    pub decided_at: Option<String>,
    pub timeout_secs: u64,
    pub timeout_action: String,  // "reject" | "approve" | "fail"
    pub item: Option<Value>,     // 上游传入的上下文数据
}

/// 获取审批文件路径
pub fn approval_path(step_id: &str) -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".workflow-engine").join("approvals").join(format!("{}.json", step_id))
}

/// 读审批状态
pub fn read_approval(step_id: &str) -> Option<ApprovalState> {
    let path = approval_path(step_id);
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(state) = serde_json::from_str::<ApprovalState>(&content) {
                return Some(state);
            }
        }
    }
    None
}

/// 写审批状态
pub fn save_approval(step_id: &str, state: &ApprovalState) {
    let path = approval_path(step_id);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(&path, json);
    }
}

/// 记录决策（由 approval_response 命令调用）
pub fn record_decision(step_id: &str, approved: bool, comment: Option<String>) -> Result<()> {
    let mut state = read_approval(step_id)
        .ok_or_else(|| anyhow!("审批请求不存在: {}", step_id))?;
    if state.status != "pending" {
        return Err(anyhow!("审批已处理"));
    }
    state.status = "decided".to_string();
    state.decision = Some(if approved { "approved".to_string() } else { "rejected".to_string() });
    state.comment = comment;
    state.decided_at = Some(chrono::Utc::now().to_rfc3339());
    save_approval(step_id, &state);
    info!("审批决策已记录: step={} decision={}", step_id, state.decision.as_deref().unwrap_or("?"));
    Ok(())
}

#[derive(Default)]
pub struct ApprovalNode;

#[async_trait]
impl NodeExecutor for ApprovalNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        let config = &step.config;

        let title = config.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("人工审批")
            .to_string();

        let message_template = config.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("请审批此操作")
            .to_string();

        // 渲染消息模板（解析 {{变量}}）
        let message = {
            let tmp_val = json!(message_template);
            let config_val = json!({"_msg": tmp_val});
            let resolved = ctx.resolve_config(&config_val);
            resolved.get("_msg")
                .and_then(|v| v.as_str())
                .unwrap_or(&message_template)
                .to_string()
        };

        let timeout_secs = config.get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(300);

        let timeout_action = config.get("timeout_action")
            .and_then(|v| v.as_str())
            .unwrap_or("reject")
            .to_string();

        // 收集当前上下文中有意义的数据作为 item
        let item = {
            let mut item_map = serde_json::Map::new();
            // 从 step_outputs 收集
            for (key, val) in &ctx.step_outputs {
                item_map.insert(key.clone(), val.clone());
            }
            // 从 variables 收集
            for (key, val) in &ctx.variables {
                if !item_map.contains_key(key) {
                    item_map.insert(key.clone(), val.clone());
                }
            }
            json!(item_map)
        };

        // 检查是否已有审批文件（续跑模式）
        if let Some(existing) = read_approval(&step.id) {
            if existing.status == "decided" {
                let decision = existing.decision.as_deref().unwrap_or("rejected");
                info!("审批节点 '{}' 已有决策: {} → 继续执行", step.name, decision);
                return Ok(json!({
                    "decision": decision,
                    "comment": existing.comment,
                    "item": existing.item,
                    "decided_at": existing.decided_at,
                }));
            }
            // 还在 pending → 超时检查
            let created = chrono::DateTime::parse_from_rfc3339(&existing.created_at)
                .map(|t| t.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            let elapsed = chrono::Utc::now().signed_duration_since(created);
            if elapsed.num_seconds() as u64 >= existing.timeout_secs {
                // 超时自动决策
                info!("审批节点 '{}' 超时 ({}s)，自动{}", step.name, elapsed.num_seconds(), timeout_action);
                let auto_approved = timeout_action == "approve";
                record_decision(&step.id, auto_approved, Some(format!("超时自动{}", timeout_action)))?;
                return Ok(json!({
                    "decision": if auto_approved { "approved" } else { "rejected" },
                    "comment": format!("超时自动{}", timeout_action),
                    "item": existing.item,
                    "timed_out": true,
                }));
            }
            // 还在等
            return Ok(json!({
                "status": "awaiting_approval",
                "title": title,
                "message": message,
                "step_id": step.id,
            }));
        }

        // 首次执行：创建审批请求
        let now = chrono::Utc::now().to_rfc3339();
        let state = ApprovalState {
            run_id: ctx.run_id.clone(),
            step_id: step.id.clone(),
            title: title.clone(),
            message: message.clone(),
            status: "pending".to_string(),
            decision: None,
            comment: None,
            created_at: now,
            decided_at: None,
            timeout_secs,
            timeout_action: timeout_action.clone(),
            item: Some(item.clone()),
        };
        save_approval(&step.id, &state);

        info!("审批节点 '{}' 已创建审批请求，等待用户决策", step.name);

        Ok(json!({
            "status": "awaiting_approval",
            "title": title,
            "message": message,
            "step_id": step.id,
            "item": item,
        }))
    }
}
