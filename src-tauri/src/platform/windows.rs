// platform/windows.rs — Windows 输入后端
//
// 封装现有的 PowerShell + user32.dll 逻辑，对上层暴露统一 trait。
// 代码逻辑与原 mouse_keyboard.rs 完全一致，零功能变更。

use crate::platform::traits::InputBackend;
use anyhow::{Result, anyhow};
use tracing::warn;

pub struct WindowsBackend;

impl WindowsBackend {
    pub fn new() -> Self { WindowsBackend }
}

fn run_ps(script: &str) -> Result<()> {
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map_err(|e| anyhow!("PowerShell 执行失败: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            return Err(anyhow!("PowerShell 错误: {}", stderr));
        }
    }
    Ok(())
}

fn validate_hotkey(keys: &str) -> Result<()> {
    let upper = keys.to_uppercase();
    let trimmed = upper.trim();
    let blocked = [
        "{LWIN}", "{RWIN}", "^%{DELETE}", "^{ESC}", "%{TAB}",
        "#R", "{LWIN}R", "{RWIN}R", "^+{ESC}", "#D",
        "{LWIN}D", "{RWIN}D", "#L", "{LWIN}L", "{RWIN}L", "^%{TAB}",
    ];
    for dangerous in &blocked {
        if trimmed.contains(dangerous) {
            return Err(anyhow!("禁止的热键组合: {} (系统安全限制)", keys));
        }
    }
    if trimmed.contains("%{F4}") {
        warn!("hotkey 包含 Alt+F4，将关闭当前活动窗口");
    }
    Ok(())
}

impl InputBackend for WindowsBackend {
    fn click(&self, x: i32, y: i32, button: &str) -> Result<()> {
        let flag = match button {
            "right" => "0x08",
            "middle" => "0x20",
            _ => "0x02",
        };
        let ps = format!(
            "Add-Type -AssemblyName System.Windows.Forms\n\
             Add-Type @\"\n\
             using System; using System.Runtime.InteropServices;\n\
             public class MK {{ [DllImport(\"user32.dll\")] public static extern bool SetCursorPos(int X, int Y); [DllImport(\"user32.dll\")] public static extern void mouse_event(uint f, int dx, int dy, uint d, IntPtr e); }}\n\
             \"@\n\
             [MK]::SetCursorPos({x}, {y})\n\
             $f = {flag}; $u = $f -bor 1\n\
             [MK]::mouse_event($f, 0, 0, 0, [IntPtr]::Zero)\n\
             Start-Sleep -Milliseconds 50\n\
             [MK]::mouse_event($u, 0, 0, 0, [IntPtr]::Zero)"
        );
        run_ps(&ps)
    }

    fn move_mouse(&self, x: i32, y: i32) -> Result<()> {
        let ps = format!(
            "Add-Type -AssemblyName System.Windows.Forms\n\
             [System.Windows.Forms.Cursor]::Position = New-Object System.Drawing.Point({x}, {y})"
        );
        run_ps(&ps)
    }

    fn type_text(&self, text: &str, delay_ms: u64) -> Result<()> {
        if text.len() > 10000 {
            return Err(anyhow!("文本长度超过限制 (最大 10000 字符)"));
        }
        let escaped = text.replace('\'', "''");
        let ps = format!(
            "Add-Type -AssemblyName System.Windows.Forms\n\
             $chars = '{escaped}'\n\
             foreach ($c in $chars.ToCharArray()) {{\n\
                 [System.Windows.Forms.SendKeys]::SendWait([string]$c)\n\
                 Start-Sleep -Milliseconds {delay_ms}\n\
             }}"
        );
        run_ps(&ps)
    }

    fn hotkey(&self, keys: &str) -> Result<()> {
        validate_hotkey(keys)?;
        let escaped = keys.replace('"', "`\"");
        let ps = format!(
            "Add-Type -AssemblyName System.Windows.Forms\n\
             [System.Windows.Forms.SendKeys]::SendWait(\"{escaped}\")"
        );
        run_ps(&ps)
    }

    fn scroll(&self, amount: i32) -> Result<()> {
        let ps = format!(
            "Add-Type @\"\n\
             using System; using System.Runtime.InteropServices;\n\
             public class SC {{ [DllImport(\"user32.dll\")] public static extern void mouse_event(uint f, int dx, int dy, int d, IntPtr e); }}\n\
             \"@\n\
             [SC]::mouse_event(0x0800, 0, 0, {amount} * 120, [IntPtr]::Zero)"
        );
        run_ps(&ps)
    }

    fn platform_name(&self) -> &str { "windows" }
}
