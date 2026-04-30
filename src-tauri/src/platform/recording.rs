// platform/recording.rs — 跨平台录制后端
//
// Linux: 通过 rdev crate 全局输入监听
// 运行时根据 cfg 自动选择实现。

#[cfg(target_os = "linux")]
use crate::platform::traits::RecordingBackend;
#[cfg(target_os = "linux")]
use anyhow::{Result, anyhow};
#[cfg(target_os = "linux")]
use std::sync::Mutex;
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicBool, Ordering};

// ─── 共享录制状态（仅 Linux 编译） ───

#[cfg(target_os = "linux")]
static RECORDING_ACTIVE: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "linux")]
static RECORDED_ACTIONS: Mutex<Vec<serde_json::Value>> = Mutex::new(Vec::new());
#[cfg(target_os = "linux")]
static LAST_KEY_TIME: Mutex<u128> = Mutex::new(0);
#[cfg(target_os = "linux")]
static TEXT_BUFFER: Mutex<String> = Mutex::new(String::new());
#[cfg(target_os = "linux")]
static LAST_MOUSE_TIME: Mutex<u128> = Mutex::new(0);

#[cfg(target_os = "linux")]
pub fn is_recording() -> bool {
    RECORDING_ACTIVE.load(Ordering::SeqCst)
}

#[cfg(target_os = "linux")]
fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(target_os = "linux")]
fn push_action(action: serde_json::Value) {
    RECORDED_ACTIONS.lock().expect("获取 RECORDED_ACTIONS 锁失败（Mutex 中毒）").push(action);
}

#[cfg(target_os = "linux")]
fn flush_text() {
    let mut buf = TEXT_BUFFER.lock().expect("获取 TEXT_BUFFER 锁失败（Mutex 中毒）");
    if !buf.is_empty() {
        push_action(serde_json::json!({
            "type": "type", "source": "desktop",
            "text": buf.clone(), "timestamp": now_ms(),
        }));
        buf.clear();
    }
}

// ─── Linux 录制后端（rdev） ───

#[cfg(target_os = "linux")]
pub struct LinuxRecordingBackend {
    handle: Mutex<Option<std::thread::JoinHandle<()>>>,
}

#[cfg(target_os = "linux")]
impl Default for LinuxRecordingBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "linux")]
impl LinuxRecordingBackend {
    pub fn new() -> Self {
        LinuxRecordingBackend { handle: Mutex::new(None) }
    }
}

