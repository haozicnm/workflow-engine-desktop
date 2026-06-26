// ─── Workflow Engine v5 数据模型 ───
// 纯类型/接口定义

// ─── 容器类型 ───

export type ContainerType = 'browser' | 'excel' | 'word' | 'logic' | 'http' | 'delay' | 'notify' | 'script' | 'cursor' | 'loop' | 'approval' | 'shell' | 'file' | 'file_read' | 'file_write' | 'condition' | 'mcp_script' | 'mcp_shell' | 'mcp_excel_read' | 'mcp_excel_write' | 'mcp_excel_create' | 'mcp_excel_filter' | 'mcp_excel_sort' | 'mcp_excel_append' | 'mcp_excel_csv' | 'mcp_word_read' | 'mcp_word_write' | 'mcp_word_create' | 'mcp_word_replace' | 'mcp_word_merge' | 'mcp_web_scrape'
  | 'web_scrape' | 'data_set' | 'data_get' | 'file_list' | 'file_exists' | 'file_checksum'
  | 'clipboard_read' | 'clipboard_write' | 'regex' | 'parallel' | 'map' | 'sub_workflow'
  | 'mouse_keyboard' | 'window' | 'ocr' | 'print' | 'while'

export interface ContainerDef {
  type: ContainerType
  label: string
  icon: string
  color: string
  description: string
  params: ActionParam[]
  paramDefs?: ParamDef[]   // 新 schema-driven 参数定义，优先级高于 params
  isContainer?: boolean  // true = 有 actions 列表的容器，false/undefined = 简单步骤
  outputHint?: string    // 输出格式提示，显示在变量选择器中
  category?: string      // 分组: 'core' | 'data' | 'file' | 'flow' | 'browser' | 'office' | 'system' | 'mcp'
  dangerous?: boolean    // true = 高危节点，UI 显示安全警告
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
  hint?: string
}

// ─── 新 Schema-driven 参数定义 ───

export interface ParamDef {
  name: string
  field_type: 'string' | 'number' | 'boolean' | 'select' | 'json' | 'code' | 'file_path' | 'text'
  required: boolean
  default?: any
  desc?: string
  options?: string[]
  group?: 'basic' | 'advanced'
  lang?: string
  /** 条件显示规则（n8n displayOptions 风格） */
  display_options?: DisplayOptions
}

// ─── DisplayOptions: 声明式条件显示（参考 n8n） ─────────────────

/** 条件运算符 */
export type ConditionOp =
  | { op: 'eq'; value: any }
  | { op: 'not'; value: any }
  | { op: 'gte'; value: number }
  | { op: 'lte'; value: number }
  | { op: 'gt'; value: number }
  | { op: 'lt'; value: number }
  | { op: 'between'; value: { from: number; to: number } }
  | { op: 'startsWith'; value: string }
  | { op: 'endsWith'; value: string }
  | { op: 'includes'; value: string }
  | { op: 'regex'; value: string }
  | { op: 'exists' }

/** 单个条件值：简单值匹配 或 高级条件运算 */
export type ConditionValue = any | { _cnd: ConditionOp }

/** 参数级条件显示规则 */
export interface DisplayOptions {
  /** show 条件：所有条件必须同时满足（AND 逻辑） */
  show?: Record<string, ConditionValue[]>
  /** hide 条件：任一条件满足即隐藏（OR 逻辑） */
  hide?: Record<string, ConditionValue[]>
}

// ─── 逻辑条件 ───

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
  conditionGroup?: LogicConditionGroup
  // 线性链兼容字段（后端 Step.next）
  next?: string
  // 重试配置
  retry?: { max_retries?: number; delay_ms?: number }
  // 步骤超时（毫秒）
  timeout?: number
}

// ─── 边的定义（v8.2 Canvas 图编辑器） ───
export interface Edge {
  id?: string
  from: string
  fromPort?: string
  to: string
  toPort?: string
}

