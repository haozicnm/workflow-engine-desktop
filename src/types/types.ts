// ─── Workflow Engine v5 数据模型 ───
// 纯类型/接口定义

// ─── 容器类型 ───

export type ContainerType = 'browser' | 'excel' | 'word' | 'logic' | 'http' | 'delay' | 'notify' | 'script' | 'cursor' | 'loop' | 'approval' | 'shell' | 'file' | 'mcp_script' | 'mcp_shell' | 'mcp_excel_read' | 'mcp_excel_write' | 'mcp_excel_create' | 'mcp_excel_filter' | 'mcp_excel_sort' | 'mcp_excel_append' | 'mcp_excel_csv' | 'mcp_word_read' | 'mcp_word_write' | 'mcp_word_create' | 'mcp_word_replace' | 'mcp_word_merge' | 'mcp_web_scrape'

export interface ContainerDef {
  type: ContainerType
  label: string
  icon: string
  color: string
  description: string
  params: ActionParam[]
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
}

export interface Workflow {
  id?: string
  name: string
  description?: string
  locked?: boolean
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
    }
  }
  return wf as unknown as Workflow
}
