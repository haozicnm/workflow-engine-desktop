// server/mod.rs — 独立 HTTP 服务器模块（axum）
pub mod auth;
pub mod events;
pub mod handlers;
pub mod managers;
pub mod routes;
pub mod sse;
pub mod state;

use crate::App;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware;
use std::sync::Arc;

/// 构建 axum Router（无状态，handlers 通过 state::get() 获取 App）
pub fn build_router(app: Arc<App>) -> Router {
    state::init(app);
    routes::build()
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB body 上限，防 OOM
        .layer(tower_http::cors::CorsLayer::permissive()) // CORS：允许跨域（本地开发 + Webhook 调用）
        .layer(middleware::from_fn(auth::auth_middleware))
}
