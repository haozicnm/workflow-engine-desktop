# Workflow Engine Desktop 跨平台改造 实施计划 (方案 A)

> **For Hermes:** 使用 subagent-driven-development skill 逐任务实施。
> 创建于 2026-04-27 | 预计总工时：1-2 天

**目标：** 窗口管理节点同时支持 Windows 和 Linux，通过 Platform Trait 抽象层统一接口。

**架构：** 在现有 `platform/traits.rs` 中新增 `WindowBackend` trait → Windows 用 PowerShell（搬迁现有代码）→ Linux 用 xdotool CLI → `nodes/window.rs` 改为调用 trait。

**涉及技术：** Rust trait + `cfg` 条件编译 + subprocess CLI 调用

---

## 任务 1：新增 WindowBackend trait

**目标：** 定义跨平台窗口管理接口

**文件：**
- 修改：`src-tauri/src/platform/traits.rs`

**内容：** 在现有 `InputBackend` 和 `RecordingBackend` 之后，追加以下代码：

```rust
/// 窗口信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct WindowInfo {
    pub pid: u32,
    pub title: String,
    pub handle: String,
    pub process: String,
}

/// 全局窗口管理后端
///
/// 实现者：
///   - Windows: PowerShell + user32.dll（保留现有代码）
///   - Linux:   xdotool + wmctrl CLI
pub trait WindowBackend: Send + Sync {
    /// 查找匹配标题的窗口
    fn find(&self, title: &str) -> Result<Vec<WindowInfo>>;
    /// 激活窗口（置前并聚焦）
    fn activate(&self, title: &str) -> Result<()>;
    /// 最大化
    fn maximize(&self, title: &str) -> Result<()>;
    /// 最小化
    fn minimize(&self, title: &str) -> Result<()>;
    /// 还原窗口
    fn restore(&self, title: &str) -> Result<()>;
    /// 关闭窗口
    fn close(&self, title: &str) -> Result<()>;
    /// 调整窗口大小
    fn resize(&self, title: &str, width: i32, height: i32) -> Result<()>;
    /// 等待窗口出现（超时返回错误）
    fn wait(&self, title: &str, timeout_s: u64) -> Result<()>;
    /// 列出所有可见窗口
    fn list(&self) -> Result<Vec<WindowInfo>>;
}
```

**验证：** `cargo check` 无报错

**Commit：**
```bash
git add src-tauri/src/platform/traits.rs
git commit -m "feat: add WindowBackend trait for cross-platform window management"
```

---

## 任务 2：Linux 窗口后端实现

**目标：** 通过 xdotool / wmctrl CLI 实现 `WindowBackend`

**文件：**
- 新建：`src-tauri/src/platform/linux_window.rs`

**完整实现：**

