// commands/template.rs — 内置模板 + 文件系统模板库
// P4 增强：list_templates 从 templates/ 目录读取，load_template 加载完整 YAML
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
            id: "monitor-excel-alert",
            name: "网页监控 → Excel异常报告",
            description: "浏览器打开状态页 → 提取文本 → 条件判断 → Excel写入报告",
            yaml: "",
            json_data: Some(include_str!("../../../templates/monitor-excel-alert.json")),
        },
        BuiltinTemplate {
            id: "excel-to-word-batch",
            name: "Excel数据 → 批量Word通知书",
            description: "Excel读取员工数据 → 条件判断有数据 → Word生成通知书",
            yaml: "",
            json_data: Some(include_str!("../../../templates/excel-to-word-batch.json")),
        },
        BuiltinTemplate {
            id: "api-excel-word-branch",
            name: "JSON数据 → 条件分流 Word/Excel",
            description: "Excel读取订单 → 筛选大额订单 → 大额走Word合同，小额走Excel汇总",
            yaml: "",
            json_data: Some(include_str!("../../../templates/api-excel-word-branch.json")),
        },
        BuiltinTemplate {
            id: "word-extract-excel",
            name: "Word文档提取 → Excel汇总分析",
            description: "Word读取合同 → 条件判断大额 → Excel写入分析结果",
            yaml: "",
            json_data: Some(include_str!("../../../templates/word-extract-excel.json")),
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

/// 获取单个内置模板的 v2.0 JSON 数据（nodes + edges）
#[tauri::command]
pub async fn template_get_json(id: String) -> Result<Option<String>, String> {
    Ok(all_templates()
        .iter()
        .find(|t| t.id == id)
        .and_then(|t| t.json_data)
        .map(|s| s.to_string()))
}

// ═══════════════════════════════════════════════════════════
// Tauri 命令：文件系统模板（P4 新增）
// ═══════════════════════════════════════════════════════════

/// 解析模板 YAML 文件，提取元数据（名称、描述、步骤数）
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

/// 列出 templates/ 目录下的所有模板文件（含内置模板 + 文件系统模板）
#[tauri::command]
pub async fn list_templates() -> Result<Vec<serde_json::Value>, String> {
    let mut all = Vec::new();

    // ── 1. 内置模板（编译期嵌入） ──
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

                        // 跳过已存在的内置模板（避免重复）
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

/// 加载指定模板的完整 YAML 内容
#[tauri::command]
pub async fn load_template(id: String) -> Result<Option<String>, String> {
    // 先查内置模板
    if let Some(yaml) = all_templates()
        .iter()
        .find(|t| t.id == id)
        .map(|t| t.yaml.to_string())
    {
        return Ok(Some(yaml));
    }

    // 再查文件系统模板
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

/// 统计 YAML 字符串中的步骤数
fn count_yaml_steps(yaml: &str) -> usize {
    let parsed: Option<serde_yaml::Value> = serde_yaml::from_str(yaml).ok();
    parsed
        .as_ref()
        .and_then(|v| v.get("steps"))
        .and_then(|s| s.as_sequence())
        .map(|s| s.len())
        .unwrap_or(0)
}
