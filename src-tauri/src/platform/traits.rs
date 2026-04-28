// platform/traits.rs — 平台抽象 trait 定义
//
// 所有节点代码通过 trait 调用平台功能，不直接依赖操作系统 API。

use anyhow::Result;

/// 全局输入模拟（键鼠操作）
///
/// 实现者：
///   - Windows: 通过 PowerShell 调 user32.dll（保留现有代码）
///   - Linux:   通过 enigo crate 调 X11/Wayland
pub trait InputBackend: Send + Sync {
    /// 鼠标点击
    fn click(&self, x: i32, y: i32, button: &str) -> Result<()>;

    /// 鼠标移动
    fn move_mouse(&self, x: i32, y: i32) -> Result<()>;

    /// 键盘输入文本
    fn type_text(&self, text: &str, delay_ms: u64) -> Result<()>;

    /// 快捷键组合（如 "Ctrl+C"→"^c", "Alt+F4"→"%{F4}"）
    fn hotkey(&self, keys: &str) -> Result<()>;

    /// 鼠标滚轮（正数向上/放大，负数向下/缩小）
    fn scroll(&self, amount: i32) -> Result<()>;

    /// 平台名称（调试用）
    fn platform_name(&self) -> &str;
}

/// 全局录制后端（键鼠事件监听）
///
/// 实现者：
///   - Windows: 通过 desktop_recorder.py sidecar（保留现有代码）
///   - Linux:   通过 rdev crate 全局钩子
pub trait RecordingBackend: Send + Sync {
    /// 开始录制
    fn start(&self) -> Result<()>;

    /// 停止录制，返回操作列表
    fn stop(&self) -> Result<Vec<serde_json::Value>>;

    /// 是否正在录制
    fn is_recording(&self) -> bool;
}

/// 窗口信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

/// 平台判断辅助
pub fn is_windows() -> bool {
    cfg!(target_os = "windows")
}

pub fn is_linux() -> bool {
    cfg!(target_os = "linux")
}
