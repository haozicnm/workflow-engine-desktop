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
use axum::http::{header, Method};
use axum::middleware;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};

/// 构建 axum Router（无状态，handlers 通过 state::get() 获取 App）
pub fn build_router(app: Arc<App>) -> Router {
    state::init(app);

    // 不能用 CorsLayer::permissive()：它会返回 `Access-Control-Allow-Origin: *`，
    // 于是用户浏览器里访问的任意网站都能 fetch http://127.0.0.1:19529
    // 并读取全部工作流 / 配置 / 运行记录。这里只回给本机来源。
    // 注意：桌面端前端走 Tauri IPC（invoke），根本不发 HTTP 请求；
    // HTTP 路径只用于浏览器 dev 模式与 webhook，因此收紧来源不影响应用本身。
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _parts| {
            auth::is_local_origin(origin)
        }))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    routes::build()
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)) // 10MB body 上限，防 OOM
        .layer(cors) // CORS：仅允许本机来源
        .layer(middleware::from_fn(auth::auth_middleware))
}
