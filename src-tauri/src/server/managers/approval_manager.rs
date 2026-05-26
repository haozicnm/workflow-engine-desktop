// server/managers/approval_manager.rs — 审批 handler
//
// 从 handlers.rs 提取的审批相关 handler 函数和类型。

use axum::{
    response::{Response, Json},
    http::StatusCode,
};
use serde::Deserialize;

use crate::server::handlers::{ok_response, err_response};

// ═══════════════════════════════════════════════════════════
// Request body types
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct ApprovalRespondBody {
    pub approval_id: String,
    pub approved: bool,
    pub comment: Option<String>,
    pub option: Option<String>,
}

// ═══════════════════════════════════════════════════════════
// 审批 handler
// ═══════════════════════════════════════════════════════════

pub async fn approval_list_pending(
) -> Response {
    let app = crate::server::state::get();
    let mut live = app.approval_store.pending().await;
    let live_ids: std::collections::HashSet<String> = live.iter().map(|e| e.id.clone()).collect();

    if let Ok(db_pending) = app.db.get_pending_approvals() {
        for rec in db_pending {
            if live_ids.contains(&rec.id) { continue; }
            let options: Vec<String> = rec.options.as_deref()
                .unwrap_or("同意,拒绝")
                .split(',').map(|s| s.trim().to_string()).collect();
            let item: Option<serde_json::Value> = rec.item.as_deref()
                .and_then(|s| serde_json::from_str(s).ok());
            live.push(crate::engine::approval_store::ApprovalEntry {
                id: rec.id,
                run_id: rec.run_id,
                step_id: rec.step_id,
                title: rec.title,
                message: rec.message,
                item,
                options,
                recommended: rec.recommended,
                timeout_secs: rec.timeout_secs as u64,
                timeout_action: rec.timeout_action,
                created_at: rec.created_at,
                recommendation_reason: None,
            });
        }
    }

    ok_response(live)
}

pub async fn approval_respond(
    Json(body): Json<ApprovalRespondBody>,
) -> Response {
    let app = crate::server::state::get();
    let option_str = body.option.unwrap_or_else(|| {
        if body.approved { "同意".into() } else { "拒绝".into() }
    });

    if let Err(e) = app.db.update_approval_decision(&body.approval_id, &option_str, body.comment.as_deref()) {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }

    let decision = crate::engine::approval_store::ApprovalDecision {
        option: option_str,
        comment: body.comment,
    };

    match app.approval_store.decide(&body.approval_id, decision).await {
        Ok(()) => ok_response(serde_json::json!({ "success": true })),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}
