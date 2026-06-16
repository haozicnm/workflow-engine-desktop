// nodes/mcp_node.rs — MCP Node Executor (transient mode)
// Spawns Python MCP server per step, JSON-RPC over stdio.
// Supports external plugins via plugin system (plugins/<name>/sidecars/)

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::plugin_manager;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{Context as _, Result};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize)]
struct ExternalMapping {
    #[serde(rename = "type")]
    node_type: String,
    script: String,
    tool: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ExternalConfig {
    mappings: Vec<ExternalMapping>,
}

struct McpMapping {
    server_script: &'static str,
    tool_name: &'static str,
}

/// 注册所有 MCP 节点映射：step_type → (Python server, MCP tool)
fn mcp_mappings() -> HashMap<String, McpMapping> {
    let mut m = HashMap::new();
    // ── 已有 ──
    m.insert(
        "http".into(),
        McpMapping {
            server_script: "mcp_http_server.py",
            tool_name: "http_request",
        },
    );
    m.insert(
        "json_parse".into(),
        McpMapping {
            server_script: "mcp_json_server.py",
            tool_name: "json_parse",
        },
    );
    // ── 仅保留无原生等价的 MCP 类型（已移除 10 个重复节点）──
    m.insert(
        "mcp_excel_csv".into(),
        McpMapping {
            server_script: "mcp_excel_server.py",
            tool_name: "excel_csv",
        },
    );
    m.insert(
        "mcp_word_write".into(),
        McpMapping {
            server_script: "mcp_word_server.py",
            tool_name: "word_write",
        },
    );
    m.insert(
        "mcp_word_create".into(),
        McpMapping {
            server_script: "mcp_word_server.py",
            tool_name: "word_create",
        },
    );
    m.insert(
        "mcp_word_replace".into(),
        McpMapping {
            server_script: "mcp_word_server.py",
            tool_name: "word_replace",
        },
    );
    m.insert(
        "mcp_word_merge".into(),
        McpMapping {
            server_script: "mcp_word_server.py",
            tool_name: "word_merge",
        },
    );
    m
}

/// 从 ~/.config/workflow-engine/mcp-external.json 加载外挂 MCP 映射
/// 兼容旧路径 ~/.hermes/mcp-external.json
fn load_external_mappings() -> Vec<(String, ExternalMapping)> {
    // 优先新路径
    let path = plugin_manager::mcp_external_path();
    if let Some(result) = try_load(&path) {
        return result;
    }

    // 回退旧路径
    let legacy = plugin_manager::legacy_hermes_dir().join("mcp-external.json");
    if let Some(result) = try_load(&legacy) {
        return result;
    }

    Vec::new()
}

fn try_load(path: &std::path::Path) -> Option<Vec<(String, ExternalMapping)>> {
    if !path.exists() {
        return None;
    }
    match std::fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str::<ExternalConfig>(&content) {
            Ok(cfg) => Some(
                cfg.mappings
                    .into_iter()
                    .map(|m| (m.node_type.clone(), m))
                    .collect(),
            ),
            Err(e) => {
                tracing::warn!(
                    "MCP external config parse error in {}: {}",
                    path.display(),
                    e
                );
                None
            }
        },
        Err(e) => {
            tracing::warn!("MCP external config read error: {}", e);
            None
        }
    }
}

/// 返回所有已知 MCP 类型（内置 + 外挂）
pub fn get_all_mcp_types() -> Vec<String> {
    let mut types: Vec<String> = mcp_mappings().keys().cloned().collect();
    for (t, _) in load_external_mappings() {
        if !types.contains(&t) {
            types.push(t);
        }
    }
    types
}

