// ─── Workflow Engine v5 数据模型 ───
// 步骤列表架构，替代 LiteGraph 图模型

// ─── 容器类型 ───

export type ContainerType = 'browser' | 'excel' | 'word' | 'logic' | 'http' | 'delay' | 'notify' | 'script' | 'clipboard' | 'cursor' | 'loop' | 'approval'

export interface ContainerDef {
  type: ContainerType
  label: string
  icon: string
  color: string
  description: string
  params: ActionParam[]
  isContainer?: boolean  // true = 有 actions 列表的容器，false/undefined = 简单步骤
}

export const CONTAINER_DEFS: ContainerDef[] = [
  { type: 'browser', label: '浏览器', icon: '🌐', color: '#79c0ff', isContainer: true, description: '网页操作：导航、点击、输入、提取', params: [
    { key: 'browser', label: '浏览器', type: 'select', options: [
      { label: 'Chromium', value: 'chromium' }, { label: 'Firefox', value: 'firefox' }, { label: 'WebKit', value: 'webkit' },
    ], default: 'chromium' },
    { key: 'headless', label: '无头模式', type: 'checkbox', default: false },
    { key: 'timeout', label: '超时(ms)', type: 'number', default: 30000 },
  ]},
  { type: 'excel', label: 'Excel', icon: '📊', color: '#58a6ff', isContainer: true, description: 'Excel 操作：读写单元格、筛选、排序', params: [
    { key: 'file_path', label: '文件路径', type: 'text', placeholder: './data.xlsx' },
    { key: 'sheet', label: '工作表', type: 'text', default: 'Sheet1' },
  ]},
  { type: 'word', label: 'Word', icon: '📄', color: '#bc8cff', isContainer: true, description: 'Word 操作：读写、替换、合并', params: [
    { key: 'file_path', label: '文件路径', type: 'text', placeholder: './document.docx' },
  ]},
  { type: 'logic', label: '条件判断', icon: '🔀', color: '#d29922', isContainer: true, description: '条件分支：满足/不满足走不同路径', params: [
    { key: 'condition', label: '条件表达式', type: 'text', placeholder: '{{step1.output}} == "异常"' },
  ]},
  // ─── 新增简单步骤类型（T11） ───
  { type: 'http', label: 'HTTP 请求', icon: '🌍', color: '#a5d6ff', description: '发送 HTTP 请求：GET/POST/PUT/DELETE', params: [
    { key: 'method', label: '方法', type: 'select', options: [
      { label: 'GET', value: 'GET' }, { label: 'POST', value: 'POST' }, { label: 'PUT', value: 'PUT' }, { label: 'DELETE', value: 'DELETE' },
    ], default: 'GET' },
    { key: 'url', label: 'URL', type: 'text', placeholder: 'https://api.example.com/data' },
    { key: 'headers', label: '请求头 (JSON)', type: 'textarea', placeholder: '{"Content-Type": "application/json"}' },
    { key: 'body', label: '请求体', type: 'textarea', placeholder: '{"key": "value"}' },
  ]},
  { type: 'delay', label: '延迟等待', icon: '⏳', color: '#d2a8ff', description: '等待指定时间后继续', params: [
    { key: 'duration_ms', label: '毫秒', type: 'number', default: 1000 },
    { key: 'max_duration_ms', label: '最大毫秒(随机)', type: 'number', default: 5000 },
  ]},
  { type: 'notify', label: '通知', icon: '🔔', color: '#f0883e', description: '发送通知：系统通知/Webhook', params: [
    { key: 'notify_type', label: '渠道', type: 'select', options: [
      { label: '系统通知', value: 'system' }, { label: 'Webhook', value: 'webhook' },
    ], default: 'system' },
    { key: 'title', label: '标题', type: 'text' },
    { key: 'body', label: '内容', type: 'textarea' },
    { key: 'url', label: 'Webhook URL', type: 'text', placeholder: 'https://hooks.example.com/...' },
  ]},
  { type: 'script', label: '脚本', icon: '📜', color: '#7ee787', description: '执行自定义脚本（Rhai）', params: [
    { key: 'script', label: '代码', type: 'textarea', placeholder: '// 你的 Rhai 脚本代码' },
  ]},
  { type: 'clipboard', label: '剪贴板', icon: '📋', color: '#8b949e', description: '读写系统剪贴板', params: [
    { key: 'action', label: '操作', type: 'select', options: [
      { label: '读取', value: 'read' }, { label: '写入', value: 'write' },
    ], default: 'read' },
    { key: 'text', label: '写入内容', type: 'textarea' },
  ]},
  { type: 'cursor', label: '游标迭代', icon: '🔁', color: '#e85d75', isContainer: true, description: '逐条迭代：每次运行处理一行/一项，游标跨次保存', params: [
    { key: 'items', label: '数据源', type: 'text', placeholder: '{{read_excel.data}}' },
  ]},
  { type: 'loop', label: '批量循环', icon: '🔄', color: '#f0883e', isContainer: true, description: '一次性遍历全部数据，适合小数据内存变换', params: [
    { key: 'items', label: '数据源', type: 'text', placeholder: '{{step1.data}} 或 [[1,2,3]]' },
  ]},
  { type: 'approval', label: '人工审批', icon: '✋', color: '#f778ba', description: '暂停流程等待人工审核：查看数据后同意或拒绝', params: [
    { key: 'title', label: '审批标题', type: 'text', placeholder: '请确认订单信息' },
    { key: 'message', label: '审批内容', type: 'textarea', placeholder: '订单号：{{step1.订单号}}，金额：{{step1.金额}}' },
    { key: 'timeout', label: '超时(秒)', type: 'number', default: 300 },
    { key: 'timeout_action', label: '超时策略', type: 'select', options: [
      { label: '自动拒绝', value: 'reject' },
      { label: '自动通过', value: 'approve' },
      { label: '标记失败', value: 'fail' },
    ], default: 'reject' },
  ]},
]

