// nodes/approval.rs — 审批节点（channel 暂停/恢复 + SQLite 持久化版）
//
// 流程：
//   1. 查 SQLite 是否已有决策 → 有则直接返回
//   2. 首次创建 → 写入 SQLite（pending）→ 注册到 ApprovalStore → 等待
//   3. 收到决策/超时 → 更新 SQLite → 返回结果
//
// 输出：
//   { decision: "选项名", comment: "...", item: {...}, auto?: true }

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use crate::engine::approval_store::{ApprovalEntry, ApprovalDecision};
use std::sync::Arc;
use anyhow::Result;
use serde_json::{Value, json};
use tracing::info;

#[derive(Default)]
pub struct ApprovalNode;

#[async_trait]
impl NodeExecutor for ApprovalNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        let config = &step.config;
        let approval_id = format!("approval:{}:{}", ctx.run_id, step.id);

        // ── 读取配置 ──
        let title = config.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("人工审批")
            .to_string();

        let message_template = config.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("请审批此操作")
            .to_string();

        let message = {
            let tmp_val = json!(message_template);
            let config_val = json!({ "_msg": tmp_val });
            let resolved = ctx.resolve_config(&config_val);
            resolved.get("_msg")
                .and_then(|v| v.as_str())
                .unwrap_or(&message_template)
                .to_string()
        };

        let options_str = config.get("options")
            .and_then(|v| v.as_str())
            .unwrap_or("同意,拒绝");
        let options: Vec<String> = options_str
            .split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

        let recommended = config.get("recommended")
            .and_then(|v| v.as_str())
            .unwrap_or("同意").to_string();

        let require_review = config.get("require_review")
            .and_then(|v| {
                if let Some(b) = v.as_bool() { Some(b) }
                else if let Some(s) = v.as_str() { Some(s == "true") }
                else { None }
            }).unwrap_or(true);

        let timeout_secs = config.get("timeout")
            .and_then(|v| v.as_u64()).unwrap_or(300);

        let timeout_action = config.get("timeout_action")
            .and_then(|v| v.as_str())
            .unwrap_or("recommended").to_string();

        let item = collect_upstream_item(ctx, config);

        // ── 不需要人工审核 → 直接用推荐选项 ──
        if !require_review {
            info!("审批节点 '{}' 自动决策（无需审核）→ {}", step.name, recommended);
            return Ok(json!({
                "decision": recommended,
                "comment": "自动决策（无需审核）",
                "item": item,
                "auto": true,
            }));
        }

        // ── 查 SQLite 是否已有决策（重启恢复场景） ──
        if let Some(existing) = executor.db.get_approval(&approval_id) {
            match existing.status.as_str() {
                "decided" => {
                    info!("审批节点 '{}' SQLite 已有决策: {} → 直接返回", step.name, existing.decision.as_deref().unwrap_or("?"));
                    // 清理 SQLite 记录
                    let _ = executor.db.delete_approval(&approval_id);
                    return Ok(json!({
                        "decision": existing.decision.as_deref().unwrap_or("拒绝"),
                        "comment": existing.comment,
                        "item": item,
                        "auto": false,
                    }));
                }
                "pending" => {
                    // 检查是否已超时
                    if let Ok(created) = chrono::DateTime::parse_from_rfc3339(&existing.created_at) {
                        let elapsed = chrono::Utc::now().signed_duration_since(created.with_timezone(&chrono::Utc));
                        if elapsed.num_seconds() as u64 >= existing.timeout_secs as u64 {
                            let auto_option = resolve_timeout_action(&existing.timeout_action, &existing.recommended);
                            info!("审批节点 '{}' 重启后发现已超时 → 自动 {}", step.name, auto_option);
                            executor.db.update_approval_decision(&approval_id, &auto_option, Some("超时自动执行（重启恢复）")).ok();
                            let _ = executor.db.delete_approval(&approval_id);
                            return Ok(json!({
                                "decision": auto_option,
                                "comment": "超时自动执行（重启恢复）",
                                "item": item,
                                "auto": true,
                            }));
                        }
                    }
                    // 还在等待中 → 继续等待（fall through to register）
                    info!("审批节点 '{}' 重启后发现仍在等待中 → 重新注册", step.name);
                }
                _ => {}
            }
        }

        // ── 首次创建：写入 SQLite ──
        let item_str = item.as_ref().map(|v| v.to_string());
        let options_str = options.join(",");
        let _ = executor.db.insert_approval(
            &approval_id, &ctx.run_id, &step.id, &title, &message,
            item_str.as_deref(), Some(&options_str), &recommended,
            timeout_secs as i64, &timeout_action, &chrono::Utc::now().to_rfc3339(),
        );

        // ── 注册到 ApprovalStore 并等待 ──
        let entry = ApprovalEntry {
            id: approval_id.clone(),
            run_id: ctx.run_id.clone(),
            step_id: step.id.clone(),
            title, message,
            item: item.clone(),
            options: options.clone(),
            recommended: recommended.clone(),
            timeout_secs,
            timeout_action: timeout_action.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let mut rx = executor.approval_store.register(entry).await;
        info!("审批节点 '{}' 已注册，等待用户决策 (id: {})", step.name, approval_id);

        // ── 等待决策或超时 ──
        let decision = tokio::select! {
            Some(d) = rx.recv() => {
                info!("审批节点 '{}' 收到用户决策: {}", step.name, d.option);
                // 用户决策 → 更新 SQLite
                executor.db.update_approval_decision(&approval_id, &d.option, d.comment.as_deref()).ok();
                let _ = executor.db.delete_approval(&approval_id);
                d
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(timeout_secs)) => {
                let auto_option = resolve_timeout_action(&timeout_action, &recommended);
                info!("审批节点 '{}' 超时，自动执行: {}", step.name, auto_option);
                // 超时 → 更新 SQLite
                executor.db.update_approval_decision(&approval_id, &auto_option, Some(&format!("超时自动执行（{}秒）", timeout_secs))).ok();
                let _ = executor.db.delete_approval(&approval_id);
                // 从 store 清理
                executor.approval_store.decide(&approval_id, ApprovalDecision {
                    option: auto_option.clone(),
                    comment: Some(format!("超时自动执行（{}秒）", timeout_secs)),
                }).await.ok();
                ApprovalDecision {
                    option: auto_option,
                    comment: Some(format!("超时自动执行（{}秒）", timeout_secs)),
                }
            }
        };

        let is_auto = decision.comment.as_deref().unwrap_or("").contains("超时");

        Ok(json!({
            "decision": decision.option,
            "comment": decision.comment,
            "item": item,
            "auto": is_auto,
        }))
    }
}

/// 解析超时动作
fn resolve_timeout_action(timeout_action: &str, recommended: &str) -> String {
    match timeout_action {
        "recommended" => recommended.to_string(),
        "reject" => "拒绝".to_string(),
        "approve" => "同意".to_string(),
        other => other.to_string(),
    }
}

/// 收集上游数据作为审批上下文
fn collect_upstream_item(ctx: &ExecutionContext, config: &Value) -> Option<Value> {
    if let Some(source) = config.get("data_source").and_then(|v| v.as_str()) {
        if let Some(output) = ctx.get_output(source) {
            return Some(output.clone());
        }
    }
    ctx.step_outputs.values().last().cloned()
}
