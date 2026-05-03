// nodes/browser.rs — 浏览器节点（Python sidecar + Playwright）
// v2: 增加健康检查、超时保护、自动重启
use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use std::time::Duration;
use anyhow::{Result, anyhow};
use tokio::sync::{Mutex, RwLock};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{info, warn};

/// Windows: 禁止子进程弹出 cmd 窗口
#[cfg(target_os = "windows")]
fn hide_console(cmd: &mut tokio::process::Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
}
#[cfg(not(target_os = "windows"))]
fn hide_console(_cmd: &mut tokio::process::Command) {}

/// 浏览器 sidecar 进程管理器
pub struct BrowserSidecar {
    #[allow(dead_code)] // 保持子进程存活，Drop 时自动清理
    child: Mutex<Option<Child>>,
    stdin: Arc<Mutex<ChildStdin>>,
    stdout: Arc<Mutex<BufReader<ChildStdout>>>,
    /// sidecar 健康状态
    healthy: std::sync::atomic::AtomicBool,
}

impl BrowserSidecar {
    /// 启动 Python sidecar 进程
    pub async fn start() -> Result<Self> {
        // 查找 Python（优先内置，其次系统）
        let python = find_python()?;

        // 查找 sidecar 脚本
        let script_path = find_sidecar_script()?;

        // 设置 Playwright 浏览器路径（指向打包的 playwright-browsers）
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));
        let mut cmd = tokio::process::Command::new(&python);
        hide_console(&mut cmd);
        cmd.arg(&script_path);
        if let Some(ref dir) = exe_dir {
            let browsers_path = dir.join("playwright-browsers");
            if browsers_path.exists() {
                cmd.env("PLAYWRIGHT_BROWSERS_PATH", &browsers_path);
                info!("Playwright 浏览器路径: {:?}", browsers_path);
            }
        }

        let mut child = cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("启动 Python sidecar 失败: {}", e))?;

        let stdin = child.stdin.take()
            .ok_or_else(|| anyhow!("无法获取 sidecar stdin"))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| anyhow!("无法获取 sidecar stdout"))?;

        let sidecar = BrowserSidecar {
            child: Mutex::new(Some(child)),
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: Arc::new(Mutex::new(BufReader::new(stdout))),
            healthy: std::sync::atomic::AtomicBool::new(true),
        };

        // 等待 ready 信号
        let ready = sidecar.read_response().await?;
        if ready.get("type").and_then(|v| v.as_str()) != Some("ready") {
            sidecar.healthy.store(false, std::sync::atomic::Ordering::SeqCst);
            return Err(anyhow!("Sidecar 启动异常: {:?}", ready));
        }

        Ok(sidecar)
    }

    /// 发送请求并等待响应（30 秒超时保护）
    pub async fn send_action(&self, action: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let result = tokio::time::timeout(
            Duration::from_secs(30),
            self.send_action_inner(action, params),
        ).await;

        match result {
            Ok(Ok(val)) => Ok(val),
            Ok(Err(e)) => {
                self.healthy.store(false, std::sync::atomic::Ordering::SeqCst);
                Err(e)
            }
            Err(_elapsed) => {
                self.healthy.store(false, std::sync::atomic::Ordering::SeqCst);
                Err(anyhow!("浏览器操作超时（30 秒）"))
            }
        }
    }

    /// 内部：无超时的发送逻辑
    async fn send_action_inner(&self, action: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let id = uuid::Uuid::new_v4().to_string();
        let request = serde_json::json!({
            "id": id,
            "action": action,
            "params": params,
        });

        // 写入请求
        {
            let mut stdin = self.stdin.lock().await;
            let line = serde_json::to_string(&request)?;
            stdin.write_all(line.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
        }

        // 读取响应
        let response = self.read_response().await?;

        let success = response.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        if success {
            Ok(response.get("data").cloned().unwrap_or(serde_json::Value::Null))
        } else {
            let error = response.get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("未知错误");
            Err(anyhow!("浏览器操作失败: {}", error))
        }
    }

    /// 从 stdout 读取一行 JSON 响应
    async fn read_response(&self) -> Result<serde_json::Value> {
        let mut stdout = self.stdout.lock().await;
        let mut line = String::new();
        let n = stdout.read_line(&mut line).await?;
        if n == 0 {
            self.healthy.store(false, std::sync::atomic::Ordering::SeqCst);
            return Err(anyhow!("Sidecar 进程已退出"));
        }
        let line = line.trim();
        if line.is_empty() {
            return Err(anyhow!("收到空响应"));
        }
        serde_json::from_str(line).map_err(|e| anyhow!("Sidecar 响应解析失败: {}", e))
    }

    /// 关闭 sidecar
    pub async fn shutdown(&self) -> Result<()> {
        self.healthy.store(false, std::sync::atomic::Ordering::SeqCst);
        let _ = self.send_action_inner("shutdown", serde_json::json!({})).await;
        // 等待子进程退出（30s 超时，超时则强制 kill）
        let mut guard = self.child.lock().await;
        if let Some(ref mut child) = *guard {
            match tokio::time::timeout(std::time::Duration::from_secs(30), child.wait()).await {
                Ok(_) => {}
                Err(_) => {
                    tracing::warn!("Sidecar shutdown 超时，强制 kill");
                    let _ = child.kill().await;
                }
            }
        }
        Ok(())
    }

    /// 检查 sidecar 是否健康
    pub fn is_healthy(&self) -> bool {
        self.healthy.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// 查找 sidecar 脚本路径
fn find_sidecar_script() -> Result<std::path::PathBuf> {
    // 1. 相对于可执行文件
    if let Ok(exe_dir) = std::env::current_exe() {
        if let Some(dir) = exe_dir.parent() {
            let p = dir.join("sidecars").join("playwright_driver.py");
            if p.exists() { return Ok(p); }
        }
    }

    // 2. 相对于当前工作目录（开发模式）
    let cwd_script = std::path::Path::new("src-tauri/sidecars/playwright_driver.py");
    if cwd_script.exists() { return Ok(cwd_script.to_path_buf()); }

    // 3. 相对于 CARGO_MANIFEST_DIR
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = std::path::PathBuf::from(manifest).join("sidecars").join("playwright_driver.py");
        if p.exists() { return Ok(p); }
    }

    Err(anyhow!("找不到 playwright_driver.py。请确保 sidecars/ 目录存在"))
}

/// 查找内置 Python（打包在 exe 旁的 embed/ 目录）
fn find_bundled_python() -> Option<std::path::PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let python = dir.join("embed").join("python.exe");
            if python.exists() {
                return Some(python);
            }
        }
    }
    None
}

