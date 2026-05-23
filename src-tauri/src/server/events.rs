// server/events.rs — SSE 事件广播层（替代 Tauri emit）
use tokio::sync::broadcast;
use serde_json::Value;

static EVENT_TX: std::sync::OnceLock<broadcast::Sender<(String, Value)>> = std::sync::OnceLock::new();

pub fn get_tx() -> broadcast::Sender<(String, Value)> {
    EVENT_TX.get_or_init(|| {
        let (tx, _) = broadcast::channel(256);
        tx
    }).clone()
}

pub fn emit(event: &str, data: Value) {
    let _ = get_tx().send((event.to_string(), data));
}
