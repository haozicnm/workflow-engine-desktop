// data/db.rs — SQLite 数据库
use rusqlite::{Connection, params};
use anyhow::Result;
use crate::data::models::WorkflowMeta;
use crate::commands::workflow::WorkflowListItem;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open_default() -> Result<Self> {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("workflow-engine");
        std::fs::create_dir_all(&data_dir)?;
        let db_path = data_dir.join("engine.db");
        Self::open(&db_path)
    }

    pub fn open(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let mut db = Database { conn };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&mut self) -> Result<()> {
        self.conn.execute_batch(r#"
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
                error TEXT,
                FOREIGN KEY (workflow_id) REFERENCES workflows(id)
            );

            CREATE TABLE IF NOT EXISTS step_runs (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                step_id TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                started_at TEXT,
                finished_at TEXT,
                output TEXT,
                error TEXT,
                FOREIGN KEY (run_id) REFERENCES runs(id)
            );

            CREATE TABLE IF NOT EXISTS step_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                step_run_id TEXT NOT NULL,
                level TEXT NOT NULL DEFAULT 'info',
                message TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                FOREIGN KEY (step_run_id) REFERENCES step_runs(id)
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
        "#)?;
        Ok(())
    }

    // Workflow CRUD
    pub fn list_workflows(&self) -> Result<Vec<WorkflowListItem>> {
        let mut stmt = self.conn.prepare(
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

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    pub fn create_workflow(&self, id: &str, name: &str, desc: &str, created: &str, updated: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO workflows (id, name, description, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, name, desc, created, updated],
        )?;
        Ok(())
    }

    pub fn get_workflow(&self, id: &str) -> Result<Option<WorkflowMeta>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, enabled, created_at, updated_at FROM workflows WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(WorkflowMeta {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
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
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(updated.to_string())];
        let mut idx = 2;

        if let Some(n) = name {
            query.push_str(&format!(", name = ?{}", idx));
            params.push(Box::new(n.to_string()));
            idx += 1;
        }
        if let Some(d) = desc {
            query.push_str(&format!(", description = ?{}", idx));
            params.push(Box::new(d.to_string()));
            idx += 1;
        }
        if let Some(e) = enabled {
            query.push_str(&format!(", enabled = ?{}", idx));
            params.push(Box::new(if e { 1i64 } else { 0i64 }));
            idx += 1;
        }

        query.push_str(&format!(" WHERE id = ?{}", idx));
        params.push(Box::new(id.to_string()));

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        self.conn.execute(&query, &param_refs[..])?;
        Ok(())
    }

    pub fn delete_workflow(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM workflows WHERE id = ?1", params![id])?;
        Ok(())
    }
}
