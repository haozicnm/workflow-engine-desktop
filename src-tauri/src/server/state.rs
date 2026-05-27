// server/state.rs — 全局 App 状态（避免 axum 0.7 Router 状态类型问题）
use crate::App;
use std::sync::Arc;

static GLOBAL_APP: std::sync::OnceLock<Arc<App>> = std::sync::OnceLock::new();

pub fn init(app: Arc<App>) {
    GLOBAL_APP.set(app).ok();
}

pub fn get() -> Arc<App> {
    GLOBAL_APP.get().expect("App not initialized").clone()
}
