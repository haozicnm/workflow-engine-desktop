// ─── Workflow Engine v5 节点注册表 ───
// 容器定义、动作列表、工厂函数

import type {
  ContainerType, ContainerDef, ActionDef,
  Action, Step,
} from './types'
import { uid, nextStepId, nextActionId } from './types'
import { getSchemaDefs } from './registry-state'

/** i18n 翻译函数类型 */
type TFn = (key: string, defaultMsg?: string) => string

// ─── 容器定义 ───

export const CONTAINER_DEFS: ContainerDef[] = [
  { type: 'browser', label: '浏览器', icon: 'Globe', color: '#79c0ff', isContainer: true, category: 'browser', description: '网页操作：导航、点击、输入、提取', outputHint: 'actionId: value, ...', params: [
    { key: 'browser', label: '浏览器', type: 'select', options: [
      { label: 'Chromium', value: 'chromium' }, { label: 'Firefox', value: 'firefox' }, { label: 'WebKit', value: 'webkit' },
    ], default: 'chromium' },
    { key: 'headless', label: '无头模式', type: 'checkbox', default: false },
    { key: 'timeout', label: '超时(ms)', type: 'number', default: 30000 },
  ]},
  { type: 'excel', label: 'Excel', icon: 'BarChart3', color: '#3fb950', isContainer: true, description: 'Excel 操作：读写单元格、筛选、排序', outputHint: 'actionId: value, ...', params: [
    { key: 'file_path', label: '文件路径', type: 'text', placeholder: './data.xlsx' },
    { key: 'sheet', label: '工作表', type: 'text', default: 'Sheet1' },
  ]},
  { type: 'word', label: 'Word', icon: 'FileText', color: '#bc8cff', isContainer: true, description: 'Word 操作：读写、替换、合并', outputHint: 'actionId: value, ...', params: [
    { key: 'file_path', label: '文件路径', type: 'text', placeholder: './document.docx' },
  ]},
  { type: 'logic', label: '条件判断', icon: 'GitBranch', color: '#d29922', isContainer: true, description: '条件分支：满足/不满足走不同路径', outputHint: 'branch: true/false, value, result', params: [
    { key: 'condition', label: '条件表达式', type: 'text', placeholder: '{{step1.output}} == "异常"' },
  ]},
  { type: 'http', label: 'HTTP 请求', icon: 'Network', color: '#539bf5', description: '发送 HTTP 请求并获取响应', outputHint: 'status, headers, body', params: [
    { key: 'method', label: '方法', type: 'select', options: [
      { label: 'GET', value: 'GET' }, { label: 'POST', value: 'POST' },
      { label: 'PUT', value: 'PUT' }, { label: 'DELETE', value: 'DELETE' },
      { label: 'PATCH', value: 'PATCH' },
    ], default: 'GET' },
    { key: 'url', label: 'URL', type: 'text', placeholder: 'https://api.example.com/data' },
    { key: 'headers', label: '请求头 (JSON)', type: 'textarea', placeholder: '{"Content-Type": "application/json"}' },
    { key: 'body', label: '请求体', type: 'textarea', placeholder: '{"key": "value"}' },
    { key: 'timeout', label: '超时(ms)', type: 'number', default: 30000 },
    { key: 'connect_timeout', label: '连接超时(ms)', type: 'number', default: 10000 },
  ]},
  { type: 'delay', label: '延迟等待', icon: 'Clock', color: '#adbac7', description: '等待指定时间后继续', outputHint: 'waited: ms', params: [
    { key: 'duration_ms', label: '毫秒', type: 'number', default: 1000 },
    { key: 'max_duration_ms', label: '最大毫秒(随机)', type: 'number', default: 5000 },
  ]},
  { type: 'notify', label: '通知', icon: 'Bell', color: '#f0883e', description: '发送通知：系统通知/Webhook', outputHint: 'sent: true', params: [
    { key: 'notify_type', label: '渠道', type: 'select', options: [
      { label: '系统通知', value: 'system' }, { label: 'Webhook', value: 'webhook' },
    ], default: 'system' },
    { key: 'title', label: '标题', type: 'text' },
    { key: 'body', label: '内容', type: 'textarea' },
    { key: 'url', label: 'Webhook URL', type: 'text', placeholder: 'https://hooks.example.com/...' },
    { key: 'method', label: '请求方法', type: 'select', options: [
      { label: 'POST', value: 'POST' }, { label: 'PUT', value: 'PUT' },
      { label: 'PATCH', value: 'PATCH' },
    ], default: 'POST' },
    { key: 'headers', label: '请求头 (JSON)', type: 'textarea', placeholder: '{"Authorization": "Bearer xxx"}' },
    { key: 'data', label: '自定义数据 (JSON)', type: 'textarea', placeholder: '{"text": "Hello"}' },
  ]},
  { type: 'script', label: '脚本', icon: 'ScrollText', color: '#7ee787', description: '执行自定义脚本（Rhai）', outputHint: '脚本返回值', params: [
    { key: 'script', label: '代码', type: 'textarea', placeholder: '// 你的 Rhai 脚本代码' },
  ]},
  { type: 'clipboard', label: '剪贴板', icon: 'ClipboardList', color: '#8b949e', description: '读写系统剪贴板', outputHint: '剪贴板内容', params: [
    { key: 'action', label: '操作', type: 'select', options: [
      { label: '读取', value: 'read' }, { label: '写入', value: 'write' },
    ], default: 'read' },
    { key: 'text', label: '写入内容', type: 'textarea' },
  ]},
  { type: 'cursor', label: '游标迭代', icon: 'Repeat', color: '#e85d75', isContainer: true, description: '逐条迭代：每次运行处理一行/一项，游标跨次保存', outputHint: 'done, item, index, total', params: [
    { key: 'items', label: '数据源', type: 'text', placeholder: '{{read_excel.data}}' },
  ]},
  { type: 'loop', label: '批量循环', icon: 'RefreshCw', color: '#daaa3e', isContainer: true, description: '一次性遍历全部数据，适合小数据内存变换', outputHint: 'count, results[]', params: [
    { key: 'items', label: '数据源', type: 'text', placeholder: '{{step1.data}} 或 [[1,2,3]]' },
    { key: 'collect', label: '结果聚合 (JSON)', type: 'textarea', placeholder: '{"field": "id", "method": "concat"}' },
    { key: 'table', label: '表格输出 (JSON)', type: 'textarea', placeholder: '{"columns": ["name","value"]}' },
  ]},
  { type: 'approval', label: '人工审批', icon: 'Hand', color: '#f778ba', description: '暂停流程等待人工审核：支持条件推荐、超时自动/手动', outputHint: 'decision: 选项名, comment, item, auto?, recommendation_reason?', params: [
    { key: 'title', label: '审批标题', type: 'text', placeholder: '请确认订单信息' },
    { key: 'message', label: '审批内容', type: 'textarea', placeholder: '订单号：{{step_1.action_1_1.订单号}}' },
    { key: 'options', label: '审批选项', type: 'text', placeholder: '同意,拒绝,需要更多信息（逗号分隔）', default: '同意,拒绝' },
    { key: 'recommended', label: '推荐选项', type: 'text', placeholder: '同意（条件评估通过时自动覆写）', default: '同意', hint: '如有审批条件，该字段会被条件评估结果自动覆写：全部通过→选项1，否则→选项N' },
    { key: 'require_review', label: '需要人工审核', type: 'select', options: [
      { label: '是', value: 'true' }, { label: '否（自动决策）', value: 'false' },
    ], default: 'true' },
    { key: 'timeout', label: '超时(秒)', type: 'number', default: 300 },
    { key: 'timeout_behavior', label: '超时行为', type: 'select', options: [
      { label: '超时自动执行推荐', value: 'auto' },
      { label: '必须人工审批（永不过期）', value: 'manual' },
    ], default: 'auto' },
    { key: 'timeout_action', label: '超时策略', type: 'select', options: [
      { label: '执行推荐选项', value: 'recommended' },
      { label: '自动拒绝', value: 'reject' },
      { label: '自动通过', value: 'approve' },
      { label: '标记失败', value: 'fail' },
    ], default: 'recommended' },
  ]},
  { type: 'shell', label: 'Shell 命令', icon: 'Terminal', color: '#768390', dangerous: true, description: '执行任意 Shell 命令（bash/powershell/cmd），支持变量引用', outputHint: 'stdout, stderr, exit_code', params: [
    { key: 'command', label: '命令', type: 'textarea', placeholder: 'echo "Hello {{name}}"' },
    { key: 'shell', label: 'Shell 类型', type: 'select', options: [
      { label: '自动检测', value: 'auto' },
      { label: 'Bash', value: 'bash' },
      { label: 'PowerShell', value: 'powershell' },
      { label: 'CMD', value: 'cmd' },
    ], default: 'auto' },
    { key: 'cwd', label: '工作目录', type: 'text', placeholder: '/home/user/project' },
    { key: 'timeout_secs', label: '超时(秒)', type: 'number', default: 300 },
  ]},
  { type: 'file', label: '文件操作', icon: 'FolderOpen', color: '#d2a8ff', description: '统一文件操作：读取/写入/复制/移动/删除/列表/搜索/Glob/Grep', outputHint: 'actionId: result, ...', isContainer: true, params: []},

  // ─── Kimi WebBridge 浏览器节点 ───
  { type: 'kimi_browser', label: 'Kimi 浏览器', icon: 'Globe', color: '#79c0ff', description: '通过 Kimi WebBridge 控制真实浏览器（有登录态），支持 17 种操作', outputHint: 'success, data', params: [
    { key: 'action', label: '操作', type: 'select', options: [
      { label: '导航', value: 'navigate' },
      { label: '快照（获取元素树）', value: 'snapshot' },
      { label: '点击元素', value: 'click' },
      { label: '物理鼠标点击', value: 'mouse_click' },
      { label: '填写表单', value: 'fill' },
      { label: '键盘输入', value: 'key_type' },
      { label: '发送按键', value: 'send_keys' },
      { label: '执行 JavaScript', value: 'evaluate' },
      { label: '截图', value: 'screenshot' },
      { label: '保存为 PDF', value: 'save_as_pdf' },
      { label: '网络拦截', value: 'network' },
      { label: '文件上传', value: 'upload' },
      { label: 'CDP 原始命令', value: 'cdp' },
      { label: '查找标签页', value: 'find_tab' },
      { label: '列出标签页', value: 'list_tabs' },
      { label: '关闭标签页', value: 'close_tab' },
      { label: '关闭会话', value: 'close_session' },
    ], default: 'navigate' },
    { key: 'args', label: '参数 (JSON)', type: 'textarea', placeholder: '{"url": "https://example.com"}' },
    { key: 'port', label: 'WebBridge 端口', type: 'number', default: 10086 },
    { key: 'timeout', label: '超时(秒)', type: 'number', default: 30 },
  ]},

  // ─── MCP 扩展节点（Python sidecar 驱动）───
  { type: 'mcp_script',       label: 'MCP 脚本',        icon: 'Code',          color: '#a5d6ff', description: '通过 MCP 执行 Python 脚本',               outputHint: '脚本返回值', params: [
    { key: 'script', label: 'Python 代码', type: 'textarea', placeholder: 'print("hello")' },
  ]},
  { type: 'mcp_shell',        label: 'MCP Shell',        icon: 'Terminal',      color: '#a5d6ff', dangerous: true, description: '通过 MCP 执行 Shell 命令',                outputHint: 'stdout, stderr, exit_code', params: [
    { key: 'command', label: '命令', type: 'text', placeholder: 'echo hello' },
    { key: 'cwd', label: '工作目录', type: 'text' },
  ]},
  { type: 'mcp_excel_read',   label: 'MCP 读取Excel',    icon: 'BookOpen',      color: '#a5d6ff', description: '通过 MCP 读取 Excel 文件',                outputHint: 'rows, cols, data', params: [
    { key: 'file_path', label: '文件路径', type: 'text', placeholder: './data.xlsx' },
    { key: 'sheet', label: '工作表', type: 'text', default: 'Sheet1' },
  ]},
  { type: 'mcp_excel_write',  label: 'MCP 写入Excel',    icon: 'Pencil',        color: '#a5d6ff', description: '通过 MCP 写入 Excel 文件',                outputHint: 'rows_written', params: [
    { key: 'file_path', label: '文件路径', type: 'text' },
    { key: 'sheet', label: '工作表', type: 'text', default: 'Sheet1' },
    { key: 'data', label: '数据', type: 'textarea', placeholder: '[["A","B"],[1,2]]' },
  ]},
  { type: 'mcp_excel_create', label: 'MCP 创建Excel',    icon: 'FileText',      color: '#a5d6ff', description: '通过 MCP 创建 Excel 文件',                outputHint: 'created: true', params: [
    { key: 'file_path', label: '文件路径', type: 'text' },
    { key: 'sheet', label: '工作表', type: 'text', default: 'Sheet1' },
    { key: 'headers', label: '表头', type: 'text', placeholder: '姓名,年龄' },
  ]},
  { type: 'mcp_excel_filter', label: 'MCP 筛选Excel',    icon: 'Filter',         color: '#a5d6ff', description: '通过 MCP 筛选 Excel 数据',                outputHint: 'data, filtered_count', params: [
    { key: 'file_path', label: '文件路径', type: 'text' },
    { key: 'column', label: '列', type: 'text' },
    { key: 'op', label: '条件', type: 'text', placeholder: 'equals / contains / gt' },
    { key: 'value', label: '值', type: 'text' },
  ]},
  { type: 'mcp_excel_sort',   label: 'MCP 排序Excel',    icon: 'ArrowUpDown',   color: '#a5d6ff', description: '通过 MCP 排序 Excel 数据',                outputHint: 'data, sorted_count', params: [
    { key: 'file_path', label: '文件路径', type: 'text' },
    { key: 'column', label: '列', type: 'text' },
    { key: 'order', label: '顺序', type: 'select', options: [{ label: '升序', value: 'asc' }, { label: '降序', value: 'desc' }], default: 'asc' },
  ]},
  { type: 'mcp_excel_append', label: 'MCP 追加Excel',    icon: 'FilePlus',       color: '#a5d6ff', description: '通过 MCP 追加行到 Excel',                outputHint: 'appended_rows', params: [
    { key: 'file_path', label: '文件路径', type: 'text' },
    { key: 'sheet', label: '工作表', type: 'text', default: 'Sheet1' },
    { key: 'data', label: '数据', type: 'textarea', placeholder: '[["new row"]]' },
  ]},
  { type: 'mcp_excel_csv',    label: 'MCP Excel↔CSV',    icon: 'FileText',      color: '#a5d6ff', description: '通过 MCP 转换 Excel 与 CSV',              outputHint: 'path, rows', params: [
    { key: 'file_path', label: '文件路径', type: 'text' },
    { key: 'direction', label: '方向', type: 'select', options: [{ label: 'CSV→Excel', value: 'csv_to_xlsx' }, { label: 'Excel→CSV', value: 'xlsx_to_csv' }], default: 'csv_to_xlsx' },
  ]},
  { type: 'mcp_word_read',    label: 'MCP 读取Word',     icon: 'BookOpen',      color: '#a5d6ff', description: '通过 MCP 读取 Word 文档内容',            outputHint: 'paragraphs', params: [
    { key: 'file_path', label: '文件路径', type: 'text' },
  ]},
  { type: 'mcp_word_write',   label: 'MCP 写入Word',     icon: 'Pencil',        color: '#a5d6ff', description: '通过 MCP 写入 Word 文档',                outputHint: 'saved: true', params: [
    { key: 'file_path', label: '文件路径', type: 'text' },
    { key: 'content', label: '内容', type: 'textarea' },
  ]},
  { type: 'mcp_word_create',  label: 'MCP 创建Word',     icon: 'FileText',      color: '#a5d6ff', description: '通过 MCP 创建 Word 文档',                outputHint: 'created: true', params: [
    { key: 'file_path', label: '文件路径', type: 'text' },
    { key: 'title', label: '标题', type: 'text', placeholder: '文档标题' },
  ]},
  { type: 'mcp_word_replace', label: 'MCP 替换Word',     icon: 'Replace',        color: '#a5d6ff', description: '通过 MCP 查找替换 Word 文本',            outputHint: 'replaced_count', params: [
    { key: 'file_path', label: '文件路径', type: 'text' },
    { key: 'old_text', label: '查找', type: 'text' },
    { key: 'new_text', label: '替换为', type: 'text' },
  ]},
  { type: 'mcp_word_merge',   label: 'MCP 合并Word',     icon: 'GitMerge',      color: '#a5d6ff', description: '通过 MCP 合并多个 Word 文档',            outputHint: 'merged: true', params: [
    { key: 'output', label: '输出路径', type: 'text' },
    { key: 'files', label: '文件列表 (JSON)', type: 'textarea', placeholder: '["a.docx","b.docx"]' },
  ]},
  { type: 'mcp_web_scrape',   label: 'MCP 网页抓取',     icon: 'Globe',         color: '#a5d6ff', description: '通过 MCP 提取网页内容',                  outputHint: 'title, text, links', params: [
    { key: 'url', label: 'URL', type: 'text', placeholder: 'https://example.com' },
    { key: 'selector', label: 'CSS选择器 (可选)', type: 'text' },
  ]},
]

