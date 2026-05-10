// composables/useVariableRefs.ts
// 统一的变量引用逻辑 — ActionRow / LogicBranch / StepCard 共用
import { computed } from 'vue'
import type { Step } from '../types/types'
import { getContainerDef } from '../types/node-registry'

export interface VarRef {
  id: string
  label: string
  icon: string
  type: 'step' | 'action'
}

export interface StepGroup {
  stepId: string
  stepLabel: string
  stepIcon: string
  stepRef: string
  outputHint?: string
  actions: { id: string; label: string; ref: string; isSameContainer?: boolean }[]
}

/**
 * 构建变量引用列表
 * @param steps 所有步骤
 * @param currentStepId 当前步骤 ID（用于标记"容器内"引用）
 */
export function useVariableRefs(
  steps: () => Step[],
  currentStepId?: () => string | undefined,
) {
  const availableRefs = computed<VarRef[]>(() => {
    const allSteps = steps()
    if (!allSteps?.length) return []
    const refs: VarRef[] = []
    for (const step of allSteps) {
      const stepIcon = getContainerDef(step.type).icon
      // 步骤级引用：{{stepId}} → 整个步骤输出
      refs.push({ id: step.id, label: step.label, icon: stepIcon, type: 'step' })
      // 动作级引用：{{stepId.actionId}} → 单个动作输出
      for (const action of (step.actions || [])) {
        const actionLabel = action.label || action.type
        refs.push({
          id: `${step.id}.${action.id}`,
          label: `${step.label} › ${actionLabel}`,
          icon: 'Zap',
          type: 'action',
        })
      }
    }
    return refs
  })

  // 按步骤分组（树形下拉用）
  const groupedRefs = computed<StepGroup[]>(() => {
    const groups = new Map<string, StepGroup>()
    const allSteps = steps()
    const curId = currentStepId?.()

    for (const r of availableRefs.value) {
      if (r.type === 'step') {
        const step = allSteps?.find(s => s.id === r.id)
        const hint = step ? getContainerDef(step.type).outputHint : undefined
        groups.set(r.id, {
          stepId: r.id,
          stepLabel: r.label,
          stepIcon: r.icon,
          stepRef: r.id,
          outputHint: hint,
          actions: [],
        })
      }
    }
    for (const r of availableRefs.value) {
      if (r.type === 'action') {
        const dotIdx = r.id.indexOf('.')
        const stepId = dotIdx > 0 ? r.id.slice(0, dotIdx) : r.id
        const group = groups.get(stepId)
        if (group) {
          const actionLabel = r.label.includes('›') ? r.label.split('›').pop()!.trim() : r.label
          const isSameContainer = !!(curId && stepId === curId)
          group.actions.push({ id: r.id, label: actionLabel, ref: r.id, isSameContainer })
        }
      }
    }
    return Array.from(groups.values())
  })

  return { availableRefs, groupedRefs }
}