export function getContainerDef(type: ContainerType): ContainerDef {
  return CONTAINER_DEFS.find(d => d.type === type) || CONTAINER_DEFS[0]
}

// ─── 动作定义 ───

export interface ActionDef {
  type: string
  label: string
  icon: string
  params: ActionParam[]
}

export interface ActionParam {
  key: string
  label: string
  type: 'text' | 'number' | 'select' | 'checkbox' | 'textarea'
  placeholder?: string
  default?: unknown
  options?: { label: string; value: string }[]
}

// 浏览器动作
export const BROWSER_ACTIONS: ActionDef[] = [
  { type: 'navigate', label: '打开页面', icon: '🔗', params: [
    { key: 'url', label: '网址', type: 'text', placeholder: 'https://example.com' },
  ]},
  { type: 'click', label: '点击元素', icon: '👆', params: [
    { key: 'selector', label: '选择器', type: 'text', placeholder: '#btn 或 .class' },
  ]},
  { type: 'input', label: '输入文本', icon: '⌨️', params: [
    { key: 'selector', label: '选择器', type: 'text' },
    { key: 'value', label: '内容', type: 'text' },
  ]},
  { type: 'wait', label: '等待', icon: '⏳', params: [
    { key: 'ms', label: '毫秒', type: 'number', default: 1000 },
  ]},
  { type: 'screenshot', label: '截图', icon: '📸', params: [
    { key: 'path', label: '保存路径', type: 'text', placeholder: './screenshot.png' },
  ]},
  { type: 'evaluate', label: '执行 JS', icon: '⚡', params: [
    { key: 'script', label: '脚本', type: 'textarea' },
  ]},
  { type: 'scroll', label: '滚动页面', icon: '📜', params: [
    { key: 'x', label: '横向像素', type: 'number', default: 0 },
    { key: 'y', label: '纵向像素', type: 'number', default: 500 },
  ]},
  { type: 'extract', label: '提取数据', icon: '📋', params: [
    { key: 'selector', label: '选择器', type: 'text', placeholder: 'body', default: 'body' },
    { key: 'mode', label: '模式', type: 'select', options: [
      { label: '文本', value: 'text' }, { label: 'HTML', value: 'html' },
      { label: '属性', value: 'attr' },
    ], default: 'text' },
  ]},
  { type: 'get_title', label: '获取标题', icon: '🏷️', params: [] },
  // ─── v1.1+ 扩展动作 ───
  { type: 'extract_table', label: '提取表格', icon: '📊', params: [
    { key: 'selector', label: '选择器', type: 'text', placeholder: 'table', default: 'table' },
  ]},
  { type: 'extract_links', label: '提取链接', icon: '🔗', params: [
    { key: 'selector', label: '范围选择器', type: 'text', placeholder: 'body', default: 'body' },
  ]},
  { type: 'select', label: '下拉选择', icon: '🔽', params: [
    { key: 'selector', label: '选择器', type: 'text' },
    { key: 'value', label: '值', type: 'text', placeholder: 'optionValue 或 显示文本' },
  ]},
  { type: 'check', label: '勾选/取消', icon: '✅', params: [
    { key: 'selector', label: '选择器', type: 'text' },
    { key: 'checked', label: '勾选', type: 'checkbox', default: true },
  ]},
  { type: 'hover', label: '鼠标悬停', icon: '🖱️', params: [
    { key: 'selector', label: '选择器', type: 'text' },
  ]},
  { type: 'cookies', label: 'Cookie 管理', icon: '🍪', params: [
    { key: 'action', label: '操作', type: 'select', options: [
      { label: '获取', value: 'get' }, { label: '设置', value: 'set' }, { label: '清除', value: 'clear' },
    ], default: 'get' },
    { key: 'cookies', label: 'Cookie 数据 (JSON)', type: 'textarea', placeholder: 'set 时填写 [{name,value,domain,...}]' },
  ]},
  { type: 'set_headers', label: '设置请求头', icon: '📡', params: [
    { key: 'headers', label: '请求头 (JSON)', type: 'textarea', placeholder: '{"Authorization": "Bearer xxx"}' },
  ]},
  { type: 'new_page', label: '新建标签页', icon: '➕', params: [
    { key: 'url', label: '初始网址', type: 'text', placeholder: 'https://...' },
  ]},
  { type: 'close_page', label: '关闭标签页', icon: '❌', params: [
    { key: 'index', label: '标签索引', type: 'number' },
  ]},
  { type: 'switch_page', label: '切换标签页', icon: '🔄', params: [
    { key: 'index', label: '目标索引', type: 'number', default: 0 },
  ]},
  { type: 'pages', label: '标签页列表', icon: '📑', params: [] },
  { type: 'back', label: '后退', icon: '⬅️', params: [] },
  { type: 'forward', label: '前进', icon: '➡️', params: [] },
  { type: 'reload', label: '刷新页面', icon: '🔄', params: [] },
  { type: 'current_url', label: '当前网址', icon: '🔗', params: [] },
  { type: 'pdf', label: '生成 PDF', icon: '📄', params: [
    { key: 'path', label: '保存路径', type: 'text', default: 'output.pdf' },
  ]},
  // ─── 智能等待 ───
  { type: 'wait_network_idle', label: '等待网络空闲', icon: '🌐', params: [
    { key: 'timeout', label: '超时(ms)', type: 'number', default: 30000 },
  ]},
  { type: 'wait_load_state', label: '等待加载状态', icon: '⏳', params: [
    { key: 'state', label: '状态', type: 'select', options: [
      { label: 'load（页面加载完成）', value: 'load' },
      { label: 'domcontentloaded（DOM就绪）', value: 'domcontentloaded' },
      { label: 'networkidle（网络空闲）', value: 'networkidle' },
    ], default: 'load' },
    { key: 'timeout', label: '超时(ms)', type: 'number', default: 30000 },
  ]},
  { type: 'wait_url_contains', label: '等待 URL 变更', icon: '🔗', params: [
    { key: 'substring', label: 'URL 包含', type: 'text', placeholder: '/dashboard' },
    { key: 'timeout', label: '超时(ms)', type: 'number', default: 30000 },
  ]},
  // ─── 动作验证 ───
  { type: 'verify', label: '验证健康', icon: '🩺', params: [] },
  // ─── 文件下载 ───
  { type: 'download', label: '下载文件', icon: '📥', params: [
    { key: 'save_dir', label: '保存目录', type: 'text', default: '.' },
    { key: 'click_selector', label: '点击选择器', type: 'text', placeholder: '先点这个再等下载（可选）' },
    { key: 'timeout', label: '超时(ms)', type: 'number', default: 30000 },
  ]},
]

