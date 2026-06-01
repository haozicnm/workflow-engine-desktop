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
use std::path::{Path, PathBuf};
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
    /// Path to immutable bundle snapshot directory (None if no bundle generated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_path: Option<String>,
}

/// Live session tracking state for Agent introspection.
/// Stored in session.json in the run's preview directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveSession {
    pub run_id: String,
    pub workflow_name: String,
    pub status: String, // "running" | "paused" | "completed" | "failed" | "cancelled"
    pub step_index: usize,
    pub total_steps: usize,
    pub current_step_id: Option<String>,
    pub current_step_name: Option<String>,
    pub current_step_type: Option<String>,
    pub latest_bundle_path: Option<String>,
    pub trajectory_summary: Vec<TrajectoryEntry>,
}

/// Compact step→preview mapping for trajectory_summary (avoids reading full trajectory.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryEntry {
    pub step_id: String,
    pub step_type: String,
    pub status: String,
    pub summary: String,
    pub bundle_path: Option<String>,
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
        bundle_path: None,
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
        bundle_path: None,
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
        bundle_path: None,
    }
}

fn build_preview(step: &Step, output: &Value) -> (String, Value) {
    match step.step_type.as_str() {
        "http" => http_preview(step, output),
        "script" => script_preview(step, output),
        "shell" => shell_preview(output),
        "delay" => delay_preview(step),
        "approval" => approval_preview(output),
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
    let method = step
        .config
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET");
    let url = step
        .config
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
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
    let script = step
        .config
        .get("script")
        .and_then(|v| v.as_str())
        .unwrap_or("");
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
    let exit_code = output
        .get("exit_code")
        .and_then(|v| v.as_i64())
        .unwrap_or(-1);
    let stdout = output.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
    let stderr = output.get("stderr").and_then(|v| v.as_str()).unwrap_or("");
    let stdout_size = stdout.len();
    let stderr_size = stderr.len();
    let stdout_preview = truncate_str(stdout, 200);

    let summary = format!(
        "Shell → 退出码 {} (stdout {}B, stderr {}B)",
        exit_code, stdout_size, stderr_size
    );
    let detail = serde_json::json!({
        "exit_code": exit_code,
        "stdout_size": stdout_size,
        "stderr_size": stderr_size,
        "stdout_preview": stdout_preview,
    });
    (summary, detail)
}

fn delay_preview(step: &Step) -> (String, Value) {
    let ms = step
        .config
        .get("duration_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let summary = format!("延迟 {}ms", ms);
    let detail = serde_json::json!({"duration_ms": ms});
    (summary, detail)
}

fn approval_preview(output: &Value) -> (String, Value) {
    let decision = output
        .get("decision")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let reason = output
        .get("recommendation_reason")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let summary = format!("审批 → {}", decision);
    let detail = serde_json::json!({
        "decision": decision,
        "recommendation_reason": reason,
    });
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
                    let path = action
                        .get("params")
                        .and_then(|p| p.get("path"))
                        .or_else(|| action.get("config").and_then(|c| c.get("path")))
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let content_size = action_output
                        .and_then(|o| o.get("content"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.len())
                        .unwrap_or(0);
                    serde_json::json!({"type": action_type, "path": path, "content_size": content_size})
                }
                "list" | "glob" => {
                    let count = action_output
                        .and_then(|o| {
                            o.as_array().map(|a| a.len()).or_else(|| {
                                o.get("count").and_then(|v| v.as_u64()).map(|c| c as usize)
                            })
                        })
                        .unwrap_or(0);
                    serde_json::json!({"type": action_type, "count": count})
                }
                "copy" | "move" => {
                    let from = action
                        .get("params")
                        .and_then(|p| p.get("from"))
                        .or_else(|| action.get("config").and_then(|c| c.get("from")))
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let to = action
                        .get("params")
                        .and_then(|p| p.get("to"))
                        .or_else(|| action.get("config").and_then(|c| c.get("to")))
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    serde_json::json!({"type": action_type, "from": from, "to": to})
                }
                "delete" | "exists" => {
                    let path = action
                        .get("params")
                        .and_then(|p| p.get("path"))
                        .or_else(|| action.get("config").and_then(|c| c.get("path")))
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    serde_json::json!({"type": action_type, "path": path})
                }
                "grep" => {
                    let pattern = action
                        .get("params")
                        .and_then(|p| p.get("pattern"))
                        .or_else(|| action.get("config").and_then(|c| c.get("pattern")))
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let matches = action_output
                        .and_then(|o| o.get("count").and_then(|v| v.as_u64()))
                        .unwrap_or(0);
                    serde_json::json!({"type": action_type, "pattern": pattern, "matches": matches})
                }
                _ => serde_json::json!({"type": action_type}),
            };
            action_details.push(info);
        }
    }

    let summary = format!(
        "文件操作 ({} 动作: {})",
        actions,
        action_details
            .iter()
            .filter_map(|a| a.get("type").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let detail = serde_json::json!({"actions": action_details});
    (summary, detail)
}

fn excel_container_preview(step: &Step, output: &Value) -> (String, Value) {
    let file_path = step
        .config
        .get("file_path")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let mut action_details = Vec::new();

    if let Some(acts) = &step.actions {
        for action in acts {
            let action_type = action.get("type").and_then(|v| v.as_str()).unwrap_or("?");
            let action_id = action.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            let action_output = output.get(action_id);
            let info = match action_type {
                "read" => {
                    let rows = action_output
                        .and_then(|o| o.as_array().map(|a| a.len()))
                        .unwrap_or(0);
                    let cols = action_output
                        .and_then(|o| {
                            o.as_array()
                                .and_then(|a| a.first())
                                .and_then(|r| r.as_array().map(|c| c.len()))
                        })
                        .unwrap_or(0);
                    serde_json::json!({"type": "read", "rows": rows, "cols": cols})
                }
                "write" | "create" => {
                    let rows = action
                        .get("params")
                        .and_then(|p| p.get("value"))
                        .and_then(|v| v.as_array().map(|a| a.len()))
                        .unwrap_or(0);
                    serde_json::json!({"type": action_type, "rows_written": rows})
                }
                "filter" | "sort" => {
                    let result_rows = action_output
                        .and_then(|o| o.as_array().map(|a| a.len()))
                        .unwrap_or(0);
                    serde_json::json!({"type": action_type, "result_rows": result_rows})
                }
                _ => serde_json::json!({"type": action_type}),
            };
            action_details.push(info);
        }
    }

    let summary = format!(
        "Excel [{}] {} 动作",
        truncate_str(file_path, 40),
        action_details.len()
    );
    let detail = serde_json::json!({"file_path": file_path, "actions": action_details});
    (summary, detail)
}

fn word_container_preview(step: &Step, output: &Value) -> (String, Value) {
    let file_path = step
        .config
        .get("file_path")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let mut action_details = Vec::new();

    if let Some(acts) = &step.actions {
        for action in acts {
            let action_type = action.get("type").and_then(|v| v.as_str()).unwrap_or("?");
            let action_id = action.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            let action_output = output.get(action_id);
            let info = match action_type {
                "read" => {
                    let content_size = action_output
                        .and_then(|o| o.get("content"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.len())
                        .unwrap_or(0);
                    serde_json::json!({"type": "read", "content_size": content_size})
                }
                "write" | "create" => {
                    let content_size = action
                        .get("params")
                        .and_then(|p| p.get("value"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.len())
                        .unwrap_or(0);
                    serde_json::json!({"type": action_type, "content_size": content_size})
                }
                _ => serde_json::json!({"type": action_type}),
            };
            action_details.push(info);
        }
    }

    let summary = format!(
        "Word [{}] {} 动作",
        truncate_str(file_path, 40),
        action_details.len()
    );
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
                    let url = action
                        .get("config")
                        .and_then(|c| c.get("url"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    serde_json::json!({"type": action_type, "url": truncate_str(url, 60)})
                }
                "screenshot" => {
                    serde_json::json!({"type": "screenshot", "captured": true})
                }
                "extract" | "extract_table" | "extract_links" => {
                    let count = output.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
                    serde_json::json!({"type": action_type, "count": count})
                }
                _ => serde_json::json!({"type": action_type}),
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
    let result = output
        .get("result")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
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

// ── Bundle snapshot generation ──

/// Generate an immutable bundle snapshot for a step's output.
/// Returns the path to the bundle directory, or None if bundle not applicable.
pub fn bundle_step_output(run_id: &str, step: &Step, output: &Value) -> Option<PathBuf> {
    let bundle_dir = preview_dir(run_id).join("bundles").join(&step.id);
    if let Err(e) = fs::create_dir_all(&bundle_dir) {
        warn!("无法创建 bundle 目录 {:?}: {}", bundle_dir, e);
        return None;
    }

    let ok = match step.step_type.as_str() {
        "http" => bundle_http(&bundle_dir, step, output),
        "shell" => bundle_shell(&bundle_dir, output),
        "script" => bundle_script(&bundle_dir, output),
        t if t.contains("browser") => bundle_browser(&bundle_dir, output),
        t if t.contains("file") => bundle_file(&bundle_dir, step, output),
        t if t.contains("excel") => bundle_json(&bundle_dir, output),
        t if t.contains("word") => bundle_json(&bundle_dir, output),
        _ => bundle_generic(&bundle_dir, output),
    };

    if ok {
        Some(bundle_dir)
    } else {
        None
    }
}

fn bundle_http(dir: &Path, _step: &Step, output: &Value) -> bool {
    let status = output.get("status").and_then(|v| v.as_u64()).unwrap_or(0);
    let body = output.get("body").and_then(|v| v.as_str()).unwrap_or("");
    let headers = output.get("headers");

    let response_json = serde_json::json!({
        "status_code": status,
        "headers": headers,
        "body_preview": truncate_str(body, 500),
    });

    let mut ok = true;
    if let Err(e) = fs::write(
        dir.join("response.json"),
        serde_json::to_string_pretty(&response_json).unwrap_or_default(),
    ) {
        warn!("bundle http response.json: {}", e);
        ok = false;
    }
    if !body.is_empty() {
        if let Err(e) = fs::write(dir.join("body.txt"), body) {
            warn!("bundle http body.txt: {}", e);
            ok = false;
        }
    }
    ok
}

fn bundle_shell(dir: &Path, output: &Value) -> bool {
    let stdout = output.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
    let stderr = output.get("stderr").and_then(|v| v.as_str()).unwrap_or("");
    let exit_code = output
        .get("exit_code")
        .and_then(|v| v.as_i64())
        .unwrap_or(-1);

    let mut ok = true;
    if let Err(e) = fs::write(dir.join("stdout.txt"), stdout) {
        warn!("bundle shell stdout: {}", e);
        ok = false;
    }
    if !stderr.is_empty() {
        if let Err(e) = fs::write(dir.join("stderr.txt"), stderr) {
            warn!("bundle shell stderr: {}", e);
            ok = false;
        }
    }
    if let Err(e) = fs::write(dir.join("exit_code.txt"), exit_code.to_string()) {
        warn!("bundle shell exit_code: {}", e);
        ok = false;
    }
    ok
}

fn bundle_script(dir: &Path, output: &Value) -> bool {
    match serde_json::to_string_pretty(output) {
        Ok(json) => {
            if let Err(e) = fs::write(dir.join("result.json"), json) {
                warn!("bundle script result.json: {}", e);
                false
            } else {
                true
            }
        }
        Err(e) => {
            warn!("bundle script serialize: {}", e);
            false
        }
    }
}

fn bundle_json(dir: &Path, output: &Value) -> bool {
    bundle_script(dir, output) // same behavior: write output as pretty JSON
}

#[allow(dead_code)]
fn bundle_text(dir: &Path, output: &Value) -> bool {
    let text = output.get("result").and_then(|v| v.as_str()).unwrap_or("");
    if let Err(e) = fs::write(dir.join("output.txt"), text) {
        warn!("bundle text output.txt: {}", e);
        false
    } else {
        true
    }
}

fn bundle_browser(dir: &Path, output: &Value) -> bool {
    let mut ok = true;
    // Save screenshot if present
    if let Some(screenshot_path) = output.get("screenshot_path").and_then(|v| v.as_str()) {
        let src = std::path::Path::new(screenshot_path);
        if src.exists() {
            let dst = dir.join("screenshot.png");
            if let Err(e) = fs::copy(src, &dst) {
                warn!("bundle browser screenshot copy: {}", e);
                // Non-fatal: screenshot may not exist in headless env
            }
        }
    }
    // Save extraction results
    if let Some(extracted) = output.get("extracted") {
        if let Ok(json) = serde_json::to_string_pretty(extracted) {
            if let Err(e) = fs::write(dir.join("extracted.json"), json) {
                warn!("bundle browser extracted.json: {}", e);
                ok = false;
            }
        }
    }
    // Save page metadata
    let meta = serde_json::json!({
        "url": output.get("url").and_then(|v| v.as_str()).unwrap_or(""),
        "title": output.get("title").and_then(|v| v.as_str()).unwrap_or(""),
    });
    if let Err(e) = fs::write(
        dir.join("meta.json"),
        serde_json::to_string_pretty(&meta).unwrap_or_default(),
    ) {
        warn!("bundle browser meta.json: {}", e);
        ok = false;
    }
    ok
}

fn bundle_file(dir: &Path, _step: &Step, output: &Value) -> bool {
    match serde_json::to_string_pretty(output) {
        Ok(json) => {
            if let Err(e) = fs::write(dir.join("file_list.json"), json) {
                warn!("bundle file file_list.json: {}", e);
                false
            } else {
                true
            }
        }
        Err(e) => {
            warn!("bundle file serialize: {}", e);
            false
        }
    }
}

fn bundle_generic(dir: &Path, output: &Value) -> bool {
    match serde_json::to_string_pretty(output) {
        Ok(json) => {
            if let Err(e) = fs::write(dir.join("output.json"), json) {
                warn!("bundle generic output.json: {}", e);
                false
            } else {
                true
            }
        }
        Err(e) => {
            warn!("bundle generic serialize: {}", e);
            false
        }
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

// ── Live session management ──

fn session_path(run_id: &str) -> PathBuf {
    preview_dir(run_id).join("session.json")
}

/// Start a live session for a workflow run.
pub fn start_live_session(run_id: &str, workflow_name: &str, total_steps: usize) {
    let session = LiveSession {
        run_id: run_id.to_string(),
        workflow_name: workflow_name.to_string(),
        status: "running".to_string(),
        step_index: 0,
        total_steps,
        current_step_id: None,
        current_step_name: None,
        current_step_type: None,
        latest_bundle_path: None,
        trajectory_summary: Vec::new(),
    };
    write_session(run_id, &session);
}

/// Update live session after a step completes.
pub fn update_live_session(run_id: &str, preview: &StepPreview, step_index: usize) {
    let mut session = match read_session(run_id) {
        Some(s) => s,
        None => return,
    };
    session.step_index = step_index + 1;
    session.current_step_id = Some(preview.step_id.clone());
    session.current_step_name = Some(preview.step_name.clone());
    session.current_step_type = Some(preview.step_type.clone());
    session.latest_bundle_path = preview.bundle_path.clone();
    session.trajectory_summary.push(TrajectoryEntry {
        step_id: preview.step_id.clone(),
        step_type: preview.step_type.clone(),
        status: preview.status.clone(),
        summary: preview.summary.clone(),
        bundle_path: preview.bundle_path.clone(),
    });
    write_session(run_id, &session);
}

/// Get live session status (for Agent introspection via wf-cli).
pub fn get_live_status(run_id: &str) -> Option<LiveSession> {
    read_session(run_id)
}

/// Mark live session as completed/failed/cancelled.
pub fn stop_live_session(run_id: &str, final_status: &str) {
    let mut session = match read_session(run_id) {
        Some(s) => s,
        None => return,
    };
    session.status = final_status.to_string();
    write_session(run_id, &session);
}

/// List all active (running/paused) sessions.
pub fn list_live_sessions() -> Vec<LiveSession> {
    let base = crate::engine::plugin_manager::workflow_engine_dir().join("previews");
    let mut sessions = Vec::new();
    if let Ok(entries) = fs::read_dir(&base) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let sp = entry.path().join("session.json");
                if sp.exists() {
                    if let Ok(content) = fs::read_to_string(&sp) {
                        if let Ok(session) = serde_json::from_str::<LiveSession>(&content) {
                            if session.status == "running" || session.status == "paused" {
                                sessions.push(session);
                            }
                        }
                    }
                }
            }
        }
    }
    sessions.sort_by(|a, b| b.run_id.cmp(&a.run_id));
    sessions
}

fn read_session(run_id: &str) -> Option<LiveSession> {
    let path = session_path(run_id);
    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_session(run_id: &str, session: &LiveSession) {
    let dir = preview_dir(run_id);
    let _ = fs::create_dir_all(&dir);
    if let Ok(json) = serde_json::to_string_pretty(session) {
        let tmp = session_path(run_id).with_extension("tmp");
        let _ = fs::write(&tmp, json);
        let _ = fs::rename(&tmp, session_path(run_id));
    }
}
