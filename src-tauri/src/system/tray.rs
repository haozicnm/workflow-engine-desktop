// system/tray.rs — 系统托盘
// 关闭窗口时最小化到托盘，双击/右键菜单恢复窗口
use tauri::{
    tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState},
    menu::{Menu, MenuItem},
    Manager,
};
use tracing::info;

pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let show_i = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

    // 获取图标（使用 clone 避免 unwrap，fallback 到空像素图）
    let icon = app.default_window_icon()
        .cloned()
        .unwrap_or_else(|| tauri::image::Image::new(&[0u8; 64], 4, 4));

    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .tooltip("Workflow Engine")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                info!("托盘菜单: 显示主窗口");
                show_window(app);
            }
            "quit" => {
                info!("托盘菜单: 退出应用");
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event {
                info!("托盘图标双击: 显示主窗口");
                show_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

pub fn show_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