/// 查找 Python（优先内置 → 用户配置 → 系统）
///
/// 返回 (path, source_label) 供错误报告使用
fn find_python() -> Result<std::path::PathBuf> {
    let mut tried: Vec<String> = Vec::new();

    // 1. 内置 Python
    if let Some(p) = find_bundled_python() {
        return Ok(p);
    }
    tried.push("内置 (embed/python.exe)".into());

    // 2. 用户在设置中配置的 Python 路径
    if let Ok(config) = crate::data::config::AppConfig::load_default() {
        if let Some(ref path) = config.python_path {
            if !path.is_empty() {
                tried.push(format!("用户配置: {}", path));
                let p = std::path::PathBuf::from(path);
                if p.exists() { return Ok(p); }
            }
        }
    }

    // 3. 系统 PATH 中的 Python
    let system_python = which::which("python3")
        .or_else(|_| which::which("python"));
    if let Ok(ref p) = system_python {
        tried.push(format!("系统 PATH: {:?}", p));
    } else {
        tried.push("系统 PATH: 未找到 python3/python".into());
    }

    // 4. Windows: 扫描常见安装位置 — 始终执行，不依赖 PATH 结果
    //    取版本最高的，优于 PATH 找到的可能过期/残缺的 python
    #[cfg(target_os = "windows")]
    {
        let windows_scan = find_windows_python();
        match (&system_python, &windows_scan) {
            (_, Ok(ref wp)) => {
                // Windows 扫描按版本降序，总是取它（最新）
                info!("Windows 自动检测到 Python: {:?}", wp);
                return Ok(wp.clone());
            }
            (Ok(_), Err(_)) => {
                tried.push("Windows 目录扫描: 未找到".into());
                // fall through to use PATH result
            }
            (Err(_), Err(_)) => {
                tried.push("Windows 目录扫描: 未找到".into());
                // will error below
            }
        }
    }

    // 5. Linux/macOS: 直接返回 PATH 结果
    system_python.map_err(|_| {
        let tried_list = tried.join("\n  • ");
        anyhow!(
            "未找到 Python 3.8+。浏览器节点需要 Python\n\
             已尝试:\n  • {tried_list}\n\
             下载地址: https://www.python.org/downloads/"
        )
    })
}

