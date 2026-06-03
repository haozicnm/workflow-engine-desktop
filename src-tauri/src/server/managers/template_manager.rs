// server/managers/template_manager.rs — 模板库 API handler
//
// GET  /api/templates              — 列出所有模板
// GET  /api/templates/{name}       — 获取模板详情
// POST /api/templates/{name}/instantiate — 从模板创建工作流

use axum::{
    extract::Path,
    http::StatusCode,
    response::{Json, Response},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::server::handlers::{err_response, ok_response};
use crate::server::state;

// ═══════════════════════════════════════════════════════════
// 数据结构
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMeta {
    pub name: String,
    pub desc: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 模板分类（data, web, file, automation 等）
    #[serde(default = "default_category")]
    pub category: String,
}

fn default_category() -> String {
    "general".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParam {
    pub name: String,
    pub desc: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFile {
    pub meta: TemplateMeta,
    #[serde(default)]
    pub params: Vec<TemplateParam>,
    pub workflow: serde_json::Value,
}

/// 模板列表项（不含 workflow 详情）
#[derive(Debug, Serialize)]
pub struct TemplateListItem {
    pub filename: String,
    pub name: String,
    pub desc: String,
    pub tags: Vec<String>,
    pub category: String,
    pub params: Vec<TemplateParam>,
}

/// 模板详情（含 workflow）
#[derive(Debug, Serialize)]
pub struct TemplateDetail {
    pub filename: String,
    pub name: String,
    pub desc: String,
    pub tags: Vec<String>,
    pub category: String,
    pub params: Vec<TemplateParam>,
    pub workflow: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct InstantiateBody {
    /// 用户提供的参数值
    pub params: std::collections::HashMap<String, String>,
    /// 可选：覆盖工作流名称
    pub name: Option<String>,
}

// ═══════════════════════════════════════════════════════════
// 模板目录解析
// ═══════════════════════════════════════════════════════════

/// 模板库目录：~/.config/workflow-engine/workflows/library/
fn template_dir() -> PathBuf {
    // 优先使用 ~/.config/workflow-engine/workflows/library/
    if let Some(home) = dirs::home_dir() {
        let dir = home
            .join(".config")
            .join("workflow-engine")
            .join("workflows")
            .join("library");
        if dir.exists() {
            return dir;
        }
    }
    // 回退到 data_dir/templates/
    crate::data::paths::resolve_data_dir().join("templates")
}

/// 加载所有模板文件
fn load_all_templates() -> Result<Vec<(String, TemplateFile)>, String> {
    let dir = template_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut templates = Vec::new();
    let entries = std::fs::read_dir(&dir)
        .map_err(|e| format!("读取模板目录失败: {e}"))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.extension().is_some_and(|ext| ext == "yaml" || ext == "yml") {
            continue;
        }
        let filename = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let tmpl: TemplateFile = match serde_yaml::from_str(&content) {
            Ok(t) => t,
            Err(_) => continue,
        };

        templates.push((filename, tmpl));
    }

    templates.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(templates)
}

/// 加载单个模板
fn load_template(name: &str) -> Result<Option<(String, TemplateFile)>, String> {
    let dir = template_dir();

    // 尝试 .yaml 和 .yml
    for ext in &["yaml", "yml"] {
        let path = dir.join(format!("{name}.{ext}"));
        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("读取模板文件失败: {e}"))?;
            let tmpl: TemplateFile = serde_yaml::from_str(&content)
                .map_err(|e| format!("解析模板 YAML 失败: {e}"))?;
            return Ok(Some((name.to_string(), tmpl)));
        }
    }

    Ok(None)
}

// ═══════════════════════════════════════════════════════════
// Handler 函数
// ═══════════════════════════════════════════════════════════

/// GET /api/templates?q=xxx&category=data — 列出所有模板（支持搜索和分类过滤）
pub async fn template_list(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    let query = params.get("q").map(|s| s.to_lowercase());
    let category_filter = params.get("category").map(|s| s.as_str());

    match load_all_templates() {
        Ok(templates) => {
            let items: Vec<TemplateListItem> = templates
                .into_iter()
                .filter(|(_, tmpl)| {
                    // 分类过滤
                    if let Some(cat) = category_filter {
                        if tmpl.meta.category != cat {
                            return false;
                        }
                    }
                    // 搜索过滤
                    if let Some(ref q) = query {
                        let q = q.as_str();
                        tmpl.meta.name.to_lowercase().contains(q)
                            || tmpl.meta.desc.to_lowercase().contains(q)
                            || tmpl.meta.tags.iter().any(|t| t.to_lowercase().contains(q))
                            || tmpl.meta.category.to_lowercase().contains(q)
                    } else {
                        true
                    }
                })
                .map(|(filename, tmpl)| TemplateListItem {
                    filename,
                    name: tmpl.meta.name,
                    desc: tmpl.meta.desc,
                    tags: tmpl.meta.tags,
                    category: tmpl.meta.category,
                    params: tmpl.params,
                })
                .collect();
            ok_response(serde_json::json!({ "templates": items, "count": items.len() }))
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

/// GET /api/templates/{name} — 获取模板详情
pub async fn template_get(Path(name): Path<String>) -> Response {
    match load_template(&name) {
        Ok(Some((filename, tmpl))) => ok_response(TemplateDetail {
            filename,
            name: tmpl.meta.name,
            desc: tmpl.meta.desc,
            tags: tmpl.meta.tags,
            category: tmpl.meta.category,
            params: tmpl.params,
            workflow: tmpl.workflow,
        }),
        Ok(None) => err_response(
            StatusCode::NOT_FOUND,
            format!("模板 '{name}' 不存在"),
        ),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

/// POST /api/templates/{name}/instantiate — 从模板创建工作流
pub async fn template_instantiate(
    Path(name): Path<String>,
    Json(body): Json<InstantiateBody>,
) -> Response {
    let (_filename, tmpl) = match load_template(&name) {
        Ok(Some(t)) => t,
        Ok(None) => {
            return err_response(
                StatusCode::NOT_FOUND,
                format!("模板 '{name}' 不存在"),
            )
        }
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e),
    };

    // 检查 required 参数是否都提供了
    for param in &tmpl.params {
        if param.required && !body.params.contains_key(&param.name) && param.default.is_none() {
            return err_response(
                StatusCode::BAD_REQUEST,
                format!("缺少必需参数: {}", param.name),
            );
        }
    }

    // 构建变量映射：先填默认值，再用用户提供的值覆盖
    let mut variables = std::collections::HashMap::new();
    for param in &tmpl.params {
        if let Some(val) = body.params.get(&param.name) {
            variables.insert(
                param.name.clone(),
                serde_json::Value::String(val.clone()),
            );
        } else if let Some(default) = &param.default {
            variables.insert(
                param.name.clone(),
                serde_json::Value::String(default.clone()),
            );
        }
    }

    // 将模板 workflow 转为 JSON 字符串，替换参数占位符
    let wf_str = serde_json::to_string(&tmpl.workflow).unwrap_or_default();
    let mut wf_str = wf_str.clone();
    for (key, val) in &variables {
        let placeholder = format!("{{{{{key}}}}}");
        let replacement = val.as_str().unwrap_or("");
        wf_str = wf_str.replace(&placeholder, replacement);
    }

    let wf_value: serde_json::Value = match serde_json::from_str(&wf_str) {
        Ok(v) => v,
        Err(e) => {
            return err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("模板 workflow 解析失败: {e}"),
            )
        }
    };

    // 注入 variables 到 workflow
    let mut wf_obj = wf_value.clone();
    if let Some(obj) = wf_obj.as_object_mut() {
        obj.insert(
            "variables".to_string(),
            serde_json::to_value(&variables).unwrap_or_default(),
        );
    }

    // 转为 YAML 并尝试解析验证
    let yaml_str = serde_yaml::to_string(&wf_obj).unwrap_or_default();

    // 通过 parser 验证
    match crate::engine::parser::parse_workflow(&yaml_str) {
        Ok(_wf) => {
            // 创建工作流记录
            let app = state::get();
            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            let wf_name = body
                .name
                .unwrap_or_else(|| format!("{} (从模板)", tmpl.meta.name));

            match app.db.create_workflow(
                &id,
                &wf_name,
                &tmpl.meta.desc,
                &now,
                &now,
            ) {
                Ok(()) => {
                    // 保存 YAML
                    if let Err(e) = app.db.save_workflow_yaml(&id, &yaml_str) {
                        return err_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("保存工作流 YAML 失败: {e}"),
                        );
                    }
                    crate::server::events::emit(
                        "workflow-changed",
                        serde_json::json!({
                            "action": "instantiate",
                            "workflow_id": &id,
                            "template": &name,
                        }),
                    );
                    ok_response(serde_json::json!({
                        "workflow_id": id,
                        "name": wf_name,
                        "template": name,
                    }))
                }
                Err(e) => err_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("创建工作流失败: {e}"),
                ),
            }
        }
        Err(e) => err_response(
            StatusCode::BAD_REQUEST,
            format!("模板实例化后验证失败: {e}"),
        ),
    }
}

// ═══════════════════════════════════════════════════════════
// 保存为模板 + 导入导出
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct SaveAsTemplateBody {
    /// 模板名称
    pub name: String,
    /// 模板描述
    pub desc: String,
    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 分类
    #[serde(default = "default_category")]
    pub category: String,
    /// 模板参数定义（可选，从工作流变量推导）
    #[serde(default)]
    pub params: Vec<TemplateParam>,
}

/// POST /api/workflows/{id}/save-as-template — 将工作流保存为模板
pub async fn workflow_save_as_template(
    Path(id): Path<String>,
    Json(body): Json<SaveAsTemplateBody>,
) -> Response {
    let app = state::get();

    // 加载工作流
    let wf_record = match app.db.get_workflow(&id) {
        Ok(Some(wf)) => wf,
        Ok(None) => return err_response(StatusCode::NOT_FOUND, "工作流不存在"),
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    // 构建 workflow JSON（从 YAML）
    let workflow_json = match serde_yaml::from_str::<serde_json::Value>(&wf_record.yaml) {
        Ok(v) => v,
        Err(e) => return err_response(StatusCode::BAD_REQUEST, format!("YAML 解析失败: {e}")),
    };

    // 构建模板文件
    let tmpl = TemplateFile {
        meta: TemplateMeta {
            name: body.name.clone(),
            desc: body.desc.clone(),
            tags: body.tags.clone(),
            category: body.category.clone(),
        },
        params: body.params.clone(),
        workflow: workflow_json,
    };

    // 序列化为 YAML
    let yaml_str = match serde_yaml::to_string(&tmpl) {
        Ok(s) => s,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("序列化失败: {e}")),
    };

    // 保存到模板目录
    let dir = template_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("创建目录失败: {e}"));
    }

    let sanitized: String = body
        .name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c.is_ascii_whitespace() {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .replace(' ', "_")
        .to_lowercase();

    let path = dir.join(format!("{sanitized}.yaml"));
    if let Err(e) = std::fs::write(&path, &yaml_str) {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("写入失败: {e}"));
    }

    ok_response(serde_json::json!({
        "success": true,
        "template_name": sanitized,
        "path": path.to_string_lossy(),
        "name": body.name,
        "category": body.category,
    }))
}

