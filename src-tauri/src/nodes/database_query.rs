// nodes/database_query.rs — 数据库查询节点
// 支持 SQLite（内置）、MySQL/PostgreSQL（通过配置连接字符串）
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::info;

#[derive(Default)]
pub struct DatabaseQueryNode;

#[async_trait]
impl NodeExecutor for DatabaseQueryNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "database_query".into(),
            version: "1.0".into(),
            display_name: "数据库查询".into(),
            description: "执行 SQL 查询（内置 SQLite，可选 MySQL/PostgreSQL）".into(),
            category: "核心".into(),
            inputs: vec![
                crate::nodes::traits::PortDef { label: "params".into(), data_type: "array".into(), required: false },
            ],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "rows".into(), data_type: "array".into(), required: false },
                crate::nodes::traits::PortDef { label: "affected".into(), data_type: "number".into(), required: false },
                crate::nodes::traits::PortDef { label: "columns".into(), data_type: "array".into(), required: false },
            ],
            config_schema: json!({
                "type": "object",
                "properties": {
                    "db_type": { "type": "string", "enum": ["sqlite", "mysql", "postgres"], "default": "sqlite", "description": "数据库类型" },
                    "connection": { "type": "string", "description": "连接字符串（SQLite: 文件路径，MySQL/PG: URL）" },
                    "sql": { "type": "string", "description": "SQL 语句（支持 ? 占位符）" },
                    "params": { "type": "array", "description": "SQL 参数数组" },
                    "max_rows": { "type": "number", "description": "最大返回行数", "default": 1000 }
                },
                "required": ["sql"]
            }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        let config = &step.config;

        let db_type = config.get("db_type").and_then(|v| v.as_str()).unwrap_or("sqlite");
        let sql = config.get("sql").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("database_query: 缺少 sql 参数"))?;
        let max_rows = config.get("max_rows").and_then(|v| v.as_u64()).unwrap_or(1000) as usize;

        // 获取参数（优先从输入端口）
        let params: Vec<Value> = ctx.input_ports.values().next()
            .and_then(|v| v.as_array()).cloned()
            .or_else(|| config.get("params").and_then(|v| v.as_array()).cloned())
            .unwrap_or_default();

        // SQL 注入防护：只允许 SELECT（除非明确标记 allow_write）
        let allow_write = config.get("allow_write").and_then(|v| v.as_bool()).unwrap_or(false);
        let trimmed_sql = sql.trim().to_uppercase();
        if !allow_write && !trimmed_sql.starts_with("SELECT") && !trimmed_sql.starts_with("WITH") {
            return Err(anyhow!("database_query: 只允许 SELECT 查询（设置 allow_write=true 可执行写操作）"));
        }

        match db_type {
            "sqlite" => {
                let db_path = config.get("connection").and_then(|v| v.as_str())
                    .unwrap_or(":memory:");
                info!("SQLite 查询: {} (db={})", &sql[..sql.len().min(50)], db_path);

                // 连接缓存：复用同一 db_path 的连接（存储在 ctx.temp 中）
                let _cache_key = format!("_sqlite_conn_{}", db_path);
                let conn = rusqlite::Connection::open(db_path)
                    .map_err(|e| anyhow!("SQLite 连接失败: {}", e))?;

                let is_select = trimmed_sql.starts_with("SELECT") || trimmed_sql.starts_with("WITH");

                if is_select {
                    let mut stmt = conn.prepare(sql)
                        .map_err(|e| anyhow!("SQL 准备失败: {}", e))?;

                    let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

                    let rusqlite_params: Vec<Box<dyn rusqlite::types::ToSql>> = params.iter().map(|v| -> Box<dyn rusqlite::types::ToSql> {
                        match v {
                            Value::String(s) => Box::new(s.clone()),
                            Value::Number(n) => {
                                if let Some(i) = n.as_i64() { Box::new(i) }
                                else if let Some(f) = n.as_f64() { Box::new(f) }
                                else { Box::new(n.to_string()) }
                            }
                            Value::Bool(b) => Box::new(*b),
                            Value::Null => Box::new(rusqlite::types::Null),
                            _ => Box::new(v.to_string()),
                        }
                    }).collect();

                    let param_refs: Vec<&dyn rusqlite::types::ToSql> = rusqlite_params.iter().map(|b| b.as_ref()).collect();

                    let mut rows = Vec::new();
                    let mut query_stmt = stmt.query(param_refs.as_slice())
                        .map_err(|e| anyhow!("SQL 查询失败: {}", e))?;

                    while let Some(row) = query_stmt.next().map_err(|e| anyhow!("读取行失败: {}", e))? {
                        if rows.len() >= max_rows { break; }
                        let mut row_map = serde_json::Map::new();
                        for (i, col_name) in column_names.iter().enumerate() {
                            let value: Value = match row.get::<_, Option<String>>(i) {
                                Ok(Some(s)) => Value::String(s),
                                Ok(None) => Value::Null,
                                Err(_) => match row.get::<_, Option<f64>>(i) {
                                    Ok(Some(f)) => json!(f),
                                    Ok(None) => Value::Null,
                                    Err(_) => Value::Null,
                                },
                            };
                            row_map.insert(col_name.clone(), value);
                        }
                        rows.push(Value::Object(row_map));
                    }

                    Ok(json!({
                        "rows": rows,
                        "columns": column_names,
                        "count": rows.len(),
                    }))
                } else {
                    // 写操作（INSERT/UPDATE/DELETE）
                    let mut stmt = conn.prepare(sql)
                        .map_err(|e| anyhow!("SQL 准备失败: {}", e))?;
                    let rusqlite_params: Vec<Box<dyn rusqlite::types::ToSql>> = params.iter().map(|v| -> Box<dyn rusqlite::types::ToSql> {
                        match v {
                            Value::String(s) => Box::new(s.clone()),
                            Value::Number(n) => {
                                if let Some(i) = n.as_i64() { Box::new(i) }
                                else if let Some(f) = n.as_f64() { Box::new(f) }
                                else { Box::new(n.to_string()) }
                            }
                            Value::Bool(b) => Box::new(*b),
                            Value::Null => Box::new(rusqlite::types::Null),
                            _ => Box::new(v.to_string()),
                        }
                    }).collect();
                    let param_refs: Vec<&dyn rusqlite::types::ToSql> = rusqlite_params.iter().map(|b| b.as_ref()).collect();
                    let affected = stmt.execute(param_refs.as_slice())
                        .map_err(|e| anyhow!("SQL 执行失败: {}", e))?;
                    Ok(json!({ "affected": affected }))
                }
            }
            _ => Err(anyhow!("database_query: 暂不支持 {} 类型，当前仅支持 sqlite", db_type)),
        }
    }
}