/// Windows: 扫描常见 Python 安装目录
#[cfg(target_os = "windows")]
fn find_windows_python() -> Result<std::path::PathBuf> {
    use std::path::PathBuf;

    let candidates: Vec<PathBuf> = vec![
        // AppData 用户安装
        std::env::var("LOCALAPPDATA")
            .map(|d| PathBuf::from(d).join("Programs").join("Python"))
            .unwrap_or_default(),
        // Program Files
        std::env::var("ProgramFiles")
            .map(|d| PathBuf::from(d).join("Python"))
            .unwrap_or_default(),
        // C 盘根目录
        PathBuf::from("C:\\Python"),
    ];

    for base in &candidates {
        if !base.exists() { continue }
        if let Ok(entries) = std::fs::read_dir(base) {
            // Find the newest Python version
            let mut pythons: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().starts_with("Python3"))
                .map(|e| e.path().join("python.exe"))
                .filter(|p| p.exists())
                .collect();
            // Sort descending by version (Python313 > Python312 > ...)
            pythons.sort_by(|a, b| b.cmp(a));
            if let Some(p) = pythons.into_iter().next() {
                info!("Windows 自动检测到 Python: {:?}", p);
                return Ok(p);
            }
        }
    }

    Err(anyhow!("Windows Python 扫描未找到"))
}

// ─── 全局 sidecar 实例（RwLock<Option<Arc<...>>> 支持健康检查和自动重启） ───

static SIDECAR: RwLock<Option<Arc<BrowserSidecar>>> = RwLock::const_new(None);

async fn is_sidecar_healthy(sidecar: &Arc<BrowserSidecar>) -> bool {
    if !sidecar.is_healthy() {
        return false;
    }
    // ping 检查：3 秒超时
    match tokio::time::timeout(
        Duration::from_secs(3),
        sidecar.send_action_inner("ping", serde_json::json!({})),
    ).await {
        Ok(Ok(_)) => true,
        _ => {
            warn!("Sidecar ping 失败，标记为不可用");
            false
        }
    }
}

/// 查找内置 wheels 目录（pip 离线安装包）
fn find_bundled_wheels() -> Option<std::path::PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let wheels = dir.join("wheels");
            if wheels.exists() && wheels.read_dir().ok()
                .map(|mut d| d.any(|e| e.ok().map(|f| f.path().extension()
                    .map(|ext| ext == "whl").unwrap_or(false)).unwrap_or(false)))
                .unwrap_or(false)
            {
                return Some(wheels);
            }
        }
    }
    None
}