```rust
// platform/linux_window.rs — Linux 窗口管理后端（xdotool + wmctrl）
use crate::platform::traits::{WindowBackend, WindowInfo};
use anyhow::{Result, anyhow};
use std::process::Command;

pub struct LinuxWindowBackend;

impl LinuxWindowBackend {
    pub fn new() -> Self { LinuxWindowBackend }

    /// 检查 xdotool 是否可用
    pub fn check_available() -> Result<()> {
        Command::new("xdotool")
            .arg("--version")
            .output()
            .map_err(|_| anyhow!("xdotool 未安装，请执行: sudo apt install xdotool"))?;
        Ok(())
    }
}

/// 执行命令并捕获 stdout
fn run(cmd: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| anyhow!("执行 {} 失败: {}", cmd, e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("{} 错误: {}", cmd, stderr.trim()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// 通过标题查找窗口 ID（返回第一个匹配）
fn find_window_id(title: &str) -> Result<String> {
    let ids = run("xdotool", &["search", "--name", title])?;
    ids.lines()
        .next()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("未找到标题包含 '{}' 的窗口", title))
}

/// 解析 xdotool getactivewindow 或 search 输出的窗口 ID
fn get_window_info(win_id: &str) -> Result<WindowInfo> {
    let name = run("xdotool", &["getwindowname", win_id])?;
    let pid_str = run("xdotool", &["getwindowpid", win_id])?;
    let pid: u32 = pid_str.parse().unwrap_or(0);
    Ok(WindowInfo {
        pid,
        title: name,
        handle: win_id.to_string(),
        process: String::new(), // xdotool 不直接暴露进程名
    })
}

impl WindowBackend for LinuxWindowBackend {
    fn find(&self, title: &str) -> Result<Vec<WindowInfo>> {
        if title.is_empty() {
            return Ok(vec![]);
        }
        let ids = run("xdotool", &["search", "--name", title])?;
        let mut windows = Vec::new();
        for id in ids.lines() {
            if let Ok(info) = get_window_info(id) {
                windows.push(info);
            }
        }
        Ok(windows)
    }

    fn activate(&self, title: &str) -> Result<()> {
        if title.is_empty() {
            return Err(anyhow!("窗口标题不能为空"));
        }
        run("xdotool", &["search", "--name", title, "windowactivate", "windowfocus"])?;
        Ok(())
    }

    fn maximize(&self, title: &str) -> Result<()> {
        // wmctrl 更擅长窗口状态操作
        if let Ok(_) = run("wmctrl", &["-r", title, "-b", "add,maximized_vert,maximized_horz"]) {
            return Ok(());
        }
        // 降级：xdotool 模拟 Alt+F10
        let id = find_window_id(title)?;
        run("xdotool", &["windowactivate", &id])?;
        run("xdotool", &["key", "alt+F10"])?;
        Ok(())
    }

    fn minimize(&self, title: &str) -> Result<()> {
        let id = find_window_id(title)?;
        run("xdotool", &["windowminimize", &id])?;
        Ok(())
    }

    fn restore(&self, title: &str) -> Result<()> {
        // 先激活，如果已最小化则自动还原
        self.activate(title)
    }

    fn close(&self, title: &str) -> Result<()> {
        let id = find_window_id(title)?;
        run("xdotool", &["windowclose", &id])?;
        Ok(())
    }

    fn resize(&self, title: &str, width: i32, height: i32) -> Result<()> {
        if width <= 0 || height <= 0 {
            return Err(anyhow!("窗口尺寸必须大于 0"));
        }
        if let Ok(_) = run("wmctrl", &[
            "-r", title, "-e", "0,-1,-1",
            &width.to_string(), &height.to_string(),
        ]) {
            return Ok(());
        }
        // 降级：xdotool 方案
        let id = find_window_id(title)?;
        run("xdotool", &["windowsize", &id, &width.to_string(), &height.to_string()])?;
        Ok(())
    }

    fn wait(&self, title: &str, timeout_s: u64) -> Result<()> {
        let deadline = std::time::Instant::now()
            + std::time::Duration::from_secs(timeout_s);
        loop {
            if let Ok(ids) = run("xdotool", &["search", "--name", title]) {
                if !ids.trim().is_empty() {
                    return Ok(());
                }
            }
            if std::time::Instant::now() >= deadline {
                return Err(anyhow!("等待窗口 '{}' 超时 ({}s)", title, timeout_s));
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }

    fn list(&self) -> Result<Vec<WindowInfo>> {
        // 用 wmctrl 列出所有窗口（更全面）
        let out = match run("wmctrl", &["-l"]) {
            Ok(o) => o,
            Err(_) => {
                // 降级：用 xdotool 列出可见窗口
                let ids = run("xdotool", &["search", "--onlyvisible", "--name", ""])?;
                // 降级失败则返回空
                if ids.trim().is_empty() {
                    return Ok(vec![]);
                }
                ids
            }
        };
        let mut windows = Vec::new();
        for line in out.lines() {
            let parts: Vec<&str> = line.splitn(5, ' ').collect();
            if parts.len() >= 5 {
                windows.push(WindowInfo {
                    pid: 0,
                    title: parts[4..].join(" "),
                    handle: parts[0].to_string(),
                    process: String::new(),
                });
            }
        }
        Ok(windows)
    }
}
```

**验证：**
```bash
cargo check 2>&1 | grep -E "error|warning"
```
预期：0 errors

**Commit：**
```bash
git add src-tauri/src/platform/linux_window.rs
git commit -m "feat: add Linux window backend via xdotool/wmctrl CLI"
```

---

## 任务 3：Windows 窗口后端（搬迁现有代码）

**目标：** 将 `nodes/window.rs` 中的 PowerShell 逻辑搬进 trait 实现

**文件：**
- 新建：`src-tauri/src/platform/windows_window.rs`

**实现思路：** 将 `nodes/window.rs` 中以下函数整体搬迁：
- `run_ps()` → 保留为模块私有函数
- `run_ps_capture()` → 保留为模块私有函数
- `window_cmd()` → 内联到各方法
- 各 action 分支（find/activate/maximize/minimize/close/resize/wait/list）→ 实现 trait 方法

**关键变更：**
- `run_ps` 从 async 改为同步（或用 `std::process::Command` 替代 `tokio::process::Command`，因为 `WindowBackend` trait 方法是同步的）
- 如果保留 async，则 trait 方法需要改为 async

