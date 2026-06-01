// engine/action_def.rs — Action 强类型定义
//
// 统一的 Action 元数据，替代前端 node-registry.ts 中的硬编码。
// build.rs 从此文件生成 src/types/action-metadata.ts。
//
// 设计原则：
//   - 每个容器在这里声明自己的 actions
//   - 前端从生成的 TS 文件中读取
//   - 加新 action 只需改此处一处

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum ActionCategory {
    Navigation,
    Interaction,
    DataRead,
    DataWrite,
    FileOperation,
    System,
    Clipboard,
    Reporting,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParamDef {
    pub key: &'static str,
    pub label: &'static str,
    pub param_type: ParamType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default_value: Option<serde_json::Value>,
    #[serde(default)]
    pub placeholder: Option<&'static str>,
    #[serde(default)]
    pub options: Option<&'static [(&'static str, &'static str)]>,
    #[serde(default)]
    pub hint: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub enum ParamType {
    Text,
    Number,
    Select,
    Checkbox,
    Textarea,
    VariableRef,
    FilePath,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionDef {
    pub action_type: &'static str,
    pub label: &'static str,
    pub category: ActionCategory,
    pub description: &'static str,
    pub params: &'static [ParamDef],
    pub output_hint: Option<&'static str>,
}

// ═══════════════════════════════════════
// 各容器 Action 声明
// ═══════════════════════════════════════

macro_rules! text_param {
    ($key:expr, $label:expr, $required:expr) => {
        ParamDef {
            key: $key,
            label: $label,
            param_type: ParamType::Text,
            required: $required,
            default_value: None,
            placeholder: None,
            options: None,
            hint: None,
        }
    };
}
macro_rules! number_param {
    ($key:expr, $label:expr) => {
        ParamDef {
            key: $key,
            label: $label,
            param_type: ParamType::Number,
            required: false,
            default_value: None,
            placeholder: None,
            options: None,
            hint: None,
        }
    };
}
macro_rules! select_param {
    ($key:expr, $label:expr, $opts:expr) => {
        ParamDef {
            key: $key,
            label: $label,
            param_type: ParamType::Select,
            required: false,
            default_value: None,
            placeholder: None,
            options: Some($opts),
            hint: None,
        }
    };
}
macro_rules! textarea_param {
    ($key:expr, $label:expr) => {
        ParamDef {
            key: $key,
            label: $label,
            param_type: ParamType::Textarea,
            required: false,
            default_value: None,
            placeholder: None,
            options: None,
            hint: None,
        }
    };
}

pub fn browser_actions() -> &'static [ActionDef] {
    &[
        ActionDef {
            action_type: "navigate",
            label: "Navigate",
            category: ActionCategory::Navigation,
            description: "Navigate to a URL",
            params: &[text_param!("url", "URL", true)],
            output_hint: Some("{ url, title }"),
        },
        ActionDef {
            action_type: "click",
            label: "Click",
            category: ActionCategory::Interaction,
            description: "Click an element by CSS selector",
            params: &[text_param!("selector", "CSS Selector", true)],
            output_hint: None,
        },
        ActionDef {
            action_type: "input",
            label: "Type Text",
            category: ActionCategory::Interaction,
            description: "Type text into an input field",
            params: &[
                text_param!("selector", "CSS Selector", true),
                text_param!("text", "Text", true),
            ],
            output_hint: None,
        },
        ActionDef {
            action_type: "scroll",
            label: "Scroll",
            category: ActionCategory::Interaction,
            description: "Scroll the page by pixels",
            params: &[number_param!("amount", "Scroll Amount (px)")],
            output_hint: None,
        },
        ActionDef {
            action_type: "extract",
            label: "Extract Content",
            category: ActionCategory::DataRead,
            description: "Extract text or HTML from elements",
            params: &[
                text_param!("selector", "CSS Selector", false),
                select_param!(
                    "mode",
                    "Extract Mode",
                    &[
                        ("text", "Text"),
                        ("html", "HTML"),
                        ("attribute", "Attribute")
                    ]
                ),
                text_param!("attribute", "Attribute Name", false),
            ],
            output_hint: Some("{ text, html, count }"),
        },
        ActionDef {
            action_type: "screenshot",
            label: "Screenshot",
            category: ActionCategory::DataRead,
            description: "Take a screenshot of the page",
            params: &[
                select_param!(
                    "mode",
                    "Mode",
                    &[
                        ("full", "Full Page"),
                        ("viewport", "Viewport"),
                        ("element", "Element")
                    ]
                ),
                text_param!("selector", "Element Selector", false),
            ],
            output_hint: Some("{ path, base64 }"),
        },
        ActionDef {
            action_type: "wait",
            label: "Wait",
            category: ActionCategory::System,
            description: "Wait for an element or timeout",
            params: &[
                text_param!("selector", "Wait for selector", false),
                number_param!("timeout_ms", "Timeout (ms)"),
            ],
            output_hint: None,
        },
        ActionDef {
            action_type: "hover",
            label: "Hover",
            category: ActionCategory::Interaction,
            description: "Hover over an element",
            params: &[text_param!("selector", "CSS Selector", true)],
            output_hint: None,
        },
        ActionDef {
            action_type: "press_key",
            label: "Press Key",
            category: ActionCategory::Interaction,
            description: "Press a keyboard key or combination",
            params: &[text_param!("key", "Key (e.g. Enter, Ctrl+C)", true)],
            output_hint: None,
        },
    ]
}

pub fn excel_actions() -> &'static [ActionDef] {
    &[
        ActionDef {
            action_type: "read",
            label: "Read Data",
            category: ActionCategory::DataRead,
            description: "Read cells from a sheet",
            params: &[],
            output_hint: Some("{ sheet, data, rows, cols }"),
        },
        ActionDef {
            action_type: "write",
            label: "Write Data",
            category: ActionCategory::DataWrite,
            description: "Write structured data to the sheet",
            params: &[textarea_param!("value", "Data (array of arrays)")],
            output_hint: None,
        },
        ActionDef {
            action_type: "create",
            label: "Create Workbook",
            category: ActionCategory::DataWrite,
            description: "Create a new Excel file with headers",
            params: &[text_param!(
                "headers",
                "Column Headers (comma-separated)",
                false
            )],
            output_hint: None,
        },
        ActionDef {
            action_type: "append",
            label: "Append Row",
            category: ActionCategory::DataWrite,
            description: "Append a row to the existing sheet",
            params: &[textarea_param!("value", "Row data (array)")],
            output_hint: None,
        },
        ActionDef {
            action_type: "sort",
            label: "Sort",
            category: ActionCategory::DataWrite,
            description: "Sort data by a column",
            params: &[
                text_param!("column", "Column (name or letter)", true),
                select_param!(
                    "order",
                    "Order",
                    &[("asc", "Ascending"), ("desc", "Descending")]
                ),
            ],
            output_hint: Some("{ sheet, rows, data }"),
        },
        ActionDef {
            action_type: "filter",
            label: "Filter",
            category: ActionCategory::DataRead,
            description: "Filter rows by condition",
            params: &[
                text_param!("column", "Column", true),
                select_param!(
                    "op",
                    "Operator",
                    &[
                        ("contains", "Contains"),
                        ("equals", "Equals"),
                        ("gt", ">"),
                        ("gte", ">="),
                        ("lt", "<"),
                        ("lte", "<="),
                        ("is_empty", "Is Empty"),
                        ("not_empty", "Not Empty"),
                    ]
                ),
                text_param!("value", "Filter Value", false),
            ],
            output_hint: Some("{ sheet, rows, data }"),
        },
        ActionDef {
            action_type: "update",
            label: "Update Cells",
            category: ActionCategory::DataWrite,
            description: "Update specific cells",
            params: &[textarea_param!("updates", "Cell updates")],
            output_hint: None,
        },
        ActionDef {
            action_type: "sheets",
            label: "List Sheets",
            category: ActionCategory::DataRead,
            description: "List all sheets in the workbook",
            params: &[],
            output_hint: Some("{ sheets: [name, ...] }"),
        },
    ]
}

pub fn word_actions() -> &'static [ActionDef] {
    &[
        ActionDef {
            action_type: "read",
            label: "Read Document",
            category: ActionCategory::DataRead,
            description: "Read the full document content",
            params: &[],
            output_hint: Some("{ paragraphs, full_text }"),
        },
        ActionDef {
            action_type: "write",
            label: "Write Content",
            category: ActionCategory::DataWrite,
            description: "Write paragraphs to the document",
            params: &[textarea_param!("value", "Content (text or paragraphs)")],
            output_hint: None,
        },
        ActionDef {
            action_type: "create",
            label: "Create Document",
            category: ActionCategory::DataWrite,
            description: "Create a new document with optional title",
            params: &[text_param!("title", "Document Title", false)],
            output_hint: None,
        },
        ActionDef {
            action_type: "replace",
            label: "Replace Text",
            category: ActionCategory::DataWrite,
            description: "Find and replace text in the document",
            params: &[
                text_param!("old_text", "Find Text", true),
                text_param!("new_text", "Replace With", true),
            ],
            output_hint: None,
        },
        ActionDef {
            action_type: "merge",
            label: "Merge Documents",
            category: ActionCategory::DataWrite,
            description: "Merge multiple docx files into one",
            params: &[textarea_param!("files", "File paths (array)")],
            output_hint: None,
        },
        ActionDef {
            action_type: "insert_table",
            label: "Insert Table",
            category: ActionCategory::DataWrite,
            description: "Insert a table into the document",
            params: &[textarea_param!("data", "Table data (array of rows)")],
            output_hint: None,
        },
    ]
}

