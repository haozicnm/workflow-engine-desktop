// nodes/recording.rs — 操作录制节点（v2：浏览器+桌面双通道 + 自动生成工作流）
//
// 支持：
//   - 浏览器录制（通过 Playwright sidecar）
//   - 桌面录制（通过 desktop_recorder.py sidecar）
//   - 混合录制（浏览器 + 桌面同时录制）
//   - 录制结果 → YAML 工作流自动转换
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::recording_converter::{self, RecordedAction, RecordingSource};
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::Mutex;
use tracing::{info, warn};

/// Windows: 禁止子进程弹出 cmd 窗口
#[cfg(target_os = "windows")]
fn hide_console(cmd: &mut tokio::process::Command) {
    #[allow(unused_imports)]
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000);
}
#[cfg(not(target_os = "windows"))]
fn hide_console(_cmd: &mut tokio::process::Command) {}

// nodes/recording.rs
pub struct DesktopRecorder {
    #[allow(dead_code)]
    child: Mutex<Option<Child>>,
    stdin: Arc<Mutex<ChildStdin>>,
    stdout: Arc<Mutex<BufReader<ChildStdout>>>,
}

impl DesktopRecorder {
    pub async fn start() -> Result<Self> {
        let python = find_desktop_recorder_python()?;
        let script_path = find_desktop_recorder_script()?;

        let mut cmd = tokio::process::Command::new(&python);
        hide_console(&mut cmd);
        let mut child = cmd
            .arg(&script_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("启动桌面录制器失败: {}", e))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("无法获取桌面录制器 stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("无法获取桌面录制器 stdout"))?;

        let recorder = DesktopRecorder {
            child: Mutex::new(Some(child)),
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: Arc::new(Mutex::new(BufReader::new(stdout))),
        };

        // 等待 ready 信号
        let ready = recorder.read_response().await?;
        if ready.get("type").and_then(|v| v.as_str()) != Some("ready") {
            return Err(anyhow!("桌面录制器启动异常: {:?}", ready));
        }

        info!("桌面录制器已启动");
        Ok(recorder)
    }

    pub async fn send_action(
        &self,
        action: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let id = uuid::Uuid::new_v4().to_string();
        let request = serde_json::json!({
            "id": id,
            "action": action,
            "params": params,
        });

        {
            let mut stdin = self.stdin.lock().await;
            let line = serde_json::to_string(&request)?;
            stdin.write_all(line.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
        }

        let response = self.read_response().await?;
        let success = response
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if success {
            Ok(response
                .get("data")
                .cloned()
                .unwrap_or(serde_json::Value::Null))
        } else {
            let error = response
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("未知错误");
            Err(anyhow!("桌面录制操作失败: {}", error))
        }
    }

    async fn read_response(&self) -> Result<serde_json::Value> {
        let mut stdout = self.stdout.lock().await;
        let mut line = String::new();
        let n = stdout.read_line(&mut line).await?;
        if n == 0 {
            return Err(anyhow!("桌面录制器进程已退出"));
        }
        let line = line.trim();
        if line.is_empty() {
            return Err(anyhow!("收到空响应"));
        }
        serde_json::from_str(line).map_err(|e| anyhow!("桌面录制器响应解析失败: {}", e))
    }

    pub async fn shutdown(&self) -> Result<()> {
        let _ = self.send_action("shutdown", serde_json::json!({})).await;
        let mut guard = self.child.lock().await;
        if let Some(ref mut child) = *guard {
            let _ = child.wait().await;
        }
        Ok(())
    }
}

impl Drop for DesktopRecorder {
    fn drop(&mut self) {
        // 应用退出时强制 kill Python 子进程，防止孤儿进程残留
        if let Ok(mut guard) = self.child.try_lock() {
            if let Some(ref mut child) = *guard {
                let _ = child.start_kill();
            }
        }
    }
}

fn find_desktop_recorder_python() -> Result<String> {
    // 1. 内置 Python
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let python = dir.join("embed").join("python.exe");
            if python.exists() {
                return Ok(python.to_string_lossy().to_string());
            }
        }
    }
    // 2. 系统 Python
    which::which("python3")
        .or_else(|_| which::which("python"))
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|_| anyhow!("未找到 Python"))
}

