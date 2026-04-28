// platform/windows_window.rs — Windows 窗口管理后端（PowerShell + user32.dll）
//
// 封装现有的 PowerShell 窗口管理逻辑，对上层暴露统一的 WindowBackend trait。
// 代码逻辑与旧 nodes/window.rs 完全一致，零功能变更。
// 从 async 改为同步（std::process::Command），与平台层其他实现保持一致。

use crate::platform::traits::{WindowBackend, WindowInfo};
use anyhow::{Result, anyhow};
use std::process::Command;

pub struct WindowsWindowBackend;

impl WindowsWindowBackend {
    pub fn new() -> Self {
        WindowsWindowBackend
    }
}

/// 执行 PowerShell 命令（无输出捕获）
fn run_ps(script: &str) -> Result<()> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map_err(|e| anyhow!("PowerShell 执行失败: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            return Err(anyhow!("{}", stderr));
        }
    }
    Ok(())
}

/// 执行 PowerShell 命令并捕获 stdout
fn run_ps_capture(script: &str) -> Result<String> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map_err(|e| anyhow!("PowerShell 执行失败: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() && stdout.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("{}", stderr));
    }
    Ok(stdout)
}

/// 窗口 ShowWindow 命令（maximize/minimize/restore 共用）
fn window_show_cmd(title: &str, show_cmd: i32) -> Result<()> {
    if title.len() > 200 {
        return Err(anyhow!("窗口标题长度超过限制 (最大 200 字符)"));
    }
    let ps = format!(
        r#"Add-Type @"
using System;
using System.Runtime.InteropServices;
public class WC {{
    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int cmd);
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);
}}
"@
$p = Get-Process | Where-Object {{ $_.MainWindowTitle -like '*{title}*' -and $_.MainWindowHandle -ne 0 }} | Select-Object -First 1
if ($p) {{
    [WC]::ShowWindow($p.MainWindowHandle, {show_cmd}) | Out-Null
    Write-Output "ok"
}} else {{ Write-Error "窗口未找到"; exit 1 }}"#,
    );
    run_ps(&ps)
}

impl WindowBackend for WindowsWindowBackend {
    fn find(&self, title: &str) -> Result<Vec<WindowInfo>> {
        if title.is_empty() {
            return Ok(vec![]);
        }
        let ps = format!(
            r#"Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class Win {{
    [DllImport("user32.dll")]
    public static extern IntPtr FindWindow(string cls, string title);
    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")]
    public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT {{ public int L,T,R,B; }}
}}
"@
$procs = Get-Process | Where-Object {{ $_.MainWindowTitle -like '*{}*' -and $_.MainWindowHandle -ne 0 }}
$results = @()
foreach ($p in $procs) {{
    $results += @{{
        pid = $p.Id
        title = $p.MainWindowTitle
        handle = $p.MainWindowHandle.ToInt64()
        process = $p.ProcessName
    }}
}}
$results | ConvertTo-Json -Compress"#,
            title.replace('\'', "''")
        );
        let output = run_ps_capture(&ps)?;
        let windows: Vec<WindowInfo> = serde_json::from_str(&output).unwrap_or_default();
        Ok(windows)
    }

    fn activate(&self, title: &str) -> Result<()> {
        if title.is_empty() {
            return Err(anyhow!("窗口标题不能为空"));
        }
        let ps = format!(
            r#"Add-Type @"
using System;
using System.Runtime.InteropServices;
public class WA {{
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int cmd);
}}
"@
$p = Get-Process | Where-Object {{ $_.MainWindowTitle -like '*{title}*' -and $_.MainWindowHandle -ne 0 }} | Select-Object -First 1
if ($p) {{
    [WA]::ShowWindow($p.MainWindowHandle, 9) | Out-Null  # SW_RESTORE
    [WA]::SetForegroundWindow($p.MainWindowHandle) | Out-Null
    Write-Output "ok:$($p.MainWindowTitle)"
}} else {{
    Write-Error "窗口未找到: {title}"
    exit 1
}}"#,
        );
        run_ps(&ps)
    }

    fn maximize(&self, title: &str) -> Result<()> {
        window_show_cmd(title, 3) // SW_MAXIMIZE
    }

    fn minimize(&self, title: &str) -> Result<()> {
        window_show_cmd(title, 6) // SW_MINIMIZE
    }

    fn restore(&self, title: &str) -> Result<()> {
        window_show_cmd(title, 9) // SW_RESTORE
    }

    fn close(&self, title: &str) -> Result<()> {
        if title.is_empty() {
            return Err(anyhow!("窗口标题不能为空"));
        }
        let ps = format!(
            r#"$p = Get-Process | Where-Object {{ $_.MainWindowTitle -like '*{}*' -and $_.MainWindowHandle -ne 0 }} | Select-Object -First 1
if ($p) {{ $p.CloseMainWindow() | Out-Null; Write-Output "ok" }} else {{ Write-Error "窗口未找到"; exit 1 }}"#,
            title.replace('\'', "''")
        );
        run_ps(&ps)
    }

    fn resize(&self, title: &str, width: i32, height: i32) -> Result<()> {
        if width <= 0 || height <= 0 {
            return Err(anyhow!("窗口尺寸必须大于 0，收到: {}x{}", width, height));
        }
        let ps = format!(
            r#"Add-Type @"
using System;
using System.Runtime.InteropServices;
public class WR {{
    [DllImport("user32.dll")]
    public static extern bool MoveWindow(IntPtr hWnd, int X, int Y, int W, int H, bool repaint);
}}
"@
$p = Get-Process | Where-Object {{ $_.MainWindowTitle -like '*{title}*' -and $_.MainWindowHandle -ne 0 }} | Select-Object -First 1
if ($p) {{
    [WR]::MoveWindow($p.MainWindowHandle, 0, 0, {width}, {height}, $true) | Out-Null
    Write-Output "ok"
}} else {{ Write-Error "窗口未找到"; exit 1 }}"#,
        );
        run_ps(&ps)
    }

    fn wait(&self, title: &str, timeout_s: u64) -> Result<()> {
        let ps = format!(
            r#"$deadline = (Get-Date).AddSeconds({timeout_s})
while ((Get-Date) -lt $deadline) {{
    $p = Get-Process | Where-Object {{ $_.MainWindowTitle -like '*{title}*' -and $_.MainWindowHandle -ne 0 }} | Select-Object -First 1
    if ($p) {{ Write-Output "found:$($p.MainWindowTitle)"; exit 0 }}
    Start-Sleep -Milliseconds 500
}}
Write-Error "等待超时: {title}"
exit 1"#,
        );
        run_ps(&ps)
    }

    fn list(&self) -> Result<Vec<WindowInfo>> {
        let ps = r#"Get-Process | Where-Object { $_.MainWindowTitle -ne '' } | ForEach-Object {
    @{ pid=$_.Id; title=$_.MainWindowTitle; process=$_.ProcessName }
} | ConvertTo-Json -Compress"#;
        let output = run_ps_capture(ps)?;
        let windows: Vec<WindowInfo> = serde_json::from_str(&output).unwrap_or_default();
        Ok(windows)
    }
}