pub fn file_actions() -> &'static [ActionDef] {
    &[
        ActionDef {
            action_type: "read",
            label: "Read File",
            category: ActionCategory::DataRead,
            description: "Read file content as text",
            params: &[text_param!("path", "File Path", true)],
            output_hint: Some("{ path, content, size, lines }"),
        },
        ActionDef {
            action_type: "write",
            label: "Write File",
            category: ActionCategory::DataWrite,
            description: "Write content to a file",
            params: &[
                text_param!("path", "File Path", true),
                textarea_param!("content", "Content"),
            ],
            output_hint: Some("{ path, size }"),
        },
        ActionDef {
            action_type: "append",
            label: "Append to File",
            category: ActionCategory::DataWrite,
            description: "Append content to a file",
            params: &[
                text_param!("path", "File Path", true),
                textarea_param!("content", "Content"),
            ],
            output_hint: None,
        },
        ActionDef {
            action_type: "copy",
            label: "Copy File",
            category: ActionCategory::FileOperation,
            description: "Copy a file to a new location",
            params: &[
                text_param!("from", "Source Path", true),
                text_param!("to", "Destination Path", true),
            ],
            output_hint: None,
        },
        ActionDef {
            action_type: "move",
            label: "Move / Rename",
            category: ActionCategory::FileOperation,
            description: "Move or rename a file",
            params: &[
                text_param!("from", "Source Path", true),
                text_param!("to", "Destination Path", true),
            ],
            output_hint: None,
        },
        ActionDef {
            action_type: "delete",
            label: "Delete",
            category: ActionCategory::FileOperation,
            description: "Delete a file or directory",
            params: &[text_param!("path", "Path", true)],
            output_hint: None,
        },
        ActionDef {
            action_type: "list",
            label: "List Directory",
            category: ActionCategory::DataRead,
            description: "List files in a directory",
            params: &[
                text_param!("path", "Directory Path", true),
                text_param!("pattern", "File Pattern (glob)", false),
            ],
            output_hint: Some("[{ name, path, size, is_dir }]"),
        },
        ActionDef {
            action_type: "exists",
            label: "Check Exists",
            category: ActionCategory::DataRead,
            description: "Check if a file or directory exists",
            params: &[text_param!("path", "Path", true)],
            output_hint: Some("{ exists: bool }"),
        },
        ActionDef {
            action_type: "glob",
            label: "Glob Search",
            category: ActionCategory::DataRead,
            description: "Find files matching a pattern",
            params: &[
                text_param!("path", "Base Directory", true),
                text_param!("pattern", "Glob Pattern", true),
            ],
            output_hint: Some("[{ name, path }]"),
        },
        ActionDef {
            action_type: "grep",
            label: "Grep Search",
            category: ActionCategory::DataRead,
            description: "Search file contents for text",
            params: &[
                text_param!("path", "File or Directory Path", true),
                text_param!("pattern", "Search Pattern", true),
            ],
            output_hint: Some("[{ file, line, content }]"),
        },
    ]
}

