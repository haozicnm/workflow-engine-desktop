// commands/template.rs — 内置模板
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct BuiltinTemplate {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub yaml: &'static str,
}

/// 所有内置模板
fn all_templates() -> Vec<BuiltinTemplate> {
    vec![
        BuiltinTemplate {
            id: "excel-browser-word",
            name: "Excel查询+浏览器+Word输出",
            description: "读取Excel A列 → 浏览器查询 → 结果保存Excel → 审批后输出Word",
            yaml: include_str!("../../../templates/excel-browser-word.yaml"),
        },
        BuiltinTemplate {
            id: "web-scrape-excel",
            name: "网页抓取→Excel",
            description: "声明式抓取网页列表数据 → 整理格式 → 写入 Excel 文件",
            yaml: include_str!("../../../templates/web-scrape-excel.yaml"),
        },
        BuiltinTemplate {
            id: "api-data-excel",
            name: "API数据采集",
            description: "调用 HTTP API → 提取 JSON 数据 → 写入 Excel",
            yaml: include_str!("../../../templates/api-data-excel.yaml"),
        },
        BuiltinTemplate {
            id: "api-monitor",
            name: "API健康监控",
            description: "定时检查多个 API 是否正常 → 异常时通知告警",
            yaml: include_str!("../../../templates/api-monitor.yaml"),
        },
        BuiltinTemplate {
            id: "multi-page-scrape",
            name: "多页翻页抓取",
            description: "自动翻页抓取多页列表数据 → 合并去重 → 写入 Excel",
            yaml: include_str!("../../../templates/multi-page-scrape.yaml"),
        },
        BuiltinTemplate {
            id: "browser-form-fill",
            name: "浏览器批量填表",
            description: "从 Excel 读取数据 → 浏览器自动填写网页表单 → 截图存档",
            yaml: include_str!("../../../templates/browser-form-fill.yaml"),
        },
        BuiltinTemplate {
            id: "excel-data-clean",
            name: "Excel数据清洗",
            description: "读取 Excel → 脚本清洗过滤 → 生成汇总 → 写入新文件",
            yaml: include_str!("../../../templates/excel-data-clean.yaml"),
        },
        BuiltinTemplate {
            id: "while-excel-browser",
            name: "While循环-逐行读取→浏览器填写",
            description: "Excel A列 → while 循环逐行读取 → 填入浏览器 → 无数据自动停止",
            yaml: include_str!("../../../templates/while-excel-browser.yaml"),
        },
    ]
}

/// 获取内置模板列表（不含 yaml 内容，仅元数据）
#[tauri::command]
pub async fn template_list() -> Result<Vec<serde_json::Value>, String> {
    Ok(all_templates().iter().map(|t| {
        serde_json::json!({
            "id": t.id,
            "name": t.name,
            "description": t.description,
        })
    }).collect())
}

/// 获取单个内置模板的完整 YAML
#[tauri::command]
pub async fn template_get_yaml(id: String) -> Result<Option<String>, String> {
    Ok(all_templates().iter()
        .find(|t| t.id == id)
        .map(|t| t.yaml.to_string()))
}