/// POST /api/templates/import — 导入模板（从 YAML 字符串）
pub async fn template_import(
    Json(body): Json<serde_json::Value>,
) -> Response {
    let yaml_content = body
        .get("yaml")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // 验证模板格式
    let tmpl: TemplateFile = match serde_yaml::from_str(yaml_content) {
        Ok(t) => t,
        Err(e) => return err_response(StatusCode::BAD_REQUEST, format!("模板 YAML 解析失败: {e}")),
    };

    // 保存
    let dir = template_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("创建目录失败: {e}"));
    }

    let sanitized: String = tmpl
        .meta
        .name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c.is_ascii_whitespace() { c } else { '_' })
        .collect::<String>()
        .replace(' ', "_")
        .to_lowercase();

    let path = dir.join(format!("{sanitized}.yaml"));
    if let Err(e) = std::fs::write(&path, yaml_content) {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("写入失败: {e}"));
    }

    ok_response(serde_json::json!({
        "success": true,
        "template_name": sanitized,
        "name": tmpl.meta.name,
        "category": tmpl.meta.category,
    }))
}

/// GET /api/templates/categories — 列出所有模板分类
pub async fn template_categories() -> Response {
    match load_all_templates() {
        Ok(templates) => {
            let mut cats: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for (_, tmpl) in &templates {
                *cats.entry(tmpl.meta.category.clone()).or_insert(0) += 1;
            }
            let mut result: Vec<serde_json::Value> = cats
                .into_iter()
                .map(|(name, count)| serde_json::json!({ "name": name, "count": count }))
                .collect();
            result.sort_by(|a, b| {
                a.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .cmp(b.get("name").and_then(|v| v.as_str()).unwrap_or(""))
            });
            ok_response(serde_json::json!({ "categories": result }))
        }
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

// ═══════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_template_yaml() {
        let yaml = r#"
meta:
  name: "测试模板"
  desc: "描述"
  tags: [test]
params:
  - name: url
    desc: "目标地址"
    required: true
  - name: output
    desc: "输出路径"
    default: "out.txt"
workflow:
  name: "test-wf"
  steps:
    - id: s1
      type: http
      config:
        url: "{{url}}"
"#;
        let tmpl: TemplateFile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(tmpl.meta.name, "测试模板");
        assert_eq!(tmpl.params.len(), 2);
        assert!(tmpl.params[0].required);
        assert!(!tmpl.params[1].required);
        assert_eq!(tmpl.params[1].default.as_deref(), Some("out.txt"));
    }

    #[test]
    fn template_param_defaults() {
        let yaml = r#"
meta:
  name: "t"
  desc: "d"
params: []
workflow:
  name: "w"
  steps: []
"#;
        let tmpl: TemplateFile = serde_yaml::from_str(yaml).unwrap();
        assert!(tmpl.params.is_empty());
        assert!(tmpl.meta.tags.is_empty());
    }
}