// Excel 动作（与后端 excel_container.rs 对齐）
export const EXCEL_ACTIONS: ActionDef[] = [
  { type: 'read', label: '读取整表', icon: '📖', params: [] },
  { type: 'write', label: '写入数据', icon: '✏️', params: [
    { key: 'value', label: '数据', type: 'textarea', placeholder: '[["姓名","年龄"],["张三",25]] 或 {{变量}}' },
  ]},
  { type: 'create', label: '创建文件', icon: '📄', params: [
    { key: 'headers', label: '表头', type: 'text', placeholder: '姓名,年龄,城市（逗号分隔）' },
  ]},
  { type: 'append', label: '追加行', icon: '➕', params: [
    { key: 'value', label: '数据', type: 'textarea', placeholder: '[["李四",30]] 或 {{变量}}' },
  ]},
  { type: 'filter', label: '筛选', icon: '🔍', params: [
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
  { type: 'sort', label: '排序', icon: '↕️', params: [
    { key: 'column', label: '列', type: 'text', placeholder: 'A' },
    { key: 'order', label: '顺序', type: 'select', options: [
      { label: '升序', value: 'asc' }, { label: '降序', value: 'desc' },
    ], default: 'asc' },
  ]},
  { type: 'formula', label: '公式', icon: '🧮', params: [
    { key: 'cell', label: '单元格', type: 'text', placeholder: 'A1' },
    { key: 'formula', label: '公式', type: 'text', placeholder: 'SUM(A1:A10)' },
  ]},
]

// Word 动作（与后端 word_container.rs 对齐）
export const WORD_ACTIONS: ActionDef[] = [
  { type: 'read', label: '读取内容', icon: '📖', params: [] },
  { type: 'write', label: '写入段落', icon: '✏️', params: [
    { key: 'value', label: '内容', type: 'textarea' },
  ]},
  { type: 'replace', label: '替换文本', icon: '🔄', params: [
    { key: 'old_text', label: '查找', type: 'text' },
    { key: 'new_text', label: '替换为', type: 'text' },
  ]},
  { type: 'create', label: '创建文档', icon: '📄', params: [
    { key: 'title', label: '标题', type: 'text', placeholder: '文档标题' },
  ]},
  { type: 'insert_table', label: '插入表格', icon: '📊', params: [
    { key: 'data', label: '表格数据', type: 'textarea', placeholder: '[["姓名","年龄"],["张三",25]]' },
  ]},
  { type: 'merge', label: '合并文档', icon: '📎', params: [
    { key: 'files', label: '文件列表 (JSON)', type: 'textarea', placeholder: '["a.docx","b.docx"]' },
  ]},
]

// 逻辑动作 —— 操作符定义
export const LOGIC_OPERATORS = [
  { type: 'contains', label: '包含', icon: '🔍', hasRight: true },
  { type: 'not_contains', label: '不包含', icon: '🚫', hasRight: true },
  { type: 'equals', label: '等于', icon: '=', hasRight: true },
  { type: 'not_equals', label: '不等于', icon: '≠', hasRight: true },
  { type: 'greater_than', label: '大于', icon: '>', hasRight: true },
  { type: 'less_than', label: '小于', icon: '<', hasRight: true },
  { type: 'greater_equal', label: '大于等于', icon: '≥', hasRight: true },
  { type: 'less_equal', label: '小于等于', icon: '≤', hasRight: true },
  { type: 'starts_with', label: '开头是', icon: '↗', hasRight: true },
  { type: 'ends_with', label: '结尾是', icon: '↘', hasRight: true },
  { type: 'is_empty', label: '为空', icon: '∅', hasRight: false },
  { type: 'not_empty', label: '不为空', icon: '∃', hasRight: false },
  { type: 'regex', label: '正则匹配', icon: '.*', hasRight: true },
]

export interface LogicCondition {
  id: string
  left: string      // 左侧值（变量引用或常量）
  op: string         // 操作符
  right: string      // 右侧值（仅 hasRight=true 时使用）
}

export interface LogicConditionGroup {
  combinator: 'and' | 'or'
  conditions: LogicCondition[]
}

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

export function getActionDefs(containerType: ContainerType): ActionDef[] {
  switch (containerType) {
    case 'browser': return BROWSER_ACTIONS
    case 'excel': return EXCEL_ACTIONS
    case 'word': return WORD_ACTIONS
    case 'logic': return LOGIC_ACTIONS
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

// ─── 运行时数据结构 ───

export interface Action {
  id: string
  type: string
  label?: string  // 用户可自定义的动作名称
  params: Record<string, unknown>
}

export type ErrorStrategy = 'fail' | 'ignore' | { branch: string }

export interface StepCondition {
  ref: string       // 引用的逻辑步骤 ID
  when: 'true' | 'false' | 'both' | 'merge'  // branch 为该值时执行，both=无条件执行，merge=等待所有分支完成后执行一次
}

export interface Step {
  id: string
  type: ContainerType
  label: string
  expanded: boolean
  actions: Action[]
  config: Record<string, unknown>  // 容器参数
  // 错误处理策略
  onError?: ErrorStrategy
  // 延迟执行（毫秒）
  delay?: number
  // 断点标记
  breakpoint?: boolean
  // 条件执行（引用逻辑步骤的 branch）
  runCondition?: StepCondition
  // logic 容器特有
  condition?: string
  conditionGroup?: import('./workflow').LogicConditionGroup
  thenSteps?: Step[]
  elseSteps?: Step[]
}

export interface Workflow {
  id?: string
  name: string
  description?: string
  steps: Step[]
}

// ─── 执行状态 ───

export type StepStatus = 'idle' | 'running' | 'success' | 'error'
export type ActionStatus = 'idle' | 'running' | 'success' | 'error'

export interface StepRunState {
  status: StepStatus
  error?: string
  duration?: number
  actionStates: Record<string, ActionStatus>
  output?: unknown
}

// ─── 工具函数 ───

export function uid(prefix = ''): string {
  const id = crypto.randomUUID().replace(/-/g, '').slice(0, 12)
  return prefix ? prefix + '_' + id : id
}

export function newAction(type: string, containerType?: ContainerType, existingActions?: Action[]): Action {
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
  return { id: uid('act'), type, label, params: {} }
}

export function newStep(containerType: ContainerType): Step {
  const def = getContainerDef(containerType)
  // 从容器定义中提取默认参数
  const config: Record<string, unknown> = {}
  for (const p of def.params) {
    if (p.default !== undefined) {
      config[p.key] = p.default
    }
  }
  const step: Step = {
    id: uid('step'),
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

/** 步骤是否为容器类型（有 actions 列表） */
export function isContainerType(type: ContainerType): boolean {
  const def = CONTAINER_DEFS.find(d => d.type === type)
  return def?.isContainer === true
}

export function newWorkflow(): Workflow {
  return {
    name: '未命名工作流',
    description: '',
    steps: [],
  }
}

// ─── 序列化 ───

export function serializeWorkflow(wf: Workflow): string {
  return JSON.stringify(wf, null, 2)
}

export function deserializeWorkflow(json: string): Workflow {
  const wf = JSON.parse(json) as Workflow
  // 确保每个步骤都有 actions 数组
  if (wf.steps) {
    for (const step of wf.steps) {
      if (!step.actions) step.actions = []
    }
  }
  return wf
}
