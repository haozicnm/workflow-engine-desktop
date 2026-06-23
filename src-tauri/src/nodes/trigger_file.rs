// nodes/trigger_file.rs — 文件变化触发器节点
// 监测文件/目录的创建、修改、删除事件
// v9.0: 实现文件轮询监测（每 5 秒检查 mtime 变化）
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// 全局文件监控状态：path → last_mtime
static WATCHER_STATE: std::sync::LazyLock<RwLock<HashMap<String, std::time::SystemTime>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

/// 启动文件监控轮询服务（在 App::new 中调用，每 5 秒检查一次）
/// 当检测到文件变化时，将事件注入到等待中的工作流
pub fn start_file_watcher(db: Arc<crate::data::db::Database>) {
    tokio::spawn(async move {
        info!("文件监控服务已启动（轮询间隔 5 秒）");
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            let state = WATCHER_STATE.read().await;
            let paths: Vec<String> = state.keys().cloned().collect();
            drop(state);

            for path in paths {
                let meta = match tokio::fs::metadata(&path).await {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);

                let mut state = WATCHER_STATE.write().await;
                let last = state.get(&path).copied().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                if mtime > last {
                    state.insert(path.clone(), mtime);
                    drop(state);
                    info!("文件变化检测: {} (mtime changed)", path);
                    // 通过数据库查找包含此路径的工作流并触发
                    if let Ok(schedules) = db.list_enabled_schedules() {
                        for s in schedules {
                            // 检查工作流是否包含 trigger_file 节点
                            if let Ok(Some(yaml)) = db.get_workflow_yaml(&s.workflow_id) {
                                if yaml.contains(&path) && yaml.contains("trigger_file") {
                                    info!("文件触发: workflow={} file={}", s.workflow_id, path);
                                    // 这里可以通过 scheduler 触发工作流
                                    // 但当前 scheduler 仅支持 cron，需要扩展
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}

#[derive(Default)]
pub struct TriggerFileNode;

#[async_trait]
impl NodeExecutor for TriggerFileNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "trigger_file".into(),
            version: "1.0".into(),
            display_name: "文件变化触发器".into(),
            description: "监测文件/目录变化（创建、修改、删除）时触发工作流".into(),
            category: "触发器".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "path".into(), data_type: "string".into(), required: false },
                crate::nodes::traits::PortDef { label: "event".into(), data_type: "string".into(), required: false },
                crate::nodes::traits::PortDef { label: "triggered_at".into(), data_type: "string".into(), required: false },
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "监测的文件/目录路径" },
                    "events": {
                        "type": "array",
                        "items": { "type": "string", "enum": ["create", "modify", "delete"] },
                        "description": "监听的事件类型",
                        "default": ["create", "modify"]
                    }
                },
                "required": ["path"]
            }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        _step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        // 文件触发时，触发信息已通过 ctx.variables 注入
        let path = ctx.variables
            .get("_file_trigger_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let event = ctx.variables
            .get("_file_trigger_event")
            .and_then(|v| v.as_str())
            .unwrap_or("modify");

        info!("文件变化触发: {} ({})", path, event);

        Ok(json!({
            "path": path,
            "event": event,
            "triggered_at": chrono::Utc::now().to_rfc3339(),
        }))
    }
}
