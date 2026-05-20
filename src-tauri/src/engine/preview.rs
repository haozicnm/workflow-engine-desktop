// engine/preview.rs
// Preview/Trajectory layer: make step outputs visible to Agent and humans.
// Inspired by CLI-Anything's Bundle/Session/Trajectory model.
//
// Each completed step generates a StepPreview — a lightweight summary
// that captures the essence of what happened without raw output bloat.
// Previews are appended to trajectory.json in the run's preview directory.

use crate::engine::workflow::Step;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tracing::warn;

/// Lightweight step result summary for human and Agent consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepPreview {
    pub step_id: String,
    pub step_name: String,
    pub step_type: String,
    /// "completed" | "failed" | "skipped"
    pub status: String,
    /// Execution time in milliseconds.
    pub duration_ms: u64,
    /// One-line human-readable summary.
    pub summary: String,
    /// Structured detail (varies by step type).
    pub detail: Value,
}

/// Generate a preview from a completed step's output.
pub fn generate_step_preview(step: &Step, output: &Value, duration_ms: u64) -> StepPreview {
    let (summary, detail) = build_preview(step, output);

    StepPreview {
        step_id: step.id.clone(),
        step_name: step.name.clone(),
        step_type: step.step_type.clone(),
        status: "completed".to_string(),
        duration_ms,
        summary,
        detail,
    }
}

/// Generate a preview for a failed step.
pub fn generate_failed_preview(step: &Step, error: &str, duration_ms: u64) -> StepPreview {
    StepPreview {
        step_id: step.id.clone(),
        step_name: step.name.clone(),
        step_type: step.step_type.clone(),
        status: "failed".to_string(),
        duration_ms,
        summary: format!("✗ 失败: {}", truncate_str(error, 80)),
        detail: serde_json::json!({"error": error}),
    }
}

/// Generate a preview for a skipped step (runCondition false).
pub fn generate_skipped_preview(step: &Step, reason: &str) -> StepPreview {
    StepPreview {
        step_id: step.id.clone(),
        step_name: step.name.clone(),
        step_type: step.step_type.clone(),
        status: "skipped".to_string(),
        duration_ms: 0,
        summary: format!("⏭ 跳过: {}", reason),
        detail: serde_json::json!({"reason": reason}),
    }
}

fn build_preview(step: &Step, output: &Value) -> (String, Value) {
    match step.step_type.as_str() {
        "http" => http_preview(step, output),
        "script" => script_preview(step, output),
        "shell" => shell_preview(output),
        "notify" => notify_preview(output),
        "delay" => delay_preview(step),
        "clipboard" => clipboard_preview(output),
        "approval" => approval_preview(output),
        "json_parse" => json_parse_preview(output),
        "array_filter" => array_op_preview("filter", output),
        "array_sort" => array_op_preview("sort", output),
        "text_template" => text_template_preview(output),
        t if t.contains("file") => file_container_preview(step, output),
        t if t.contains("excel") => excel_container_preview(step, output),
        t if t.contains("word") => word_container_preview(step, output),
        t if t.contains("browser") => browser_container_preview(step, output),
        t if t.contains("logic") => logic_container_preview(output),
        "cursor" | "loop" | "while" => iteration_preview(step, output),
        _ => generic_preview(step, output),
    }
}

// ── Per-type preview builders ──

fn http_preview(step: &Step, output: &Value) -> (String, Value) {
    let method = step.config.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
    let url = step.config.get("url").and_then(|v| v.as_str()).unwrap_or("?");
    let status = output.get("status").and_then(|v| v.as_u64()).unwrap_or(0);
    let body = output.get("body").and_then(|v| v.as_str()).unwrap_or("");
    let body_size = body.len();
    let body_preview = truncate_str(body, 200);

    let summary = format!("HTTP {} {} → {}", method, truncate_str(url, 50), status);
    let detail = serde_json::json!({
        "method": method,
        "url": url,
        "status_code": status,
        "body_size": body_size,
        "body_preview": body_preview,
    });
    (summary, detail)
}

fn script_preview(step: &Step, output: &Value) -> (String, Value) {
    let script = step.config.get("script").and_then(|v| v.as_str()).unwrap_or("");
    let code_preview = truncate_str(script, 100);
    let result_type = describe_value(output);

    let summary = format!("脚本 → {}", result_type);
    let detail = serde_json::json!({
        "rhai_code": code_preview,
        "result_type": result_type,
    });
    (summary, detail)
}

