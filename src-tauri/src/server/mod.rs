// server/mod.rs — 独立 HTTP 服务器模块（axum）
pub mod events;
pub mod handlers;
pub mod managers;
pub mod routes;
pub mod sse;
pub mod state;

use crate::App;
use axum::Router;
use std::sync::Arc;

/// 构建 axum Router（无状态，handlers 通过 state::get() 获取 App）
pub fn build_router(app: Arc<App>) -> Router {
    state::init(app);
    routes::build()
}
