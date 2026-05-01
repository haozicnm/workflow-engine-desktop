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
  category: 'source' | 'process' | 'output' | 'ai'
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

// ─── P1 基础节点注册表 ───
export const NODE_REGISTRY: NodeDefinition[] = [
  {
    type: 'http',
    label: 'HTTP 请求',
    icon: '🌐',
    color: '#58a6ff',
    category: 'source',
    description: '发送 HTTP 请求获取数据',
    inputs: [
      { id: 'headers', label: 'headers', type: 'object' },
      { id: 'body', label: 'body', type: 'object' },
    ],
    outputs: [
      { id: 'result', label: 'result', type: 'object' },
    ],
    defaultConfig: { url: '', method: 'GET', timeout: 30 },
  },
  {
    type: 'file',
    label: '文件操作',
    icon: '📄',
    color: '#9CA3AF',
    category: 'source',
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
    category: 'source',
    description: '读取系统剪贴板内容',
    inputs: [],
    outputs: [
      { id: 'text', label: 'text', type: 'string' },
    ],
    defaultConfig: { format: 'text' },
  },
  {
    type: 'json_parse',
    label: 'JSON 解析',
    icon: '🔀',
    color: '#bc8cff',
    category: 'process',
    description: '用 JSONPath 提取字段',
    inputs: [
      { id: 'input', label: 'input', type: 'object', required: true },
    ],
    outputs: [
      { id: 'result', label: 'result', type: 'object' },
    ],
    defaultConfig: { expression: '$', target_field: '' },
  },
  {
    type: 'regex',
    label: '正则处理',
    icon: '🔤',
    color: '#C084FC',
    category: 'process',
    description: '使用正则表达式提取/替换文本',
    inputs: [
      { id: 'input', label: 'input', type: 'string', required: true },
    ],
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
    category: 'process',
    description: '数组过滤、映射、排序、去重',
    inputs: [
      { id: 'input', label: 'input', type: 'array', required: true },
    ],
    outputs: [
      { id: 'result', label: 'result', type: 'array' },
    ],
    defaultConfig: { action: 'filter', expression: '', limit: 100 },
  },
  {
    type: 'convert',
    label: '类型转换',
    icon: '🔄',
    color: '#fbbf24',
    category: 'process',
    description: '字符串↔数字↔布尔↔JSON 等类型互转',
    inputs: [
      { id: 'input', label: 'input', type: 'any', required: true },
    ],
    outputs: [
      { id: 'result', label: 'result', type: 'any' },
    ],
    defaultConfig: { from: 'auto', to: 'string', encoding: 'utf-8' },
  },
  {
    type: 'text_template',
    label: '文本拼接',
    icon: '📝',
    color: '#a371f7',
    category: 'process',
    description: '模板替换拼接文本',
    inputs: [
      { id: 'data', label: 'data', type: 'object' },
    ],
    outputs: [
      { id: 'result', label: 'result', type: 'string' },
    ],
    defaultConfig: { template: '', output_key: '' },
  },
  {
    type: 'file_save',
    label: '保存文件',
    icon: '💾',
    color: '#8b949e',
    category: 'output',
    description: '写入文件到本地',
    inputs: [
      { id: 'content', label: 'content', type: 'object', required: true },
    ],
    outputs: [
      { id: 'path', label: 'path', type: 'string' },
    ],
    defaultConfig: { path: '', format: 'json', encoding: 'utf-8' },
  },
  {
    type: 'print',
    label: '控制台打印',
    icon: '🖨️',
    color: '#6EE7B7',
    category: 'output',
    description: '将数据输出到控制台日志',
    inputs: [
      { id: 'data', label: 'data', type: 'any', required: true },
    ],
    outputs: [
      { id: 'pass', label: 'pass', type: 'any' },
    ],
    defaultConfig: { prefix: '', colorize: true },
  },
  // ─── P3 AI 节点 ───
  {
    type: 'ai',
    label: 'AI 调用',
    icon: '🤖',
    color: '#7c3aed',
    category: 'ai',
    description: '调用 LLM 执行翻译/摘要/分类/情感分析/实体提取',
    inputs: [
      { id: 'text', label: 'text', type: 'string' },
    ],
    outputs: [
      { id: 'result', label: 'result', type: 'object' },
    ],
    defaultConfig: { action: 'call_llm', model: 'gpt-3.5-turbo', prompt: '', temperature: 0.7, max_tokens: 1024 },
  },
  {
    type: 'ai_translate',
    label: 'AI 翻译',
    icon: '🌍',
    color: '#10b981',
    category: 'ai',
    description: '使用 AI 翻译文本',
    inputs: [
      { id: 'text', label: 'text', type: 'string', required: true },
    ],
    outputs: [
      { id: 'translated', label: 'translated', type: 'string' },
    ],
    defaultConfig: { action: 'translate', source_lang: 'auto', target_lang: 'en', model: 'gpt-3.5-turbo' },
  },
  {
    type: 'ai_summarize',
    label: 'AI 摘要',
    icon: '📝',
    color: '#f59e0b',
    category: 'ai',
    description: '使用 AI 生成文本摘要',
    inputs: [
      { id: 'text', label: 'text', type: 'string', required: true },
    ],
    outputs: [
      { id: 'summary', label: 'summary', type: 'string' },
    ],
    defaultConfig: { action: 'summarize', max_length: 200, model: 'gpt-3.5-turbo' },
  },
  {
    type: 'ai_classify',
    label: 'AI 分类',
    icon: '🏷️',
    color: '#ef4444',
    category: 'ai',
    description: '使用 AI 对文本进行分类',
    inputs: [
      { id: 'text', label: 'text', type: 'string', required: true },
      { id: 'labels', label: 'labels', type: 'array' },
    ],
    outputs: [
      { id: 'label', label: 'label', type: 'string' },
      { id: 'confidence', label: 'confidence', type: 'number' },
    ],
    defaultConfig: { action: 'classify', labels: [] },
  },
  {
    type: 'ai_sentiment',
    label: 'AI 情感分析',
    icon: '💬',
    color: '#ec4899',
    category: 'ai',
    description: '分析文本的情感倾向（正面/负面/中性）',
    inputs: [
      { id: 'text', label: 'text', type: 'string', required: true },
    ],
    outputs: [
      { id: 'sentiment', label: 'sentiment', type: 'string' },
      { id: 'score', label: 'score', type: 'number' },
    ],
    defaultConfig: { action: 'sentiment' },
  },
  {
    type: 'ai_entities',
    label: 'AI 实体提取',
    icon: '🔍',
    color: '#06b6d4',
    category: 'ai',
    description: '使用 AI 提取文本中的命名实体',
    inputs: [
      { id: 'text', label: 'text', type: 'string', required: true },
      { id: 'types', label: 'types', type: 'array' },
    ],
    outputs: [
      { id: 'entities', label: 'entities', type: 'array' },
    ],
    defaultConfig: { action: 'extract_entities', entity_types: [] },
  },
  // ─── P4: 子流程图（复合节点） ───
  {
    type: 'sub_workflow',
    label: '子流程',
    icon: '📦',
    color: '#fb923c',
    category: 'process',
    description: '打包一组节点为可复用的子流程图',
    inputs: [
      { id: 'input', label: 'input', type: 'object' },
    ],
    outputs: [
      { id: 'result', label: 'result', type: 'object' },
    ],
    defaultConfig: {
      inline_steps: [],
      vars_mapping: {},
      output_mapping: {},
      output_key: 'result',
    },
  },
  // ─── M2 控制流节点 ───
  {
    type: 'loop',
    label: '循环',
    icon: '🔁',
    color: '#f97316',
    category: 'process',
    description: '遍历数组，对每个元素执行子步骤',
    inputs: [
      { id: 'items', label: 'items', type: 'array', required: true },
    ],
    outputs: [
      { id: 'results', label: 'results', type: 'array' },
      { id: 'count', label: 'count', type: 'number' },
    ],
    defaultConfig: {
      items: '',
      body: [],
      max_iterations: 1000,
      on_error: 'fail',
    },
  },
  {
    type: 'while',
    label: 'While 循环',
    icon: '🔄',
    color: '#ea580c',
    category: 'process',
    description: '条件循环，满足条件时重复执行子步骤',
    inputs: [
      { id: 'items', label: 'items', type: 'array' },
    ],
    outputs: [
      { id: 'results', label: 'results', type: 'array' },
      { id: 'count', label: 'count', type: 'number' },
      { id: 'stopped_at', label: 'stopped_at', type: 'number' },
    ],
    defaultConfig: {
      items: '',
      condition: { check: '__current', op: 'not_empty' },
      body: [],
      max_iterations: 1000,
    },
  },
  {
    type: 'condition',
    label: '条件判断',
    icon: '🔀',
    color: '#a78bfa',
    category: 'process',
    description: '根据条件表达式选择执行分支',
    inputs: [
      { id: 'left', label: 'left', type: 'any' },
      { id: 'right', label: 'right', type: 'any' },
    ],
    outputs: [
      { id: 'result', label: 'result', type: 'boolean' },
      { id: 'branch', label: 'branch', type: 'string' },
    ],
    defaultConfig: {
      left: '',
      op: '>',
      right: '0',
    },
  },
  {
    type: 'data',
    label: '变量操作',
    icon: '📊',
    color: '#06b6d4',
    category: 'process',
    description: '变量设置/读取/合并/默认值/长度计算',
    inputs: [
      { id: 'value', label: 'value', type: 'any' },
    ],
    outputs: [
      { id: 'result', label: 'result', type: 'any' },
    ],
    defaultConfig: {
      action: 'set',
      key: '',
      value: '',
    },
  },
  {
    type: 'delay',
    label: '延时',
    icon: '⏱️',
    color: '#9ca3af',
    category: 'process',
    description: '暂停执行指定时长（毫秒）',
    inputs: [],
    outputs: [
      { id: 'duration_ms', label: 'duration_ms', type: 'number' },
    ],
    defaultConfig: {
      duration_ms: 1000,
      max_duration_ms: 300000,
    },
  },
]

// ─── 辅助函数 ───

export function getNodeDef(type: string): NodeDefinition | undefined {
  return NODE_REGISTRY.find(d => d.type === type)
}

export function pinColor(type: string): string {
  return (PIN_COLORS as Record<string, string>)[type] || '#8b949e'
}