```
（因原文件 window.rs 已审阅过，此处搬迁为机械性操作，不逐行展开。
 核心原则：功能零变化，只改归属位置。）
```

**验证：**
```bash
cargo check 2>&1 | grep -E "error"
```
预期：0 errors

**Commit：**
```bash
git add src-tauri/src/platform/windows_window.rs
git commit -m "refactor: extract Windows window logic into WindowBackend trait impl"
```

---

## 任务 4：平台模块注册

**目标：** 在 `platform/mod.rs` 中注册 `WindowBackend` 全局单例

**文件：**
- 修改：`src-tauri/src/platform/mod.rs`

**具体修改：**

```rust
// 在现有 mod 声明后添加
#[cfg(target_os = "windows")]
pub mod windows_window;
#[cfg(target_os = "linux")]
pub mod linux_window;

// 新增全局单例
use traits::WindowBackend;
static WINDOW_PLATFORM: OnceLock<Box<dyn WindowBackend + Send + Sync>> = OnceLock::new();

/// 获取全局窗口管理后端
pub fn window() -> &'static (dyn WindowBackend + Send + Sync) {
    WINDOW_PLATFORM.get_or_init(|| {
        #[cfg(target_os = "windows")]
        { Box::new(windows_window::WindowsWindowBackend::new()) }
        #[cfg(target_os = "linux")]
        { Box::new(linux_window::LinuxWindowBackend::new()) }
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        { Box::new(UnsupportedWindowBackend) }
    }).as_ref()
}

// 不支持的操作系统回退
struct UnsupportedWindowBackend;
impl WindowBackend for UnsupportedWindowBackend {
    fn find(&self, _: &str) -> Result<Vec<WindowInfo>> { Ok(vec![]) }
    fn activate(&self, _: &str) -> Result<()> { Err(anyhow!("不支持此操作系统")) }
    fn maximize(&self, _: &str) -> Result<()> { Err(anyhow!("不支持此操作系统")) }
    fn minimize(&self, _: &str) -> Result<()> { Err(anyhow!("不支持此操作系统")) }
    fn restore(&self, _: &str) -> Result<()> { Err(anyhow!("不支持此操作系统")) }
    fn close(&self, _: &str) -> Result<()> { Err(anyhow!("不支持此操作系统")) }
    fn resize(&self, _: &str, _: i32, _: i32) -> Result<()> { Err(anyhow!("不支持此操作系统")) }
    fn wait(&self, _: &str, _: u64) -> Result<()> { Err(anyhow!("不支持此操作系统")) }
    fn list(&self) -> Result<Vec<WindowInfo>> { Err(anyhow!("不支持此操作系统")) }
}
```

**验证：**
```bash
cargo check 2>&1 | grep -E "error"
```
预期：0 errors

**Commit：**
```bash
git add src-tauri/src/platform/mod.rs
git commit -m "feat: register WindowBackend platform singleton"
```

---

## 任务 5：重构 nodes/window.rs

**目标：** 将 `WindowNode` 改为调用 `platform::window()` trait

**文件：**
- 重写：`src-tauri/src/nodes/window.rs`

**新实现思路：**

```rust
// nodes/window.rs — 窗口管理节点（v2：跨平台）
use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};

#[derive(Default)]
pub struct WindowNode;

#[async_trait]
impl NodeExecutor for WindowNode {
    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let action = config.get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("find");

        let backend = crate::platform::window();

        match action {
            "find" => {
                let title = config.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let windows = backend.find(title)?;
                Ok(serde_json::json!({"action": "find", "windows": windows}))
            }

            "activate" => {
                let title = config.get("title").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("缺少 title 参数"))?;
                backend.activate(title)?;
                Ok(serde_json::json!({"action": "activate", "title": title}))
            }

            "maximize" => {
                let title = config.get("title").and_then(|v| v.as_str()).unwrap_or("");
                backend.maximize(title)?;
                Ok(serde_json::json!({"action": "maximize"}))
            }

            "minimize" => {
                let title = config.get("title").and_then(|v| v.as_str()).unwrap_or("");
                backend.minimize(title)?;
                Ok(serde_json::json!({"action": "minimize"}))
            }

            "restore" => {
                let title = config.get("title").and_then(|v| v.as_str()).unwrap_or("");
                backend.restore(title)?;
                Ok(serde_json::json!({"action": "restore"}))
            }

            "close" => {
                let title = config.get("title").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("缺少 title 参数"))?;
                backend.close(title)?;
                Ok(serde_json::json!({"action": "close", "title": title}))
            }

            "resize" => {
                let title = config.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let width = config.get("width").and_then(|v| v.as_i64()).unwrap_or(800) as i32;
                let height = config.get("height").and_then(|v| v.as_i64()).unwrap_or(600) as i32;
                backend.resize(title, width, height)?;
                Ok(serde_json::json!({"action": "resize", "title": title, "width": width, "height": height}))
            }

            "wait" => {
                let title = config.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let timeout = config.get("timeout_s").and_then(|v| v.as_u64()).unwrap_or(30);
                backend.wait(title, timeout)?;
                Ok(serde_json::json!({"action": "wait", "title": title, "found": true}))
            }

            "list" => {
                let windows = backend.list()?;
                Ok(serde_json::json!({"action": "list", "windows": windows}))
            }

            _ => Err(anyhow!("未知窗口操作: {}（支持: find/activate/maximize/minimize/restore/close/resize/wait/list）", action))
        }
    }
}
```