/** 动态注册的后端节点类型（插件安装后由 node_list_types 同步） */
const _dynamicDefs: ContainerDef[] = []

/** 合并静态 + 动态定义的容器列表 */
export function allContainerDefs(): ContainerDef[] {
  return getSchemaDefs() || [...CONTAINER_DEFS, ..._dynamicDefs]
}

/** 注册插件提供的动态节点类型 */
export function registerDynamicNode(def: ContainerDef) {
  if (!CONTAINER_DEFS.some(d => d.type === def.type) && !_dynamicDefs.some(d => d.type === def.type)) {
    _dynamicDefs.push(def)
  }
}

/** 清空动态注册（插件卸载时） */
export function clearDynamicNodes() {
  _dynamicDefs.length = 0
}

export function getContainerDef(type: string, t?: TFn): ContainerDef {
  const all = allContainerDefs()
  const found = all.find(d => d.type === type)
  const def = found || all[0]
  if (!found) console.warn(`getContainerDef: 未知类型 "${type}"，使用 fallback`)
  if (!t) return def
  return {
    ...def,
    label: t(`nodeLabel.${type}`, def.label),
    description: t(`nodeDesc.${type}`, def.description),
  }
}

/** 获取容器类型的主色 */
export function getContainerColorVar(type: string): string {
  const all = getSchemaDefs() || CONTAINER_DEFS
  const def = all.find(d => d.type === type)
  return def?.color || '#8b949e'
}