// ═══════════════════════════════════════
// TypeScript 生成
// ═══════════════════════════════════════

/// PascalCase → snake_case conversion
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

/// 生成 TypeScript 类型定义
pub fn generate_ts_metadata() -> String {
    let container_actions: &[(&str, &[ActionDef])] = &[
        ("browser", browser_actions()),
        ("excel", excel_actions()),
        ("word", word_actions()),
        ("file", file_actions()),
    ];

    let mut ts = String::from(
        "// AUTO-GENERATED by src-tauri/src/engine/action_def.rs\n\
         // Do not edit manually. Run `cargo build` to regenerate.\n\n\
         export type ActionCategory =\n  | 'navigation'\n  | 'interaction'\n  | 'data_read'\n  | 'data_write'\n  | 'file_operation'\n  | 'system'\n  | 'clipboard'\n  | 'reporting'\n\n\
         export type ParamType = 'text' | 'number' | 'select' | 'checkbox' | 'textarea' | 'variable_ref' | 'file_path'\n\n\
         export interface ParamDef {\n  key: string\n  label: string\n  param_type: ParamType\n  required?: boolean\n  default_value?: unknown\n  placeholder?: string\n  options?: { label: string; value: string }[]\n  hint?: string\n}\n\n\
         export interface ActionDef {\n  action_type: string\n  label: string\n  category: ActionCategory\n  description: string\n  params: ParamDef[]\n  output_hint?: string\n}\n\n\
         export const CONTAINER_ACTIONS: Record<string, ActionDef[]> = {\n"
    );

    for (container_type, actions) in container_actions {
        ts.push_str(&format!("  {}: [\n", container_type));
        for action in *actions {
            ts.push_str(&format!("    {{\n      action_type: '{}',\n      label: '{}',\n      category: '{}',\n      description: '{}',\n      params: [\n",
                action.action_type, action.label,
                to_snake_case(serde_json::to_string(&action.category).unwrap_or_default().trim_matches('"')),
                action.description));
            for param in action.params {
                let opts_str = if let Some(opts) = param.options {
                    format!(
                        "[{}]",
                        opts.iter()
                            .map(|(k, v)| format!("{{ label: '{}', value: '{}' }}", v, k))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                } else {
                    "undefined".to_string()
                };
                ts.push_str(&format!(
                    "        {{ key: '{}', label: '{}', param_type: '{}', required: {}, default_value: {}, placeholder: {}, options: {}, hint: {} }},\n",
                    param.key, param.label,
                    to_snake_case(serde_json::to_string(&param.param_type).unwrap_or_default().trim_matches('"')),
                    param.required,
                    param.default_value.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "undefined".to_string()),
                    param.placeholder.map(|s| format!("'{}'", s)).unwrap_or_else(|| "undefined".to_string()),
                    opts_str,
                    param.hint.map(|s| format!("'{}'", s)).unwrap_or_else(|| "undefined".to_string()),
                ));
            }
            ts.push_str("      ],\n");
            let output = action
                .output_hint
                .map(|s| format!("'{}'", s))
                .unwrap_or_else(|| "undefined".to_string());
            ts.push_str(&format!("      output_hint: {},\n", output));
            ts.push_str("    },\n");
        }
        ts.push_str("  ],\n");
    }

    ts.push_str("};\n");
    ts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_containers_have_actions() {
        assert!(!browser_actions().is_empty());
        assert!(!excel_actions().is_empty());
        assert!(!word_actions().is_empty());
        assert!(!file_actions().is_empty());
    }

    #[test]
    fn browser_has_essential_actions() {
        let types: Vec<&str> = browser_actions().iter().map(|a| a.action_type).collect();
        assert!(types.contains(&"navigate"));
        assert!(types.contains(&"click"));
        assert!(types.contains(&"input"));
        assert!(types.contains(&"extract"));
    }

    #[test]
    fn excel_has_essential_actions() {
        let types: Vec<&str> = excel_actions().iter().map(|a| a.action_type).collect();
        assert!(types.contains(&"read"));
        assert!(types.contains(&"write"));
        assert!(types.contains(&"create"));
        assert!(types.contains(&"sort"));
    }

    #[test]
    fn file_has_essential_actions() {
        let types: Vec<&str> = file_actions().iter().map(|a| a.action_type).collect();
        assert!(types.contains(&"read"));
        assert!(types.contains(&"write"));
        assert!(types.contains(&"list"));
    }

    #[test]
    fn all_params_have_keys() {
        for actions in [
            browser_actions(),
            excel_actions(),
            word_actions(),
            file_actions(),
        ] {
            for action in actions {
                for param in action.params {
                    assert!(
                        !param.key.is_empty(),
                        "Action {} has param with empty key",
                        action.action_type
                    );
                }
            }
        }
    }

    #[test]
    fn generate_ts_produces_valid_output() {
        let ts = generate_ts_metadata();
        assert!(ts.contains("export const CONTAINER_ACTIONS"));
        assert!(ts.contains("browser: ["));
        assert!(ts.contains("excel: ["));
        assert!(ts.contains("navigate"));
        assert!(ts.contains("click"));
    }
}
