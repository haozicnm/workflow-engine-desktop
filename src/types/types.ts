// ─── Workflow Engine v5 数据模型 ───
// 纯类型/接口定义

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
  outputHint?: string    // 输出格式提示，显示在变量选择器中
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