fn find_desktop_recorder_script() -> Result<std::path::PathBuf> {
    // 1. 相对可执行文件
    if let Ok(exe_dir) = std::env::current_exe() {
        if let Some(dir) = exe_dir.parent() {
            let p = dir.join("sidecars").join("desktop_recorder.py");
            if p.exists() {
                return Ok(p);
            }
        }
    }
    // 2. 开发模式
    let cwd_script = std::path::Path::new("src-tauri/sidecars/desktop_recorder.py");
    if cwd_script.exists() {
        return Ok(cwd_script.to_path_buf());
    }
    // 3. CARGO_MANIFEST_DIR
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = std::path::PathBuf::from(manifest)
            .join("sidecars")
            .join("desktop_recorder.py");
        if p.exists() {
            return Ok(p);
        }
    }
    Err(anyhow!("找不到 desktop_recorder.py"))
}

// ─── 全局录制状态（跨 step_test 调用共享） ───
use std::sync::atomic::{AtomicBool, Ordering};
static DESKTOP_RECORDER: tokio::sync::RwLock<Option<Arc<DesktopRecorder>>> =
    tokio::sync::RwLock::const_new(None);
static DESKTOP_RECORDING: AtomicBool = AtomicBool::new(false);
static RECORDING_MODE: tokio::sync::RwLock<String> = tokio::sync::RwLock::const_new(String::new());
static RECORDING_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Linux 录制后端全局实例（跨 step_test 调用共享线程句柄）
#[cfg(target_os = "linux")]
static LINUX_RECORDER: std::sync::Mutex<Option<crate::platform::recording::LinuxRecordingBackend>> =
    std::sync::Mutex::new(None);

#[cfg(target_os = "linux")]
fn get_linux_recorder(
) -> std::sync::MutexGuard<'static, Option<crate::platform::recording::LinuxRecordingBackend>> {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        // 初始化占位
    });
    LINUX_RECORDER.lock().unwrap_or_else(|e| {
        tracing::error!("LINUX_RECORDER Mutex 中毒: {:?}", e);
        // 恢复锁并继续——poisoned Mutex 仍可访问，数据完整性由调用方保证
        e.into_inner()
    })
}

#[allow(dead_code)]
async fn get_desktop_recorder() -> Result<Arc<DesktopRecorder>> {
    let guard = DESKTOP_RECORDER.read().await;
    if let Some(ref r) = *guard {
        return Ok(Arc::clone(r));
    }
    drop(guard);

    let mut guard = DESKTOP_RECORDER.write().await;
    if let Some(ref r) = *guard {
        return Ok(Arc::clone(r));
    }

    let recorder = DesktopRecorder::start().await?;
    let arc = Arc::new(recorder);
    *guard = Some(Arc::clone(&arc));
    Ok(arc)
}

/// 获取录制状态（供 commands 层调用，跨 step_test 会话）
pub async fn get_recording_status() -> serde_json::Value {
    let mode_guard = RECORDING_MODE.read().await;
    let mode = if mode_guard.is_empty() {
        "none"
    } else {
        mode_guard.as_str()
    };
    let is_active = RECORDING_ACTIVE.load(Ordering::SeqCst);
    serde_json::json!({
        "recording": is_active,
        "mode": mode,
    })
}

// ─── 录制节点实现 ───

#[derive(Default)]
pub struct RecordingNode;

