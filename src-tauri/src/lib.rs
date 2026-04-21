// lib.rs — 库导出
pub mod commands;
pub mod data;
pub mod engine;
pub mod nodes;
pub mod system;

use std::sync::Arc;
use tokio::sync::RwLock;

/// 应用全局状态
pub struct App {
    pub db: Arc<RwLock<data::db::Database>>,
    pub config: Arc<RwLock<data::config::AppConfig>>,
}

impl App {
    pub fn new() -> Self {
        let db = data::db::Database::open_default().expect("failed to open database");
        let config = data::config::AppConfig::load_default().unwrap_or_default();

        App {
            db: Arc::new(RwLock::new(db)),
            config: Arc::new(RwLock::new(config)),
        }
    }
}
