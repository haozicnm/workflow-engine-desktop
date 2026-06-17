// nodes/browser_container.rs — 浏览器容器执行器
//
// v4.0: 在一个浏览器窗口内按顺序执行多个 action，
// 支持通过 DAG 连线从上游容器端口接收数据并产出输出端口数据。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing;

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};

// ─── 数据结构 ───

/// 容器内的单个 action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerAction {
    pub id: String,
    #[serde(rename = "type")]
    pub action_type: String,
    pub label: String,
    #[serde(default)]
    pub config: HashMap<String, Value>,
}

/// 浏览器容器配置（来自前端节点的 config）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserContainerConfig {
    #[serde(default = "default_browser")]
    pub browser: String,
    #[serde(default)]
    pub headless: bool,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    pub actions: Vec<ContainerAction>,
}

fn default_browser() -> String {
    "chromium".to_string()
}

fn default_timeout() -> u64 {
    30000
}

/// 校验选择器：不能为空，返回友好错误
fn require_selector(action: &ContainerAction) -> Result<&str> {
    let selector = action
        .config
        .get("selector")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if selector.is_empty() {
        Err(anyhow!(
            "Action '{}' (type={}) 缺少 selector 参数，请在配置中指定 CSS 选择器",
            action.label,
            action.action_type
        ))
    } else {
        Ok(selector)
    }
}

/// 发送浏览器命令（通过 WebBridge 扩展）
async fn send_browser_action(action: &str, params: &Value) -> Result<Value> {
    crate::nodes::webbridge::send_command(action, params.clone()).await
}

/// 浏览器容器执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerResult {
    pub output_ports: HashMap<String, Value>,
    pub error: Option<String>,
}

// ─── 核心执行逻辑 ───

/// 执行浏览器容器
///
/// 使用现有的 browser.rs sidecar 机制，按顺序执行所有 action：
/// - input/fill 类型优先从 input_ports（连线传入）获取值
/// - extract/screenshot/evaluate/get_title 等产出数据存入 output_ports
pub async fn execute_browser_container(
    config: &BrowserContainerConfig,
    input_ports: &HashMap<String, Value>,
) -> Result<ContainerResult> {
    // ── WebBridge 预检 ──
    if !crate::nodes::webbridge::is_available().await {
        return Err(anyhow!(
            "浏览器容器不可用：WebBridge 扩展未连接。\n\
             请确保已安装 Workflow WebBridge 浏览器扩展，\n\
             然后刷新页面或重启浏览器以建立连接。\n\
             (Chrome/Edge: chrome://extensions → 启用 Workflow WebBridge)"
        ));
    }

    // 整体超时保护：默认 120s，可通过 config.timeout 配置（秒）
    let overall_timeout = if config.timeout > 1000 {
        // 兼容旧配置：毫秒 → 秒
        std::time::Duration::from_millis(config.timeout)
    } else {
        std::time::Duration::from_secs(120)
    };

    match tokio::time::timeout(overall_timeout, execute_actions(config, input_ports)).await {
        Ok(result) => result,
        Err(_) => Err(anyhow!(
            "浏览器容器整体超时（{}秒），{} 个 action 未全部完成",
            overall_timeout.as_secs(),
            config.actions.len()
        )),
    }
}