#[async_trait]
impl NodeExecutor for RecordingNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let action = config
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("start");

        match action {
            // ─── 开始浏览器录制 ───
            "browser_start" => {
                let headless = config.get("headless").and_then(|v| v.as_bool()).unwrap_or(false);
                let params = serde_json::json!({"headless": headless});
                let result = crate::nodes::browser::send_sidecar_action("recording_start", &params).await?;
                ctx.set_var("__recording_mode".to_string(), serde_json::json!("browser"));
                Ok(serde_json::json!({"action": "browser_start", "result": result}))
            }

            // ─── 停止浏览器录制 → 返回操作列表 ───
            "browser_stop" => {
                let result = crate::nodes::browser::send_sidecar_action("recording_stop", &serde_json::json!({})).await?;
                let actions = result.get("actions").cloned().unwrap_or_else(|| serde_json::json!([]));
                ctx.set_var("__recorded_actions".to_string(), actions.clone());
                ctx.set_var("__recording_source".to_string(), serde_json::json!("browser"));
                Ok(serde_json::json!({
                    "action": "browser_stop",
                    "actions": actions,
                    "count": actions.as_array().map(|a| a.len()).unwrap_or(0),
                }))
            }

            // ─── 开始桌面录制 ───
            "desktop_start" => {
                #[cfg(target_os = "linux")]
                {
                    use crate::platform::traits::RecordingBackend;
                    {
                        let mut guard = get_linux_recorder();
                        *guard = Some(crate::platform::recording::LinuxRecordingBackend::new());
                        match guard.as_ref() {
                            Some(r) => r.start()?,
                            None => {
                                tracing::error!("LinuxRecordingBackend 意外为 None");
                                return Err(anyhow::anyhow!("Linux 录制后端未初始化"));
                            }
                        }
                    }
                    DESKTOP_RECORDING.store(true, Ordering::SeqCst);
                    RECORDING_ACTIVE.store(true, Ordering::SeqCst);
                    *RECORDING_MODE.write().await = "desktop".to_string();
                    return Ok(serde_json::json!({"action": "desktop_start", "mode": "linux_native", "result": {"message": "Linux 录制已开始"}}));
                }
                #[cfg(not(target_os = "linux"))]
                {
                    let recorder = get_desktop_recorder().await?;
                    let result = recorder.send_action("start", serde_json::json!({})).await?;
                    DESKTOP_RECORDING.store(true, Ordering::SeqCst);
                    RECORDING_ACTIVE.store(true, Ordering::SeqCst);
                    *RECORDING_MODE.write().await = "desktop".to_string();
                    return Ok(serde_json::json!({"action": "desktop_start", "result": result}));
                }
            }

            // ─── 停止桌面录制 → 返回操作列表 ───
            "desktop_stop" => {
                #[cfg(target_os = "linux")]
                {
                    use crate::platform::traits::RecordingBackend;
                    let (actions, _dropped_guard) = {
                        let mut guard = get_linux_recorder();
                        let actions = if let Some(ref recorder) = *guard {
                            recorder.stop().unwrap_or_else(|e| {
                                tracing::warn!("录制停止失败: {}", e);
                                Vec::new()
                            })
                        } else {
                            Vec::new()
                        };
                        *guard = None;
                        (actions, ())
                    };
                    DESKTOP_RECORDING.store(false, Ordering::SeqCst);
                    RECORDING_ACTIVE.store(false, Ordering::SeqCst);
                    *RECORDING_MODE.write().await = String::new();
                    let count = actions.len();
                    let actions_json = serde_json::json!(actions);
                    return Ok(serde_json::json!({
                        "action": "desktop_stop",
                        "mode": "linux_native",
                        "actions": actions_json,
                        "count": count,
                    }));
                }
                #[cfg(not(target_os = "linux"))]
                {
                    let recorder = get_desktop_recorder().await?;
                    let result = recorder.send_action("stop", serde_json::json!({})).await?;
                    DESKTOP_RECORDING.store(false, Ordering::SeqCst);
                    RECORDING_ACTIVE.store(false, Ordering::SeqCst);
                    *RECORDING_MODE.write().await = String::new();
                    let actions = result.get("actions").cloned().unwrap_or_else(|| serde_json::json!([]));
                    let count = result.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
                    return Ok(serde_json::json!({
                        "action": "desktop_stop",
                        "actions": actions,
                        "count": count,
                    }));
                }
            }

            // ─── 开始混合录制（浏览器 + 桌面） ───
            "start" => {
                let headless = config.get("headless").and_then(|v| v.as_bool()).unwrap_or(false);
                let mode = config.get("mode").and_then(|v| v.as_str()).unwrap_or("browser");

                match mode {
                    "desktop" => {
                        // Linux：使用本地 rdev 录制
                        #[cfg(target_os = "linux")]
                        {
                            use crate::platform::traits::RecordingBackend;
                            let started = {
                                let mut guard = get_linux_recorder();
                                *guard = Some(crate::platform::recording::LinuxRecordingBackend::new());
                                match guard.as_ref() {
                                    Some(r) => r.start(),
                                    None => {
                                        tracing::error!("LinuxRecordingBackend 意外为 None");
                                        Err(anyhow::anyhow!("Linux 录制后端未初始化"))
                                    }
                                }
                            };
                            match started {
                                Ok(()) => {
                                    DESKTOP_RECORDING.store(true, Ordering::SeqCst);
                                    RECORDING_ACTIVE.store(true, Ordering::SeqCst);
                                    *RECORDING_MODE.write().await = "desktop".to_string();
                                    Ok(serde_json::json!({"action": "start", "mode": "desktop", "backend": "linux_native"}))
                                }
                                Err(e) => {
                                    warn!("Linux 桌面录制启动失败: {}", e);
                                    let params = serde_json::json!({"headless": headless});
                                    let result = crate::nodes::browser::send_sidecar_action("recording_start", &params).await?;
                                    RECORDING_ACTIVE.store(true, Ordering::SeqCst);
                                    *RECORDING_MODE.write().await = "browser".to_string();
                                    Ok(serde_json::json!({"action": "start", "mode": "browser", "note": "桌面录制失败，回退浏览器", "result": result}))
                                }
                            }
                        }
                        // Windows：使用 desktop_recorder.py sidecar
                        #[cfg(not(target_os = "linux"))]
                        {
                            let recorder = get_desktop_recorder().await?;
                            match recorder.send_action("start", serde_json::json!({})).await {
                                Ok(result) => {
                                    DESKTOP_RECORDING.store(true, Ordering::SeqCst);
                                    RECORDING_ACTIVE.store(true, Ordering::SeqCst);
                                    *RECORDING_MODE.write().await = "desktop".to_string();
                                    Ok(serde_json::json!({"action": "start", "mode": "desktop", "result": result}))
                                }
                                Err(e) => {
                                    warn!("桌面录制启动失败，回退到浏览器录制: {}", e);
                                    let params = serde_json::json!({"headless": headless});
                                    let result = crate::nodes::browser::send_sidecar_action("recording_start", &params).await?;
                                    RECORDING_ACTIVE.store(true, Ordering::SeqCst);
                                    *RECORDING_MODE.write().await = "browser".to_string();
                                    Ok(serde_json::json!({"action": "start", "mode": "browser", "note": "桌面录制不可用，回退到浏览器录制", "result": result}))
                                }
                            }
                        }
                    }
                    _ => {
                        let params = serde_json::json!({"headless": headless});
                        let result = crate::nodes::browser::send_sidecar_action("recording_start", &params).await?;
                        RECORDING_ACTIVE.store(true, Ordering::SeqCst);
                        *RECORDING_MODE.write().await = "browser".to_string();
                        Ok(serde_json::json!({"action": "start", "mode": "browser", "result": result}))
                    }
                }
            }

            // ─── 停止录制（自适应） → 返回操作 + YAML ───
            "stop" => {
                let mode = RECORDING_MODE.read().await.clone();
                let mode = if mode.is_empty() { "browser".to_string() } else { mode };

                let result = match mode.as_str() {
                    "desktop" => {
                        // Linux：本地 rdev 录制
                        #[cfg(target_os = "linux")]
                        {
                            use crate::platform::traits::RecordingBackend;
                            let mut guard = get_linux_recorder();
                            let actions = if let Some(ref recorder) = *guard {
                                recorder.stop().unwrap_or_else(|e| {
                                tracing::warn!("录制停止失败: {}", e);
                                Vec::new()
                            })
                            } else {
                                Vec::new()
                            };
                            *guard = None;
                            DESKTOP_RECORDING.store(false, Ordering::SeqCst);
                            RECORDING_ACTIVE.store(false, Ordering::SeqCst);
                            serde_json::json!({"actions": actions, "count": actions.len()})
                        }
                        // Windows：desktop_recorder.py sidecar
                        #[cfg(not(target_os = "linux"))]
                        {
                            DESKTOP_RECORDING.store(false, Ordering::SeqCst);
                            RECORDING_ACTIVE.store(false, Ordering::SeqCst);
                            let recorder = get_desktop_recorder().await?;
                            recorder.send_action("stop", serde_json::json!({})).await?
                        }
                    }
                    _ => {
                        RECORDING_ACTIVE.store(false, Ordering::SeqCst);
                        crate::nodes::browser::send_sidecar_action("recording_stop", &serde_json::json!({})).await?
                    }
                };

                *RECORDING_MODE.write().await = String::new();

                let actions_value = result.get("actions").cloned().unwrap_or_else(|| serde_json::json!([]));
                let count = result.get("count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                // ─── 自动转换为工作流 YAML ───
                let workflow_name = config.get("workflow_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("录制的自动化流程");

                let (yaml, summary) = if count > 0 {
                    let actions: Vec<RecordedAction> = serde_json::from_value(actions_value.clone())
                        .unwrap_or_default();
                    let source = if mode == "desktop" {
                        recording_converter::RecordingSource::Desktop
                    } else {
                        recording_converter::RecordingSource::Browser
                    };
                    let conversion = recording_converter::convert_actions_to_workflow(
                        &actions, workflow_name, source,
                    );
                    ctx.set_var("__generated_yaml".to_string(), serde_json::json!(conversion.yaml));
                    (conversion.yaml, conversion.step_summary)
                } else {
                    (String::new(), vec![])
                };

                Ok(serde_json::json!({
                    "action": "stop",
                    "mode": mode,
                    "raw_actions": actions_value,
                    "count": count,
                    "yaml": yaml,
                    "step_summary": summary,
                }))
            }

            // ─── 获取录制状态（跨 step_test 调用可用） ───
            "status" => {
                let mode_guard = RECORDING_MODE.read().await;
                let mode = if mode_guard.is_empty() { "none" } else { mode_guard.as_str() };
                let is_active = RECORDING_ACTIVE.load(Ordering::SeqCst);
                let is_desktop = DESKTOP_RECORDING.load(Ordering::SeqCst);

                Ok(serde_json::json!({
                    "action": "status",
                    "recording": is_active,
                    "mode": mode,
                    "desktop_active": is_desktop,
                }))
            }

            // ─── 仅转换已有的操作记录为 YAML ───
            "convert" => {
                let actions: Vec<RecordedAction> = config
                    .get("actions")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();

                let workflow_name = config.get("workflow_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("录制的自动化流程");

                let source_str = config.get("source")
                    .and_then(|v| v.as_str())
                    .unwrap_or("browser");

                let source = match source_str {
                    "desktop" => RecordingSource::Desktop,
                    "mixed" => RecordingSource::Mixed,
                    _ => RecordingSource::Browser,
                };

                let conversion = recording_converter::convert_actions_to_workflow(
                    &actions, workflow_name, source,
                );

                Ok(serde_json::json!({
                    "yaml": conversion.yaml,
                    "step_count": conversion.step_count,
                    "action_count": conversion.action_count,
                    "merged_count": conversion.merged_count,
                    "step_summary": conversion.step_summary,
                }))
            }

            _ => Err(anyhow!("未知录制操作: {}（支持 start/stop/status/convert/browser_start/browser_stop/desktop_start/desktop_stop）", action)),
        }
    }
}
