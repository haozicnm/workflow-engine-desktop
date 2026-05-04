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
export type NodeStatus = 'idle' | 'queued' | 'running' | 'success' | 'error' | 'warning' | 'paused'

export const STATUS_COLORS: Record<NodeStatus, string> = {
  idle: '#484f58',
  queued: '#d29922',
  running: '#58a6ff',
  success: '#3fb950',
  error: '#f85149',
  warning: '#d29922',
  paused: '#d29922',
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
// --- 节点注册表 --- 只保留容器节点 + IF
// ─── 节点注册表 ─── 只保留容器节点 + IF
export const NODE_REGISTRY: NodeDefinition[] = [
  {
    type: 'browser_container',
    label: '🌐 浏览器容器',
    icon: '🌐',
    color: '#79c0ff',
    category: 'browser',
    description: '在一个浏览器窗口内完成导航/等待/输入/点击/提取等操作',
    inputs: [],
    outputs: [],
    defaultConfig: { browser: 'chromium', headless: false, timeout: 30000 },
  },
  {
    type: 'excel_container',
    label: '📊 Excel 容器',
    icon: '📊',
    color: '#58a6ff',
    category: 'excel',
    description: '在一个 Excel 文件内完成读取/写入/筛选/排序等操作',
    inputs: [],
    outputs: [],
    defaultConfig: { file_path: '', sheet: 'Sheet1' },
  },
  {
    type: 'word_container',
    label: '📄 Word 容器',
    icon: '📄',
    color: '#bc8cff',
    category: 'word',
    description: '在一个 Word 文件内完成读取/写入/替换/合并等操作',
    inputs: [],
    outputs: [],
    defaultConfig: { file_path: '' },
  },
  {
    type: 'logic_container',
    label: '🔀 逻辑判断',
    icon: '🔀',
    color: '#d29922',
    category: 'logic',
    description: '判断动作全部通过则原值透传到 true出口，否则到 false出口',
    inputs: [{ id: 'input', label: '输入', type: 'any' }],
    outputs: [
      { id: 'true', label: 'true出口', type: 'any' },
      { id: 'false', label: 'false出口', type: 'any' },
    ],
    defaultConfig: { actions: [] },
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