/// 内部：执行所有 action（无超时包装）
async fn execute_actions(
    config: &BrowserContainerConfig,
    external_input_ports: &HashMap<String, Value>,
) -> Result<ContainerResult> {
    let mut output_ports: HashMap<String, Value> = HashMap::new();
    // Merge external input_ports with accumulated output_ports for intra-container data flow
    let mut resolved_inputs = external_input_ports.clone();

    // ── 使用 WebBridge 作为唯一浏览器后端 ──
    tracing::info!("使用 WebBridge 扩展（WebSocket）");

    // NOTE: 当前使用 match 分发 31 个 action，模式清晰但文件较长。
    // 如果 action 数量继续增长，考虑参考 executor.rs 的注册表模式重构。
    for action in &config.actions {
        tracing::info!(
            "BrowserContainer — 执行 action: {} ({})",
            action.label,
            action.action_type
        );

        match action.action_type.as_str() {
            "navigate" => {
                let url = action
                    .config
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("about:blank");
                let wait_until = action
                    .config
                    .get("wait_until")
                    .and_then(|v| v.as_str())
                    .unwrap_or("load");
                let new_tab = action
                    .config
                    .get("new_tab")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let params = serde_json::json!({
                    "url": url,
                    "wait_until": wait_until,
                    "newTab": new_tab,
                });
                send_browser_action("navigate", &params)
                    .await
                    .map_err(|e| anyhow!("导航失败: {}", e))?;
            }

            "wait" => {
                // 支持两种模式：时间等待（ms）和选择器等待（selector）
                if let Some(ms) = action.config.get("ms").and_then(|v| v.as_u64()) {
                    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                } else {
                    let selector = require_selector(action)?;
                    let timeout = action
                        .config
                        .get("timeout")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(5000);

                    let params = serde_json::json!({
                        "selector": selector,
                        "timeout": timeout,
                    });
                    send_browser_action("wait_for", &params)
                        .await
                        .map_err(|e| anyhow!("等待失败: {}", e))?;
                }
            }

            "click" => {
                let selector = require_selector(action)?;

                let params = serde_json::json!({
                    "selector": selector,
                });
                send_browser_action("click", &params)
                    .await
                    .map_err(|e| anyhow!("点击失败: {}", e))?;
            }

            "scroll" => {
                let y = action
                    .config
                    .get("y")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                // Py sidecar: scroll_to(to: "bottom"|"top"|pixels)
                let to: serde_json::Value = if y > 0.0 {
                    serde_json::json!(y)
                } else {
                    serde_json::json!("bottom")
                };
                let params = serde_json::json!({
                    "to": to,
                });
                send_browser_action("scroll", &serde_json::json!({ "y": to }))
                    .await
                    .map_err(|e| anyhow!("滚动失败: {}", e))?;
            }

            "input" | "fill" => {
                let selector = require_selector(action)?;
                // 优先从 resolved_inputs（连线传入 + 前置 action 输出）获取值
                let value = resolved_inputs
                    .get(&format!("{}_in", &action.id))
                    .and_then(|v| v.as_str())
                    .or_else(|| resolved_inputs.get(&action.id).and_then(|v| v.as_str()))
                    .or_else(|| action.config.get("value").and_then(|v| v.as_str()))
                    .unwrap_or("");
                let clear = action
                    .config
                    .get("clear")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                let params = serde_json::json!({
                    "selector": selector,
                    "value": value,
                    "clear": clear,
                });
                send_browser_action("fill", &params)
                    .await
                    .map_err(|e| anyhow!("填写失败: {}", e))?;
            }

            "extract" => {
                let selector = action
                    .config
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .unwrap_or("body");
                let mode = action
                    .config
                    .get("mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("text");

                // Py sidecar has extract_text / extract_html / extract_attribute (no "extract")
                let (py_action, py_params) = match mode {
                    "html" => ("extract_html", serde_json::json!({ "selector": selector })),
                    "attr" => {
                        let attribute = action
                            .config
                            .get("attribute")
                            .and_then(|v| v.as_str())
                            .unwrap_or("href");
                        (
                            "extract_attribute",
                            serde_json::json!({ "selector": selector, "attribute": attribute }),
                        )
                    }
                    _ => ("extract_text", serde_json::json!({ "selector": selector })),
                };
                let resp = send_browser_action(py_action, &py_params)
                    .await
                    .map_err(|e| anyhow!("提取失败: {}", e))?;

                let data = resp
                    .get("data")
                    .or_else(|| resp.get("result"))
                    .cloned()
                    .unwrap_or(Value::Null);
                output_ports.insert(action.id.clone(), data);
            }

            "screenshot" => {
                let full_page = action
                    .config
                    .get("full_page")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let params = serde_json::json!({
                    "fullPage": full_page,
                });
                let resp = send_browser_action("screenshot", &params)
                    .await
                    .map_err(|e| anyhow!("截图失败: {}", e))?;

                let data = resp
                    .get("data")
                    .or_else(|| resp.get("screenshot"))
                    .cloned()
                    .unwrap_or(Value::Null);
                output_ports.insert(action.id.clone(), data);
            }

            "evaluate" => {
                let script = action
                    .config
                    .get("script")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let params = serde_json::json!({
                    "script": script,
                });
                let resp = send_browser_action("evaluate", &params)
                    .await
                    .map_err(|e| anyhow!("执行JS失败: {}", e))?;

                let data = resp
                    .get("data")
                    .or_else(|| resp.get("result"))
                    .cloned()
                    .unwrap_or(Value::Null);
                output_ports.insert(action.id.clone(), data);
            }

            "get_title" => {
                let params = serde_json::json!({
                    "script": "document.title",
                });
                let resp = send_browser_action("evaluate", &params)
                    .await
                    .map_err(|e| anyhow!("获取标题失败: {}", e))?;

                let title = resp
                    .get("data")
                    .or_else(|| resp.get("result"))
                    .cloned()
                    .unwrap_or(Value::String("".to_string()));
                output_ports.insert(action.id.clone(), title);
            }

            // ─── v1.1+ 扩展动作 ───
            "extract_table" => {
                let selector = action
                    .config
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .unwrap_or("table");
                let params = serde_json::json!({ "selector": selector });
                let resp = send_browser_action("extract_table", &params)
                    .await
                    .map_err(|e| anyhow!("表格提取失败: {}", e))?;
                let data = resp.get("data").cloned().unwrap_or(Value::Null);
                output_ports.insert(action.id.clone(), data);
            }

            "extract_links" => {
                let selector = action
                    .config
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .unwrap_or("body");
                let params = serde_json::json!({ "selector": selector });
                let resp = send_browser_action("extract_links", &params)
                    .await
                    .map_err(|e| anyhow!("链接提取失败: {}", e))?;
                let data = resp.get("data").cloned().unwrap_or(Value::Null);
                output_ports.insert(action.id.clone(), data);
            }

            "select" => {
                let selector = require_selector(action)?;
                let value = action
                    .config
                    .get("value")
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        resolved_inputs
                            .get(&format!("{}_in", &action.id))
                            .and_then(|v| v.as_str())
                    })
                    .unwrap_or("");
                let params = serde_json::json!({ "selector": selector, "value": value });
                send_browser_action("select", &params)
                    .await
                    .map_err(|e| anyhow!("下拉选择失败: {}", e))?;
            }

            "check" => {
                let selector = require_selector(action)?;
                let checked = action
                    .config
                    .get("checked")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let params = serde_json::json!({ "selector": selector, "checked": checked });
                send_browser_action("check", &params)
                    .await
                    .map_err(|e| anyhow!("勾选失败: {}", e))?;
            }

            "hover" => {
                let selector = require_selector(action)?;
                // Playwright hover via evaluate (no native hover action in sidecar)
                let script = format!(
                    "document.querySelector({})?.dispatchEvent(new MouseEvent('mouseover', {{bubbles:true}}))",
                    serde_json::to_string(selector).unwrap_or_default()
                );
                let params = serde_json::json!({ "expression": script });
                send_browser_action("evaluate", &params)
                    .await
                    .map_err(|e| anyhow!("悬停失败: {}", e))?;
            }

            "new_page" => {
                let url = action.config.get("url").and_then(|v| v.as_str());
                let params = serde_json::json!({ "url": url });
                send_browser_action("new_page", &params)
                    .await
                    .map_err(|e| anyhow!("新建标签页失败: {}", e))?;
            }

            "close_page" => {
                let index = action.config.get("index").and_then(|v| v.as_u64());
                let params = serde_json::json!({ "index": index });
                send_browser_action("close_tab", &serde_json::json!({ "tabId": index }))
                    .await
                    .map_err(|e| anyhow!("关闭标签页失败: {}", e))?;
            }

            "switch_page" => {
                let index = action
                    .config
                    .get("index")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let params = serde_json::json!({ "index": index });
                send_browser_action("switch_page", &params)
                    .await
                    .map_err(|e| anyhow!("切换标签页失败: {}", e))?;
            }

            "pages" => {
                let resp = send_browser_action("list_tabs", &serde_json::json!({}))
                    .await
                    .map_err(|e| anyhow!("获取标签页列表失败: {}", e))?;
                let data = resp.get("data").cloned().unwrap_or(Value::Null);
                output_ports.insert(action.id.clone(), data);
            }

            "cookies" => {
                let cookie_action = action
                    .config
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("get");
                let mut params = serde_json::json!({ "action": cookie_action });
                if cookie_action == "set" {
                    if let Some(cookies) = action.config.get("cookies") {
                        params["cookies"] = cookies.clone();
                    }
                }
                let resp = send_browser_action("cookies", &params)
                    .await
                    .map_err(|e| anyhow!("Cookie 操作失败: {}", e))?;
                let data = resp.get("data").cloned().unwrap_or(Value::Null);
                output_ports.insert(action.id.clone(), data);
            }

            "set_headers" => {
                let headers = action
                    .config
                    .get("headers")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));
                let params = serde_json::json!({ "headers": headers });
                send_browser_action("set_headers", &params)
                    .await
                    .map_err(|e| anyhow!("设置请求头失败: {}", e))?;
            }

            "back" => {
                send_browser_action("go_back", &serde_json::json!({}))
                    .await
                    .map_err(|e| anyhow!("后退失败: {}", e))?;
            }

            "forward" => {
                send_browser_action("go_forward", &serde_json::json!({}))
                    .await
                    .map_err(|e| anyhow!("前进失败: {}", e))?;
            }

            "reload" => {
                send_browser_action("reload", &serde_json::json!({}))
                    .await
                    .map_err(|e| anyhow!("刷新失败: {}", e))?;
            }

            "current_url" => {
                let resp = send_browser_action("current_url", &serde_json::json!({}))
                    .await
                    .map_err(|e| anyhow!("获取URL失败: {}", e))?;
                let data = resp.get("data").cloned().unwrap_or(Value::Null);
                output_ports.insert(action.id.clone(), data);
            }

            "pdf" => {
                let path = action
                    .config
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("output.pdf");
                send_browser_action("save_as_pdf", &serde_json::json!({}))
                    .await
                    .map_err(|e| anyhow!("生成PDF失败: {}", e))?;
            }

            // ─── 智能等待 (v2) ───
            "wait_network_idle" => {
                let timeout = action
                    .config
                    .get("timeout")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30000);
                let params = serde_json::json!({ "timeout_ms": timeout });
                send_browser_action("wait_network_idle", &params)
                    .await
                    .map_err(|e| anyhow!("等待网络空闲失败: {}", e))?;
            }

            "wait_load_state" => {
                let state = action
                    .config
                    .get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("load");
                let timeout = action
                    .config
                    .get("timeout")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30000);
                let params = serde_json::json!({ "state": state, "timeout_ms": timeout });
                send_browser_action("wait_load_state", &params)
                    .await
                    .map_err(|e| anyhow!("等待加载状态失败: {}", e))?;
            }

            "wait_url_contains" => {
                let substring = action
                    .config
                    .get("substring")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let timeout = action
                    .config
                    .get("timeout")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30000);
                let params = serde_json::json!({ "substring": substring, "timeout_ms": timeout });
                send_browser_action("wait_url_contains", &params)
                    .await
                    .map_err(|e| anyhow!("等待URL失败: {}", e))?;
            }

            // ─── 动作验证 (v2) ───
            "verify" => {
                let resp = send_browser_action("verify", &serde_json::json!({ "conditions": [] }))
                    .await
                    .map_err(|e| anyhow!("验证失败: {}", e))?;
                let data = resp.clone();
                let clean = data.get("clean").and_then(|v| v.as_bool()).unwrap_or(true);
                if !clean {
                    let count = data.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
                    tracing::warn!(
                        "ActionGuard: action '{}' 后检测到 {} 个问题",
                        action.label,
                        count
                    );
                }
                output_ports.insert(action.id.clone(), data);
            }

            // ─── 文件下载 (v1.6) ───
            "download" => {
                let save_dir = action
                    .config
                    .get("save_dir")
                    .and_then(|v| v.as_str())
                    .unwrap_or(".");
                let click_selector = action.config.get("click_selector").and_then(|v| v.as_str());
                let timeout = action
                    .config
                    .get("timeout")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30000);
                let params = serde_json::json!({
                    "selector": click_selector,
                    "saveAs": save_dir,
                });
                let resp = send_browser_action("download", &params)
                    .await
                    .map_err(|e| anyhow!("下载失败: {}", e))?;
                output_ports.insert(action.id.clone(), resp);
            }

            // ─── v1.7 办公自动化新动作 ───
            "upload" => {
                let selector = require_selector(action)?;
                let file_paths: Vec<String> = match action.config.get("file_paths") {
                    Some(v) if v.is_array() => v
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect(),
                    Some(v) => match v.as_str() {
                        Some(s) => vec![s.to_string()],
                        None => return Err(anyhow!("file_paths 必须是字符串或数组")),
                    },
                    _ => return Err(anyhow!("upload 缺少 file_paths 参数")),
                };
                let params = serde_json::json!({
                    "selector": selector,
                    "filePaths": file_paths,
                });
                send_browser_action("upload", &params)
                    .await
                    .map_err(|e| anyhow!("文件上传失败: {}", e))?;
            }

            "keyboard" => {
                let key = action
                    .config
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let text = action
                    .config
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let delay = action
                    .config
                    .get("delay")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                if key.is_empty() && text.is_empty() {
                    return Err(anyhow!("keyboard 需要 key 或 text 参数"));
                }
                // 映射到扩展工具：key → send_keys, text → key_type
                if !key.is_empty() {
                    let params = serde_json::json!({ "key": key });
                    send_browser_action("send_keys", &params)
                        .await
                        .map_err(|e| anyhow!("键盘操作失败: {}", e))?;
                }
                if !text.is_empty() {
                    let params = serde_json::json!({ "text": text, "delay": delay });
                    send_browser_action("key_type", &params)
                        .await
                        .map_err(|e| anyhow!("文本输入失败: {}", e))?;
                }
            }

            "double_click" => {
                let selector = require_selector(action)?;
                let timeout = action
                    .config
                    .get("timeout")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(5000);
                let params = serde_json::json!({
                    "selector": selector,
                    "timeout_ms": timeout,
                });
                send_browser_action("double_click", &params)
                    .await
                    .map_err(|e| anyhow!("双击失败: {}", e))?;
            }

            "drag_to" => {
                let source = action
                    .config
                    .get("source")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("drag_to 缺少 source 参数"))?;
                let target = action
                    .config
                    .get("target")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("drag_to 缺少 target 参数"))?;
                let mut params = serde_json::json!({
                    "source": source,
                    "target": target,
                });
                if let Some(sp) = action.config.get("source_position") {
                    params["source_position"] = sp.clone();
                }
                if let Some(tp) = action.config.get("target_position") {
                    params["target_position"] = tp.clone();
                }
                send_browser_action("drag_to", &params)
                    .await
                    .map_err(|e| anyhow!("拖拽失败: {}", e))?;
            }

            "context_menu" => {
                let selector = require_selector(action)?;
                let timeout = action
                    .config
                    .get("timeout")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(5000);
                let params = serde_json::json!({
                    "selector": selector,
                    "timeout_ms": timeout,
                });
                send_browser_action("context_menu", &params)
                    .await
                    .map_err(|e| anyhow!("右键菜单失败: {}", e))?;
            }

            "switch_frame" => {
                let selector = action
                    .config
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .unwrap_or("main");
                let params = serde_json::json!({
                    "selector": selector,
                });
                send_browser_action("switch_frame", &params)
                    .await
                    .map_err(|e| anyhow!("iframe 切换失败: {}", e))?;
            }

            "handle_dialog" => {
                let action_type = action
                    .config
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("accept");
                let prompt_text = action
                    .config
                    .get("prompt_text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let params = serde_json::json!({
                    "action": action_type,
                    "prompt_text": prompt_text,
                });
                send_browser_action("handle_dialog", &params)
                    .await
                    .map_err(|e| anyhow!("弹窗处理设置失败: {}", e))?;
            }

            "scroll_to_element" => {
                let selector = require_selector(action)?;
                let behavior = action
                    .config
                    .get("behavior")
                    .and_then(|v| v.as_str())
                    .unwrap_or("instant");
                let block = action
                    .config
                    .get("block")
                    .and_then(|v| v.as_str())
                    .unwrap_or("center");
                let params = serde_json::json!({
                    "selector": selector,
                    "behavior": behavior,
                    "block": block,
                });
                send_browser_action("scroll_to_element", &params)
                    .await
                    .map_err(|e| anyhow!("滚动到元素失败: {}", e))?;
            }

            // ─── v1.8 snapshot + @e ref（对标 Kimi WebBridge）───
            "snapshot" => {
                let resp = send_browser_action("snapshot", &serde_json::json!({}))
                    .await
                    .map_err(|e| anyhow!("snapshot 失败: {}", e))?;
                let data = resp.get("data").cloned().unwrap_or(Value::Null);
                output_ports.insert(action.id.clone(), data);
            }

            _ => {
                return Err(anyhow!(
                    "浏览器容器遇到未知 action 类型: '{}' (label: '{}')。支持的类型: navigate, wait, click, scroll, input, fill, extract, screenshot, evaluate, get_title, extract_table, extract_links, select, check, hover, new_page, close_page, switch_page, pages, cookies, set_headers, back, forward, reload, current_url, pdf, wait_network_idle, wait_load_state, wait_url_contains, verify, download, upload, keyboard, double_click, drag_to, context_menu, switch_frame, handle_dialog, scroll_to_element, snapshot",
                    action.action_type, action.label
                ));
            }
        }
        // 将当前 action 的输出合并到 resolved_inputs，供后续 action 引用
        for (k, v) in &output_ports {
            resolved_inputs.entry(k.clone()).or_insert(v.clone());
        }
    }

    Ok(ContainerResult {
        output_ports,
        error: None,
    })
}

