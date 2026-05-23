// server/mod.rs — 独立 HTTP 服务器模块（axum）
pub mod events;
pub mod handlers;
pub mod routes;
pub mod sse;

use axum::Router;
use std::sync::Arc;
use crate::App;

/// 构建 axum Router（挂载所有路由）
pub fn build_router(app: Arc<App>) -> Router {
    Router::new()
        .merge(routes::build(app))
}
