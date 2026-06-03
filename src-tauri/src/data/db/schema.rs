// data/db/schema.rs — SQLite schema 版本迁移
use crate::data::db::Database;
use anyhow::Result;
use std::time::Instant;
use tracing::{debug, info};

impl Database {
    pub(crate) fn init_tables(&self) -> Result<()> {
        let conn = self.conn_ref()?;

        // 创建 schema 版本表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL)",
            [],
        )?;

        // 读取当前版本号
        let version: i64 = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| r.get(0),
        )?;

        // 迁移：为旧数据库添加 locked 列
        let _ = conn.execute(
            "ALTER TABLE workflows ADD COLUMN locked INTEGER DEFAULT 0",
            [],
        );

        if version < 1 {
            info!("[db] 执行 v1 初始化…");
            let start = Instant::now();
            conn.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS workflows (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    description TEXT DEFAULT '',
                    enabled INTEGER DEFAULT 1,
                    locked INTEGER DEFAULT 0,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS runs (
                    id TEXT PRIMARY KEY,
                    workflow_id TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'pending',
                    current_step TEXT,
                    started_at TEXT,
                    finished_at TEXT,
                    error TEXT
                );

                CREATE TABLE IF NOT EXISTS step_runs (
                    id TEXT PRIMARY KEY,
                    run_id TEXT NOT NULL,
                    step_id TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'pending',
                    started_at TEXT,
                    finished_at TEXT,
                    output TEXT,
                    error TEXT
                );

                CREATE TABLE IF NOT EXISTS step_logs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    step_run_id TEXT NOT NULL,
                    level TEXT NOT NULL DEFAULT 'info',
                    message TEXT NOT NULL,
                    timestamp TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS settings (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS approvals (
                    id TEXT PRIMARY KEY,
                    run_id TEXT NOT NULL,
                    step_id TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'pending',
                    created_at TEXT NOT NULL,
                    decided_at TEXT,
                    decided_by TEXT,
                    message TEXT
                );

                CREATE TABLE IF NOT EXISTS schedules (
                    id TEXT PRIMARY KEY,
                    workflow_id TEXT NOT NULL,
                    cron_expr TEXT NOT NULL,
                    enabled INTEGER DEFAULT 1,
                    last_run_at TEXT,
                    created_at TEXT NOT NULL
                );
            "#,
            )?;
            conn.execute("INSERT INTO schema_version (version) VALUES (1)", [])?;
            debug!("[db] v1 初始化完成，耗时 {:?}", start.elapsed());
        }

        // v2: 添加 yaml_content 列（如果不存在）
        if version < 2 {
            info!("[db] 执行 v2 迁移…");
            let start = Instant::now();
            conn.execute_batch("ALTER TABLE workflows ADD COLUMN yaml_content TEXT DEFAULT ''")
                .ok(); // 忽略"列已存在"的错误
            conn.execute("INSERT INTO schema_version (version) VALUES (2)", [])?;
            debug!("[db] v2 迁移完成，耗时 {:?}", start.elapsed());
        }

        // v3: 添加 workflow_name 列到 runs 表
        if version < 3 {
            info!("[db] 执行 v3 迁移…");
            let start = Instant::now();
            conn.execute_batch("ALTER TABLE runs ADD COLUMN workflow_name TEXT DEFAULT ''")
                .ok();
            conn.execute("INSERT INTO schema_version (version) VALUES (3)", [])?;
            debug!("[db] v3 迁移完成，耗时 {:?}", start.elapsed());
        }

        // v4: 重建 approvals 表（审批系统重构）
        if version < 4 {
            info!("[db] 执行 v4 迁移（审批系统重构）…");
            let start = Instant::now();
            conn.execute_batch(
                r#"
                DROP TABLE IF EXISTS approvals;
                CREATE TABLE approvals (
                    id TEXT PRIMARY KEY,
                    run_id TEXT NOT NULL,
                    step_id TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'pending',
                    title TEXT DEFAULT '',
                    message TEXT DEFAULT '',
                    item TEXT,
                    options TEXT,
                    recommended TEXT DEFAULT '',
                    timeout_secs INTEGER DEFAULT 300,
                    timeout_action TEXT DEFAULT 'recommended',
                    created_at TEXT NOT NULL,
                    decided_at TEXT,
                    decision TEXT,
                    comment TEXT
                );
            "#,
            )?;
            conn.execute("INSERT INTO schema_version (version) VALUES (4)", [])?;
            debug!("[db] v4 迁移完成，耗时 {:?}", start.elapsed());
        }

        Ok(())
    }
}