// ─── 浏览器动作 ───

export const BROWSER_ACTIONS: ActionDef[] = [
  { type: 'navigate', label: '打开页面', icon: 'Link', params: [
    { key: 'url', label: '网址', type: 'text', placeholder: 'https://example.com' },
  ]},
  { type: 'click', label: '点击元素', icon: 'MousePointerClick', params: [
    { key: 'selector', label: '选择器', type: 'text', placeholder: '#btn 或 .class' },
  ]},
  { type: 'input', label: '输入文本', icon: 'Keyboard', params: [
    { key: 'selector', label: '选择器', type: 'text' },
    { key: 'value', label: '内容', type: 'text' },
  ]},
  { type: 'wait', label: '等待', icon: 'Clock', params: [
    { key: 'ms', label: '毫秒', type: 'number', default: 1000 },
  ]},
  { type: 'screenshot', label: '截图', icon: 'Camera', params: [
    { key: 'path', label: '保存路径', type: 'text', placeholder: './screenshot.png' },
  ]},
  { type: 'evaluate', label: '执行 JS', icon: 'Zap', params: [
    { key: 'script', label: '脚本', type: 'textarea' },
  ]},
  { type: 'scroll', label: '滚动页面', icon: 'ScrollText', params: [
    { key: 'x', label: '横向像素', type: 'number', default: 0 },
    { key: 'y', label: '纵向像素', type: 'number', default: 500 },
  ]},
  { type: 'extract', label: '提取数据', icon: 'ClipboardList', params: [
    { key: 'selector', label: '选择器', type: 'text', placeholder: 'body', default: 'body' },
    { key: 'mode', label: '模式', type: 'select', options: [
      { label: '文本', value: 'text' }, { label: 'HTML', value: 'html' },
      { label: '属性', value: 'attr' },
    ], default: 'text' },
  ]},
  { type: 'get_title', label: '获取标题', icon: 'Tag', params: [] },
  // ─── v1.1+ 扩展动作 ───
  { type: 'extract_table', label: '提取表格', icon: 'BarChart3', params: [
    { key: 'selector', label: '选择器', type: 'text', placeholder: 'table', default: 'table' },
  ]},
  { type: 'extract_links', label: '提取链接', icon: 'Link', params: [
    { key: 'selector', label: '范围选择器', type: 'text', placeholder: 'body', default: 'body' },
  ]},
  { type: 'select', label: '下拉选择', icon: 'ChevronDown', params: [
    { key: 'selector', label: '选择器', type: 'text' },
    { key: 'value', label: '值', type: 'text', placeholder: 'optionValue 或 显示文本' },
  ]},
  { type: 'check', label: '勾选/取消', icon: 'CheckSquare', params: [
    { key: 'selector', label: '选择器', type: 'text' },
    { key: 'checked', label: '勾选', type: 'checkbox', default: true },
  ]},
  { type: 'hover', label: '鼠标悬停', icon: 'MousePointer', params: [
    { key: 'selector', label: '选择器', type: 'text' },
  ]},
  { type: 'cookies', label: 'Cookie 管理', icon: 'Cookie', params: [
    { key: 'action', label: '操作', type: 'select', options: [
      { label: '获取', value: 'get' }, { label: '设置', value: 'set' }, { label: '清除', value: 'clear' },
    ], default: 'get' },
    { key: 'cookies', label: 'Cookie 数据 (JSON)', type: 'textarea', placeholder: 'set 时填写 [{name,value,domain,...}]' },
  ]},
  { type: 'set_headers', label: '设置请求头', icon: 'Radio', params: [
    { key: 'headers', label: '请求头 (JSON)', type: 'textarea', placeholder: '{"Authorization": "Bearer xxx"}' },
  ]},
  { type: 'new_page', label: '新建标签页', icon: 'Plus', params: [
    { key: 'url', label: '初始网址', type: 'text', placeholder: 'https://...' },
  ]},
  { type: 'close_page', label: '关闭标签页', icon: 'X', params: [
    { key: 'index', label: '标签索引', type: 'number' },
  ]},
  { type: 'switch_page', label: '切换标签页', icon: 'RefreshCw', params: [
    { key: 'index', label: '目标索引', type: 'number', default: 0 },
  ]},
  { type: 'pages', label: '标签页列表', icon: 'BookOpen', params: [] },
  { type: 'back', label: '后退', icon: 'ArrowLeft', params: [] },
  { type: 'forward', label: '前进', icon: 'ArrowRight', params: [] },
  { type: 'reload', label: '刷新页面', icon: 'RefreshCw', params: [] },
  { type: 'current_url', label: '当前网址', icon: 'Link', params: [] },
  { type: 'pdf', label: '生成 PDF', icon: 'FileText', params: [
    { key: 'path', label: '保存路径', type: 'text', default: 'output.pdf' },
  ]},
  // ─── 智能等待 ───
  { type: 'wait_network_idle', label: '等待网络空闲', icon: 'Globe', params: [
    { key: 'timeout', label: '超时(ms)', type: 'number', default: 30000 },
  ]},
  { type: 'wait_load_state', label: '等待加载状态', icon: 'Clock', params: [
    { key: 'state', label: '状态', type: 'select', options: [
      { label: 'load（页面加载完成）', value: 'load' },
      { label: 'domcontentloaded（DOM就绪）', value: 'domcontentloaded' },
      { label: 'networkidle（网络空闲）', value: 'networkidle' },
    ], default: 'load' },
    { key: 'timeout', label: '超时(ms)', type: 'number', default: 30000 },
  ]},
  { type: 'wait_url_contains', label: '等待 URL 变更', icon: 'Link', params: [
    { key: 'substring', label: 'URL 包含', type: 'text', placeholder: '/dashboard' },
    { key: 'timeout', label: '超时(ms)', type: 'number', default: 30000 },
  ]},
  // ─── 动作验证 ───
  { type: 'verify', label: '验证健康', icon: 'Activity', params: [] },
  // ─── 文件下载 ───
  { type: 'download', label: '下载文件', icon: 'Download', params: [
    { key: 'save_dir', label: '保存目录', type: 'text', default: '.' },
    { key: 'click_selector', label: '点击选择器', type: 'text', placeholder: '先点这个再等下载（可选）' },
    { key: 'timeout', label: '超时(ms)', type: 'number', default: 30000 },
  ]},
  // ─── v1.1+ 扩展动作（第二批）───
  { type: 'upload', label: '上传文件', icon: 'Upload', params: [
    { key: 'selector', label: '选择器', type: 'text', placeholder: 'input[type=file]' },
    { key: 'file_paths', label: '文件路径', type: 'text', placeholder: 'C:\\a.xlsx,D:\\b.pdf（逗号分隔）' },
  ]},
  { type: 'keyboard', label: '键盘操作', icon: 'Keyboard', params: [
    { key: 'key', label: '按键', type: 'text', placeholder: 'Enter / Control+s / Tab' },
    { key: 'text', label: '输入文本', type: 'text', placeholder: '直接输入的字符（与 key 二选一）' },
    { key: 'delay', label: '按键间隔(ms)', type: 'number', default: 0 },
  ]},
  { type: 'double_click', label: '双击元素', icon: 'MousePointerClick', params: [
    { key: 'selector', label: '选择器', type: 'text' },
    { key: 'timeout_ms', label: '超时(ms)', type: 'number', default: 10000 },
  ]},
  { type: 'drag_to', label: '拖拽元素', icon: 'MoveHorizontal', params: [
    { key: 'source', label: '源选择器', type: 'text' },
    { key: 'target', label: '目标选择器', type: 'text' },
    { key: 'source_position', label: '源位置', type: 'text', placeholder: '{"x":5,"y":5}' },
    { key: 'target_position', label: '目标位置', type: 'text', placeholder: '{"x":10,"y":10}' },
  ]},
  { type: 'context_menu', label: '右键菜单', icon: 'ClipboardList', params: [
    { key: 'selector', label: '选择器', type: 'text' },
    { key: 'timeout_ms', label: '超时(ms)', type: 'number', default: 10000 },
  ]},
  { type: 'switch_frame', label: '切换 iframe', icon: 'Frame', params: [
    { key: 'selector', label: 'iframe选择器', type: 'text', placeholder: '留空=回主文档' },
  ]},
  { type: 'handle_dialog', label: '处理弹窗', icon: 'MessageSquare', params: [
    { key: 'action', label: '操作', type: 'select', options: [
      { label: '接受', value: 'accept' }, { label: '拒绝', value: 'reject' },
    ], default: 'accept' },
    { key: 'prompt_text', label: '输入文本', type: 'text', placeholder: 'prompt 弹窗填值' },
  ]},
  { type: 'scroll_to_element', label: '滚动到元素', icon: 'Target', params: [
    { key: 'selector', label: '选择器', type: 'text' },
    { key: 'behavior', label: '行为', type: 'select', options: [
      { label: '平滑', value: 'smooth' }, { label: '瞬时', value: 'instant' },
    ], default: 'smooth' },
    { key: 'block', label: '对齐', type: 'select', options: [
      { label: '居中', value: 'center' }, { label: '起始', value: 'start' },
      { label: '结束', value: 'end' }, { label: '最近', value: 'nearest' },
    ], default: 'center' },
  ]},
]

