// nodes/registry.rs — 节点注册清单（集中管理节点元数据）
//
// 设计原则：所有节点类型在一处声明，避免散落在 executor.rs、前端、文档中。
// 新增节点只需：
//   1. 实现 NodeExecutor trait
//   2. 在此注册 NodeManifest
//   3. 在 executor.rs 的 register 中关联类型字符串

use serde_json::Value;

/// 节点元数据清单
#[derive(Debug, Clone)]
pub struct NodeManifest {
    /// 节点类型标识（对应 Step.step_type）
    pub node_type: &'static str,
    /// 中文标签
    pub label: &'static str,
    /// 功能描述
    pub description: &'static str,
    /// 图标名（前端使用）
    pub icon: &'static str,
    /// 默认配置 JSON
    pub default_config: Value,
}

/// 返回所有已注册的节点清单
pub fn all_nodes() -> Vec<NodeManifest> {
    vec![
        // ── P0 核心节点 ──
        NodeManifest {
            node_type: "http",
            label: "HTTP 请求",
            description: "发送 HTTP 请求，支持 GET/POST/PUT/DELETE 等方法",
            icon: "globe",
            default_config: serde_json::json!({
                "method": "GET",
                "url": "",
                "headers": {},
                "body": null,
            }),
        },
        NodeManifest {
            node_type: "data",
            label: "数据处理",
            description: "变量赋值、表达式计算、数据合并等操作",
            icon: "database",
            default_config: serde_json::json!({
                "action": "set",
                "key": "",
                "value": "",
            }),
        },
        NodeManifest {
            node_type: "script",
            label: "脚本",
            description: "执行 Rhai 脚本，支持变量读写和复杂逻辑",
            icon: "code",
            default_config: serde_json::json!({
                "script": "// Rhai 脚本\nlet result = 42;\nresult",
            }),
        },
        NodeManifest {
            node_type: "condition",
            label: "条件判断",
            description: "根据条件表达式结果选择执行分支",
            icon: "git-branch",
            default_config: serde_json::json!({
                "expression": "{{step_xxx.result}} > 0",
                "true_next": "",
                "false_next": "",
            }),
        },

        // ── P2 文件节点 ──
        NodeManifest {
            node_type: "excel",
            label: "Excel",
            description: "读写 Excel 文件（.xlsx/.xls），支持公式计算",
            icon: "file-spreadsheet",
            default_config: serde_json::json!({
                "action": "read",
                "file": "",
                "sheet": "",
            }),
        },
        NodeManifest {
            node_type: "word",
            label: "Word",
            description: "读写 Word 文档（.docx），支持模板替换",
            icon: "file-text",
            default_config: serde_json::json!({
                "action": "read",
                "file": "",
            }),
        },

        // ── P2.5+ 新节点 ──
        NodeManifest {
            node_type: "browser",
            label: "浏览器",
            description: "通过 Playwright 控制浏览器，支持导航、点击、截图等",
            icon: "chrome",
            default_config: serde_json::json!({
                "action": "navigate",
                "params": {"url": ""},
            }),
        },
        NodeManifest {
            node_type: "notify",
            label: "通知",
            description: "发送系统通知或消息到指定渠道",
            icon: "bell",
            default_config: serde_json::json!({
                "title": "",
                "message": "",
            }),
        },
        NodeManifest {
            node_type: "approval",
            label: "审批",
            description: "暂停执行等待人工审批（通过/拒绝）",
            icon: "check-circle",
            default_config: serde_json::json!({
                "message": "请审批此操作",
                "options": ["approve", "reject"],
            }),
        },

        // ── 循环/并行 ──
        NodeManifest {
            node_type: "loop",
            label: "循环",
            description: "遍历数组，对每个元素执行子步骤",
            icon: "repeat",
            default_config: serde_json::json!({
                "array": "{{steps.xxx}}",
                "body": [],
            }),
        },
        NodeManifest {
            node_type: "while",
            label: "While 循环",
            description: "条件循环，满足条件时重复执行子步骤",
            icon: "refresh-cw",
            default_config: serde_json::json!({
                "condition": "",
                "body": [],
            }),
        },
        NodeManifest {
            node_type: "parallel",
            label: "并行",
            description: "并行执行多个子步骤，等待全部完成",
            icon: "layers",
            default_config: serde_json::json!({
                "branches": [],
            }),
        },

        // ── v0.7.0 声明式数据 ──
        NodeManifest {
            node_type: "map",
            label: "数据映射",
            description: "声明式数据转换（数组映射）",
            icon: "shuffle",
            default_config: serde_json::json!({
                "array": "",
                "mapping": {},
            }),
        },

        // ── v1.1 声明式网页抓取 ──
        NodeManifest {
            node_type: "web_scrape",
            label: "网页抓取",
            description: "声明式提取网页数据，支持 CSS 选择器",
            icon: "scissors",
            default_config: serde_json::json!({
                "url": "",
                "fields": {},
                "list": null,
            }),
        },

        NodeManifest {
            node_type: "delay",
            label: "延时",
            description: "暂停执行指定时长（支持毫秒级精度）",
            icon: "clock",
            default_config: serde_json::json!({
                "duration_ms": 1000,
                "max_duration_ms": 300000,
            }),
        },

        // ── 桌面交互节点 ──
        NodeManifest {
            node_type: "mouse_keyboard",
            label: "键鼠操作",
            description: "模拟鼠标点击、键盘输入等桌面操作",
            icon: "mouse-pointer",
            default_config: serde_json::json!({
                "action": "click",
                "x": 0,
                "y": 0,
            }),
        },
        NodeManifest {
            node_type: "window",
            label: "窗口管理",
            description: "查找、激活、调整桌面窗口",
            icon: "layout",
            default_config: serde_json::json!({
                "action": "find",
                "title": "",
            }),
        },
        NodeManifest {
            node_type: "sub_workflow",
            label: "子工作流",
            description: "调用另一个工作流作为子流程",
            icon: "git-merge",
            default_config: serde_json::json!({
                "workflow_id": "",
                "input": {},
            }),
        },
        NodeManifest {
            node_type: "ocr",
            label: "OCR 识别",
            description: "图片文字识别（光学字符识别）",
            icon: "type",
            default_config: serde_json::json!({
                "image": "",
                "language": "chi_sim",
            }),
        },
        NodeManifest {
            node_type: "recording",
            label: "录屏",
            description: "录制屏幕操作视频",
            icon: "video",
            default_config: serde_json::json!({
                "action": "start",
                "output": "",
            }),
        },

        // ── v2 通用节点 ──
        NodeManifest {
            node_type: "file",
            label: "文件操作",
            description: "读写、列出、删除、检查文件或目录",
            icon: "file",
            default_config: serde_json::json!({
                "action": "read",
                "path": "",
            }),
        },
        NodeManifest {
            node_type: "clipboard",
            label: "剪贴板",
            description: "读写系统剪贴板文本内容",
            icon: "clipboard",
            default_config: serde_json::json!({
                "action": "read",
            }),
        },
        NodeManifest {
            node_type: "regex",
            label: "正则处理",
            description: "正则表达式匹配、提取、替换文本",
            icon: "regex",
            default_config: serde_json::json!({
                "action": "extract",
                "pattern": "",
                "input": "",
            }),
        },
        NodeManifest {
            node_type: "array",
            label: "数组操作",
            description: "数组过滤、排序、去重、分页、映射、聚合",
            icon: "list",
            default_config: serde_json::json!({
                "action": "filter",
                "source": [],
                "condition": {"field": "", "op": "==", "value": ""},
            }),
        },
        NodeManifest {
            node_type: "convert",
            label: "类型转换",
            description: "文本、数字、JSON、CSV、HTML、Base64 互转",
            icon: "refresh-cw",
            default_config: serde_json::json!({
                "action": "to_text",
                "input": "",
            }),
        },
        NodeManifest {
            node_type: "print",
            label: "控制台打印",
            description: "打印信息到后端日志和前端控制台",
            icon: "terminal",
            default_config: serde_json::json!({
                "message": "",
                "level": "info",
            }),
        },
    ]
}
