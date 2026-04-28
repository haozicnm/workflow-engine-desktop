// platform/linux.rs — Linux 输入后端
//
// 通过 enigo crate 实现跨显示协议的键鼠模拟。
// 支持 X11 和 Wayland（通过 libei 协议）。

use crate::platform::traits::InputBackend;
use anyhow::{Result, anyhow};
use tracing::info;
use enigo::{Keyboard, Mouse};

pub struct LinuxBackend {
    enigo: std::sync::Mutex<enigo::Enigo>,
}

impl Default for LinuxBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxBackend {
    pub fn new() -> Self {
        let settings = enigo::Settings::default();
        let enigo = enigo::Enigo::new(&settings)
            .expect("enigo 初始化失败——请确认 X11/Wayland 会话可用");
        info!("Linux 输入后端初始化完成");
        LinuxBackend {
            enigo: std::sync::Mutex::new(enigo),
        }
    }

    fn enigo_lock(&self) -> Result<std::sync::MutexGuard<'_, enigo::Enigo>> {
        self.enigo.lock()
            .map_err(|e| anyhow!("enigo 锁获取失败: {}", e))
    }

    fn resolve_button(button: &str) -> enigo::Button {
        match button {
            "right" => enigo::Button::Right,
            "middle" => enigo::Button::Middle,
            _ => enigo::Button::Left,
        }
    }
}

impl InputBackend for LinuxBackend {
    fn click(&self, x: i32, y: i32, button: &str) -> Result<()> {
        let mut e = self.enigo_lock()?;
        // enigo 0.3 的 move_mouse 接受 i32
        e.move_mouse(x, y, enigo::Coordinate::Abs)
            .map_err(|err| anyhow!("移动鼠标失败: {}", err))?;
        std::thread::sleep(std::time::Duration::from_millis(20));
        let btn = Self::resolve_button(button);
        e.button(btn, enigo::Direction::Click)
            .map_err(|err| anyhow!("鼠标点击失败: {}", err))?;
        Ok(())
    }

    fn move_mouse(&self, x: i32, y: i32) -> Result<()> {
        let mut e = self.enigo_lock()?;
        e.move_mouse(x, y, enigo::Coordinate::Abs)
            .map_err(|err| anyhow!("移动鼠标失败: {}", err))?;
        Ok(())
    }

    fn type_text(&self, text: &str, delay_ms: u64) -> Result<()> {
        if text.len() > 10000 {
            return Err(anyhow!("文本长度超过限制 (最大 10000 字符)"));
        }
        let mut e = self.enigo_lock()?;
        // 逐字符输入以保证可靠性
        for ch in text.chars() {
            let s = ch.to_string();
            e.text(&s)
                .map_err(|err| anyhow!("输入文本失败: {}", err))?;
            if delay_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
        }
        Ok(())
    }

    fn hotkey(&self, keys: &str) -> Result<()> {
        // 阻止危险系统快捷键
        let upper = keys.to_uppercase();
        let blocked = ["CTRL+ALT+DEL", "CTRL+ALT+DELETE", "ALT+F4"];
        for dangerous in &blocked {
            if upper.contains(dangerous) {
                return Err(anyhow!("禁止的热键组合: {} (系统安全限制)", keys));
            }
        }

        let mut e = self.enigo_lock()?;

        // 解析 "Ctrl+C" / "Alt+Tab" 等格式
        let parts: Vec<&str> = keys.split('+').map(|s| s.trim()).collect();
        let mut modifiers: Vec<enigo::Key> = Vec::new();
        let mut main_key: Option<enigo::Key> = None;

        for part in &parts {
            match part.to_uppercase().as_str() {
                "CTRL" | "CONTROL" => modifiers.push(enigo::Key::Control),
                "ALT" => modifiers.push(enigo::Key::Alt),
                "SHIFT" => modifiers.push(enigo::Key::Shift),
                "META" | "WIN" | "SUPER" => modifiers.push(enigo::Key::Meta),
                "TAB" => main_key = Some(enigo::Key::Tab),
                "ENTER" | "RETURN" => main_key = Some(enigo::Key::Return),
                "ESC" | "ESCAPE" => main_key = Some(enigo::Key::Escape),
                "SPACE" => main_key = Some(enigo::Key::Space),
                "BACKSPACE" => main_key = Some(enigo::Key::Backspace),
                "DELETE" => main_key = Some(enigo::Key::Delete),
                "UP" => main_key = Some(enigo::Key::UpArrow),
                "DOWN" => main_key = Some(enigo::Key::DownArrow),
                "LEFT" => main_key = Some(enigo::Key::LeftArrow),
                "RIGHT" => main_key = Some(enigo::Key::RightArrow),
                "HOME" => main_key = Some(enigo::Key::Home),
                "END" => main_key = Some(enigo::Key::End),
                "PAGEUP" => main_key = Some(enigo::Key::PageUp),
                "PAGEDOWN" => main_key = Some(enigo::Key::PageDown),
                single if single.len() == 1 => {
                    // 单字母 → 先按修饰键再模拟字符
                    let ch = single.chars().next().unwrap();
                    let key = match ch {
                        'A'..='Z' => enigo::Key::Unicode(ch),
                        '0'..='9' => enigo::Key::Unicode(ch),
                        _ => enigo::Key::Unicode(ch),
                    };
                    // 对于 F1-F12
                    main_key = Some(key);
                }
                fkey if fkey.starts_with('F') => {
                    if let Ok(n) = fkey[1..].parse::<u8>() {
                        main_key = Some(match n {
                            1 => enigo::Key::F1, 2 => enigo::Key::F2,
                            3 => enigo::Key::F3, 4 => enigo::Key::F4,
                            5 => enigo::Key::F5, 6 => enigo::Key::F6,
                            7 => enigo::Key::F7, 8 => enigo::Key::F8,
                            9 => enigo::Key::F9, 10 => enigo::Key::F10,
                            11 => enigo::Key::F11, 12 => enigo::Key::F12,
                            _ => continue,
                        });
                    }
                }
                _ => {}
            }
        }

        // 按下修饰键
        for m in &modifiers {
            e.key(*m, enigo::Direction::Press)
                .map_err(|err| anyhow!("按下修饰键失败: {}", err))?;
        }

        // 按下主键
        if let Some(key) = main_key {
            e.key(key, enigo::Direction::Click)
                .map_err(|err| anyhow!("按键失败: {}", err))?;
        }

        // 松开修饰键
        for m in modifiers.iter().rev() {
            e.key(*m, enigo::Direction::Release)
                .map_err(|err| anyhow!("释放修饰键失败: {}", err))?;
        }

        Ok(())
    }

    fn scroll(&self, amount: i32) -> Result<()> {
        let mut e = self.enigo_lock()?;
        // enigo 0.3 scroll 方向: 正数向下, 负数向上
        e.scroll(-amount, enigo::Axis::Vertical)
            .map_err(|err| anyhow!("滚动失败: {}", err))?;
        Ok(())
    }

    fn platform_name(&self) -> &str { "linux" }
}