// Excel 动作（与后端 excel.rs / excel_container.rs 对齐）
export const EXCEL_ACTIONS: ActionDef[] = [
  { type: 'read', label: '读取整表', icon: 'BookOpen', params: [] },
  { type: 'write', label: '写入数据', icon: 'Pencil', params: [
    { key: 'value', label: '数据', type: 'textarea', placeholder: '[["姓名","年龄"],["张三",25]] 或 {{变量}}' },
  ]},
  { type: 'create', label: '创建文件', icon: 'FileText', params: [
    { key: 'headers', label: '表头', type: 'text', placeholder: '姓名,年龄,城市（逗号分隔）' },
  ]},
  { type: 'append', label: '追加行', icon: 'Plus', params: [
    { key: 'value', label: '数据', type: 'textarea', placeholder: '[["李四",30]] 或 {{变量}}' },
  ]},
  { type: 'filter', label: '筛选', icon: 'Search', params: [
    { key: 'column', label: '列', type: 'text', placeholder: 'A 或 列名' },
    { key: 'op', label: '条件', type: 'select', options: [
      { label: '包含', value: 'contains' }, { label: '等于', value: 'equals' },
      { label: '不等于', value: 'not_equals' }, { label: '大于', value: 'gt' },
      { label: '大于等于', value: 'gte' }, { label: '小于', value: 'lt' },
      { label: '小于等于', value: 'lte' }, { label: '为空', value: 'is_empty' },
      { label: '不为空', value: 'not_empty' },
    ], default: 'contains' },
    { key: 'value', label: '值', type: 'text' },
  ]},
  { type: 'sort', label: '排序', icon: 'ArrowUpDown', params: [
    { key: 'column', label: '列', type: 'text', placeholder: 'A' },
    { key: 'order', label: '顺序', type: 'select', options: [
      { label: '升序', value: 'asc' }, { label: '降序', value: 'desc' },
    ], default: 'asc' },
  ]},
  { type: 'formula', label: '公式', icon: 'Calculator', params: [
    { key: 'cell', label: '单元格', type: 'text', placeholder: 'A1' },
    { key: 'formula', label: '公式', type: 'text', placeholder: 'SUM(A1:A10)' },
  ]},
]

