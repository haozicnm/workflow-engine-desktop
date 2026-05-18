import type { Workflow, Step } from '../types/types'

/** 确保所有 step 都有 actions 数组（兼容旧格式/损坏数据） */
export function normalizeSteps(steps: Step[]) {
  for (const step of steps) {
    if (!Array.isArray(step.actions)) step.actions = []
    // 清理已废弃的嵌套步骤字段
    delete (step as any).thenSteps
    delete (step as any).elseSteps
  }
}

/** 校验变量引用：检查所有 {{...}} 引用的 stepId.actionId 是否存在 */
export function validateRefs(wf: Workflow): string[] {
  const warnings: string[] = []
  const stepIds = new Set(wf.steps.map(s => s.id))
  const actionIds = new Map<string, Set<string>>()
  for (const step of wf.steps) {
    actionIds.set(step.id, new Set((step.actions || []).map(a => a.id)))
  }

  const refRegex = /\{\{([^}]+)\}\}/g
  function checkRefs(text: string, context: string) {
    for (const m of text.matchAll(refRegex)) {
      const ref = m[1]
      const dotIdx = ref.indexOf('.')
      if (dotIdx === -1) {
        if (!stepIds.has(ref)) {
          warnings.push(`${context}: 引用了不存在的步骤 {{${ref}}}`)
        }
      } else {
        const refStep = ref.slice(0, dotIdx)
        const refAction = ref.slice(dotIdx + 1)
        if (!stepIds.has(refStep)) {
          warnings.push(`${context}: 引用了不存在的步骤 {{${ref}}}`)
        } else if (!actionIds.get(refStep)?.has(refAction)) {
          warnings.push(`${context}: 引用了不存在的动作 {{${ref}}}`)
        }
      }
    }
  }

  for (const step of wf.steps) {
    const configStr = JSON.stringify(step.config || {})
    checkRefs(configStr, step.label)
    if (step.condition) checkRefs(step.condition, step.label)
    for (const action of (step.actions || [])) {
      const actionLabel = action.label || action.type
      const paramsStr = JSON.stringify(action.params || {})
      checkRefs(paramsStr, `${step.label} › ${actionLabel}`)
    }
  }
  return warnings
}
