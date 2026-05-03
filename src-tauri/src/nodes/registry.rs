// nodes/registry.rs — 节点注册清单（v3: 集中管理所有节点元数据）
//
// 设计原则：所有节点类型在一处声明，避免散落在 executor.rs、前端、文档中。
// v3: 所有节点独立 executor，无 action 参数分发。

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
            node_type: "http", label: "HTTP 请求",
            description: "发送 HTTP 请求，支持 GET/POST/PUT/DELETE 等方法",
            icon: "globe", default_config: serde_json::json!({ "method": "GET", "url": "", "headers": {}, "body": null }),
        },
        NodeManifest {
            node_type: "script", label: "脚本",
            description: "执行 Rhai 脚本，支持变量读写和复杂逻辑",
            icon: "code", default_config: serde_json::json!({ "script": "// Rhai 脚本\nlet result = 42;\nresult" }),
        },
        NodeManifest {
            node_type: "condition", label: "条件判断",
            description: "根据条件表达式结果选择执行分支",
            icon: "git-branch", default_config: serde_json::json!({ "expression": "{{step_xxx.result}} > 0", "true_next": "", "false_next": "" }),
        },

        // ── 数据处理节点 ──
        NodeManifest {
            node_type: "data_set", label: "设置变量",
            description: "写入或覆盖上下文变量",
            icon: "database", default_config: serde_json::json!({ "key": "", "value": "" }),
        },
        NodeManifest {
            node_type: "data_get", label: "读取变量",
            description: "从上下文读取指定变量的值",
            icon: "database", default_config: serde_json::json!({ "key": "" }),
        },
        NodeManifest {
            node_type: "data_length", label: "获取长度",
            description: "获取字符串、数组、对象的长度",
            icon: "hash", default_config: serde_json::json!({ "source": "" }),
        },
        NodeManifest {
            node_type: "data_default", label: "默认值",
            description: "变量为空时设置默认值",
            icon: "shield", default_config: serde_json::json!({ "key": "", "value": "" }),
        },
        NodeManifest {
            node_type: "data_merge", label: "合并变量",
            description: "将源对象合并到目标对象",
            icon: "git-merge", default_config: serde_json::json!({ "target": "", "source": "" }),
        },
        NodeManifest {
            node_type: "json_parse", label: "JSON 解析",
            description: "用点号路径提取 JSON 字段",
            icon: "braces", default_config: serde_json::json!({ "data": "", "expression": "$" }),
        },
        NodeManifest {
            node_type: "text_template", label: "文本拼接",
            description: "使用 {{变量}} 占位符模板替换",
            icon: "type", default_config: serde_json::json!({ "template": "", "output_key": "" }),
        },

        // ── 文件节点 ──
        NodeManifest {
            node_type: "file_read", label: "读取文件",
            description: "读取文件内容（支持 text/base64 编码）",
            icon: "file-text", default_config: serde_json::json!({ "path": "", "encoding": "text" }),
        },
        NodeManifest {
            node_type: "file_write", label: "写入文件",
            description: "写入内容到文件（自动创建父目录）",
            icon: "file-plus", default_config: serde_json::json!({ "path": "", "content": "", "encoding": "text" }),
        },
        NodeManifest {
            node_type: "file_list", label: "列出文件",
            description: "列出目录内容（支持递归和扩展名过滤）",
            icon: "folder", default_config: serde_json::json!({ "path": "", "recursive": false }),
        },
        NodeManifest {
            node_type: "file_delete", label: "删除文件",
            description: "删除文件或目录（目录需 recursive:true）",
            icon: "trash-2", default_config: serde_json::json!({ "path": "", "recursive": false }),
        },
        NodeManifest {
            node_type: "file_exists", label: "存在检查",
            description: "检查文件/目录是否存在",
            icon: "search", default_config: serde_json::json!({ "path": "" }),
        },
        NodeManifest {
            node_type: "file_save", label: "保存文件",
            description: "保存数据到文件（JSON/YAML/CSV/TXT/Base64）",
            icon: "save", default_config: serde_json::json!({ "path": "output.txt", "data": "", "format": "auto", "encoding": "utf-8" }),
        },

        // ── 剪贴板节点 ──
        NodeManifest {
            node_type: "clipboard_read", label: "读取剪贴板",
            description: "读取系统剪贴板中的文本内容",
            icon: "clipboard", default_config: serde_json::json!({}),
        },
        NodeManifest {
            node_type: "clipboard_write", label: "写入剪贴板",
            description: "写入文本到系统剪贴板",
            icon: "clipboard-copy", default_config: serde_json::json!({ "text": "" }),
        },

        // ── 正则节点 ──
        NodeManifest {
            node_type: "regex_extract", label: "正则提取",
            description: "用正则表达式提取捕获组",
            icon: "regex", default_config: serde_json::json!({ "pattern": "", "input": "", "global": true }),
        },
        NodeManifest {
            node_type: "regex_replace", label: "正则替换",
            description: "替换正则匹配内容（支持 $1 引用）",
            icon: "replace", default_config: serde_json::json!({ "pattern": "", "replacement": "", "input": "", "global": true }),
        },
        NodeManifest {
            node_type: "regex_match", label: "正则匹配",
            description: "查找所有匹配位置",
            icon: "search", default_config: serde_json::json!({ "pattern": "", "input": "", "global": true }),
        },

        // ── 数组节点 ──
        NodeManifest {
            node_type: "array_filter", label: "数组过滤",
            description: "按条件过滤数组元素",
            icon: "filter", default_config: serde_json::json!({ "source": "", "condition": { "field": "", "op": "==", "value": "" } }),
        },
        NodeManifest {
            node_type: "array_sort", label: "数组排序",
            description: "按指定字段排序",
            icon: "arrow-up-down", default_config: serde_json::json!({ "source": "", "field": "", "order": "asc" }),
        },
        NodeManifest {
            node_type: "array_dedup", label: "数组去重",
            description: "按字段值去重",
            icon: "copy-check", default_config: serde_json::json!({ "source": "", "field": "" }),
        },
        NodeManifest {
            node_type: "array_paginate", label: "数组分页",
            description: "数组分页，返回指定页的数据",
            icon: "files", default_config: serde_json::json!({ "source": "", "page": 1, "page_size": 10 }),
        },
        NodeManifest {
            node_type: "array_map", label: "数组映射",
            description: "模板映射每个元素（{{__item}}）",
            icon: "shuffle", default_config: serde_json::json!({ "source": "", "template": "{{__item}}" }),
        },
        NodeManifest {
            node_type: "array_join", label: "数组连接",
            description: "将数组元素连接为字符串",
            icon: "link", default_config: serde_json::json!({ "source": "", "separator": ",", "field": "" }),
        },
        NodeManifest {
            node_type: "array_reduce", label: "数组聚合",
            description: "聚合计算（sum/avg/min/max/count/first/last）",
            icon: "sigma", default_config: serde_json::json!({ "source": "", "aggregator": "count", "field": "" }),
        },

        // ── 转换节点 ──
        NodeManifest {
            node_type: "convert_to_text", label: "转文本",
            description: "任意值 → JSON 字符串",
            icon: "type", default_config: serde_json::json!({ "input": "" }),
        },
        NodeManifest {
            node_type: "convert_to_number", label: "转数字",
            description: "文本 → 数字（整数/浮点自动识别）",
            icon: "hash", default_config: serde_json::json!({ "input": "" }),
        },
        NodeManifest {
            node_type: "convert_to_json", label: "转 JSON",
            description: "JSON 文本 → 对象/数组解析",
            icon: "braces", default_config: serde_json::json!({ "input": "" }),
        },
        NodeManifest {
            node_type: "convert_to_csv", label: "转 CSV",
            description: "对象数组 → CSV 文本",
            icon: "table", default_config: serde_json::json!({ "input": "", "delimiter": ",", "include_header": true }),
        },
        NodeManifest {
            node_type: "convert_to_html", label: "转 HTML",
            description: "对象/数组 → HTML 表格/列表",
            icon: "code-2", default_config: serde_json::json!({ "input": "", "template": "" }),
        },
        NodeManifest {
            node_type: "convert_to_base64", label: "转 Base64",
            description: "文本 → Base64 编码",
            icon: "binary", default_config: serde_json::json!({ "input": "" }),
        },

        // ── Excel 节点 ──
        NodeManifest {
            node_type: "excel", label: "Excel(通用)",
            description: "读写 Excel 文件（.xlsx/.xls），兼容旧版 action 调用",
            icon: "file-spreadsheet", default_config: serde_json::json!({ "action": "read", "file": "", "sheet": "" }),
        },
        NodeManifest {
            node_type: "excel_read", label: "读取表格",
            description: "读取 Excel 工作表中的数据",
            icon: "file-spreadsheet", default_config: serde_json::json!({ "path": "", "sheet": "" }),
        },
        NodeManifest {
            node_type: "excel_write", label: "写入表格",
            description: "写入数据到 Excel 工作表",
            icon: "file-spreadsheet", default_config: serde_json::json!({ "path": "", "sheet": "Sheet1", "data": [], "write_mode": "overwrite" }),
        },
        NodeManifest {
            node_type: "excel_create", label: "创建表格",
            description: "新建 Excel 工作簿",
            icon: "file-plus", default_config: serde_json::json!({ "path": "新表格.xlsx", "sheet": "Sheet1", "headers": [] }),
        },
        NodeManifest {
            node_type: "excel_filter", label: "筛选数据",
            description: "内存筛选 Excel 数据（8种运算符）",
            icon: "filter", default_config: serde_json::json!({ "data": [], "column": "", "op": "==", "value": "" }),
        },
        NodeManifest {
            node_type: "excel_sort", label: "排序数据",
            description: "内存排序 Excel 数据（asc/desc）",
            icon: "arrow-up-down", default_config: serde_json::json!({ "data": [], "column": "", "order": "asc" }),
        },
        NodeManifest {
            node_type: "excel_append", label: "追加行",
            description: "追加数据行到现有 Excel 文件",
            icon: "list-plus", default_config: serde_json::json!({ "path": "", "sheet": "Sheet1", "data": [] }),
        },
        NodeManifest {
            node_type: "excel_csv", label: "CSV 互转",
            description: "CSV ↔ Excel 格式互转",
            icon: "arrow-left-right", default_config: serde_json::json!({ "path": "", "direction": "csv_to_xlsx", "delimiter": ",", "output": "" }),
        },

        // ── Word 节点 ──
        NodeManifest {
            node_type: "word", label: "Word(通用)",
            description: "读写 Word 文档（.docx），兼容旧版 action 调用",
            icon: "file-text", default_config: serde_json::json!({ "action": "read", "file": "" }),
        },
        NodeManifest {
            node_type: "word_read", label: "读取文档",
            description: "提取 Word 文档纯文本和段落",
            icon: "file-text", default_config: serde_json::json!({ "path": "" }),
        },
        NodeManifest {
            node_type: "word_write", label: "写入文档",
            description: "写入富文本到 Word 文档",
            icon: "file-plus", default_config: serde_json::json!({ "path": "", "content": "", "mode": "overwrite" }),
        },
        NodeManifest {
            node_type: "word_create", label: "创建文档",
            description: "新建 Word 文档（标题+内容）",
            icon: "file-plus", default_config: serde_json::json!({ "path": "新文档.docx", "title": "", "content": "" }),
        },
        NodeManifest {
            node_type: "word_replace", label: "查找替换",
            description: "占位符替换 {{name}} → value",
            icon: "replace", default_config: serde_json::json!({ "path": "", "find": "", "replace": "", "count": 0 }),
        },
        NodeManifest {
            node_type: "word_merge", label: "合并文档",
            description: "多个 Word 文档合并为单个（加分页）",
            icon: "combine", default_config: serde_json::json!({ "files": [], "output": "合并文档.docx" }),
        },

        // ── 浏览器节点 ──
        NodeManifest {
            node_type: "browser", label: "浏览器(万能)",
            description: "通过 Playwright 控制浏览器，支持自定义 action",
            icon: "chrome", default_config: serde_json::json!({ "action": "navigate", "params": { "url": "" } }),
        },
        NodeManifest {
            node_type: "browser_navigate", label: "浏览器导航",
            description: "导航到指定 URL",
            icon: "navigation", default_config: serde_json::json!({ "params": { "url": "https://example.com", "wait_until": "load" } }),
        },
        NodeManifest {
            node_type: "browser_click", label: "浏览器点击",
            description: "点击页面元素",
            icon: "mouse-pointer-click", default_config: serde_json::json!({ "params": { "selector": "" } }),
        },
        NodeManifest {
            node_type: "browser_fill", label: "浏览器填写",
            description: "填写表单输入框",
            icon: "pencil", default_config: serde_json::json!({ "params": { "selector": "", "value": "" } }),
        },
        NodeManifest {
            node_type: "browser_extract", label: "浏览器提取",
            description: "提取页面数据（文本/HTML/表格/链接/属性）",
            icon: "scissors", default_config: serde_json::json!({ "params": { "selector": "", "mode": "text" } }),
        },
        NodeManifest {
            node_type: "browser_screenshot", label: "浏览器截图",
            description: "截取页面或元素截图",
            icon: "camera", default_config: serde_json::json!({ "params": { "path": "screenshot.png", "full_page": false } }),
        },
        NodeManifest {
            node_type: "browser_evaluate", label: "浏览器执行JS",
            description: "在页面中执行 JavaScript 代码",
            icon: "terminal", default_config: serde_json::json!({ "params": { "script": "document.title" } }),
        },
        NodeManifest {
            node_type: "browser_scroll", label: "浏览器滚动",
            description: "滚动页面到底部/顶部",
            icon: "move-vertical", default_config: serde_json::json!({ "params": { "direction": "bottom", "times": 1, "delay_ms": 500 } }),
        },
        NodeManifest {
            node_type: "browser_wait", label: "浏览器等待",
            description: "等待指定元素出现",
            icon: "clock", default_config: serde_json::json!({ "params": { "selector": "", "timeout_ms": 30000 } }),
        },
        NodeManifest {
            node_type: "browser_pdf", label: "浏览器PDF",
            description: "将当前页面生成 PDF 文件",
            icon: "file", default_config: serde_json::json!({ "params": { "path": "output.pdf" } }),
        },

        // ── 其他节点 ──
        NodeManifest {
            node_type: "notify", label: "通知",
            description: "发送系统通知或消息到指定渠道",
            icon: "bell", default_config: serde_json::json!({ "title": "", "message": "" }),
        },
        NodeManifest {
            node_type: "approval", label: "审批",
            description: "暂停执行等待人工审批（通过/拒绝）",
            icon: "check-circle", default_config: serde_json::json!({ "message": "请审批此操作", "options": ["approve", "reject"] }),
        },
        NodeManifest {
            node_type: "loop", label: "循环",
            description: "遍历数组，对每个元素执行子步骤",
            icon: "repeat", default_config: serde_json::json!({ "array": "{{steps.xxx}}", "body": [] }),
        },
        NodeManifest {
            node_type: "while", label: "While 循环",
            description: "条件循环，满足条件时重复执行子步骤",
            icon: "refresh-cw", default_config: serde_json::json!({ "condition": "", "body": [] }),
        },
        NodeManifest {
            node_type: "parallel", label: "并行",
            description: "并行执行多个子步骤，等待全部完成",
            icon: "layers", default_config: serde_json::json!({ "branches": [] }),
        },
        NodeManifest {
            node_type: "map", label: "数据映射",
            description: "声明式数据转换（数组映射）",
            icon: "shuffle", default_config: serde_json::json!({ "array": "", "mapping": {} }),
        },
        NodeManifest {
            node_type: "web_scrape", label: "网页抓取",
            description: "声明式提取网页数据，支持 CSS 选择器",
            icon: "scissors", default_config: serde_json::json!({ "url": "", "fields": {}, "list": null }),
        },
        NodeManifest {
            node_type: "delay", label: "延时",
            description: "暂停执行指定时长（支持毫秒级精度）",
            icon: "clock", default_config: serde_json::json!({ "duration_ms": 1000, "max_duration_ms": 300000 }),
        },
        NodeManifest {
            node_type: "mouse_keyboard", label: "键鼠操作",
            description: "模拟鼠标点击、键盘输入等桌面操作",
            icon: "mouse-pointer", default_config: serde_json::json!({ "action": "click", "x": 0, "y": 0 }),
        },
        NodeManifest {
            node_type: "window", label: "窗口管理",
            description: "查找、激活、调整桌面窗口",
            icon: "layout", default_config: serde_json::json!({ "action": "find", "title": "" }),
        },
        NodeManifest {
            node_type: "sub_workflow", label: "子工作流",
            description: "调用另一个工作流作为子流程",
            icon: "git-merge", default_config: serde_json::json!({ "workflow_id": "", "input": {} }),
        },
        NodeManifest {
            node_type: "ocr", label: "OCR 识别",
            description: "图片文字识别（光学字符识别）",
            icon: "type", default_config: serde_json::json!({ "image": "", "language": "chi_sim" }),
        },
        NodeManifest {
            node_type: "recording", label: "录屏",
            description: "录制屏幕操作视频",
            icon: "video", default_config: serde_json::json!({ "action": "start", "output": "" }),
        },
        NodeManifest {
            node_type: "print", label: "控制台打印",
            description: "打印信息到后端日志和前端控制台",
            icon: "terminal", default_config: serde_json::json!({ "message": "", "level": "info" }),
        },
    ]
}