// Word 动作（与后端 word_container.rs 对齐）
export const WORD_ACTIONS: ActionDef[] = [
  { type: 'read', label: '读取内容', icon: 'BookOpen', params: [] },
  { type: 'write', label: '写入段落', icon: 'Pencil', params: [
    { key: 'value', label: '内容', type: 'textarea' },
  ]},
  { type: 'replace', label: '替换文本', icon: 'RefreshCw', params: [
    { key: 'old_text', label: '查找', type: 'text' },
    { key: 'new_text', label: '替换为', type: 'text' },
  ]},
  { type: 'create', label: '创建文档', icon: 'FileText', params: [
    { key: 'title', label: '标题', type: 'text', placeholder: '文档标题' },
  ]},
  { type: 'insert_table', label: '插入表格', icon: 'BarChart3', params: [
    { key: 'data', label: '表格数据', type: 'textarea', placeholder: '[["姓名","年龄"],["张三",25]]' },
  ]},
  { type: 'merge', label: '合并文档', icon: 'Paperclip', params: [
    { key: 'files', label: '文件列表 (JSON)', type: 'textarea', placeholder: '["a.docx","b.docx"]' },
  ]},
]

// 逻辑动作 —— 操作符定义
export const LOGIC_OPERATORS = [
  { type: 'contains', label: '包含', icon: 'Search', hasRight: true },
  { type: 'not_contains', label: '不包含', icon: 'Ban', hasRight: true },
  { type: 'equals', label: '等于', icon: '=', hasRight: true },
  { type: 'not_equals', label: '不等于', icon: 'X', hasRight: true },
  { type: 'greater_than', label: '大于', icon: '>', hasRight: true },
  { type: 'less_than', label: '小于', icon: '<', hasRight: true },
  { type: 'greater_equal', label: '大于等于', icon: 'TrendingUp', hasRight: true },
  { type: 'less_equal', label: '小于等于', icon: 'TrendingDown', hasRight: true },
  { type: 'starts_with', label: '开头是', icon: 'ArrowUpRight', hasRight: true },
  { type: 'ends_with', label: '结尾是', icon: 'ArrowDownRight', hasRight: true },
  { type: 'is_empty', label: '为空', icon: 'CircleOff', hasRight: false },
  { type: 'not_empty', label: '不为空', icon: 'CheckCircle', hasRight: false },
  { type: 'regex', label: '正则匹配', icon: 'Asterisk', hasRight: true },
]