// ─── NodeExecutor trait 实现 ───

#[derive(Default)]
pub struct BrowserContainerNode;

#[async_trait]
impl NodeExecutor for BrowserContainerNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "browser_container".into(),
            version: "1.0".into(),
            display_name: "浏览器容器".into(),
            description: "在浏览器中按顺序执行多个操作（导航、点击、提取、截图等），支持 DAG 连线".into(),
            category: "浏览器".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "output_ports".into(), data_type: "object".into(), required: false },
                crate::nodes::traits::PortDef { label: "_container_type".into(), data_type: "string".into(), required: false },
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "required": ["actions"],
                "properties": {
                    "browser": {"type": "string", "enum": ["chromium", "firefox", "webkit"], "description": "浏览器类型"},
                    "headless": {"type": "boolean", "description": "是否无头模式"},
                    "timeout": {"type": "number", "description": "超时毫秒数"},
                    "actions": {"type": "array", "description": "浏览器操作列表"}
                }
            }),
        }
    }

    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        // 解析配置
        let config: BrowserContainerConfig = serde_json::from_value(step.config.clone())
            .map_err(|e| anyhow!("浏览器容器配置解析失败: {}", e))?;

        // Phase 3: 占位符机制已在 executor 层处理，不需要容器内部 resolve

        // 从 ctx.input_ports 获取连线传入的数据
        let input_ports = ctx.input_ports.clone();

        // 执行
        let result = execute_browser_container(&config, &input_ports).await?;

        if let Some(ref err) = result.error {
            return Err(anyhow!("浏览器容器执行失败: {}", err));
        }

        // 返回 output_ports（含元数据）供下游节点使用
        let mut output = result.output_ports.clone();
        output.insert(
            "_container_type".to_string(),
            Value::String("browser".to_string()),
        );
        output.insert("_step_name".to_string(), Value::String(step.name.clone()));
        Ok(serde_json::to_value(&output).unwrap_or(Value::Null))
    }
}
