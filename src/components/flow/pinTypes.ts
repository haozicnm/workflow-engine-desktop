// ─── 针脚类型系统：类型枚举、颜色映射、辅助函数 ───

export type PinType = 'string' | 'number' | 'boolean' | 'object' | 'array' | 'file' | 'any' | 'error'

/** 针脚类型 → 颜色（ComfyUI 风格） */
export const PIN_COLORS: Record<PinType, string> = {
  string: '#4A9EFF',
  number: '#4ADE80',
  boolean: '#FBBF24',
  object: '#FB923C',
  array: '#C084FC',
  file: '#9CA3AF',
  any: '#E5E7EB',
  error: '#EF4444',
}

/** 针脚类型 → 简写符号 */
export const PIN_BADGES: Record<PinType, string> = {
  string: 'Aa',
  number: '#',
  boolean: '◉',
  object: '{}',
  array: '[]',
  file: '📄',
  any: '*',
  error: '!',
}

/** 节点执行状态 */
export type NodeStatus = 'idle' | 'queued' | 'running' | 'success' | 'error' | 'warning'

export const STATUS_COLORS: Record<NodeStatus, string> = {
  idle: '#484f58',
  queued: '#d29922',
  running: '#58a6ff',
  success: '#3fb950',
  error: '#f85149',
  warning: '#d29922',
}

/** 针脚定义 */
export interface PinDefinition {
  id: string
  label: string
  type: PinType
  required?: boolean
}

/** 节点定义（注册表条目） */
export interface NodeDefinition {
  type: string
  label: string
  icon: string
  color: string
  category: 'browser' | 'excel' | 'word' | 'data' | 'logic' | 'output' | 'other'
  description?: string
  inputs: PinDefinition[]
  outputs: PinDefinition[]
  defaultConfig: Record<string, unknown>
}

/** 画布上的运行时节点 */
export interface FlowNode {
  id: string
  type: string
  label: string
  position: { x: number; y: number }
  config: Record<string, unknown>
  status?: NodeStatus
  output?: unknown
  error?: string
  duration?: number
}

/** 连线 */
export interface FlowEdge {
  id: string
  source: string
  target: string
  sourceHandle: string
  targetHandle: string
}