// 兼容旧的 LOGIC_ACTIONS（ActionPanel 用）
export const LOGIC_ACTIONS: ActionDef[] = LOGIC_OPERATORS.map(op => ({
  type: op.type,
  label: op.label,
  icon: op.icon,
  params: op.hasRight
    ? [
        { key: 'left', label: '左侧值', type: 'text' as const, placeholder: '{{变量}}' },
        { key: 'right', label: '右侧值', type: 'text' as const },
      ]
    : [
        { key: 'left', label: '检查值', type: 'text' as const, placeholder: '{{变量}}' },
      ],
}))

// ─── 迭代节点 body 步骤定义（cursor/loop 的子步骤） ───
export const BODY_STEP_ACTIONS: ActionDef[] = [
  { type: 'http', label: 'HTTP 请求', icon: 'Globe', params: [
    { key: 'method', label: '方法', type: 'select', options: [
      { label: 'GET', value: 'GET' }, { label: 'POST', value: 'POST' },
    ], default: 'GET' },
    { key: 'url', label: 'URL', type: 'text', placeholder: 'https://api.example.com' },
    { key: 'body', label: '请求体', type: 'textarea' },
  ]},
  { type: 'script', label: '脚本', icon: 'ScrollText', params: [
    { key: 'script', label: '代码', type: 'textarea' },
  ]},
  { type: 'delay', label: '延迟等待', icon: 'Clock', params: [
    { key: 'duration_ms', label: '毫秒', type: 'number', default: 1000 },
  ]},
  { type: 'notify', label: '通知', icon: 'Bell', params: [
    { key: 'title', label: '标题', type: 'text' },
    { key: 'body', label: '内容', type: 'textarea' },
  ]},
  { type: 'clipboard', label: '剪贴板', icon: 'ClipboardList', params: [
    { key: 'action', label: '操作', type: 'select', options: [
      { label: '读取', value: 'read' }, { label: '写入', value: 'write' },
    ], default: 'read' },
    { key: 'text', label: '写入内容', type: 'textarea' },
  ]},
]

