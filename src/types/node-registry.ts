// ─── Workflow Engine v5 节点注册表 ───
// 容器定义、动作列表、工厂函数

import type {
  ContainerType, ContainerDef, ActionDef,
  Action, Step,
} from './types'
import { uid, nextStepId, nextActionId } from './types'

// ─── 容器定义 ───

export const CONTAINER_DEFS: ContainerDef[] = [
  { type: 'browser', label: '浏览器', icon: 'Globe', color: '#79c0ff', isContainer: true, description: '网页操作：导航、点击、输入、提取', outputHint: '{ actionId: value, ... }', params: [
    { key: 'browser', label: '浏览器', type: 'select', options: [
      { label: 'Chromium', value: 'chromium' }, { label: 'Firefox', value: 'firefox' }, { label: 'WebKit', value: 'webkit' },
    ], default: 'chromium' },
    { key: 'headless', label: '无头模式', type: 'checkbox', default: false },
    { key: 'timeout', label: '超时(ms)', type: 'number', default: 30000 },
  ]},
  { type: 'excel', label: 'Excel', icon: 'BarChart3', color: '#3fb950', isContainer: true, description: 'Excel 操作：读写单元格、筛选、排序', outputHint: '{ actionId: value, ... }', params: [
    { key: 'file_path', label: '文件路径', type: 'text', placeholder: './data.xlsx' },
    { key: 'sheet', label: '工作表', type: 'text', default: 'Sheet1' },
  ]},
  { type: 'word', label: 'Word', icon: 'FileText', color: '#bc8cff', isContainer: true, description: 'Word 操作：读写、替换、合并', outputHint: '{ actionId: value, ... }', params: [
    { key: 'file_path', label: '文件路径', type: 'text', placeholder: './document.docx' },
  ]},
  { type: 'logic', label: '条件判断', icon: 'GitBranch', color: '#d29922', isContainer: true, description: '条件分支：满足/不满足走不同路径', outputHint: '{ branch: "true"/"false", value, result }', params: [
    { key: 'condition', label: '条件表达式', type: 'text', placeholder: '{{step1.output}} == "异常"' },
  ]},
  // ─── 新增简单步骤类型（T11） ───
  { type: 'http', label: 'HTTP 请求', icon: 'Globe', color: '#39d2c0', description: '发送 HTTP 请求：GET/POST/PUT/DELETE', outputHint: '{ status, body, headers }', params: [
    { key: 'method', label: '方法', type: 'select', options: [
      { label: 'GET', value: 'GET' }, { label: 'POST', value: 'POST' }, { label: 'PUT', value: 'PUT' }, { label: 'DELETE', value: 'DELETE' },
    ], default: 'GET' },
    { key: 'url', label: 'URL', type: 'text', placeholder: 'https://api.example.com/data' },
    { key: 'headers', label: '请求头 (JSON)', type: 'textarea', placeholder: '{"Content-Type": "application/json"}' },
    { key: 'body', label: '请求体', type: 'textarea', placeholder: '{"key": "value"}' },
  ]},
  { type: 'delay', label: '延迟等待', icon: 'Clock', color: '#adbac7', description: '等待指定时间后继续', outputHint: '{ waited: ms }', params: [
    { key: 'duration_ms', label: '毫秒', type: 'number', default: 1000 },
    { key: 'max_duration_ms', label: '最大毫秒(随机)', type: 'number', default: 5000 },
  ]},
  { type: 'notify', label: '通知', icon: 'Bell', color: '#f0883e', description: '发送通知：系统通知/Webhook', outputHint: '{ sent: true }', params: [
    { key: 'notify_type', label: '渠道', type: 'select', options: [
      { label: '系统通知', value: 'system' }, { label: 'Webhook', value: 'webhook' },
    ], default: 'system' },
    { key: 'title', label: '标题', type: 'text' },
    { key: 'body', label: '内容', type: 'textarea' },
    { key: 'url', label: 'Webhook URL', type: 'text', placeholder: 'https://hooks.example.com/...' },
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
  { type: 'cursor', label: '游标迭代', icon: 'Repeat', color: '#e85d75', isContainer: true, description: '逐条迭代：每次运行处理一行/一项，游标跨次保存', outputHint: '{ done, item, index, total }', params: [
    { key: 'items', label: '数据源', type: 'text', placeholder: '{{read_excel.data}}' },
  ]},
  { type: 'loop', label: '批量循环', icon: 'RefreshCw', color: '#daaa3e', isContainer: true, description: '一次性遍历全部数据，适合小数据内存变换', outputHint: '{ count, results[] }', params: [
    { key: 'items', label: '数据源', type: 'text', placeholder: '{{step1.data}} 或 [[1,2,3]]' },
  ]},
  { type: 'approval', label: '人工审批', icon: 'Hand', color: '#f778ba', description: '暂停流程等待人工审核：支持自定义选项和推荐', outputHint: '{ decision: "选项名", comment, item, auto? }', params: [
    { key: 'title', label: '审批标题', type: 'text', placeholder: '请确认订单信息' },
    { key: 'message', label: '审批内容', type: 'textarea', placeholder: '订单号：{{step_1.action_1_1.订单号}}' },
    { key: 'options', label: '审批选项', type: 'text', placeholder: '同意,拒绝,需要更多信息（逗号分隔）', default: '同意,拒绝' },
    { key: 'recommended', label: '推荐选项', type: 'text', placeholder: '同意', default: '同意' },
    { key: 'require_review', label: '需要人工审核', type: 'select', options: [
      { label: '是', value: 'true' }, { label: '否（自动决策）', value: 'false' },
    ], default: 'true' },
    { key: 'timeout', label: '超时(秒)', type: 'number', default: 300 },
    { key: 'timeout_action', label: '超时策略', type: 'select', options: [
      { label: '执行推荐选项', value: 'recommended' },
      { label: '自动拒绝', value: 'reject' },
      { label: '自动通过', value: 'approve' },
      { label: '标记失败', value: 'fail' },
    ], default: 'recommended' },
  ]},
]

export function getContainerDef(type: string): ContainerDef {
  const found = CONTAINER_DEFS.find(d => d.type === type)
  if (!found) console.warn(`getContainerDef: 未知类型 "${type}"，使用 fallback`)
  return found || CONTAINER_DEFS[0]
}

/** 获取容器类型的主色 */
export function getContainerColorVar(type: string): string {
  const def = CONTAINER_DEFS.find(d => d.type === type)
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

// Excel 动作（与后端 excel_container.rs 对齐）
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

// ─── 注册表查询函数 ───

export function getActionDefs(containerType: ContainerType): ActionDef[] {
  switch (containerType) {
    case 'browser': return BROWSER_ACTIONS
    case 'excel': return EXCEL_ACTIONS
    case 'word': return WORD_ACTIONS
    case 'logic': return LOGIC_ACTIONS
    case 'cursor': return BODY_STEP_ACTIONS
    case 'loop': return BODY_STEP_ACTIONS
    default: return []
  }
}

export function getActionDef(containerType: ContainerType, actionType: string): ActionDef | undefined {
  return getActionDefs(containerType).find(a => a.type === actionType)
}

/** 获取动作显示名称（兼容旧数据） */
export function getActionLabel(action: Action, containerType?: ContainerType): string {
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

export function newAction(type: string, containerType?: ContainerType, existingActions?: Action[], stepId?: string): Action {
  const def = containerType ? getActionDef(containerType, type) : undefined
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

export function newStep(containerType: ContainerType, existingSteps?: Step[]): Step {
  const def = getContainerDef(containerType)
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
