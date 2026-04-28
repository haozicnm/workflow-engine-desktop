// ─── 工作流核心类型 ───

export interface WorkflowStep {
  id: string
  name: string
  type: string
  config: Record<string, unknown>
  next?: string
  timeout?: number
  retry?: { max: number; delay_ms: number }
}

export interface WorkflowData {
  name: string
  description?: string
  version?: string
  variables?: Record<string, unknown>
  steps: WorkflowStep[]
}

// ─── 步骤类型定义 ───

export interface StepTypeInfo {
  type: string
  label: string
  icon: string
  color: string
}

export const STEP_TYPES: StepTypeInfo[] = [
  { type: 'http', label: 'HTTP 请求', icon: '🌐', color: '#3b82f6' },
  { type: 'data', label: '数据处理', icon: '📊', color: '#10b981' },
  { type: 'script', label: '脚本', icon: '📝', color: '#8b5cf6' },
  { type: 'condition', label: '条件', icon: '❓', color: '#f59e0b' },
  { type: 'loop', label: '循环', icon: '🔄', color: '#06b6d4' },
  { type: 'while', label: 'While循环', icon: '🔁', color: '#0891b2' },
  { type: 'browser', label: '浏览器', icon: '🌍', color: '#ec4899' },
  { type: 'web_scrape', label: '网页抓取', icon: '🕷️', color: '#14b8a6' },
  { type: 'notify', label: '通知', icon: '🔔', color: '#ef4444' },
  { type: 'approval', label: '审批', icon: '✅', color: '#84cc16' },
  { type: 'excel', label: 'Excel', icon: '📗', color: '#22c55e' },
  { type: 'word', label: 'Word', icon: '📘', color: '#3b82f6' },
  { type: 'map', label: '数据映射', icon: '🔀', color: '#a855f7' },
  { type: 'parallel', label: '并行', icon: '⚡', color: '#f97316' },
  // v1.2 新节点
  { type: 'mouse_keyboard', label: '鼠标/键盘', icon: '🖱️', color: '#6366f1' },
  { type: 'window', label: '窗口管理', icon: '🪟', color: '#0ea5e9' },
  { type: 'sub_workflow', label: '子流程', icon: '📦', color: '#d946ef' },
  { type: 'ocr', label: 'OCR识别', icon: '🔍', color: '#f43f5e' },
  { type: 'recording', label: '操作录制', icon: '🎬', color: '#8b5cf6' },
]

export const STEP_COLORS: Record<string, string> = Object.fromEntries(
  STEP_TYPES.map(s => [s.type, s.color])
)
export const STEP_LABELS: Record<string, string> = Object.fromEntries(
  STEP_TYPES.map(s => [s.type, s.label])
)
export const STEP_ICONS: Record<string, string> = Object.fromEntries(
  STEP_TYPES.map(s => [s.type, s.icon])
)

// ─── 执行状态 ───

export interface StepStatus {
  status: 'running' | 'completed' | 'failed'
  output?: unknown
  error?: string
  startedAt?: number
  duration?: number
}

// ─── Tauri invoke 返回类型 ───

export interface WorkflowListItem {
  id: string
  name: string
  description: string
  enabled: boolean
  created_at: string
  updated_at: string
}

export interface WorkflowFull extends WorkflowListItem {
  yaml: string | null
}

// ─── 工具函数 ───

export function generateId(): string {
  return 'step_' + Math.random().toString(36).substring(2, 8)
}

export function defaultStep(type: string): WorkflowStep {
  const configs: Record<string, Record<string, unknown>> = {
    http: { action: 'GET', url: 'https://httpbin.org/get', headers: {} },
    data: { action: 'set', key: 'result', value: '' },
    script: { script: '1 + 1' },
    condition: { left: '', op: '==', right: '', true_next: '', false_next: '' },
    loop: { items: [], body: [] },
    while: { items: [], condition: { op: 'not_empty' }, body: [], max_iterations: 10000 },
    map: { source: '', template: {} },
    parallel: { branches: [] },
    browser: { action: 'navigate', params: { url: 'https://example.com' } },
    notify: { notify_type: 'system', title: '通知标题', body: '通知内容' },
    approval: { message: '请审批此操作', timeout: 300 },
    excel: { action: 'read', path: './input.xlsx' },
    word: { action: 'read', path: './input.docx' },
    web_scrape: {
      url: 'https://example.com',
      wait_for: 'body',
      extract: [{ selector: '.item', fields: { title: '.title', link: 'a[href]' } }],
      pagination: { next: '.next-page', max_pages: 3 },
      scroll: false,
      delay_ms: 1000,
    },
    mouse_keyboard: { action: 'click', x: 0, y: 0, button: 'left' },
    window: { action: 'find', title: '' },
    sub_workflow: { inline_steps: [] },
    ocr: { action: 'read' },
    recording: { action: 'start', headless: false },
  }
  return {
    id: generateId(),
    name: `新${STEP_LABELS[type] || type}`,
    type,
    config: configs[type] || {},
  }
}

export function createDefaultYaml(): string {
  return `name: 新工作流
description: ""
version: "1.0"
variables: {}
steps:
  - id: step_001
    name: HTTP 请求示例
    type: http
    config:
      action: GET
      url: https://httpbin.org/get
      headers: {}
`
}