#[cfg(target_os = "linux")]
impl RecordingBackend for LinuxRecordingBackend {
    fn start(&self) -> Result<()> {
        if RECORDING_ACTIVE.load(Ordering::SeqCst) {
            return Err(anyhow!("已在录制中"));
        }
        RECORDED_ACTIONS.lock().expect("获取 RECORDED_ACTIONS 锁失败（Mutex 中毒）").clear();
        *TEXT_BUFFER.lock().expect("获取 TEXT_BUFFER 锁失败（Mutex 中毒）") = String::new();
        *LAST_KEY_TIME.lock().expect("获取 LAST_KEY_TIME 锁失败（Mutex 中毒）") = 0;
        *LAST_MOUSE_TIME.lock().expect("获取 LAST_MOUSE_TIME 锁失败（Mutex 中毒）") = 0;
        RECORDING_ACTIVE.store(true, Ordering::SeqCst);

        let handle = std::thread::spawn(move || {
            use rdev::{listen, Event, EventType, Key, Button};

            let callback = |event: Event| {
                if !RECORDING_ACTIVE.load(Ordering::SeqCst) { return; }
                let now = now_ms();

                match event.event_type {
                    EventType::KeyPress(key) => {
                        match key {
                            Key::ShiftLeft | Key::ShiftRight |
                            Key::ControlLeft | Key::ControlRight |
                            Key::Alt | Key::AltGr | Key::MetaLeft | Key::MetaRight => (),
                            Key::Return => {
                                flush_text();
                            }
                            Key::Tab => {
                                flush_text();
                                push_action(serde_json::json!({
                                    "type": "hotkey", "source": "desktop",
                                    "keys": "Tab", "timestamp": now,
                                }));
                            }
                            Key::Backspace => {
                                TEXT_BUFFER.lock().expect("获取 TEXT_BUFFER 锁失败（Mutex 中毒）").pop();
                            }
                            _ => {
                                if let Some(ch) = key_to_char(&key) {
                                    let mut last = LAST_KEY_TIME.lock().expect("获取 LAST_KEY_TIME 锁失败（Mutex 中毒）");
                                    if now.saturating_sub(*last) > 1000 {
                                        drop(last);
                                        flush_text();
                                    } else {
                                        *last = now;
                                        drop(last);
                                    }
                                    TEXT_BUFFER.lock().expect("获取 TEXT_BUFFER 锁失败（Mutex 中毒）").push_str(&ch);
                                }
                            }
                        }
                    }
                    EventType::ButtonPress(button) => {
                        let mut last_mouse = LAST_MOUSE_TIME.lock().expect("获取 LAST_MOUSE_TIME 锁失败（Mutex 中毒）");
                        if now.saturating_sub(*last_mouse) > 100 {
                            *last_mouse = now;
                            drop(last_mouse);
                            let btn = match button {
                                Button::Left => "left",
                                Button::Right => "right",
                                Button::Middle => "middle",
                                _ => return,
                            };
                            push_action(serde_json::json!({
                                "type": "click", "source": "desktop",
                                "button": btn, "timestamp": now,
                            }));
                        }
                    }
                    EventType::Wheel { delta_y, .. } => {
                        push_action(serde_json::json!({
                            "type": "scroll", "source": "desktop",
                            "amount": delta_y, "timestamp": now,
                        }));
                    }
                    _ => {}
                }
            };

            if let Err(e) = listen(callback) {
tracing::error!("rdev listen 错误: {:?}", e);
            }
            flush_text();
        });

        *self.handle.lock().expect("获取录制句柄锁失败（Mutex 中毒）") = Some(handle);
        Ok(())
    }

    fn stop(&self) -> Result<Vec<serde_json::Value>> {
        RECORDING_ACTIVE.store(false, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_millis(300));
        if let Some(h) = self.handle.lock().expect("获取录制句柄锁失败（Mutex 中毒）").take() {
            let _ = h.join();
        }
        let actions = RECORDED_ACTIONS.lock().expect("获取 RECORDED_ACTIONS 锁失败（Mutex 中毒）").clone();
        RECORDED_ACTIONS.lock().expect("获取 RECORDED_ACTIONS 锁失败（Mutex 中毒）").clear();
        Ok(actions)
    }

    fn is_recording(&self) -> bool {
        RECORDING_ACTIVE.load(Ordering::SeqCst)
    }
}

#[cfg(target_os = "linux")]
fn key_to_char(key: &rdev::Key) -> Option<String> {
    use rdev::Key;
    let c = match key {
        Key::KeyA => "a", Key::KeyB => "b", Key::KeyC => "c", Key::KeyD => "d",
        Key::KeyE => "e", Key::KeyF => "f", Key::KeyG => "g", Key::KeyH => "h",
        Key::KeyI => "i", Key::KeyJ => "j", Key::KeyK => "k", Key::KeyL => "l",
        Key::KeyM => "m", Key::KeyN => "n", Key::KeyO => "o", Key::KeyP => "p",
        Key::KeyQ => "q", Key::KeyR => "r", Key::KeyS => "s", Key::KeyT => "t",
        Key::KeyU => "u", Key::KeyV => "v", Key::KeyW => "w", Key::KeyX => "x",
        Key::KeyY => "y", Key::KeyZ => "z",
        Key::Num0 => "0", Key::Num1 => "1", Key::Num2 => "2", Key::Num3 => "3",
        Key::Num4 => "4", Key::Num5 => "5", Key::Num6 => "6", Key::Num7 => "7",
        Key::Num8 => "8", Key::Num9 => "9",
        Key::Space => " ",
        Key::Minus => "-", Key::Equal => "=",
        Key::LeftBracket => "[", Key::RightBracket => "]",
        Key::SemiColon => ";", Key::Quote => "'",
        Key::Comma => ",", Key::Dot => ".", Key::Slash => "/",
        Key::BackSlash => "\\",
        _ => return None,
    };
    Some(c.to_string())
}