async fn preflight_check() -> Result<()> {
    let python = find_python()?;
    info!("使用 Python: {:?}", python);

    // 1. 检查 Python 版本
    let mut cmd = tokio::process::Command::new(&python);
        hide_console(&mut cmd);
        let output = cmd.arg("--version")
        .output()
        .await
        .map_err(|e| anyhow!("执行 Python 失败: {}", e))?;

    let version_str = String::from_utf8_lossy(&output.stdout);
    let version_str = version_str.trim();
    if let Some(ver) = version_str.strip_prefix("Python ") {
        let major: u32 = ver.split('.').next().unwrap_or("0").parse().unwrap_or(0);
        if major < 3 {
            return Err(anyhow!("Python 版本过低: {}。需要 Python 3.8+", version_str));
        }
        info!("Python 版本: {}", version_str);
    }

    // 2. 检查 Playwright — 优先离线安装（内置 wheels），其次在线安装
    let mut cmd = tokio::process::Command::new(&python);
        hide_console(&mut cmd);
        let output = cmd.args(["-c", "import playwright; print('ok')"])
        .output()
        .await
        .map_err(|e| anyhow!("检查 Playwright 失败: {}", e))?;

    if !output.status.success() {
        // 优先尝试离线安装（内置 wheels）
        if let Some(wheels_dir) = find_bundled_wheels() {
            info!("使用内置 wheels 离线安装 Playwright: {:?}", wheels_dir);
            let mut cmd = tokio::process::Command::new(&python);
                hide_console(&mut cmd);
                let install = cmd.args([
                    "-m", "pip", "install", "playwright", "-q",
                    "--no-index",
                    "--find-links", &wheels_dir.to_string_lossy(),
                ])
                .output()
                .await
                .map_err(|e| anyhow!("离线安装 Playwright 失败: {}", e))?;

            if install.status.success() {
                info!("Playwright 离线安装完成 ✓");
            } else {
                let stderr = String::from_utf8_lossy(&install.stderr);
                warn!("离线安装失败，回退在线安装: {}", stderr);
                // Fallback to online install
                return install_playwright_online(&python).await;
            }
        } else {
            // No bundled wheels → online install
            install_playwright_online(&python).await?;
        }
    }

    // 3. 检查浏览器 — 优先内置（Full 包自带），其次系统浏览器
    let has_bundled_chromium = {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));
        if let Some(ref dir) = exe_dir {
            let browsers_path = dir.join("playwright-browsers");
            if browsers_path.exists() {
                info!("检测到内置 playwright-browsers: {:?}", browsers_path);
                true
            } else { false }
        } else { false }
    };

    if has_bundled_chromium {
        info!("使用内置 Chromium ✓");
    } else {
        // 检查系统 Edge/Chrome
        let has_system_browser = {
            #[cfg(target_os = "windows")]
            {
                let pf_x86 = std::env::var("PROGRAMFILES(X86)").unwrap_or_default();
                let pf = std::env::var("PROGRAMFILES").unwrap_or_default();
                let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
                let edge1 = std::path::PathBuf::from(&pf_x86).join("Microsoft/Edge/Application/msedge.exe");
                let edge2 = std::path::PathBuf::from(&pf).join("Microsoft/Edge/Application/msedge.exe");
                let chrome1 = std::path::PathBuf::from(&pf).join("Google/Chrome/Application/chrome.exe");
                let chrome2 = std::path::PathBuf::from(&pf_x86).join("Google/Chrome/Application/chrome.exe");
                let chrome3 = std::path::PathBuf::from(&local).join("Google/Chrome/Application/chrome.exe");
                let found = edge1.exists() || edge2.exists() || chrome1.exists() || chrome2.exists() || chrome3.exists();
                if found {
                    let which = if edge1.exists() || edge2.exists() { "Edge" } else { "Chrome" };
                    info!("检测到系统浏览器: {}", which);
                }
                found
            }
            #[cfg(not(target_os = "windows"))]
            {
                let found = which::which("microsoft-edge").is_ok()
                    || which::which("google-chrome-stable").is_ok()
                    || which::which("google-chrome").is_ok()
                    || which::which("chromium-browser").is_ok()
                    || which::which("chromium").is_ok();
                if found { info!("检测到系统浏览器 (Linux) ✓"); }
                found
            }
        };

        if has_system_browser {
            info!("检测到系统浏览器 ✓");
        } else {
            // 检查 ms-playwright 缓存
            let mut cmd = tokio::process::Command::new(&python);
                hide_console(&mut cmd);
                let output = cmd.args(["-c", r#"
import os, sys
home = os.environ.get('PLAYWRIGHT_BROWSERS_PATH',
    os.path.join(os.environ.get('LOCALAPPDATA', ''), 'ms-playwright') if sys.platform == 'win32'
    else os.path.join(os.path.expanduser('~'), '.cache', 'ms-playwright'))
chromium_dir = [d for d in os.listdir(home) if d.startswith('chromium-')] if os.path.exists(home) else []
print('ok' if chromium_dir else 'missing')
"#])
                .output()
                .await;

            let has_cache = match output {
                Ok(o) => String::from_utf8_lossy(&o.stdout).trim() == "ok",
                Err(_) => false,
            };

            if !has_cache {
                return Err(anyhow!(
                    "未找到 Chromium 浏览器。\n\
                     Full 安装包应内置 playwright-browsers/，请检查安装是否完整。\n\
                     或安装 Edge/Chrome 浏览器。"
                ));
            }
        }
    }

    info!("浏览器节点预检通过 ✓");
    Ok(())
}

/// 在线安装 Playwright（PyPI → 清华镜像 fallback）
async fn install_playwright_online(python: &std::path::Path) -> Result<()> {
    info!("在线安装 Playwright...");

    // PyPI 直连
    let mut cmd = tokio::process::Command::new(python);
        hide_console(&mut cmd);
        let install = cmd.args(["-m", "pip", "install", "playwright", "-q"])
        .output()
        .await
        .map_err(|e| anyhow!("执行 pip 失败: {}", e))?;

    if !install.status.success() {
        // 清华镜像重试
        info!("PyPI 直连失败，尝试清华镜像...");
        let mut cmd = tokio::process::Command::new(python);
            hide_console(&mut cmd);
            let install_mirror = cmd.args(["-m", "pip", "install", "playwright", "-q",
                   "-i", "https://pypi.tuna.tsinghua.edu.cn/simple",
                   "--trusted-host", "pypi.tuna.tsinghua.edu.cn"])
            .output()
            .await
            .map_err(|e| anyhow!("执行 pip (镜像) 失败: {}", e))?;

        if !install_mirror.status.success() {
            let stderr = String::from_utf8_lossy(&install_mirror.stderr);
            return Err(anyhow!(
                "自动安装 Playwright 失败。\n\
                 请手动执行: pip install playwright\n\n\
                 错误: {}",
                stderr
            ));
        }
    }
    info!("Playwright 在线安装完成 ✓");
    Ok(())
}

/// 获取或启动 sidecar，支持健康检查和自动重启
async fn get_or_start_sidecar() -> Result<Arc<BrowserSidecar>> {
    // 先检查现有实例是否健康
    {
        let guard = SIDECAR.read().await;
        if let Some(ref sidecar) = *guard {
            if is_sidecar_healthy(sidecar).await {
                return Ok(Arc::clone(sidecar));
            }
            warn!("Sidecar 无响应，准备重新启动...");
        }
        // guard drops here, releasing read lock
    }

    // 需要启动新的 sidecar — 先清理旧进程
    let mut guard = SIDECAR.write().await;
    if let Some(ref old) = *guard {
        warn!("清理旧 sidecar 进程...");
        let mut child_guard = old.child.lock().await;
        if let Some(ref mut old_child) = *child_guard {
            let _ = old_child.kill().await;
            let _ = old_child.wait().await;
        }
    }
    *guard = None; // 确保旧实例被丢弃

    // 双重检查：可能在等待写锁期间已有其他任务启动了
    // (已在上方清理，此处重新获取健康 sidecar)

    // 执行 preflight 检查并启动新 sidecar
    preflight_check().await?;
    let sidecar = BrowserSidecar::start().await?;
    let arc = Arc::new(sidecar);
    *guard = Some(Arc::clone(&arc));
    info!("Sidecar 启动成功 ✓");
    Ok(arc)
}

// ─── 浏览器节点实现 ───

/// 公共接口：发送 action 到 sidecar（供 web_scrape 等节点调用）
pub async fn send_sidecar_action(action: &str, params: &serde_json::Value) -> Result<serde_json::Value> {
    let sidecar = get_or_start_sidecar().await?;

    // 自动 launch
    if action != "launch" && action != "close" && action != "shutdown" && action != "ping" {
        // 录制/拾取需要用户可见浏览器，非 headless
        let headless = action != "recording_start" && action != "pick";
        let launch_params = serde_json::json!({"headless": headless});
        let _ = sidecar.send_action("launch", launch_params).await;
    }

    sidecar.send_action(action, params.clone()).await
}

#[derive(Default)]
pub struct BrowserNode;

#[async_trait]
impl NodeExecutor for BrowserNode {
    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let action = config.get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("navigate");

        let mut params = config.get("params")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        // navigate 操作默认 wait_until 为 "load"
        if action == "navigate" {
            if let Some(obj) = params.as_object_mut() {
                if !obj.contains_key("wait_until") {
                    obj.insert("wait_until".to_string(), serde_json::json!("load"));
                }
            }
        }

        let sidecar = get_or_start_sidecar().await?;

        // 对于需要 launch 的操作，自动先 launch
        if action != "launch" && action != "close" && action != "ping" {
            let headless = action != "recording_start" && action != "pick";
            let mut launch_params = config.get("launch")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({"headless": headless}));

            // 从上下文读取浏览器通道设置（用户配置 > step config > 自动检测）
            let channel = step.config.get("channel").and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .or_else(|| {
                    if ctx.browser_channel != "auto" && !ctx.browser_channel.is_empty() {
                        Some(ctx.browser_channel.clone())
                    } else {
                        None
                    }
                });

            if let Some(ch) = channel {
                if let Some(obj) = launch_params.as_object_mut() {
                    obj.insert("channel".to_string(), serde_json::json!(ch));
                }
            }

            let _ = sidecar.send_action("launch", launch_params).await;
        }

        let result = sidecar.send_action(action, params.clone()).await;

        match result {
            Ok(r) => Ok(serde_json::json!({
                "action": action,
                "result": r,
            })),
            Err(e) => {
                // 错误截图：尝试捕获浏览器当前画面
                let screenshot_info = try_error_screenshot(step).await;
                let mut err_msg = format!("{}", e);
                if let Some(path) = screenshot_info {
                    err_msg = format!("{}\n📸 错误截图已保存: {}", path, err_msg);
                }
                Err(anyhow!(err_msg))
            }
        }
    }
}

