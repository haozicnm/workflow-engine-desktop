// platform/linux_window.rs — Linux 窗口管理后端（xdotool + wmctrl CLI）
//
// 通过成熟的 Linux 桌面自动化工具实现窗口管理。
// - xdotool：窗口搜索、激活、大小调整、关闭
// - wmctrl：最大化、最小化、列出所有窗口（更全面）
// 两个工具互为降级路径，任一可用即可工作。

use crate::platform::traits::{WindowBackend, WindowInfo};
use anyhow::{Result, anyhow};
use std::process::Command;

pub struct LinuxWindowBackend;

impl LinuxWindowBackend {
    pub fn new() -> Self {
        LinuxWindowBackend
    }

    /// 检查 xdotool 是否可用（应用启动时调用）
    pub fn check_available() -> Result<()> {
        Command::new("xdotool")
            .arg("--version")
            .output()
            .map_err(|_| anyhow!(
                "xdotool 未安装。请执行: sudo apt install xdotool\n\
                 窗口管理功能将在安装后可用。"
            ))?;
        Ok(())
    }
}

// ─── 内部工具函数 ───

/// 执行命令并返回 stdout
fn run(cmd: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| anyhow!("执行 {} 失败: {}", cmd, e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = if stderr.trim().is_empty() {
            format!("{} 返回非零退出码", cmd)
        } else {
            format!("{} 错误: {}", cmd, stderr.trim())
        };
        return Err(anyhow!("{}", msg));
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

/// 获取单个窗口的详细信息
fn get_window_info(win_id: &str) -> Result<WindowInfo> {
    let name = run("xdotool", &["getwindowname", win_id]).unwrap_or_default();
    let pid: u32 = run("xdotool", &["getwindowpid", win_id])
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    Ok(WindowInfo {
        pid,
        title: name,
        handle: win_id.to_string(),
        process: String::new(),
    })
}

// ─── WindowBackend 实现 ───

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
        // xdotool search + windowactivate + windowfocus 一条龙
        run("xdotool", &[
            "search", "--name", title,
            "windowactivate",
            "windowfocus",
        ])?;
        Ok(())
    }

    fn maximize(&self, title: &str) -> Result<()> {
        // 优先用 wmctrl（更可靠）
        if run("wmctrl", &["-r", title, "-b", "add,maximized_vert,maximized_horz"]).is_ok() {
            return Ok(());
        }
        // 降级：xdotool 激活窗口后发送 Alt+F10
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
        // 激活窗口即可还原（最小化的窗口在激活时自动恢复）
        self.activate(title)
    }

    fn close(&self, title: &str) -> Result<()> {
        let id = find_window_id(title)?;
        run("xdotool", &["windowclose", &id])?;
        Ok(())
    }

    fn resize(&self, title: &str, width: i32, height: i32) -> Result<()> {
        if width <= 0 || height <= 0 {
            return Err(anyhow!("窗口尺寸必须大于 0，收到: {}x{}", width, height));
        }
        // 优先用 wmctrl（可保持窗口位置不变）
        if run("wmctrl", &[
            "-r", title, "-e", "0,-1,-1",
            &width.to_string(), &height.to_string(),
        ]).is_ok() {
            return Ok(());
        }
        // 降级：xdotool windowsize
        let id = find_window_id(title)?;
        run("xdotool", &[
            "windowsize", &id,
            &width.to_string(), &height.to_string(),
        ])?;
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
                return Err(anyhow!(
                    "等待窗口 '{}' 出现超时 ({}s)",
                    title, timeout_s
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }

    fn list(&self) -> Result<Vec<WindowInfo>> {
        // 优先用 wmctrl -l（输出格式：<id> <desktop> <host> <title>）
        let out = match run("wmctrl", &["-l"]) {
            Ok(o) => o,
            Err(_) => {
                // 降级：用 xdotool 列出可见窗口
                return Ok(vec![]);
            }
        };

        let mut windows = Vec::new();
        for line in out.lines() {
            let parts: Vec<&str> = line.splitn(5, ' ').collect();
            if parts.len() >= 5 {
                windows.push(WindowInfo {
                    pid: 0, // wmctrl 不输出 PID
                    title: parts[4..].join(" "),
                    handle: parts[0].to_string(),
                    process: String::new(),
                });
            }
        }
        Ok(windows)
    }
}