// ─── 文件容器 — 10 个动作 v6.9.0 ───
export const FILE_ACTIONS: ActionDef[] = [
  { type: 'read', label: '读取文件', icon: 'FileText', params: [
    { key: 'path', label: '文件路径', type: 'text', placeholder: 'C:\\data\\file.txt' },
    { key: 'encoding', label: '编码', type: 'select', options: [
      { label: 'UTF-8', value: 'utf-8' }, { label: 'GBK', value: 'gbk' },
    ], default: 'utf-8' },
  ]},
  { type: 'write', label: '写入文件', icon: 'FilePlus', params: [
    { key: 'path', label: '文件路径', type: 'text' },
    { key: 'content', label: '内容', type: 'textarea' },
  ]},
  { type: 'append', label: '追加内容', icon: 'FileUp', params: [
    { key: 'path', label: '文件路径', type: 'text' },
    { key: 'content', label: '追加内容', type: 'textarea' },
    { key: 'newline', label: '自动换行', type: 'select', options: [
      { label: '是', value: 'true' }, { label: '否', value: 'false' },
    ], default: 'true' },
  ]},
  { type: 'copy', label: '复制文件', icon: 'Copy', params: [
    { key: 'from', label: '源路径', type: 'text' },
    { key: 'to', label: '目标路径', type: 'text' },
  ]},
  { type: 'move', label: '移动/重命名', icon: 'MoveRight', params: [
    { key: 'from', label: '源路径', type: 'text' },
    { key: 'to', label: '目标路径', type: 'text' },
  ]},
  { type: 'delete', label: '删除', icon: 'Trash2', params: [
    { key: 'path', label: '路径', type: 'text' },
  ]},
  { type: 'list', label: '列出目录', icon: 'FolderTree', params: [
    { key: 'path', label: '目录路径', type: 'text', placeholder: 'C:\\Users\\haozi\\Desktop' },
    { key: 'pattern', label: '过滤', type: 'text', placeholder: '*.txt' },
  ]},
  { type: 'glob', label: '模式查找', icon: 'Search', params: [
    { key: 'pattern', label: '匹配模式', type: 'text', placeholder: '*.rs' },
    { key: 'path', label: '搜索目录', type: 'text', placeholder: '.' },
    { key: 'max_depth', label: '最大深度', type: 'number', default: 10 },
  ]},
  { type: 'exists', label: '检查存在', icon: 'FileCheck', params: [
    { key: 'path', label: '路径', type: 'text' },
  ]},
  { type: 'grep', label: '搜索内容', icon: 'ScanSearch', params: [
    { key: 'path', label: '文件/目录', type: 'text' },
    { key: 'pattern', label: '搜索模式', type: 'text', placeholder: 'regex 或文本' },
    { key: 'file_pattern', label: '文件过滤', type: 'text', placeholder: '*.rs' },
    { key: 'max_depth', label: '最大深度', type: 'number', default: 5 },
    { key: 'max_results', label: '最大结果', type: 'number', default: 100 },
  ]},
]