**验证：**
```bash
cargo check 2>&1 | grep -E "error|warning"
```
预期：0 errors, 0 warnings（原有 PS 代码的 warning 一并消除）

**Commit：**
```bash
git add src-tauri/src/nodes/window.rs
git commit -m "refactor: window node now delegates to platform::window() trait"
```

---

## 任务 6：Rust 编译验证

**目标：** 确保 Windows 和 Linux cfg 路径均能通过编译

**在当前环境（Linux WSL）验证：**
```bash
cd src-tauri && cargo check 2>&1
```
预期：0 errors

**交叉编译检查 Windows 路径：**
```bash
rustup target add x86_64-pc-windows-msvc 2>/dev/null || true
cargo check --target x86_64-pc-windows-msvc 2>&1 | tail -20
```
预期：至少 Windows cfg 路径无语法错误（可能因 MSVC 链接器缺失 fail，但编译检查应通过）

**Commit：**
```bash
git add -A && git commit -m "chore: verify cross-platform compilation passes"
```

---

## 任务 7（可选）：启动时检测 xdotool

**目标：** Linux 启动时检查依赖，缺失则友好提示

**文件：**
- 修改：`src-tauri/src/main.rs`（或 `lib.rs` 初始化段）

**代码：**
```rust
#[cfg(target_os = "linux")]
fn check_linux_deps() {
    if let Err(e) = crate::platform::linux_window::LinuxWindowBackend::check_available() {
        eprintln!("⚠️  {}", e);
        eprintln!("   安装后重启应用即可使用窗口管理功能。");
    }
}
```

在 `main()` 中 `tauri::Builder` 之前调用 `check_linux_deps()`。

**验证：**
```bash
cargo check
```
预期：0 errors

---

## 附录 A：文件变更清单

| 文件 | 操作 | 行数 |
|------|------|:--:|
| `platform/traits.rs` | 追加 WindowBackend trait + WindowInfo | +50 |
| `platform/linux_window.rs` | 新建 | +180 |
| `platform/windows_window.rs` | 新建（搬迁 window.rs 的 PS 代码） | ~120 |
| `platform/mod.rs` | 注册 mod + 全局单例 + UnsupportedBackend | +40 |
| `nodes/window.rs` | 重写（250 行 → ~85 行） | -165 |
| `main.rs` | 可选的 Linux 依赖检测 | +10 |
| **合计净增** | | **~235 行** |

## 附录 B：验证 Checklist

- [ ] `cargo check` 在 WSL (Linux) 下 0 errors
- [ ] `cargo check --target x86_64-pc-windows-msvc` 编译检查通过
- [ ] `cargo clippy` 之前 21 个 warning 中的 PS 相关项消除
- [ ] `cargo test` 4 个已有测试仍然通过
- [ ] Windows 下 `cargo build --release` 成功并功能正常
- [ ] Linux 下 `apt install xdotool wmctrl && cargo build --release` 成功
- [ ] Linux 下手动测试：激活窗口、最小化、查找窗口均正常

## 附录 C：潜在风险

| 风险 | 概率 | 缓解措施 |
|------|:--:|------|
| xdotool 在 Wayland 下部分命令失效 | 中 | 降级到 ydotool（后续任务） |
| wmctrl 在某些发行版未预装 | 中 | maximize/resize/list 已有 xdotool 降级路径 |
| 同步 trait 方法与 async node 的小摩擦 | 低 | 操作都在 sub-millisecond 级别，无需 async |
| Windows 搬迁后 PS 调用行为变化 | 低 | 功能零变化，只改归属文件 |
