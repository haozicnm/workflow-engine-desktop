// engine/skill_generator.rs — Auto-generate SKILL.md from workflow definitions.
//
// v2: 利用 node-schema.json 的自描述能力，生成 Agent 可直接使用的 Skill 文档。
// 包含：步骤表、参数详情、变量引用、验证约束、执行说明。

use crate::engine::workflow::Workflow;
use crate::nodes::registry;

/// Generate a SKILL.md string from a workflow definition.
pub fn generate_skill(workflow: &Workflow) -> String {
    let name = slugify(&workflow.name);
    let description = workflow.description.as_deref().unwrap_or("");
    let desc_line = if description.is_empty() {
        format!("Execute the '{}' workflow", workflow.name)
    } else {
        description.to_string()
    };

    let mut md = String::new();

    // YAML frontmatter
    md.push_str("---\n");
    md.push_str(&format!("name: {}\n", name));
    md.push_str(&format!("description: \"{}\"\n", desc_line));
    md.push_str(&format!("version: {}\n",
        workflow.version.as_deref().unwrap_or("1.0")));
    md.push_str("tags: [workflow");
    if let Some(ref meta) = workflow.meta {
        for tag in &meta.tags {
            md.push_str(&format!(", {}", tag));
        }
    }
    md.push_str("]\n");
    md.push_str("---\n\n");

    // Title
    md.push_str(&format!("# {}\n\n", workflow.name));
    if !description.is_empty() {
        md.push_str(&format!("{}\n\n", description));
    }

    // Metadata
    if let Some(ref meta) = workflow.meta {
        if meta.author.is_some() || meta.created_at.is_some() {
            md.push_str("**元数据：**\n");
            if let Some(ref author) = meta.author {
                md.push_str(&format!("- 作者: {}\n", author));
            }
            if let Some(ref created) = meta.created_at {
                md.push_str(&format!("- 创建: {}\n", created));
            }
            md.push('\n');
        }
    }

    // Variables section
    if let Some(ref vars) = workflow.variables {
        if !vars.is_empty() {
            md.push_str("## 变量\n\n");
            md.push_str("| 变量名 | 默认值 |\n");
            md.push_str("|--------|--------|\n");
            for (key, val) in vars {
                let val_str = match val {
                    serde_json::Value::String(s) => s.clone(),
                    _ => serde_json::to_string(val).unwrap_or_default(),
                };
                md.push_str(&format!("| `{}` | `{}` |\n", key, val_str));
            }
            md.push('\n');
        }
    }

    // Steps section with rich details
    md.push_str("## 步骤\n\n");
    if workflow.steps.is_empty() {
        md.push_str("*(无步骤)*\n\n");
    } else {
        // Summary table
        md.push_str("| # | ID | 类型 | 名称 | 说明 |\n");
        md.push_str("|---|----|----|------|------|\n");
        for (i, step) in workflow.steps.iter().enumerate() {
            let desc = step_description(step);
            md.push_str(&format!(
                "| {} | `{}` | `{}` | {} | {} |\n",
                i + 1,
                &step.id,
                &step.step_type,
                &step.name,
                desc
            ));
        }
        md.push('\n');

        // Detailed step info with schema
        for step in &workflow.steps {
            md.push_str(&format!("### `{}` — {}\n\n", step.id, step.name));
            md.push_str(&format!("- **类型:** `{}`\n", step.step_type));

            // Show outputs from schema
            if let Some(manifest) = registry::get_node(&step.step_type) {
                if !manifest.outputs.is_empty() {
                    md.push_str("- **输出:**\n");
                    for out in &manifest.outputs {
                        md.push_str(&format!(
                            "  - `{}.{} ({})` — {}\n",
                            step.id, out.name, out.data_type.as_str(), out.desc
                        ));
                    }
                }

                // Show required params validation
                let required: Vec<_> = manifest.params.iter().filter(|p| p.required).collect();
                if !required.is_empty() {
                    md.push_str("- **必需参数:**\n");
                    for param in &required {
                        let val = step.config.get(&param.name);
                        let val_str = val
                            .map(|v| match v {
                                serde_json::Value::String(s) => format!("`\"{}\"`", s),
                                _ => format!("`{}`", v),
                            })
                            .unwrap_or_else(|| "*(未设置)*".to_string());
                        md.push_str(&format!("  - `{}`: {} — {}\n", param.name, val_str, param.desc.as_deref().unwrap_or("")));
                    }
                }
            }

            // Run condition
            if let Some(ref rc) = step.run_condition {
                md.push_str(&format!(
                    "- **条件执行:** 当 `{}` 的 branch = `{}` 时执行\n",
                    rc.ref_step, rc.when
                ));
            }

            // Error strategy
            if let Some(ref on_error) = step.on_error {
                use crate::engine::workflow::ErrorStrategy;
                match on_error {
                    ErrorStrategy::Fail => md.push_str("- **错误策略:** 终止\n"),
                    ErrorStrategy::Ignore => md.push_str("- **错误策略:** 忽略\n"),
                    ErrorStrategy::Branch { step_id } => {
                        md.push_str(&format!("- **错误策略:** 跳转到 `{}`\n", step_id))
                    }
                };
            }

            // Retry
            if let Some(ref retry) = step.retry {
                md.push_str(&format!(
                    "- **重试:** 最多 {} 次，间隔 {}ms\n",
                    retry.max, retry.delay_ms
                ));
            }

            md.push('\n');
        }
    }

    // Variable reference guide
    md.push_str("## 变量引用\n\n");
    md.push_str("在 config 中使用 `{{step_id.field}}` 引用上游步骤的输出：\n\n");
    md.push_str("```\n");
    for step in &workflow.steps {
        if let Some(manifest) = registry::get_node(&step.step_type) {
            for out in &manifest.outputs {
                md.push_str(&format!("{{{{step_{}.{}  →  {} ({})\n",
                    step.id, out.name, out.desc, out.data_type.as_str()));
            }
        }
    }
    md.push_str("```\n\n");

    // Usage section
    md.push_str("## 执行\n\n");
    md.push_str("```bash\n");
    md.push_str(&format!("# 通过 API 执行\n"));
    md.push_str(&format!("curl -X POST http://localhost:9700/api/runs \\\n"));
    md.push_str(&format!("  -H 'Content-Type: application/json' \\\n"));
    md.push_str(&format!("  -d '{{\"workflow_id\": \"<id>\"}}'\n"));
    md.push_str("```\n");

    md
}

fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn step_description(step: &crate::engine::workflow::Step) -> String {
    // 优先用 schema 生成精确描述
    if let Some(manifest) = registry::get_node(&step.step_type) {
        let required_missing: Vec<&str> = manifest
            .params
            .iter()
            .filter(|p| p.required)
            .filter(|p| {
                step.config
                    .get(&p.name)
                    .map(|v| v.is_null() || (v.is_string() && v.as_str().unwrap_or("").is_empty()))
                    .unwrap_or(true)
            })
            .map(|p| p.name.as_str())
            .collect();

        if !required_missing.is_empty() {
            return format!("⚠️ 缺少必需参数: {}", required_missing.join(", "));
        }
    }

    // 回退到类型特定描述
    match step.step_type.as_str() {
        "http" => {
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
            let url_short = if url.len() > 40 {
                format!("{}...", &url[..37])
            } else {
                url.to_string()
            };
            format!("HTTP {} {}", method, url_short)
        }
        "shell" => {
            let cmd = step
                .config
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let cmd_short = if cmd.len() > 40 {
                format!("{}...", &cmd[..37])
            } else {
                cmd.to_string()
            };
            format!("`{}`", cmd_short)
        }
        "script" => "执行 Rhai 脚本".to_string(),
        "delay" => {
            let ms = step
                .config
                .get("ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            format!("延迟 {}ms", ms)
        }
        "condition" => "条件分支".to_string(),
        "approval" => "等待审批".to_string(),
        t if t.contains("file") => "文件操作".to_string(),
        t if t.contains("excel") => "Excel 操作".to_string(),
        t if t.contains("word") => "Word 操作".to_string(),
        t if t.contains("browser") => "浏览器自动化".to_string(),
        "cursor" | "loop" | "while" => "循环迭代".to_string(),
        _ => step.step_type.clone(),
    }
}
