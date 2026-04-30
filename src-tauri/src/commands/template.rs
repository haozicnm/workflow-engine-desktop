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
            id: "api-data-excel",
            name: "API数据采集",
            description: "调用 HTTP API → 解析 JSON → 保存到本地文件",
            yaml: "",
            json_data: Some(include_str!("../../../templates/api-data-excel.json")),
        },
        BuiltinTemplate {
            id: "api-monitor",
            name: "API健康监控",
            description: "定时检查多个 API 健康状态 → 汇总输出",
            yaml: "",
            json_data: Some(include_str!("../../../templates/api-monitor.json")),
        },
        BuiltinTemplate {
            id: "web-content-fetch",
            name: "网页内容获取",
            description: "获取网页 → 提取正文 → 保存为文件",
            yaml: "",
            json_data: Some(include_str!("../../../templates/web-content-fetch.json")),
        },
        BuiltinTemplate {
            id: "data-clean-pipeline",
            name: "数据清洗流水线",
            description: "读取文件 → JSON解析 → 数组过滤转换 → 保存",
            yaml: "",
            json_data: Some(include_str!("../../../templates/data-clean-pipeline.json")),
        },
        BuiltinTemplate {
            id: "text-process",
            name: "文本批量处理",
            description: "读取文本 → 正则提取关键信息 → 格式化 → 保存",
            yaml: "",
            json_data: Some(include_str!("../../../templates/text-process.json")),
        },
        BuiltinTemplate {
            id: "json-extract",
            name: "JSON数据提取",
            description: "读取JSON文件 → JSONPath提取 → 数组排序 → 保存",
            yaml: "",
            json_data: Some(include_str!("../../../templates/json-extract.json")),
        },
        BuiltinTemplate {
            id: "ai-translate",
            name: "AI翻译工作流",
            description: "读取文本文件 → AI 翻译为英文 → 保存翻译结果",
            yaml: "",
            json_data: Some(include_str!("../../../templates/ai-translate.json")),
        },
        BuiltinTemplate {
            id: "ai-summarize",
            name: "AI文本摘要",
            description: "读取长文本 → AI 生成摘要 → 打印输出",
            yaml: "",
            json_data: Some(include_str!("../../../templates/ai-summarize.json")),
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