fn shell_preview(output: &Value) -> (String, Value) {
    let exit_code = output.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(-1);
    let stdout = output.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
    let stderr = output.get("stderr").and_then(|v| v.as_str()).unwrap_or("");
    let stdout_size = stdout.len();
    let stderr_size = stderr.len();
    let stdout_preview = truncate_str(stdout, 200);

    let summary = format!("Shell → 退出码 {} (stdout {}B, stderr {}B)", exit_code, stdout_size, stderr_size);
    let detail = serde_json::json!({
        "exit_code": exit_code,
        "stdout_size": stdout_size,
        "stderr_size": stderr_size,
        "stdout_preview": stdout_preview,
    });
    (summary, detail)
}

fn notify_preview(output: &Value) -> (String, Value) {
    let notify_type = output.get("type").and_then(|v| v.as_str()).unwrap_or("?");
    let title = output.get("title").and_then(|v| v.as_str()).unwrap_or("?");
    let summary = format!("通知 [{}] {}", notify_type, truncate_str(title, 50));
    let detail = output.clone();
    (summary, detail)
}

fn delay_preview(step: &Step) -> (String, Value) {
    let ms = step.config.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(0);
    let summary = format!("延迟 {}ms", ms);
    let detail = serde_json::json!({"duration_ms": ms});
    (summary, detail)
}

fn clipboard_preview(output: &Value) -> (String, Value) {
    let action = output.get("action").and_then(|v| v.as_str()).unwrap_or("?");
    let text = output.get("text").and_then(|v| v.as_str()).unwrap_or("");
    let preview = truncate_str(text, 100);
    let summary = format!("剪贴板 [{}] {}", action, preview);
    let detail = output.clone();
    (summary, detail)
}

fn approval_preview(output: &Value) -> (String, Value) {
    let decision = output.get("decision").and_then(|v| v.as_str()).unwrap_or("?");
    let reason = output.get("recommendation_reason").and_then(|v| v.as_str()).unwrap_or("");
    let summary = format!("审批 → {}", decision);
    let detail = serde_json::json!({
        "decision": decision,
        "recommendation_reason": reason,
    });
    (summary, detail)
}

fn json_parse_preview(output: &Value) -> (String, Value) {
    let count = match output {
        Value::Array(arr) => arr.len(),
        Value::Object(obj) => obj.len(),
        _ => 0,
    };
    let summary = format!("JSON 解析 → {} 项", count);
    let detail = serde_json::json!({"item_count": count});
    (summary, detail)
}

fn array_op_preview(op: &str, output: &Value) -> (String, Value) {
    let source = output.get("source_count").and_then(|v| v.as_u64()).unwrap_or(0);
    let result = output.get("result_count").and_then(|v| v.as_u64()).unwrap_or(0);
    let summary = format!("数组{} {} → {}", op, source, result);
    let detail = serde_json::json!({"source_count": source, "result_count": result});
    (summary, detail)
}

fn text_template_preview(output: &Value) -> (String, Value) {
    let result = output.get("result").and_then(|v| v.as_str()).unwrap_or("");
    let preview = truncate_str(result, 100);
    let summary = format!("模板 → {}", preview);
    let detail = output.clone();
    (summary, detail)
}

