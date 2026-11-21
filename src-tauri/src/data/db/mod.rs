// data/db/mod.rs — SQLite 数据库（r2d2 连接池）
pub mod queries;
pub mod schema;

use anyhow::{Context, Result};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tracing::info;

pub struct Database {
    pool: Pool<SqliteConnectionManager>,
}

impl Database {
    pub fn open_default() -> Result<Self> {
        let data_dir = crate::data::paths::resolve_data_dir();
        std::fs::create_dir_all(&data_dir)?;
        let db_path = data_dir.join("engine.db");
        Self::open(&db_path)
    }

    /// 测试专用：在系统临时目录下创建一个**独立**数据库。
    ///
    /// 不要用 `open_default()` 写测试：那会打开并迁移用户真实的
    /// `%APPDATA%/com.workflow-engine.desktop/engine.db`，使测试既污染生产数据、
    /// 又依赖全局可写的数据目录（在只读 HOME / 沙箱 / CI 中直接失败）。
    ///
    /// 每次调用使用独立目录，确保测试数据库及其 WAL 文件完全隔离。
    pub fn open_temp() -> Result<Self> {
        let unique = uuid::Uuid::new_v4();
        let dir = std::env::temp_dir().join(format!(
            "wf-engine-test-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir)?;
        Self::open(&dir.join("engine.db"))
    }

    pub fn open(path: &std::path::Path) -> Result<Self> {
        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::builder()
            .max_size(8)
            .build(manager)
            .context("创建 SQLite 连接池失败")?;

        // 启用 WAL 模式提升并发读取性能
        {
            let conn = pool.get()?;
            conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")?;
        }

        let db = Database { pool };
        info!("SQLite 数据库已打开: {:?}", path);
        db.init_tables()?;
        Ok(db)
    }

    /// 供 schema 和 queries 模块内部使用的连接获取
    pub(crate) fn conn_ref(&self) -> Result<r2d2::PooledConnection<SqliteConnectionManager>> {
        self.pool.get().context("获取数据库连接失败")
    }
}
