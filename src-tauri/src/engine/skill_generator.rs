// engine/skill_generator.rs — Auto-generate SKILL.md from workflow definitions.
//
// Inspired by CLI-Anything's Phase 6.5: skill_generator.py (18KB).

use crate::engine::workflow::Workflow;

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
    md.push_str("version: 1.0.0\n");
    md.push_str("tags: [workflow]\n");
    md.push_str("---\n\n");

    // Title
    md.push_str(&format!("# {}\n\n", workflow.name));
    if description.is_empty() {
        md.push_str(&format!("{}\n\n", desc_line));
    } else {
        md.push_str(&format!("{}\n\n", description));
    }

    // Steps section
    md.push_str("## Steps\n\n");
    if workflow.steps.is_empty() {
        md.push_str("*(no steps)*\n\n");
    } else {
        md.push_str("| # | Step | Type | Description |\n");
        md.push_str("|---|------|------|-------------|\n");
        for (i, step) in workflow.steps.iter().enumerate() {
            let desc = step_description(step);
            md.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                i + 1,
                &step.name,
                &step.step_type,
                desc
            ));
        }
        md.push_str("\n");
    }

    // Configuration section
    if !workflow.steps.is_empty() {
        md.push_str("## Configuration\n\n");
        md.push_str("```json\n");
        for step in &workflow.steps {
            let config = &step.config;
            if config.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
                md.push_str(&format!("// {} config:\n", step.name));
                md.push_str(&format!(
                    "// {}\n",
                    serde_json::to_string_pretty(config).unwrap_or_default()
                ));
            }
        }
        md.push_str("```\n\n");
    }

    // Usage section
    md.push_str("## Usage\n\n");
    md.push_str("```bash\n");
    md.push_str("# Run this workflow:\n");
    md.push_str(&format!("wf-cli run-file <path-to-{}.wf.json>\n", name));
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
            format!("HTTP {} {}", method, url)
        }
        "script" => "Execute script".to_string(),
        "shell" => "Run shell command".to_string(),
        "notify" => "Send notification".to_string(),
        "delay" => {
            let ms = step
                .config
                .get("duration_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            format!("Delay {}ms", ms)
        }
        "json_parse" => "Parse JSON".to_string(),
        "text_template" => "Render text template".to_string(),
        "condition" => "Conditional branch".to_string(),
        "approval" => "Wait for approval".to_string(),
        t if t.contains("file") => "File operations".to_string(),
        t if t.contains("excel") => "Excel operations".to_string(),
        t if t.contains("word") => "Word operations".to_string(),
        t if t.contains("browser") => "Browser automation".to_string(),
        "cursor" | "loop" | "while" => "Iteration".to_string(),
        _ => step.step_type.clone(),
    }
}
