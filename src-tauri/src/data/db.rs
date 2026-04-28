// data/db.rs — SQLite 数据库（r2d2 连接池 + schema 版本迁移）
use rusqlite::params;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::time::Instant;
use anyhow::{Result, Context};
use tracing::{info, debug};
use crate::data::models::{WorkflowMeta, RunInfo, StepRunInfo, RunHistoryItem, RunDetail, ScheduleInfo};
use crate::commands::workflow::WorkflowListItem;

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

    pub fn open(path: &std::path::Path) -> Result<Self> {
        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::builder()
            .max_size(8) // 连接池最大连接数
            .build(manager)
            .context("创建 SQLite 连接池失败")?;

        // 启用 WAL 模式提升并发读取性能
        {
            let conn = pool.get()?;
            conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")?;
        }

        let db = Database { pool };
        db.init_tables()?;
        Ok(db)
    }

    /// 获取连接（用于调试日志）
    fn conn(&self) -> Result<r2d2::PooledConnection<SqliteConnectionManager>> {
        let conn = self.pool.get()
            .context("获取数据库连接失败")?;
        Ok(conn)
    }

    // ─── Schema 版本迁移 ───

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn()?;

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

        if version < 1 {
            info!("[db] 执行 v1 初始化…");
            let start = Instant::now();
            conn.execute_batch(r#"
                CREATE TABLE IF NOT EXISTS workflows (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    description TEXT DEFAULT '',
                    enabled INTEGER DEFAULT 1,
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
            "#)?;
            conn.execute("INSERT INTO schema_version (version) VALUES (1)", [])?;
            debug!("[db] v1 初始化完成，耗时 {:?}", start.elapsed());
        }

        // v2: 添加 yaml_content 列（如果不存在）
        if version < 2 {
            info!("[db] 执行 v2 迁移…");
            let start = Instant::now();
            conn.execute_batch(
                "ALTER TABLE workflows ADD COLUMN yaml_content TEXT DEFAULT ''"
            ).ok(); // 忽略"列已存在"的错误
            conn.execute("INSERT INTO schema_version (version) VALUES (2)", [])?;
            debug!("[db] v2 迁移完成，耗时 {:?}", start.elapsed());
        }

        // 未来: if version < 3 { ... }
        Ok(())
    }

    // ─── Workflow CRUD ───

    pub fn list_workflows(&self) -> Result<Vec<WorkflowListItem>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, description, enabled, created_at, updated_at FROM workflows ORDER BY updated_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(WorkflowListItem {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                enabled: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn create_workflow(&self, id: &str, name: &str, desc: &str, created: &str, updated: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO workflows (id, name, description, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, name, desc, created, updated],
        )?;
        Ok(())
    }

    pub fn get_workflow(&self, id: &str) -> Result<Option<WorkflowMeta>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, description, enabled, created_at, updated_at, COALESCE(yaml_content, '') FROM workflows WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(WorkflowMeta {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                yaml: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                enabled: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn update_workflow(&self, id: &str, name: Option<&str>, desc: Option<&str>, enabled: Option<bool>, updated: &str) -> Result<()> {
        let mut query = String::from("UPDATE workflows SET updated_at = ?1");
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(updated.to_string())];
        let mut idx = 2;

        if let Some(n) = name {
            query.push_str(&format!(", name = ?{}", idx));
            params_vec.push(Box::new(n.to_string()));
            idx += 1;
        }
        if let Some(d) = desc {
            query.push_str(&format!(", description = ?{}", idx));
            params_vec.push(Box::new(d.to_string()));
            idx += 1;
        }
        if let Some(e) = enabled {
            query.push_str(&format!(", enabled = ?{}", idx));
            params_vec.push(Box::new(if e { 1i64 } else { 0i64 }));
            idx += 1;
        }

        query.push_str(&format!(" WHERE id = ?{}", idx));
        params_vec.push(Box::new(id.to_string()));

        let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let conn = self.conn()?;
        conn.execute(&query, &param_refs[..])?;
        Ok(())
    }

    pub fn delete_workflow(&self, id: &str) -> Result<()> {
        let conn = self.conn()?;
        // 级联删除：step_runs → runs → schedules → workflow
        conn.execute(
            "DELETE FROM step_runs WHERE run_id IN (SELECT id FROM runs WHERE workflow_id = ?1)",
            params![id],
        )?;
        conn.execute("DELETE FROM runs WHERE workflow_id = ?1", params![id])?;
        conn.execute("DELETE FROM schedules WHERE workflow_id = ?1", params![id])?;
        conn.execute("DELETE FROM workflows WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_workflow_yaml(&self, id: &str) -> Result<Option<String>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT yaml_content FROM workflows WHERE id = ?1")?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(row.get::<_, String>(0)?)
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn save_workflow_yaml(&self, id: &str, yaml: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn()?;
        conn.execute(
            "UPDATE workflows SET yaml_content = ?1, updated_at = ?2 WHERE id = ?3",
            params![yaml, now, id],
        )?;
        Ok(())
    }

    // ─── Run 持久化 ───

    pub fn create_run(&self, run_id: &str, workflow_id: &str, started_at: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO runs (id, workflow_id, status, started_at) VALUES (?1, ?2, 'running', ?3)",
            params![run_id, workflow_id, started_at],
        )?;
        Ok(())
    }

    pub fn update_run_status(&self, run_id: &str, status: &str, error: Option<&str>) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn()?;
        conn.execute(
            "UPDATE runs SET status = ?1, finished_at = ?2, error = ?3 WHERE id = ?4",
            params![status, now, error, run_id],
        )?;
        Ok(())
    }

    pub fn get_run(&self, run_id: &str) -> Result<Option<RunInfo>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, workflow_id, status, current_step, started_at, finished_at, error FROM runs WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![run_id], |row| {
            Ok(RunInfo {
                id: row.get(0)?,
                workflow_id: row.get(1)?,
                status: row.get(2)?,
                current_step: row.get(3)?,
                started_at: row.get(4)?,
                finished_at: row.get(5)?,
                error: row.get(6)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    // ─── StepRun 持久化 ───

    pub fn create_step_run(&self, run_id: &str, step_id: &str) -> Result<()> {
        let id = format!("{}:{}", run_id, step_id);
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn()?;
        conn.execute(
            "INSERT OR REPLACE INTO step_runs (id, run_id, step_id, status, started_at) VALUES (?1, ?2, ?3, 'running', ?4)",
            params![id, run_id, step_id, now],
        )?;
        Ok(())
    }

    pub fn complete_step_run(&self, run_id: &str, step_id: &str, output: Option<&serde_json::Value>, error: Option<&str>) -> Result<()> {
        let id = format!("{}:{}", run_id, step_id);
        let now = chrono::Utc::now().to_rfc3339();
        let status = if error.is_some() { "failed" } else { "completed" };
        let output_str = output.map(|v| v.to_string());
        let conn = self.conn()?;
        conn.execute(
            "UPDATE step_runs SET status = ?1, finished_at = ?2, output = ?3, error = ?4 WHERE id = ?5",
            params![status, now, output_str, error, id],
        )?;
        Ok(())
    }

    pub fn get_step_runs(&self, run_id: &str) -> Result<Vec<StepRunInfo>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, run_id, step_id, status, started_at, finished_at, output, error FROM step_runs WHERE run_id = ?1 ORDER BY started_at"
        )?;
        let rows = stmt.query_map(params![run_id], |row| {
            let output_str: Option<String> = row.get(6)?;
            let output = output_str.and_then(|s| serde_json::from_str(&s).ok());
            Ok(StepRunInfo {
                id: row.get(0)?,
                run_id: row.get(1)?,
                step_id: row.get(2)?,
                status: row.get(3)?,
                started_at: row.get(4)?,
                finished_at: row.get(5)?,
                output,
                error: row.get(7)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    // ─── 运行历史查询 ───

    /// 查询运行列表（可选按工作流过滤），关联工作流名称
    pub fn list_runs(&self, workflow_id: Option<&str>, limit: usize) -> Result<Vec<RunHistoryItem>> {
        let conn = self.conn()?;
        if let Some(wf_id) = workflow_id {
            let mut stmt = conn.prepare(
                "SELECT r.id, r.workflow_id, w.name, r.status, r.started_at, r.finished_at, r.error \
                 FROM runs r JOIN workflows w ON r.workflow_id = w.id \
                 WHERE r.workflow_id = ?1 \
                 ORDER BY r.started_at DESC LIMIT ?2"
            )?;
            let rows = stmt.query_map(rusqlite::params![wf_id, limit as i64], |row| {
                Ok(RunHistoryItem {
                    id: row.get(0)?,
                    workflow_id: row.get(1)?,
                    workflow_name: row.get(2)?,
                    status: row.get(3)?,
                    started_at: row.get(4)?,
                    finished_at: row.get(5)?,
                    error: row.get(6)?,
                })
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
        } else {
            let mut stmt = conn.prepare(
                "SELECT r.id, r.workflow_id, w.name, r.status, r.started_at, r.finished_at, r.error \
                 FROM runs r JOIN workflows w ON r.workflow_id = w.id \
                 ORDER BY r.started_at DESC LIMIT ?1"
            )?;
            let rows = stmt.query_map(rusqlite::params![limit as i64], |row| {
                Ok(RunHistoryItem {
                    id: row.get(0)?,
                    workflow_id: row.get(1)?,
                    workflow_name: row.get(2)?,
                    status: row.get(3)?,
                    started_at: row.get(4)?,
                    finished_at: row.get(5)?,
                    error: row.get(6)?,
                })
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
        }
    }

    /// 查询单次运行详情（运行信息 + 工作流名称 + 步骤执行记录）
    pub fn get_run_detail(&self, run_id: &str) -> Result<Option<RunDetail>> {
        let run = match self.get_run(run_id)? {
            Some(r) => r,
            None => return Ok(None),
        };

        let workflow_name = {
            let conn = self.conn()?;
            let mut stmt = conn.prepare("SELECT name FROM workflows WHERE id = ?1")?;
            stmt.query_row(params![run.workflow_id], |row| row.get::<_, String>(0))
                .unwrap_or_default()
        };

        let steps = self.get_step_runs(run_id)?;

        Ok(Some(RunDetail {
            run,
            workflow_name,
            steps,
        }))
    }

    /// 删除指定工作流的所有运行记录
    pub fn delete_runs_by_workflow(&self, workflow_id: &str) -> Result<()> {
        let conn = self.conn()?;
        // 先删 step_runs，再删 runs
        conn.execute(
            "DELETE FROM step_runs WHERE run_id IN (SELECT id FROM runs WHERE workflow_id = ?1)",
            params![workflow_id],
        )?;
        conn.execute("DELETE FROM runs WHERE workflow_id = ?1", params![workflow_id])?;
        Ok(())
    }

    // ─── Schedule CRUD ───

    pub fn create_schedule(&self, id: &str, workflow_id: &str, cron_expr: &str, created_at: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO schedules (id, workflow_id, cron_expr, enabled, created_at) VALUES (?1, ?2, ?3, 1, ?4)",
            params![id, workflow_id, cron_expr, created_at],
        )?;
        Ok(())
    }

    pub fn list_schedules(&self) -> Result<Vec<ScheduleInfo>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT s.id, s.workflow_id, w.name, s.cron_expr, s.enabled, s.last_run_at, s.created_at \
             FROM schedules s JOIN workflows w ON s.workflow_id = w.id \
             ORDER BY s.created_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ScheduleInfo {
                id: row.get(0)?,
                workflow_id: row.get(1)?,
                workflow_name: row.get(2)?,
                cron_expr: row.get(3)?,
                enabled: row.get::<_, i64>(4)? != 0,
                last_run_at: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_enabled_schedules(&self) -> Result<Vec<ScheduleInfo>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT s.id, s.workflow_id, w.name, s.cron_expr, s.enabled, s.last_run_at, s.created_at \
             FROM schedules s JOIN workflows w ON s.workflow_id = w.id \
             WHERE s.enabled = 1"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ScheduleInfo {
                id: row.get(0)?,
                workflow_id: row.get(1)?,
                workflow_name: row.get(2)?,
                cron_expr: row.get(3)?,
                enabled: row.get::<_, i64>(4)? != 0,
                last_run_at: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn update_schedule(&self, id: &str, cron_expr: Option<&str>, enabled: Option<bool>) -> Result<()> {
        let mut query = String::from("UPDATE schedules SET id = id");
        let mut param_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut idx = 1;

        if let Some(cron) = cron_expr {
            query.push_str(&format!(", cron_expr = ?{}", idx));
            param_vec.push(Box::new(cron.to_string()));
            idx += 1;
        }
        if let Some(en) = enabled {
            query.push_str(&format!(", enabled = ?{}", idx));
            param_vec.push(Box::new(if en { 1i64 } else { 0i64 }));
            idx += 1;
        }

        query.push_str(&format!(" WHERE id = ?{}", idx));
        param_vec.push(Box::new(id.to_string()));

        let param_refs: Vec<&dyn rusqlite::ToSql> = param_vec.iter().map(|p| p.as_ref()).collect();
        let conn = self.conn()?;
        conn.execute(&query, &param_refs[..])?;
        Ok(())
    }

    pub fn delete_schedule(&self, id: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM schedules WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn update_schedule_last_run(&self, id: &str, last_run_at: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE schedules SET last_run_at = ?1 WHERE id = ?2",
            params![last_run_at, id],
        )?;
        Ok(())
    }
}