// ═══════════════════════════════════════════════
// v3: Browser sub-nodes — 每个浏览器操作独立 executor
// ═══════════════════════════════════════════════

/// 辅助：获取 sidecar，自动 launch
async fn ensure_sidecar(action: &str, config: &serde_json::Value, ctx: &ExecutionContext) -> Result<Arc<BrowserSidecar>> {
    let sidecar = get_or_start_sidecar().await?;
    if action != "launch" && action != "close" && action != "ping" {
        let headless = action != "recording_start" && action != "pick";
        let mut launch_params = config.get("launch").cloned()
            .unwrap_or_else(|| serde_json::json!({"headless": headless}));
        let channel = config.get("channel").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
            .or_else(|| if ctx.browser_channel != "auto" && !ctx.browser_channel.is_empty() { Some(ctx.browser_channel.as_str()) } else { None });
        if let Some(ch) = channel {
            if let Some(obj) = launch_params.as_object_mut() { obj.insert("channel".to_string(), serde_json::json!(ch)); }
        }
        let _ = sidecar.send_action("launch", launch_params).await;
    }
    Ok(sidecar)
}

macro_rules! browser_sub_node {
    ($name:ident, $action:literal, $label:literal) => {
        #[derive(Default)]
        pub struct $name;

        #[async_trait]
        impl NodeExecutor for $name {
            async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
                let config = &step.config;
                let sidecar = ensure_sidecar($action, config, ctx).await?;
                let params = config.get("params").cloned().unwrap_or_else(|| serde_json::json!({}));
                let result = sidecar.send_action($action, params).await;
                match result {
                    Ok(r) => Ok(serde_json::json!({ "result": r })),
                    Err(e) => {
                        let screenshot_info = try_error_screenshot(step).await;
                        let mut err_msg = format!("{}", e);
                        if let Some(path) = screenshot_info { err_msg = format!("{}\n📸 错误截图已保存: {}", path, err_msg); }
                        Err(anyhow!(err_msg))
                    }
                }
            }
        }
    };
}