/// 跨平台 Python 检测
/// 优先使用 config 中配置的 python_path，否则自动检测
fn find_python() -> String {
    // 1. 按优先级尝试命令名
    #[cfg(target_os = "windows")]
    let candidates = ["python", "python3", "py"];
    #[cfg(not(target_os = "windows"))]
    let candidates = ["python3", "python"];

    for c in &candidates {
        if Command::new(c)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return c.to_string();
        }
    }

    // 2. Windows: 检查常见安装路径
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let paths = [
                format!("{}\\Programs\\Python\\Python312\\python.exe", local_app_data),
                format!("{}\\Programs\\Python\\Python311\\python.exe", local_app_data),
                format!("{}\\Programs\\Python\\Python310\\python.exe", local_app_data),
                format!("{}\\Programs\\Python\\Python39\\python.exe", local_app_data),
            ];
            for p in &paths {
                if std::path::Path::new(p).exists() {
                    return p.clone();
                }
            }
        }
    }

    // 3. 兜底
    "python".into()
}

fn resolve_path(script: &str) -> Result<std::path::PathBuf> {
    // 1. 环境变量
    if let Ok(d) = std::env::var("MCP_SERVERS_DIR") {
        let p = std::path::PathBuf::from(&d).join(script);
        if p.exists() {
            return Ok(p);
        }
    }
    // 2. 可执行文件同级 sidecars/
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("sidecars").join(script);
            if p.exists() {
                return Ok(p);
            }
        }
    }
    // 3. 已安装插件的 sidecars/
    let plugins_dir = plugin_manager::plugins_dir();
    if plugins_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
            for entry in entries.flatten() {
                let p = entry.path().join("sidecars").join(script);
                if p.exists() {
                    return Ok(p);
                }
            }
        }
    }
    // 4. 开发模式（CARGO_MANIFEST_DIR）
    let dev = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("sidecars")
        .join(script);
    if dev.exists() {
        return Ok(dev);
    }
    anyhow::bail!("MCP server not found: {}", script)
}

fn call_mcp(script: &str, tool: &str, args: &Value) -> Result<Value> {
    let path = resolve_path(script)?;
    let init = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    let call = serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":tool,"arguments":args}});
    let input = format!(
        "{}\n{}\n",
        serde_json::to_string(&init)?,
        serde_json::to_string(&call)?
    );

    let python = find_python();
    let mut child = Command::new(&python)
        .arg(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("spawn {} {}", python, path.display()))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes())?;
    }
    let stdout = child.stdout.take().context("no stdout")?;
    let mut last: Option<Value> = None;
    for line in BufReader::new(stdout).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let r: Value = serde_json::from_str(&line)?;
        if r.get("id").and_then(|v| v.as_i64()) == Some(2) && r.get("result").is_some() {
            last = Some(r);
        }
    }
    child.wait()?;
    let result = last.ok_or_else(|| anyhow::anyhow!("no result from MCP tool: {}", tool))?;
    let text = result
        .get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|i| i.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow::anyhow!("bad MCP response"))?;
    Ok(serde_json::from_str(text).unwrap_or(Value::String(text.into())))
}

pub struct McpNode {
    script: String,
    tool: String,
}

impl McpNode {
    pub fn new(s: &str, t: &str) -> Self {
        Self {
            script: s.into(),
            tool: t.into(),
        }
    }
}

#[async_trait]
impl NodeExecutor for McpNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "mcp".into(),
            version: "1.0".into(),
            display_name: "MCP 服务".into(),
            description: "调用 MCP 协议的 Python 服务器执行外部工具（HTTP/JSON/Excel CSV/Word 等）".into(),
            category: "AI".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "result".into(), data_type: "any".into(), required: false },
            ],
            config_schema: serde_json::json!({"type": "object"}),
        }
    }

    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _exec: &Arc<StepExecutor>,
    ) -> Result<Value> {
        call_mcp(&self.script, &self.tool, &step.config)
    }
}

pub fn create_mcp_executor(step_type: &str) -> Option<Box<dyn NodeExecutor>> {
    // 先查内置映射
    if let Some(m) = mcp_mappings().get(step_type) {
        return Some(Box::new(McpNode::new(m.server_script, m.tool_name)) as Box<dyn NodeExecutor>);
    }
    // 再查外挂映射
    for (t, ext) in load_external_mappings() {
        if t == step_type {
            // 外挂脚本支持相对路径（相对于 sidecars/）和绝对路径
            return Some(Box::new(McpNode::new(&ext.script, &ext.tool)) as Box<dyn NodeExecutor>);
        }
    }
    None
}
