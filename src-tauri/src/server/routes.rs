// server/routes.rs — 路由定义

use axum::Router;
use std::sync::Arc;
use crate::App;

/// 构建所有路由
pub fn build(_app: Arc<App>) -> Router {
    Router::new()
        // 占位：后续 phase 添加具体路由
}
