// commands/template.rs — 内置模板 + 文件系统模板库
use serde::Serialize;
use tracing::warn;

#[derive(Debug, Serialize)]
pub struct BuiltinTemplate {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub yaml: &'static str,
    pub json_data: Option<&'static str>,
}

/// 所有内置模板（编译期嵌入）
fn all_templates() -> Vec<BuiltinTemplate> {
    vec![
        BuiltinTemplate {
            id: "order-to-contracts",
            name: "订单逐笔生成合同",
            description: "Excel读取订单 → cursor游标逐行迭代 → Word生成合同 → 全部完成通知",
            yaml: "",
            json_data: Some(include_str!("../../../templates/order-to-contracts.json")),
        },
        BuiltinTemplate {
            id: "monitor-to-report",
            name: "网页监控 → 条件分流报告",
            description: "浏览器打开状态页 → 提取文本 → 条件判断 → 异常走Excel / 正常走Word",
            yaml: "",
            json_data: Some(include_str!("../../../templates/monitor-to-report.json")),
        },
    ]
}

// ═══════════════════════════════════════════════════════════
// Tauri 命令：内置模板
// ═══════════════════════════════════════════════════════════

/// 获取内置模板列表（不含 yaml 内容，仅元数据）
#[tauri::command]
pub async fn template_list() -> Result<Vec<serde_json::Value>, String> {
    tracing::info!("template_list 被调用");
    let builtins: Vec<serde_json::Value> = all_templates()
        .iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "name": t.name,
                "description": t.description,
                "source": "builtin",
                "category": "template",
            })
        })
        .collect();

    Ok(builtins)
}

/// 获取单个内置模板的完整 YAML
#[tauri::command]
pub async fn template_get_yaml(id: String) -> Result<Option<String>, String> {
    Ok(all_templates()
        .iter()
        .find(|t| t.id == id)
        .map(|t| t.yaml.to_string()))
}

/// 获取单个内置模板的 JSON 数据
#[tauri::command]
pub async fn template_get_json(id: String) -> Result<Option<String>, String> {
    Ok(all_templates()
        .iter()
        .find(|t| t.id == id)
        .and_then(|t| t.json_data)
        .map(|s| s.to_string()))
}

// ═══════════════════════════════════════════════════════════
// Tauri 命令：文件系统模板
// ═══════════════════════════════════════════════════════════

fn parse_template_meta(path: &std::path::Path) -> Option<serde_json::Value> {
    let content = std::fs::read_to_string(path).ok()?;
    let wf: crate::engine::workflow::Workflow = serde_yaml::from_str(&content).ok()?;

    let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");

    Some(serde_json::json!({
        "id": filename,
        "name": wf.name,
        "description": wf.description.unwrap_or_default(),
        "step_count": wf.steps.len(),
        "source": "filesystem",
        "category": "template",
        "file_path": path.to_string_lossy().to_string(),
    }))
}

#[tauri::command]
pub async fn list_templates() -> Result<Vec<serde_json::Value>, String> {
    let mut all = Vec::new();

    // ── 1. 内置模板 ──
    for t in all_templates() {
        let step_count = count_yaml_steps(t.yaml);
        all.push(serde_json::json!({
            "id": t.id,
            "name": t.name,
            "description": t.description,
            "step_count": step_count,
            "source": "builtin",
            "category": "template",
        }));
    }

    // ── 2. 文件系统模板目录 ──
    let template_dirs = vec![
        std::path::PathBuf::from("templates"),
        std::env::current_dir()
            .unwrap_or_default()
            .join("templates"),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("templates")))
            .unwrap_or_default(),
    ];

    for dir in template_dirs {
        if !dir.exists() {
            continue;
        }
        match std::fs::read_dir(&dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("yaml")
                        || path.extension().and_then(|e| e.to_str()) == Some("yml")
                    {
                        let filename = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown");

                        if all.iter().any(|t| {
                            t.get("id").and_then(|v| v.as_str()) == Some(filename)
                        }) {
                            continue;
                        }

                        if let Some(meta) = parse_template_meta(&path) {
                            all.push(meta);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("无法读取模板目录 {}: {}", dir.display(), e);
            }
        }
    }

    Ok(all)
}

#[tauri::command]
pub async fn load_template(id: String) -> Result<Option<String>, String> {
    if let Some(yaml) = all_templates()
        .iter()
        .find(|t| t.id == id)
        .map(|t| t.yaml.to_string())
    {
        return Ok(Some(yaml));
    }

    let template_dirs = vec![
        std::path::PathBuf::from("templates"),
        std::env::current_dir()
            .unwrap_or_default()
            .join("templates"),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("templates")))
            .unwrap_or_default(),
    ];

    for dir in template_dirs {
        if !dir.exists() {
            continue;
        }
        for ext in &["yaml", "yml"] {
            let path = dir.join(format!("{}.{}", id, ext));
            if path.exists() {
                match std::fs::read_to_string(&path) {
                    Ok(content) => return Ok(Some(content)),
                    Err(e) => {
                        warn!("读取模板文件失败 {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    Ok(None)
}

fn count_yaml_steps(yaml: &str) -> usize {
    let parsed: Option<serde_yaml::Value> = serde_yaml::from_str(yaml).ok();
    parsed
        .as_ref()
        .and_then(|v| v.get("steps"))
        .and_then(|s| s.as_sequence())
        .map(|s| s.len())
        .unwrap_or(0)
}
