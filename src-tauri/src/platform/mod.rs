// platform/mod.rs — 平台抽象层
//
// 设计原则：
//   - trait 定义在 traits.rs，与平台无关
//   - 各平台实现在 windows.rs / linux.rs
//   - 录制实现在 recording.rs
//   - 启动时根据 cfg 自动选择实现
//   - 所有节点代码只依赖 trait，不直接调平台 API

pub mod recording;
pub mod traits;

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub use windows as current;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub use linux as current;

#[cfg(target_os = "linux")]
pub mod linux_window;
#[cfg(target_os = "windows")]
pub mod windows_window;

use std::sync::OnceLock;
use traits::InputBackend;
use traits::WindowBackend;

/// 全局平台实例（启动时初始化一次）
static PLATFORM: OnceLock<Box<dyn InputBackend + Send + Sync>> = OnceLock::new();

/// 获取全局平台后端（键鼠操作）
///
/// Linux: tries enigo first, falls back to graceful error messages if unavailable
pub fn input() -> &'static (dyn InputBackend + Send + Sync) {
    PLATFORM.get_or_init(|| {
        #[cfg(target_os = "windows")]
        { Box::new(windows::WindowsBackend::new()) }
        #[cfg(target_os = "linux")]
        {
            match linux::LinuxBackend::new() {
                Ok(backend) => Box::new(backend),
                Err(e) => {
                    tracing::warn!("Linux input backend unavailable (enigo init failed): {}. Falling back to stub.", e);
                    Box::new(UnsupportedBackend)
                }
            }
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        { Box::new(UnsupportedBackend) }
    }).as_ref()
}

/// 不支持的操作系统（编译通过但运行时报错）
#[allow(dead_code)]
struct UnsupportedBackend;

impl InputBackend for UnsupportedBackend {
    fn click(&self, _x: i32, _y: i32, _button: &str) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("当前操作系统暂不支持键鼠操作"))
    }
    fn type_text(&self, _text: &str, _delay_ms: u64) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("当前操作系统暂不支持键鼠操作"))
    }
    fn hotkey(&self, _keys: &str) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("当前操作系统暂不支持键鼠操作"))
    }
    fn scroll(&self, _amount: i32) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("当前操作系统暂不支持键鼠操作"))
    }
    fn move_mouse(&self, _x: i32, _y: i32) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("当前操作系统暂不支持键鼠操作"))
    }
    fn platform_name(&self) -> &str {
        "unsupported"
    }
}

// ─── 窗口管理平台抽象 ───

/// 全局窗口管理后端
static WINDOW_BACKEND: OnceLock<Box<dyn WindowBackend + Send + Sync>> = OnceLock::new();

/// 获取全局窗口管理后端
pub fn window() -> &'static (dyn WindowBackend + Send + Sync) {
    WINDOW_BACKEND
        .get_or_init(|| {
            #[cfg(target_os = "windows")]
            {
                Box::new(windows_window::WindowsWindowBackend::new())
            }
            #[cfg(target_os = "linux")]
            {
                Box::new(linux_window::LinuxWindowBackend::new())
            }
            #[cfg(not(any(target_os = "windows", target_os = "linux")))]
            {
                Box::new(UnsupportedWindowBackend)
            }
        })
        .as_ref()
}

#[allow(dead_code)]
struct UnsupportedWindowBackend;

impl WindowBackend for UnsupportedWindowBackend {
    fn find(&self, _title: &str) -> anyhow::Result<Vec<traits::WindowInfo>> {
        Ok(vec![])
    }
    fn activate(&self, _title: &str) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("当前操作系统暂不支持窗口管理"))
    }
    fn maximize(&self, _title: &str) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("当前操作系统暂不支持窗口管理"))
    }
    fn minimize(&self, _title: &str) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("当前操作系统暂不支持窗口管理"))
    }
    fn restore(&self, _title: &str) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("当前操作系统暂不支持窗口管理"))
    }
    fn close(&self, _title: &str) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("当前操作系统暂不支持窗口管理"))
    }
    fn resize(&self, _title: &str, _width: i32, _height: i32) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("当前操作系统暂不支持窗口管理"))
    }
    fn wait(&self, _title: &str, _timeout_s: u64) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("当前操作系统暂不支持窗口管理"))
    }
    fn list(&self) -> anyhow::Result<Vec<traits::WindowInfo>> {
        Err(anyhow::anyhow!("当前操作系统暂不支持窗口管理"))
    }
}