export interface Workflow {
  id?: string
  name: string
  description?: string
  locked?: boolean
  steps: Step[]
  edges?: Edge[]
  // 工作流级变量
  variables?: Record<string, unknown>
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

// ─── 位置类型（Canvas 编辑用） ───

export interface NodePosition {
  x: number
  y: number
}

// ─── 工具函数 ───

export function uid(prefix = ''): string {
  const id = crypto.randomUUID().replace(/-/g, '').slice(0, 12)
  return prefix ? prefix + '_' + id : id
}

/**
 * Generate the next sequential step ID by finding the highest existing step number.
 * Parses existing step IDs matching `step_(\d+)` and returns `step_{max+1}`.
 */
export function nextStepId(steps: Step[]): string {
  let maxNum = 0
  const regex = /^step_(\d+)$/
  for (const step of steps) {
    const m = step.id.match(regex)
    if (m) {
      const n = parseInt(m[1], 10)
      if (n > maxNum) maxNum = n
    }
  }
  return `step_${maxNum + 1}`
}

/**
 * Generate the next sequential action ID for a given step.
 * Extracts the step number from stepId (e.g. `step_3` → `3`) and
 * counts existing actions to produce `action_{stepNum}_{n+1}`.
 */
export function nextActionId(stepId: string, existingActions: Action[]): string {
  const m = stepId.match(/^step_(\d+)$/)
  const stepNum = m ? parseInt(m[1], 10) : 0
  let maxIdx = 0
  const regex = new RegExp(`^action_${stepNum}_(\\d+)$`)
  for (const act of existingActions) {
    const am = act.id.match(regex)
    if (am) {
      const n = parseInt(am[1], 10)
      if (n > maxIdx) maxIdx = n
    }
  }
  return `action_${stepNum}_${maxIdx + 1}`
}

export function newWorkflow(): Workflow {
  return {
    name: '',
    description: '',
    steps: [],
  }
}

// ─── 序列化 ───

export function serializeWorkflow(wf: Workflow): string {
  return JSON.stringify(wf, null, 2)
}

export function deserializeWorkflow(json: string): Workflow {
  let parsed: unknown
  try {
    parsed = JSON.parse(json)
  } catch (e) {
    throw new Error(`Invalid workflow JSON: ${e instanceof Error ? e.message : String(e)}`)
  }
  if (!parsed || typeof parsed !== 'object') {
    throw new Error('Invalid workflow: expected an object')
  }
  const wf = parsed as Record<string, unknown>
  if (typeof wf.name !== 'string') {
    throw new Error('Invalid workflow: missing "name" field')
  }
  if (!Array.isArray(wf.steps)) {
    throw new Error('Invalid workflow: "steps" must be an array')
  }
  // 确保每个步骤都有 actions 数组
  // 归一化: JSON 中 step 名可能是 "name"（旧格式）或 "label"（新格式）
  for (const step of wf.steps) {
    if (step && typeof step === 'object') {
      const s = step as Record<string, unknown>
      if (!Array.isArray(s.actions)) s.actions = []
      // name → label 归一化
      if (s.name !== undefined && s.label === undefined) {
        s.label = s.name
        delete s.name
      }
      // snake_case → camelCase 归一化（后端序列化输出 snake_case）
      if (s.on_error !== undefined && s.onError === undefined) {
        s.onError = s.on_error
        delete s.on_error
      }
      if (s.condition_group !== undefined && s.conditionGroup === undefined) {
        s.conditionGroup = s.condition_group
        delete s.condition_group
      }
      if (s.run_condition !== undefined && s.runCondition === undefined) {
        s.runCondition = s.run_condition
        delete s.run_condition
      }
      if (s.body_steps !== undefined && s.bodySteps === undefined) {
        s.bodySteps = s.body_steps
        delete s.body_steps
      }
    }
  }
  // 归一化 Edge 字段名: 后端序列化为 from_port/to_port，前端期望 fromPort/toPort
  if (Array.isArray(wf.edges)) {
    for (const edge of wf.edges) {
      if (edge && typeof edge === 'object') {
        const e = edge as Record<string, unknown>
        if (e.from_port !== undefined && e.fromPort === undefined) {
          e.fromPort = e.from_port
          delete e.from_port
        }
        if (e.to_port !== undefined && e.toPort === undefined) {
          e.toPort = e.to_port
          delete e.to_port
        }
      }
    }
  }
  return wf as unknown as Workflow
}