// ─── 注册表查询函数 ───

export function getActionDefs(containerType: ContainerType, t?: TFn): ActionDef[] {
  const defs = (() => { switch (containerType) {
    case 'file': return FILE_ACTIONS
    case 'browser': return BROWSER_ACTIONS
    case 'excel': return EXCEL_ACTIONS
    case 'word': return WORD_ACTIONS
    case 'logic': return LOGIC_ACTIONS
    case 'cursor': return BODY_STEP_ACTIONS
    case 'loop': return BODY_STEP_ACTIONS
    default: return []
  }})()
  if (!t) return defs
  return defs.map(d => ({ ...d, label: t(`actionLabel.${d.type}`, d.label) }))
}

export function getActionDef(containerType: ContainerType, actionType: string, t?: TFn): ActionDef | undefined {
  return getActionDefs(containerType, t).find(a => a.type === actionType)
}

/** 获取动作显示名称 — t 存在时优先翻译，否则回退到已存储的 label */
export function getActionLabel(action: Action, containerType?: ContainerType, t?: TFn): string {
  if (t && containerType) {
    const def = getActionDef(containerType, action.type, t)
    if (def) return def.label
  }
  if (action.label) return action.label
  if (containerType) {
    const def = getActionDef(containerType, action.type)
    if (def) return def.label
  }
  return action.type
}

/** 步骤是否为容器类型（有 actions 列表） */
export function isContainerType(type: ContainerType): boolean {
  const def = CONTAINER_DEFS.find(d => d.type === type)
  return def?.isContainer === true
}

// ─── 工厂函数 ───

export function newAction(type: string, containerType?: ContainerType, existingActions?: Action[], stepId?: string, t?: TFn): Action {
  const def = containerType ? getActionDef(containerType, type, t) : undefined
  let label = def?.label || type
  // 重名处理：自动加 _2, _3 后缀
  if (existingActions && existingActions.length > 0) {
    const existingLabels = existingActions.map(a => a.label)
    if (existingLabels.includes(label)) {
      let suffix = 2
      while (existingLabels.includes(`${label}_${suffix}`)) suffix++
      label = `${label}_${suffix}`
    }
  }
  const id = stepId ? nextActionId(stepId, existingActions || []) : uid('act')
  return { id, type, label, params: {} }
}

export function newStep(containerType: ContainerType, existingSteps?: Step[], t?: TFn): Step {
  const def = getContainerDef(containerType, t)
  // 从容器定义中提取默认参数
  const config: Record<string, unknown> = {}
  for (const p of def.params) {
    if (p.default !== undefined) {
      config[p.key] = p.default
    }
  }
  const step: Step = {
    id: nextStepId(existingSteps || []),
    type: containerType,
    label: def.label,
    expanded: true,
    actions: [],
    config,
  }
  if (containerType === 'logic') {
    step.condition = ''
  }
  return step
}