browser_sub_node!(BrowserNavigateNode,   "navigate",   "浏览器导航");
browser_sub_node!(BrowserClickNode,      "click",      "浏览器点击");
browser_sub_node!(BrowserFillNode,       "fill",       "浏览器填写");
browser_sub_node!(BrowserExtractNode,    "extract",    "浏览器提取");
browser_sub_node!(BrowserScreenshotNode, "screenshot", "浏览器截图");
browser_sub_node!(BrowserEvaluateNode,   "evaluate",   "浏览器执行JS");
browser_sub_node!(BrowserScrollNode,     "scroll",     "浏览器滚动");
browser_sub_node!(BrowserWaitNode,       "wait",       "浏览器等待");
browser_sub_node!(BrowserPdfNode,        "pdf",        "浏览器PDF");

/// 错误截图：保存浏览器当前画面到 screenshots/ 目录
/// 直接从 sidecar 实例截图，避免 send_sidecar_action 的 auto-launch 逻辑
async fn try_error_screenshot(step: &Step) -> Option<String> {
    let sidecar = SIDECAR.read().await;
    let sidecar = sidecar.as_ref()?;
    if !sidecar.is_healthy() {
        return None;
    }

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let dir = std::env::current_exe().ok()?.parent()?.join("screenshots");
    std::fs::create_dir_all(&dir).ok()?;
    let filename = format!("error_{}_{}.png", step.id, timestamp);
    let path = dir.join(&filename);

    let params = serde_json::json!({"path": path.to_string_lossy().to_string()});
    let _ = sidecar.send_action("screenshot", params).await;

    if path.exists() {
        Some(path.to_string_lossy().to_string())
    } else {
        None
    }
}