fn file_container_preview(step: &Step, output: &Value) -> (String, Value) {
    let actions = step.actions.as_ref().map(|a| a.len()).unwrap_or(0);
    let mut action_details = Vec::new();

    if let Some(acts) = &step.actions {
        for action in acts {
            let action_type = action.get("type").and_then(|v| v.as_str()).unwrap_or("?");
            let action_id = action.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            let action_output = output.get(action_id);
            let info = match action_type {
                "read" | "write" | "append" => {
                    let path = action.get("params").and_then(|p| p.get("path"))
                        .or_else(|| action.get("config").and_then(|c| c.get("path")))
                        .and_then(|v| v.as_str()).unwrap_or("?");
                    let content_size = action_output.and_then(|o| o.get("content"))
                        .and_then(|v| v.as_str()).map(|s| s.len()).unwrap_or(0);
                    serde_json::json!({"type": action_type, "path": path, "content_size": content_size})
                }
                "list" | "glob" => {
                    let count = action_output.and_then(|o| {
                        o.as_array().map(|a| a.len())
                            .or_else(|| o.get("count").and_then(|v| v.as_u64()).map(|c| c as usize))
                    }).unwrap_or(0);
                    serde_json::json!({"type": action_type, "count": count})
                }
                "copy" | "move" => {
                    let from = action.get("params").and_then(|p| p.get("from"))
                        .or_else(|| action.get("config").and_then(|c| c.get("from")))
                        .and_then(|v| v.as_str()).unwrap_or("?");
                    let to = action.get("params").and_then(|p| p.get("to"))
                        .or_else(|| action.get("config").and_then(|c| c.get("to")))
                        .and_then(|v| v.as_str()).unwrap_or("?");
                    serde_json::json!({"type": action_type, "from": from, "to": to})
                }
                "delete" | "exists" => {
                    let path = action.get("params").and_then(|p| p.get("path"))
                        .or_else(|| action.get("config").and_then(|c| c.get("path")))
                        .and_then(|v| v.as_str()).unwrap_or("?");
                    serde_json::json!({"type": action_type, "path": path})
                }
                "grep" => {
                    let pattern = action.get("params").and_then(|p| p.get("pattern"))
                        .or_else(|| action.get("config").and_then(|c| c.get("pattern")))
                        .and_then(|v| v.as_str()).unwrap_or("?");
                    let matches = action_output.and_then(|o| o.get("count").and_then(|v| v.as_u64())).unwrap_or(0);
                    serde_json::json!({"type": action_type, "pattern": pattern, "matches": matches})
                }
                _ => serde_json::json!({"type": action_type})
            };
            action_details.push(info);
        }
    }

    let summary = format!("文件操作 ({} 动作: {})", actions, action_details.iter()
        .filter_map(|a| a.get("type").and_then(|v| v.as_str()))
        .collect::<Vec<_>>().join(", "));
    let detail = serde_json::json!({"actions": action_details});
    (summary, detail)
}

fn excel_container_preview(step: &Step, output: &Value) -> (String, Value) {
    let file_path = step.config.get("file_path").and_then(|v| v.as_str()).unwrap_or("?");
    let mut action_details = Vec::new();

    if let Some(acts) = &step.actions {
        for action in acts {
            let action_type = action.get("type").and_then(|v| v.as_str()).unwrap_or("?");
            let action_id = action.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            let action_output = output.get(action_id);
            let info = match action_type {
                "read" => {
                    let rows = action_output.and_then(|o| o.as_array().map(|a| a.len())).unwrap_or(0);
                    let cols = action_output.and_then(|o| o.as_array()
                        .and_then(|a| a.first())
                        .and_then(|r| r.as_array().map(|c| c.len()))).unwrap_or(0);
                    serde_json::json!({"type": "read", "rows": rows, "cols": cols})
                }
                "write" | "create" => {
                    let rows = action.get("params").and_then(|p| p.get("value"))
                        .and_then(|v| v.as_array().map(|a| a.len())).unwrap_or(0);
                    serde_json::json!({"type": action_type, "rows_written": rows})
                }
                "filter" | "sort" => {
                    let result_rows = action_output.and_then(|o| o.as_array().map(|a| a.len())).unwrap_or(0);
                    serde_json::json!({"type": action_type, "result_rows": result_rows})
                }
                _ => serde_json::json!({"type": action_type})
            };
            action_details.push(info);
        }
    }

    let summary = format!("Excel [{}] {} 动作", truncate_str(file_path, 40), action_details.len());
    let detail = serde_json::json!({"file_path": file_path, "actions": action_details});
    (summary, detail)
}

fn word_container_preview(step: &Step, output: &Value) -> (String, Value) {
    let file_path = step.config.get("file_path").and_then(|v| v.as_str()).unwrap_or("?");
    let mut action_details = Vec::new();

    if let Some(acts) = &step.actions {
        for action in acts {
            let action_type = action.get("type").and_then(|v| v.as_str()).unwrap_or("?");
            let action_id = action.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            let action_output = output.get(action_id);
            let info = match action_type {
                "read" => {
                    let content_size = action_output.and_then(|o| o.get("content"))
                        .and_then(|v| v.as_str()).map(|s| s.len()).unwrap_or(0);
                    serde_json::json!({"type": "read", "content_size": content_size})
                }
                "write" | "create" => {
                    let content_size = action.get("params").and_then(|p| p.get("value"))
                        .and_then(|v| v.as_str()).map(|s| s.len()).unwrap_or(0);
                    serde_json::json!({"type": action_type, "content_size": content_size})
                }
                _ => serde_json::json!({"type": action_type})
            };
            action_details.push(info);
        }
    }

    let summary = format!("Word [{}] {} 动作", truncate_str(file_path, 40), action_details.len());
    let detail = serde_json::json!({"file_path": file_path, "actions": action_details});
    (summary, detail)
}