// ─── 节点注册表 ───
export const NODE_REGISTRY: NodeDefinition[] = [
  // ═══════════════════════════════════════════
  // 🖥️ 浏览器（核心功能，最上面）
  // ═══════════════════════════════════════════
  {
    type: 'browser_navigate',
    label: '浏览器导航',
    icon: '🧭',
    color: '#3fb950',
    category: 'browser',
    description: '导航到指定 URL',
    inputs: [{ id: 'url', label: 'URL', type: 'string' }],
    outputs: [{ id: 'page', label: '页面数据', type: 'object' }],
    defaultConfig: { url: 'https://example.com', wait_until: 'load' },
  },
  {
    type: 'browser_click',
    label: '浏览器点击',
    icon: '👆',
    color: '#3fb950',
    category: 'browser',
    description: '点击页面元素',
    inputs: [{ id: 'selector', label: '选择器', type: 'string' }],
    outputs: [{ id: 'data', label: '结果', type: 'object' }],
    defaultConfig: { selector: '' },
  },
  {
    type: 'browser_fill',
    label: '浏览器填写',
    icon: '⌨️',
    color: '#3fb950',
    category: 'browser',
    description: '填写表单字段',
    inputs: [
      { id: 'selector', label: '选择器', type: 'string' },
      { id: 'value', label: '值', type: 'string' },
    ],
    outputs: [{ id: 'data', label: '结果', type: 'object' }],
    defaultConfig: { selector: '', value: '', clear: true },
  },
  {
    type: 'browser_extract',
    label: '浏览器提取',
    icon: '⛏️',
    color: '#3fb950',
    category: 'browser',
    description: '提取页面数据（文本/HTML/表格/链接/属性）',
    inputs: [{ id: 'selector', label: '选择器', type: 'string' }],
    outputs: [{ id: 'data', label: '数据', type: 'array' }],
    defaultConfig: { mode: 'text', selector: '', attribute: 'href' },
  },
  {
    type: 'browser_screenshot',
    label: '浏览器截图',
    icon: '📸',
    color: '#3fb950',
    category: 'browser',
    description: '截取当前页面',
    inputs: [],
    outputs: [{ id: 'path', label: '路径', type: 'string' }],
    defaultConfig: { path: 'screenshot.png', full_page: false },
  },
  {
    type: 'browser_evaluate',
    label: '浏览器执行JS',
    icon: '⚡',
    color: '#3fb950',
    category: 'browser',
    description: '在页面中执行 JavaScript',
    inputs: [{ id: 'script', label: '脚本', type: 'string' }],
    outputs: [{ id: 'result', label: '结果', type: 'any' }],
    defaultConfig: { script: 'return document.title' },
  },
  {
    type: 'browser_scroll',
    label: '浏览器滚动',
    icon: '📜',
    color: '#3fb950',
    category: 'browser',
    description: '滚动页面',
    inputs: [],
    outputs: [{ id: 'data', label: '结果', type: 'object' }],
    defaultConfig: { direction: 'down', distance: 300 },
  },
  {
    type: 'browser_wait',
    label: '浏览器等待',
    icon: '⏳',
    color: '#3fb950',
    category: 'browser',
    description: '等待元素出现或指定时长',
    inputs: [{ id: 'selector', label: '选择器', type: 'string' }],
    outputs: [{ id: 'found', label: '找到', type: 'boolean' }],
    defaultConfig: { selector: '', timeout_ms: 30000 },
  },
  {
    type: 'browser_pdf',
    label: '浏览器PDF',
    icon: '📑',
    color: '#3fb950',
    category: 'browser',
    description: '导出当前页面为 PDF',
    inputs: [],
    outputs: [{ id: 'path', label: '路径', type: 'string' }],
    defaultConfig: { path: 'page.pdf' },
  },
  {
    type: 'browser',
    label: '浏览器（万能）',
    icon: '🌐',
    color: '#3fb950',
    category: 'browser',
    description: '通用浏览器操作（兼容旧版）',
    inputs: [{ id: 'input', label: '输入', type: 'any' }],
    outputs: [
      { id: 'data', label: '数据', type: 'any' },
      { id: 'error', label: '错误', type: 'string' },
    ],
    defaultConfig: { action: 'navigate', url: '', selector: '' },
  },
  {
    type: 'web_scrape',
    label: '网页抓取',
    icon: '🕸️',
    color: '#3fb950',
    category: 'browser',
    description: '抓取网页内容（无需浏览器）',
    inputs: [{ id: 'url', label: 'URL', type: 'string' }],
    outputs: [{ id: 'data', label: '数据', type: 'string' }],
    defaultConfig: { url: '', method: 'GET' },
  },
  {
    type: 'recording',
    label: '录制回放',
    icon: '🔴',
    color: '#f85149',
    category: 'browser',
    description: '录制浏览器操作并回放',
    inputs: [],
    outputs: [{ id: 'data', label: '结果', type: 'object' }],
    defaultConfig: { source: '' },
  },

  // ═══════════════════════════════════════════
  // 📊 Excel 文档处理
  // ═══════════════════════════════════════════
  {
    type: 'excel',
    label: 'Excel 操作',
    icon: '📊',
    color: '#4ADE80',
    category: 'excel',
    description: '读取/写入 Excel 文件',
    inputs: [{ id: 'input', label: '输入', type: 'any' }],
    outputs: [{ id: 'data', label: '数据', type: 'object' }],
    defaultConfig: { path: '', sheet: '', action: 'read' },
  },

  // ═══════════════════════════════════════════
  // 📄 Word 文档处理
  // ═══════════════════════════════════════════
  {
    type: 'word',
    label: 'Word 操作',
    icon: '📝',
    color: '#60A5FA',
    category: 'word',
    description: '读取/写入 Word 文档',
    inputs: [{ id: 'input', label: '输入', type: 'any' }],
    outputs: [{ id: 'data', label: '数据', type: 'object' }],
    defaultConfig: { path: '', action: 'read' },
  },

  // ═══════════════════════════════════════════
  // 🔧 数据处理
  // ═══════════════════════════════════════════
  {
    type: 'http',
    label: 'HTTP 请求',
    icon: '🌐',
    color: '#58a6ff',
    category: 'data',
    description: '发送 HTTP 请求获取数据',
    inputs: [
      { id: 'headers', label: 'headers', type: 'object' },
      { id: 'body', label: 'body', type: 'object' },
    ],
    outputs: [{ id: 'result', label: 'result', type: 'object' }],
    defaultConfig: { url: '', method: 'GET' },
  },
  {
    type: 'file',
    label: '文件读取',
    icon: '📄',
    color: '#9CA3AF',
    category: 'data',
    description: '读取本地文件内容',
    inputs: [],
    outputs: [
      { id: 'content', label: 'content', type: 'string' },
      { id: 'bytes', label: 'bytes', type: 'file' },
    ],
    defaultConfig: { path: '', encoding: 'utf-8' },
  },
  {
    type: 'clipboard',
    label: '剪贴板',
    icon: '📋',
    color: '#FBBF24',
    category: 'data',
    description: '读取系统剪贴板内容',
    inputs: [],
    outputs: [{ id: 'text', label: 'text', type: 'string' }],
    defaultConfig: { format: 'text' },
  },
  {
    type: 'json_parse',
    label: 'JSON 解析',
    icon: '🔀',
    color: '#bc8cff',
    category: 'data',
    description: '用 JSONPath 提取字段',
    inputs: [{ id: 'input', label: 'input', type: 'object', required: true }],
    outputs: [{ id: 'result', label: 'result', type: 'object' }],
    defaultConfig: { expression: '$', target_field: '' },
  },
  {
    type: 'regex',
    label: '正则处理',
    icon: '🔤',
    color: '#C084FC',
    category: 'data',
    description: '使用正则表达式提取/替换文本',
    inputs: [{ id: 'input', label: 'input', type: 'string', required: true }],
    outputs: [
      { id: 'matches', label: 'matches', type: 'array' },
      { id: 'result', label: 'result', type: 'string' },
    ],
    defaultConfig: { pattern: '', action: 'extract', replacement: '', flags: '' },
  },
  {
    type: 'array',
    label: '数组操作',
    icon: '🔢',
    color: '#f97316',
    category: 'data',
    description: '数组过滤、映射、排序、去重',
    inputs: [{ id: 'input', label: 'input', type: 'array', required: true }],
    outputs: [{ id: 'result', label: 'result', type: 'array' }],
    defaultConfig: { action: 'filter', expression: '', limit: 100 },
  },
  {
    type: 'convert',
    label: '类型转换',
    icon: '🔄',
    color: '#fbbf24',
    category: 'data',
    description: '字符串↔数字↔布尔↔JSON 等类型互转',
    inputs: [{ id: 'input', label: 'input', type: 'any', required: true }],
    outputs: [{ id: 'result', label: 'result', type: 'any' }],
    defaultConfig: { from: 'auto', to: 'string', encoding: 'utf-8' },
  },
  {
    type: 'text_template',
    label: '文本拼接',
    icon: '📝',
    color: '#a371f7',
    category: 'data',
    description: '模板替换拼接文本',
    inputs: [{ id: 'data', label: 'data', type: 'object' }],
    outputs: [{ id: 'result', label: 'result', type: 'string' }],
    defaultConfig: { template: '', output_key: '' },
  },
  {
    type: 'data',
    label: '变量操作',
    icon: '📦',
    color: '#06b6d4',
    category: 'data',
    description: '变量设置/读取/合并',
    inputs: [{ id: 'value', label: 'value', type: 'any' }],
    outputs: [{ id: 'result', label: 'result', type: 'any' }],
    defaultConfig: { action: 'set', key: '', value: '' },
  },

  // ═══════════════════════════════════════════
  // 🔀 逻辑判断
  // ═══════════════════════════════════════════
  {
    type: 'loop',
    label: '循环',
    icon: '🔁',
    color: '#f97316',
    category: 'logic',
    description: '遍历数组，对每个元素执行子步骤',
    inputs: [{ id: 'items', label: 'items', type: 'array', required: true }],
    outputs: [
      { id: 'results', label: 'results', type: 'array' },
      { id: 'count', label: 'count', type: 'number' },
    ],
    defaultConfig: { items: '', body: [], max_iterations: 1000, on_error: 'fail' },
  },
  {
    type: 'while',
    label: 'While 循环',
    icon: '🔄',
    color: '#ea580c',
    category: 'logic',
    description: '条件循环，满足条件时重复执行',
    inputs: [{ id: 'items', label: 'items', type: 'array' }],
    outputs: [
      { id: 'results', label: 'results', type: 'array' },
      { id: 'count', label: 'count', type: 'number' },
      { id: 'stopped_at', label: 'stopped_at', type: 'number' },
    ],
    defaultConfig: { items: '', condition: { check: '__current', op: 'not_empty' }, body: [], max_iterations: 1000 },
  },
  {
    type: 'condition',
    label: '条件判断',
    icon: '🔀',
    color: '#a78bfa',
    category: 'logic',
    description: '根据条件表达式选择执行分支',
    inputs: [
      { id: 'left', label: 'left', type: 'any' },
      { id: 'right', label: 'right', type: 'any' },
    ],
    outputs: [
      { id: 'result', label: 'result', type: 'boolean' },
      { id: 'branch', label: 'branch', type: 'string' },
    ],
    defaultConfig: { op: '==', left: '', right: '' },
  },
  {
    type: 'delay',
    label: '延时',
    icon: '⏱️',
    color: '#9ca3af',
    category: 'logic',
    description: '暂停执行指定时长',
    inputs: [],
    outputs: [{ id: 'duration_ms', label: 'duration_ms', type: 'number' }],
    defaultConfig: { duration_ms: 1000, max_duration_ms: 300000 },
  },
  {
    type: 'sub_workflow',
    label: '子流程',
    icon: '📦',
    color: '#fb923c',
    category: 'logic',
    description: '打包一组节点为可复用的子流程图',
    inputs: [{ id: 'input', label: 'input', type: 'object' }],
    outputs: [{ id: 'result', label: 'result', type: 'object' }],
    defaultConfig: { inline_steps: [], vars_mapping: {}, output_mapping: {}, output_key: 'result' },
  },
  {
    type: 'map',
    label: '映射处理',
    icon: '🗺️',
    color: '#c084fc',
    category: 'logic',
    description: '并行处理数组每个元素',
    inputs: [{ id: 'items', label: 'items', type: 'array' }],
    outputs: [{ id: 'results', label: 'results', type: 'array' }],
    defaultConfig: { items: '', body: [], concurrency: 4 },
  },
  {
    type: 'parallel',
    label: '并行执行',
    icon: '⚡',
    color: '#a78bfa',
    category: 'logic',
    description: '同时执行多个子步骤',
    inputs: [{ id: 'input', label: 'input', type: 'object' }],
    outputs: [{ id: 'results', label: 'results', type: 'array' }],
    defaultConfig: { branch_count: 2 },
  },
  {
    type: 'approval',
    label: '审批节点',
    icon: '✅',
    color: '#f59e0b',
    category: 'logic',
    description: '等待人工审批',
    inputs: [{ id: 'trigger', label: '触发', type: 'any' }],
    outputs: [
      { id: 'approved', label: '通过', type: 'object' },
      { id: 'rejected', label: '拒绝', type: 'object' },
    ],
    defaultConfig: { message: '请审批此操作', timeout: 300 },
  },

  // ═══════════════════════════════════════════
  // 📤 输出
  // ═══════════════════════════════════════════
  {
    type: 'file_save',
    label: '保存文件',
    icon: '💾',
    color: '#8b949e',
    category: 'output',
    description: '写入文件到本地',
    inputs: [{ id: 'content', label: 'content', type: 'object', required: true }],
    outputs: [{ id: 'path', label: 'path', type: 'string' }],
    defaultConfig: { path: '', format: 'json', encoding: 'utf-8' },
  },
  {
    type: 'print',
    label: '控制台打印',
    icon: '🖨️',
    color: '#6EE7B7',
    category: 'output',
    description: '将数据输出到控制台日志',
    inputs: [{ id: 'data', label: 'data', type: 'any', required: true }],
    outputs: [{ id: 'pass', label: 'pass', type: 'any' }],
    defaultConfig: { prefix: '', colorize: true },
  },
  {
    type: 'notify',
    label: '通知',
    icon: '🔔',
    color: '#fbbf24',
    category: 'output',
    description: '发送桌面通知',
    inputs: [{ id: 'message', label: '消息', type: 'string' }],
    outputs: [],
    defaultConfig: { title: '', message: '', level: 'info' },
  },

  // ═══════════════════════════════════════════
  // 📦 其它
  // ═══════════════════════════════════════════
  {
    type: 'script',
    label: '脚本执行',
    icon: '💻',
    color: '#ec4899',
    category: 'other',
    description: '运行自定义脚本',
    inputs: [{ id: 'input', label: '输入', type: 'any' }],
    outputs: [{ id: 'result', label: '结果', type: 'any' }],
    defaultConfig: { language: 'python', code: '', timeout: 30 },
  },
  {
    type: 'window',
    label: '窗口操作',
    icon: '🪟',
    color: '#06b6d4',
    category: 'other',
    description: '操作桌面窗口',
    inputs: [],
    outputs: [{ id: 'data', label: '结果', type: 'object' }],
    defaultConfig: { action: 'list', title: '' },
  },
  {
    type: 'mouse_keyboard',
    label: '键鼠操作',
    icon: '🖱️',
    color: '#84cc16',
    category: 'other',
    description: '模拟键盘鼠标输入',
    inputs: [],
    outputs: [{ id: 'data', label: '结果', type: 'object' }],
    defaultConfig: { action: 'type', text: '', key: '' },
  },
  {
    type: 'ocr',
    label: 'OCR 识别',
    icon: '👁️',
    color: '#8b5cf6',
    category: 'other',
    description: '图片文字识别',
    inputs: [{ id: 'image', label: '图片', type: 'file' }],
    outputs: [{ id: 'text', label: '文字', type: 'string' }],
    defaultConfig: { path: '', lang: 'chi_sim+eng' },
  },
]

/** 通过类型获取节点定义 */
export function getNodeDef(type: string): NodeDefinition | undefined {
  return NODE_REGISTRY.find(d => d.type === type)
}

/** 针脚类型 → 颜色 */
export function pinColor(type: string): string {
  return (PIN_COLORS as Record<string, string>)[type] || '#8b949e'
}
