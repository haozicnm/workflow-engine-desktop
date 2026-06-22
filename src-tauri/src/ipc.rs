// ipc.rs — WebSocket IPC server (localhost:19527)
//
// 桌面应用是唯一执行引擎。wf-cli 通过 WebSocket 发送请求，
// 桌面统一执行并推送实时 step_update / var_snapshot 事件。

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::net::{TcpListener, TcpStream};

use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{error, info, warn};

use crate::engine::{parser, scheduler};
use crate::App;
use tauri::Emitter;

const IPC_PORT: u16 = 19527;
const TOKEN_FILE: &str = ".hermes/daemon-token";

// ─── 消息类型 ───

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum IpcRequest {
    #[serde(rename = "run")]
    Run {
        id: String,
        workflow_id: String,
        vars: Option<HashMap<String, String>>,
    },
    #[serde(rename = "library_run")]
    LibraryRun {
        id: String,
        template: String,
        params: Option<HashMap<String, String>>,
    },
    #[serde(rename = "status")]
    Status { id: String, run_id: String },
    #[serde(rename = "list_runs")]
    ListRuns { id: String },
    #[serde(rename = "control")]
    Control {
        id: String,
        run_id: String,
        action: String,
    },
    #[serde(rename = "ping")]
    Ping { id: String },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum IpcResponse {
    #[serde(rename = "ack")]
    Ack {
        id: String,
        run_id: Option<String>,
        message: Option<String>,
    },
    #[serde(rename = "step_update")]
    #[allow(dead_code)]
    StepUpdate {
        run_id: String,
        step_id: String,
        step_name: String,
        status: String,
        output: Option<Value>,
        error: Option<String>,
    },
    #[serde(rename = "var_snapshot")]
    #[allow(dead_code)]
    VarSnapshot {
        run_id: String,
        variables: Value,
        step_outputs: Value,
    },
    #[serde(rename = "run_complete")]
    RunComplete {
        run_id: String,
        status: String,
        elapsed_secs: f64,
        error: Option<String>,
    },
    #[serde(rename = "pong")]
    Pong { id: String },
    #[serde(rename = "error")]
    Error { id: String, message: String },
}

// ─── Server ───

pub struct IpcServer {
    pub token: String,
    app: Arc<App>,
    app_handle: tauri::AppHandle,
}

impl IpcServer {
    pub fn new(app: Arc<App>, handle: tauri::AppHandle) -> Self {
        let token = uuid::Uuid::new_v4().to_string();
        IpcServer {
            token,
            app,
            app_handle: handle,
        }
    }

    /// 启动 IPC 服务器（非阻塞，返回 JoinHandle）
    pub async fn start(self: Arc<Self>) {
        let addr = SocketAddr::from(([127, 0, 0, 1], IPC_PORT));

        // 写入 token 文件
        if let Some(home) = dirs::home_dir() {
            let token_path = home.join(TOKEN_FILE);
            if let Some(parent) = token_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(&token_path, &self.token) {
                warn!("写入 token 文件失败: {}", e);
            } else {
                info!("IPC token 已写入: {}", token_path.display());
                // 设置文件权限 600（仅所有者可读写）
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(
                        &token_path,
                        std::fs::Permissions::from_mode(0o600),
                    );
                }
            }
        }

        let listener = match TcpListener::bind(addr).await {
            Ok(l) => {
                info!("IPC WebSocket server 启动: ws://{}", addr);
                l
            }
            Err(e) => {
                error!("IPC server 启动失败 (端口 {} 可能被占用): {}", IPC_PORT, e);
                return;
            }
        };

        let server = self.clone();
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, peer)) => {
                        info!("IPC 客户端连接: {}", peer);
                        let s = server.clone();
                        tokio::spawn(async move {
                            if let Err(e) = s.handle_connection(stream).await {
                                warn!("IPC 连接处理失败 ({}): {}", peer, e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("IPC accept 错误: {}", e);
                    }
                }
            }
        });
    }

    async fn handle_connection(&self, stream: TcpStream) -> Result<(), String> {
        let ws = accept_async(stream)
            .await
            .map_err(|e| format!("WebSocket handshake failed: {}", e))?;

        let (mut sender, mut receiver) = ws.split();

        // 认证阶段：等待第一个消息，必须包含正确的 token
        let mut authenticated = false;
        while let Some(msg) = receiver.next().await {
            let msg = msg.map_err(|e| format!("WS read error: {}", e))?;

            let text = match msg {
                Message::Text(t) => t,
                Message::Close(_) => {
                    if !authenticated {
                        warn!("IPC client disconnected without authentication");
                        return Err("Not authenticated".to_string());
                    }
                    info!("IPC 客户端断开");
                    break;
                }
                Message::Ping(data) => {
                    let _ = sender.send(Message::Pong(data)).await;
                    continue;
                }
                _ => continue,
            };

            // 解析消息中的 token（所有请求类型的公共字段）
            let raw_msg: serde_json::Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(e) => {
                    let err = IpcResponse::Error {
                        id: "unknown".to_string(),
                        message: format!("Invalid JSON: {}", e),
                    };
                    let _ = sender
                        .send(Message::Text(
                            serde_json::to_string(&err).unwrap_or_default(),
                        ))
                        .await;
                    continue;
                }
            };

            // Token 认证：首条消息必须包含正确的 token
            if !authenticated {
                let client_token = raw_msg.get("token").and_then(|v| v.as_str()).unwrap_or("");
                if !client_token.is_empty() && client_token != self.token {
                    warn!("IPC authentication failed: token mismatch");
                    let err = IpcResponse::Error {
                        id: raw_msg
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        message: "Authentication failed: invalid token".to_string(),
                    };
                    let _ = sender.send(Message::Text(
                        serde_json::to_string(&err).unwrap_or_else(|_| r#"{"type":"error","id":"unknown","message":"Authentication failed"}"#.to_string()),
                    )).await;
                    let _ = sender.close().await;
                    return Err("Authentication failed".to_string());
                }
                authenticated = true;
            }

            let request: IpcRequest = match serde_json::from_str(&text) {
                Ok(r) => r,
                Err(e) => {
                    let err = IpcResponse::Error {
                        id: "unknown".to_string(),
                        message: format!("Message parse error: {}", e),
                    };
                    let _ = sender
                        .send(Message::Text(serde_json::to_string(&err).unwrap_or_else(
                            |e| {
                                tracing::warn!("序列化 IPC 错误响应失败: {}", e);
                                r#"{"type":"error","id":"unknown","message":"Internal error"}"#
                                    .to_string()
                            },
                        )))
                        .await;
                    continue;
                }
            };

            let response = self.handle_request(request, &mut sender).await;
            if let Some(resp) = response {
                let json = serde_json::to_string(&resp).unwrap_or_default();
                let _ = sender.send(Message::Text(json)).await;
            }
        }

        Ok(())
    }

    async fn handle_request(
        &self,
        request: IpcRequest,
        sender: &mut futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<TcpStream>,
            Message,
        >,
    ) -> Option<IpcResponse> {
        match request {
            IpcRequest::Ping { id } => Some(IpcResponse::Pong { id }),

            IpcRequest::Run {
                id,
                workflow_id,
                vars,
            } => self.handle_run(id, workflow_id, vars, sender).await,

            IpcRequest::LibraryRun {
                id,
                template,
                params,
            } => self.handle_library_run(id, template, params, sender).await,

            IpcRequest::Status { id, run_id } => match self.app.db.get_run_detail(&run_id) {
                Ok(Some(detail)) => Some(IpcResponse::Ack {
                    id,
                    run_id: Some(run_id),
                    message: Some(format!("status: {}", detail.run.status)),
                }),
                Ok(None) => Some(IpcResponse::Error {
                    id,
                    message: "运行记录不存在".into(),
                }),
                Err(e) => Some(IpcResponse::Error {
                    id,
                    message: e.to_string(),
                }),
            },

            IpcRequest::ListRuns { id } => match self.app.db.list_runs(None, 10) {
                Ok(runs) => {
                    let list: Vec<String> = runs
                        .iter()
                        .map(|r| format!("{}: {}", r.id, r.status))
                        .collect();
                    Some(IpcResponse::Ack {
                        id,
                        run_id: None,
                        message: Some(list.join("\n")),
                    })
                }
                Err(e) => Some(IpcResponse::Error {
                    id,
                    message: e.to_string(),
                }),
            },

            IpcRequest::Control { id, run_id, action } => match action.as_str() {
                "cancel" => {
                    if let Some(token) = self.app.cancel_tokens.read().await.get(&run_id) {
                        token.cancel();
                        Some(IpcResponse::Ack {
                            id,
                            run_id: Some(run_id),
                            message: Some("已取消".into()),
                        })
                    } else {
                        Some(IpcResponse::Error {
                            id,
                            message: "运行记录不存在或已完成".into(),
                        })
                    }
                }
                "pause" => {
                    self.app
                        .pause_flags
                        .write()
                        .await
                        .entry(run_id.clone())
                        .or_insert_with(|| Arc::new(AtomicBool::new(false)))
                        .store(true, Ordering::SeqCst);
                    Some(IpcResponse::Ack {
                        id,
                        run_id: Some(run_id),
                        message: Some("已暂停".into()),
                    })
                }
                "resume" => {
                    if let Some(flag) = self.app.pause_flags.read().await.get(&run_id) {
                        flag.store(false, Ordering::SeqCst);
                        Some(IpcResponse::Ack {
                            id,
                            run_id: Some(run_id),
                            message: Some("已恢复".into()),
                        })
                    } else {
                        Some(IpcResponse::Error {
                            id,
                            message: "运行记录不存在".into(),
                        })
                    }
                }
                _ => Some(IpcResponse::Error {
                    id,
                    message: format!("未知操作: {}", action),
                }),
            },
        }
    }

    async fn handle_run(
        &self,
        id: String,
        workflow_id: String,
        vars: Option<HashMap<String, String>>,
        sender: &mut futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<TcpStream>,
            Message,
        >,
    ) -> Option<IpcResponse> {
        let yaml = match self.app.db.get_workflow_yaml(&workflow_id) {
            Ok(Some(y)) => y,
            Ok(None) => {
                return Some(IpcResponse::Error {
                    id,
                    message: "Workflow not found".into(),
                })
            }
            Err(e) => {
                return Some(IpcResponse::Error {
                    id,
                    message: e.to_string(),
                })
            }
        };

        let workflow = match parser::parse_workflow(&yaml) {
            Ok(w) => w,
            Err(e) => {
                return Some(IpcResponse::Error {
                    id,
                    message: format!("Parse error: {}", e),
                })
            }
        };

        let run_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let workflow_name = workflow.name.clone();

        if let Err(e) = self
            .app
            .db
            .create_run(&run_id, &workflow_id, &workflow_name, &now)
        {
            return Some(IpcResponse::Error {
                id,
                message: format!("Failed to create run record: {}", e),
            });
        }

        let _total = workflow.steps.len();
        let _permit = match self.app.run_semaphore.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                return Some(IpcResponse::Error {
                    id,
                    message: "并发限制，请稍后重试".into(),
                })
            }
        };

        let ctrl = scheduler::RunControl {
            cancel_flag: Arc::new(AtomicBool::new(false)),
            cancel_token: tokio_util::sync::CancellationToken::new(),
            pause_flag: Arc::new(AtomicBool::new(false)),
            breakpoint_flag: Arc::new(AtomicBool::new(false)),
            step_mode_flag: Arc::new(AtomicBool::new(false)),
            debug_snapshots: self.app.debug_snapshots.clone(),
        };

        // 注册到共享状态（支持 cancel/pause/resume）
        self.app.cancel_flags.insert(run_id.clone(), ctrl.cancel_flag.clone());
        self.app.cancel_tokens.insert(run_id.clone(), ctrl.cancel_token.clone());
        self.app.pause_flags.insert(run_id.clone(), ctrl.pause_flag.clone());
        self.app.breakpoint_flags.insert(run_id.clone(), ctrl.breakpoint_flag.clone());
        self.app.step_mode_flags.insert(run_id.clone(), ctrl.step_mode_flag.clone());

        // 发送 ack
        let ack = IpcResponse::Ack {
            id: id.clone(),
            run_id: Some(run_id.clone()),
            message: None,
        };
        let json = serde_json::to_string(&ack).unwrap_or_else(|e| {
            tracing::error!("序列化 ack 失败: {}", e);
            r#"{"type":"error","id":"internal","message":"Serialization failed"}"#.to_string()
        });
        if sender.send(Message::Text(json)).await.is_err() {
            return None;
        }

        // 推送到 Tauri 前端
        let app_handle = self.app_handle.clone();
        let run_id_clone = run_id.clone();
        let workflow_name_clone = workflow_name.clone();

        // 启动执行（后台任务，边跑边推事件）
        let db = self.app.db.clone();
        let approval_store = self.app.approval_store.clone();
        let vars_vec: Vec<(String, String)> = vars.unwrap_or_default().into_iter().collect();

        let start = std::time::Instant::now();
        let cfg = self.app.config.read().await;
        let timeouts = cfg.timeouts.clone();
        let shell_allowed = cfg.execution.shell_allowed_commands.clone();
        drop(cfg);
        let result = scheduler::run_workflow(
            &workflow,
            &run_id,
            Some(&app_handle),
            &db,
            approval_store,
            &vars_vec,
            &ctrl,
            &timeouts,
            &shell_allowed,
        )
        .await;

        let elapsed = start.elapsed().as_secs_f64();

        // 清理共享状态中的运行标志
        self.app.cancel_flags.remove(&run_id);
        self.app.cancel_tokens.remove(&run_id);
        self.app.pause_flags.remove(&run_id);
        self.app.breakpoint_flags.remove(&run_id);
        self.app.step_mode_flags.remove(&run_id);
        self.app.debug_snapshots.write().await.remove(&run_id);

        match result {
            Ok(_) => {
                let _ = db.update_run_status(&run_id, "completed", None);
                // 推送到 Tauri 前端
                let _ = app_handle.emit(
                    "run-update",
                    serde_json::json!({
                        "run_id": &run_id_clone,
                        "workflow_name": &workflow_name_clone,
                        "status": "completed",
                    }),
                );
                Some(IpcResponse::RunComplete {
                    run_id: run_id_clone,
                    status: "completed".into(),
                    elapsed_secs: elapsed,
                    error: None,
                })
            }
            Err(e) => {
                let err_msg = e.to_string();
                let _ = db.update_run_status(&run_id, "failed", Some(&err_msg));
                let _ = app_handle.emit(
                    "run-update",
                    serde_json::json!({
                        "run_id": &run_id_clone,
                        "workflow_name": &workflow_name_clone,
                        "status": "failed",
                    }),
                );
                Some(IpcResponse::RunComplete {
                    run_id: run_id_clone,
                    status: "failed".into(),
                    elapsed_secs: elapsed,
                    error: Some(err_msg),
                })
            }
        }
    }

    async fn handle_library_run(
        &self,
        id: String,
        template_name: String,
        params: Option<HashMap<String, String>>,
        sender: &mut futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<TcpStream>,
            Message,
        >,
    ) -> Option<IpcResponse> {
        // 加载 template（复用 cli 中 library 逻辑）
        let library_dir = {
            let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
            home.join(".hermes/workflows")
        };

        // 简化版：直接从 catalog.toml 找
        let catalog_path = library_dir.join("catalog.toml");
        let catalog_content = match std::fs::read_to_string(&catalog_path) {
            Ok(c) => c,
            Err(e) => {
                return Some(IpcResponse::Error {
                    id,
                    message: format!("Failed to read catalog.toml: {}", e),
                })
            }
        };

        #[derive(Deserialize)]
        struct Catalog {
            templates: Vec<TemplateEntry>,
        }
        #[derive(Deserialize)]
        struct TemplateEntry {
            name: String,
            file: String,
        }

        let catalog: Catalog = match toml::from_str(&catalog_content) {
            Ok(c) => c,
            Err(e) => {
                return Some(IpcResponse::Error {
                    id,
                    message: format!("Failed to parse catalog.toml: {}", e),
                })
            }
        };

        let entry = match catalog.templates.iter().find(|t| t.name == template_name) {
            Some(e) => e,
            None => {
                return Some(IpcResponse::Error {
                    id,
                    message: format!("Template '{}' not found", template_name),
                })
            }
        };

        let file_path = library_dir.join(&entry.file);
        let mut content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                return Some(IpcResponse::Error {
                    id,
                    message: format!("Failed to read template: {}", e),
                })
            }
        };

        // 参数替换
        if let Some(p) = params {
            for (key, val) in &p {
                let placeholder = format!("{{{{params.{}}}}}", key);
                content = content.replace(&placeholder, val);
            }
        }

        // 导入 + 运行
        let workflow_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        if let Err(e) = self
            .app
            .db
            .create_workflow(&workflow_id, &template_name, "", &now, &now)
        {
            return Some(IpcResponse::Error {
                id,
                message: format!("Import failed: {}", e),
            });
        }
        if let Err(e) = self.app.db.save_workflow_yaml(&workflow_id, &content) {
            return Some(IpcResponse::Error {
                id,
                message: format!("Save failed: {}", e),
            });
        }

        self.handle_run(id, workflow_id, None, sender).await
    }
}