fn browser_container_preview(step: &Step, output: &Value) -> (String, Value) {
    let mut action_details = Vec::new();

    if let Some(acts) = &step.actions {
        for action in acts {
            let action_type = action.get("type").and_then(|v| v.as_str()).unwrap_or("?");
            let info = match action_type {
                "navigate" | "current_url" => {
                    let url = action.get("config").and_then(|c| c.get("url")).and_then(|v| v.as_str()).unwrap_or("?");
                    serde_json::json!({"type": action_type, "url": truncate_str(url, 60)})
                }
                "screenshot" => {
                    serde_json::json!({"type": "screenshot", "captured": true})
                }
                "extract" | "extract_table" | "extract_links" => {
                    let count = output.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
                    serde_json::json!({"type": action_type, "count": count})
                }
                _ => serde_json::json!({"type": action_type})
            };
            action_details.push(info);
        }
    }

    let summary = format!("浏览器 {} 动作", action_details.len());
    let detail = serde_json::json!({"actions": action_details});
    (summary, detail)
}

fn logic_container_preview(output: &Value) -> (String, Value) {
    let branch = output.get("branch").and_then(|v| v.as_str()).unwrap_or("?");
    let result = output.get("result").and_then(|v| v.as_bool()).unwrap_or(false);
    let summary = format!("逻辑判断 → 分支 [{}] = {}", branch, result);
    let detail = output.clone();
    (summary, detail)
}

fn iteration_preview(step: &Step, output: &Value) -> (String, Value) {
    let count = output.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
    let summary = format!("迭代 → {} 项", count);
    let detail = serde_json::json!({"iterations": count, "type": step.step_type});
    (summary, detail)
}

fn generic_preview(_step: &Step, output: &Value) -> (String, Value) {
    let desc = describe_value(output);
    let summary = format!("→ {}", desc);
    let detail = output.clone();
    (summary, detail)
}

// ── Helpers ──

fn describe_value(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => format!("bool({})", b),
        Value::Number(n) => format!("number({})", n),
        Value::String(s) => format!("string({}B)", s.len()),
        Value::Array(arr) => format!("array({})", arr.len()),
        Value::Object(obj) => {
            let keys: Vec<&str> = obj.keys().map(|k| k.as_str()).take(5).collect();
            if keys.len() < obj.len() {
                format!("object({} keys: {}...)", obj.len(), keys.join(", "))
            } else {
                format!("object({} keys: {})", obj.len(), keys.join(", "))
            }
        }
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

// ── Trajectory file management ──

/// Get the preview directory for a given run.
pub fn preview_dir(run_id: &str) -> PathBuf {
    crate::engine::plugin_manager::workflow_engine_dir()
        .join("previews")
        .join(run_id)
}

fn trajectory_path(run_id: &str) -> PathBuf {
    preview_dir(run_id).join("trajectory.json")
}

/// Append a StepPreview to the run's trajectory file.
/// Creates the directory and file if they don't exist.
pub fn append_trajectory(run_id: &str, preview: &StepPreview) {
    let dir = preview_dir(run_id);
    if let Err(e) = fs::create_dir_all(&dir) {
        warn!("无法创建 preview 目录 {:?}: {}", dir, e);
        return;
    }

    let path = trajectory_path(run_id);

    // Read existing, append new entry, write back
    let mut trajectory: Vec<Value> = match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    trajectory.push(serde_json::json!(preview));

    let content = match serde_json::to_string_pretty(&trajectory) {
        Ok(s) => s,
        Err(e) => {
            warn!("trajectory 序列化失败: {}", e);
            return;
        }
    };

    // Atomic write: write to temp file then rename
    let tmp_path = path.with_extension("tmp");
    if let Err(e) = fs::File::create(&tmp_path).and_then(|mut f| f.write_all(content.as_bytes())) {
        warn!("无法写入 trajectory: {}", e);
        return;
    }
    if let Err(e) = fs::rename(&tmp_path, &path) {
        warn!("无法重命名 trajectory: {}", e);
    }
}

/// Read the trajectory for a run. Returns empty vec if not found.
pub fn read_trajectory(run_id: &str) -> Vec<StepPreview> {
    let path = trajectory_path(run_id);
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

/// List all runs that have preview data.
pub fn list_preview_runs() -> Vec<String> {
    let base = crate::engine::plugin_manager::workflow_engine_dir().join("previews");
    let mut runs = Vec::new();
    if let Ok(entries) = fs::read_dir(&base) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    runs.push(name.to_string());
                }
            }
        }
    }
    runs.sort();
    runs
}
