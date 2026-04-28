// ─── DAG 节点类型定义：针脚类型、节点定义、图结构 ───

// ─── 针脚类型 ───
export interface PinDefinition {
  id: string
  label: string
  type: PinType
  required?: boolean
}

export type PinType = 'string' | 'number' | 'boolean' | 'object' | 'array' | 'file' | 'any' | 'error'

export const PIN_TYPE_COLORS: Record<PinType, string> = {
  string: '#4A9EFF',
  number: '#4ADE80',
  boolean: '#FBBF24',
  object: '#FB923C',
  array: '#C084FC',
  file: '#9CA3AF',
  any: '#E5E7EB',
  error: '#EF4444',
}

export const PIN_TYPE_LABELS: Record<PinType, string> = {
  string: 'Aa',
  number: '#',
  boolean: '◉',
  object: '{}',
  array: '[]',
  file: '📄',
  any: '*',
  error: '!',
}

// ─── 节点定义（注册表项）───
export interface DAGNodeDefinition {
  type: string
  label: string
  icon: string
  color: string
  category: 'source' | 'process' | 'ai' | 'output'
  inputs: PinDefinition[]
  outputs: PinDefinition[]
  defaultConfig: Record<string, unknown>
}

// ─── 运行时节点 ───
export interface DAGNode<T = Record<string, unknown>> {
  id: string
  type: string
  label: string
  position: { x: number; y: number }
  config: T
  status?: DAGNodeStatus
  output?: unknown
  error?: string
  duration?: number
}

export type DAGNodeStatus = 'idle' | 'queued' | 'running' | 'success' | 'error'

export const STATUS_COLORS: Record<DAGNodeStatus, string> = {
  idle: '#9CA3AF',
  queued: '#FBBF24',
  running: '#3B82F6',
  success: '#4ADE80',
  error: '#EF4444',
}

// ─── 连线 ───
export interface DAGEdge {
  id: string
  source: string
  target: string
  sourceHandle: string
  targetHandle: string
}

// ─── 工作流图 ───
export interface DAGWorkflow {
  name: string
  description?: string
  nodes: DAGNode[]
  edges: DAGEdge[]
  variables?: Record<string, unknown>
}

// ─── 基础节点注册表 ───
export const BASE_NODE_DEFINITIONS: DAGNodeDefinition[] = [
  {
    type: 'http',
    label: 'HTTP 请求',
    icon: '🌐',
    color: '#3B82F6',
    category: 'source',
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
    type: 'json_parse',
    label: 'JSON 解析',
    icon: '🔀',
    color: '#A855F7',
    category: 'process',
    inputs: [
      { id: 'input', label: 'input', type: 'object', required: true },
    ],
    outputs: [
      { id: 'result', label: 'result', type: 'object' },
    ],
    defaultConfig: { expression: '$' },
  },
  {
    type: 'text_template',
    label: '文本拼接',
    icon: '📝',
    color: '#8B5CF6',
    category: 'process',
    inputs: [
      { id: 'data', label: 'data', type: 'object' },
    ],
    outputs: [
      { id: 'result', label: 'result', type: 'string' },
    ],
    defaultConfig: { template: '' },
  },
  {
    type: 'file_save',
    label: '保存文件',
    icon: '💾',
    color: '#9CA3AF',
    category: 'output',
    inputs: [
      { id: 'content', label: 'content', type: 'object', required: true },
    ],
    outputs: [],
    defaultConfig: { path: '', format: 'json' },
  },
]
