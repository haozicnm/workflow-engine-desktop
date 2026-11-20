import { defineStore } from 'pinia'
import { ref } from 'vue'
import { deepMerge } from '../lib/utils'
import { normalizeSteps, validateRefs } from '../composables/useWorkflowValidate'
import { safeInvoke } from '../utils/tauri'
import { useToast } from '../composables/useToast'
import type { Workflow, Step, Action, ContainerType, StepRunState } from '../types/types'
import { newWorkflow, serializeWorkflow, deserializeWorkflow, nextStepId, nextActionId } from '../types/types'
import { newStep, newAction } from '../types/node-registry'

// ─── Backend response types ───

export interface WorkflowListItem {
  id: string
  name: string
  description: string
  enabled: boolean
  locked: boolean
  created_at: string
  updated_at: string
}

export interface WorkflowFull {
  id: string
  name: string
  description: string
  enabled: boolean
  locked: boolean
  yaml: string   // backend field — now holds JSON string
  created_at: string
  updated_at: string
}

export const useWorkflowStore = defineStore('workflow', () => {
  // ─── State ───
  const workflowList = ref<WorkflowListItem[]>([])
  const current = ref<Workflow | null>(null)
  const dirty = ref(false)
  const runStates = ref<Record<string, StepRunState>>({})
  const loading = ref(false)
  const saving = ref(false)
  const lastWarnings = ref<string[]>([])
  const toast = useToast()

  // ─── List ───

  async function fetchList() {
    loading.value = true
    try {
      workflowList.value = await safeInvoke<WorkflowListItem[]>('workflow_list') || []
    } catch (e) {
      toast.error('Failed to fetch workflow list: ' + (e as Error).message)
    } finally {
      loading.value = false
    }
  }

  // ─── Load ───

  let loadSeq = 0 // 防止竞态：后到的旧请求不覆盖先到的新数据

  async function loadWorkflow(id: string) {
    const seq = ++loadSeq
    loading.value = true
    try {
      const wf = await safeInvoke<WorkflowFull | null>('workflow_get', { id })
      if (seq !== loadSeq) return // 更新的请求已发起，丢弃过期响应
      if (wf) {
        let parsed: Workflow
        try {
          const raw = JSON.parse(wf.yaml || '{}')
          parsed = raw as Workflow
        } catch {
          parsed = newWorkflow()
        }
        parsed.id = wf.id
        parsed.name = wf.name
        parsed.description = wf.description || ''
        parsed.locked = wf.locked
        // 确保所有 step 都有 actions 数组（兼容旧格式）
        normalizeSteps(parsed.steps)
        current.value = parsed
        dirty.value = false
        runStates.value = {}
      }
    } catch (e) {
      toast.error('Failed to load workflow: ' + (e as Error).message)
    } finally {
      loading.value = false
    }
  }

  // ─── Save ───

  async function saveWorkflow(): Promise<boolean> {
    if (!current.value) return false
    if (current.value.locked) {
      console.warn('[WorkflowStore] 工作流已锁定，无法保存')
      return false
    }
    // 校验变量引用
    lastWarnings.value = validateRefs(current.value)
    // 必须在创建记录**之前**拦住空工作流：后端 parse_workflow 会以
    // "工作流至少需要一个步骤" 拒绝保存，而 workflow_create 已经建好了记录
    // （并立即出现在侧边栏）—— 用户会看到"保存失败"但工作流已经存在。
    if (!current.value.steps || current.value.steps.length === 0) {
      toast.error('Workflow needs at least one step before it can be saved')
      return false
    }
    saving.value = true
    try {
      // Create new if no id
      if (!current.value.id) {
        const id = await safeInvoke<string>('workflow_create', {
          name: current.value.name,
          description: current.value.description || '',
        })
        if (!id) return false
        current.value.id = id
      }
      // 先保存内容（YAML），再更新元数据
      // 快照当前状态，保存后比较是否有新变更
      const snapshot = serializeWorkflow(current.value)
      await safeInvoke('workflow_save_yaml', {
        id: current.value.id,
        yaml: snapshot,
      })
      // Update metadata
      await safeInvoke('workflow_update', {
        id: current.value.id,
        name: current.value.name,
        description: current.value.description || '',
        enabled: true,
      })
      // 只有保存期间没有新变更才清除 dirty 标志
      if (serializeWorkflow(current.value) === snapshot) {
        dirty.value = false
      }
      return true
    } catch (e) {
      toast.error('Failed to save workflow: ' + (e as Error).message)
      return false
    } finally {
      saving.value = false
    }
  }

  // ─── Delete ───

  async function deleteWorkflow(id: string) {
    try {
      await safeInvoke('workflow_delete', { id })
      workflowList.value = workflowList.value.filter(w => w.id !== id)
    } catch (e) {
      toast.error('Failed to delete workflow: ' + (e as Error).message)
    }
  }

  // ─── Clone ───

  async function cloneWorkflow(id: string): Promise<string | null> {
    try {
      const wf = await safeInvoke<WorkflowFull | null>('workflow_get', { id })
      if (!wf) return null
      const newId = await safeInvoke<string>('workflow_create', {
        name: wf.name + '（副本）',
        description: wf.description,
      })
      if (!newId) return null
      if (wf.yaml) {
        await safeInvoke('workflow_save_yaml', { id: newId, yaml: wf.yaml })
      }
      return newId
    } catch (e) {
      toast.error('Failed to clone workflow: ' + (e as Error).message)
      return null
    }
  }

  // ─── Export / Import ───

  function exportJson(wf: Workflow) {
    const json = serializeWorkflow(wf)
    const blob = new Blob([json], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = (wf.name || 'workflow') + '.json'
    a.click()
    URL.revokeObjectURL(url)
  }

  async function importJson(file: File): Promise<Workflow | null> {
    try {
      // 文件大小限制（5MB）
      const MAX_FILE_SIZE = 5 * 1024 * 1024
      if (file.size > MAX_FILE_SIZE) {
        throw new Error(`文件过大 (${(file.size / 1024 / 1024).toFixed(1)}MB > 5MB)`)
      }
      const text = await file.text()
      const wf = deserializeWorkflow(text)
      if (!wf || !Array.isArray(wf.steps)) {
        throw new Error('无效的工作流 JSON')
      }
      return wf
    } catch (e) {
      toast.error('Failed to import workflow: ' + (e as Error).message)
      return null
    }
  }

  // ─── Step Operations ───

  function addStep(containerType: ContainerType) {
    if (!current.value) return
    current.value.steps.push(newStep(containerType, current.value.steps))
    dirty.value = true
  }

  function removeStep(stepId: string) {
    if (!current.value) return
    current.value.steps = current.value.steps.filter(s => s.id !== stepId)
    // 清理引用已删除步骤的边
    if (current.value.edges) {
      current.value.edges = current.value.edges.filter(
        e => e.from !== stepId && e.to !== stepId
      )
    }
    // 清理其他步骤中引用已删除步骤的 runCondition
    for (const step of current.value.steps) {
      if (step.runCondition?.ref === stepId) {
        delete step.runCondition
      }
    }
    dirty.value = true
  }

  /**
   * 复制一个步骤（含容器 actions）并插入到原步骤之后。
   *
   * StepCard 的 ⋯ 菜单会 emit `duplicate-step`，但此前没有任何父组件监听，
   * 点击后菜单关闭、什么都不发生。
   */
  function duplicateStep(stepId: string) {
    if (!current.value) return
    const steps = current.value.steps
    const index = steps.findIndex(s => s.id === stepId)
    if (index < 0) return
    const source = steps[index]
    const clone: Step = JSON.parse(JSON.stringify(source))
    clone.id = nextStepId(steps)
    clone.label = `${source.label || source.type} (copy)`
    // 不复刻 next：线性模式下原步骤与副本指向同一后继会导致重复执行
    delete clone.next
    // action id 必须重新生成，否则更新某个 action 会同时命中两个步骤
    if (clone.actions?.length) {
      const fresh: Action[] = []
      for (const act of clone.actions) {
        fresh.push({ ...act, id: nextActionId(clone.id, fresh) })
      }
      clone.actions = fresh
    }
    steps.splice(index + 1, 0, clone)
    dirty.value = true
  }

  function moveStep(fromIndex: number, toIndex: number) {
    if (!current.value) return
    const steps = current.value.steps
    if (fromIndex < 0 || fromIndex >= steps.length) return
    if (toIndex < 0 || toIndex >= steps.length) return
    const [moved] = steps.splice(fromIndex, 1)
    steps.splice(toIndex, 0, moved)
    dirty.value = true
  }

  // ─── Action Operations ───

  function addAction(stepId: string, actionType: string) {
    const step = findStep(stepId)
    if (!step) return
    if (!step.actions) step.actions = []
    step.actions.push(newAction(actionType, step.type, step.actions, stepId))
    dirty.value = true
  }

  function removeAction(stepId: string, actionId: string) {
    const step = findStep(stepId)
    if (!step) return
    step.actions = (step.actions || []).filter(a => a.id !== actionId)
    dirty.value = true
  }

  function renameStep(stepId: string, label: string) {
    const step = findStep(stepId)
    if (!step) return
    step.label = label
    dirty.value = true
  }

  function renameAction(stepId: string, actionId: string, label: string) {
    const step = findStep(stepId)
    if (!step) return
    const action = step.actions?.find(a => a.id === actionId)
    if (!action) return
    action.label = label
    dirty.value = true
  }

  function updateConditionGroup(stepId: string, group: import('../types/types').LogicConditionGroup) {
    const step = findStep(stepId)
    if (!step) return
    step.conditionGroup = group
    dirty.value = true
  }

  function updateActionParams(stepId: string, actionId: string, params: Record<string, unknown>) {
    const step = findStep(stepId)
    if (!step) return
    const action = step.actions?.find(a => a.id === actionId)
    if (!action) return
    action.params = deepMerge(action.params ?? {}, params) as typeof action.params
    dirty.value = true
  }

  function updateStepConfig(stepId: string, key: string, value: unknown) {
    const step = findStep(stepId)
    if (!step) return
    step.config[key] = value
    dirty.value = true
  }

  function updateRunCondition(stepId: string, condition: import('../types/types').StepCondition | null) {
    const step = findStep(stepId)
    if (!step) return
    if (condition) {
      step.runCondition = condition
    } else {
      delete step.runCondition
    }
    dirty.value = true
  }

  // ─── UI State ───

  function toggleStepExpanded(stepId: string) {
    const step = findStep(stepId)
    if (step) {
      step.expanded = !step.expanded
    }
  }
  // ─── Edge Operations (v8.2 Canvas) ───

  function addEdge(from: string, to: string, fromPort?: string, toPort?: string) {
    if (!current.value) return
    if (from === to) return // 禁止自环
    if (!current.value.edges) current.value.edges = []
    // Avoid duplicates (same from+to+fromPort)
    if (current.value.edges.some(e => e.from === from && e.to === to && (e.fromPort || '') === (fromPort || ''))) return
    // 简单环检测：添加这条边后，是否能从 to 回到 from？
    const visited = new Set<string>()
    const queue = [to]
    while (queue.length) {
      const node = queue.shift()!
      if (node === from) return // 会形成环，拒绝
      if (visited.has(node)) continue
      visited.add(node)
      for (const e of current.value.edges) {
        if (e.from === node) queue.push(e.to)
      }
    }
    const edge: any = { from, to }
    if (fromPort) edge.fromPort = fromPort
    if (toPort) edge.toPort = toPort
    current.value.edges.push(edge)
    dirty.value = true
  }

  function removeEdge(from: string, to: string) {
    if (!current.value?.edges) return
    current.value.edges = current.value.edges.filter(e => !(e.from === from && e.to === to))
    dirty.value = true
  }

  // ─── Helpers ───

  function findStep(stepId: string): Step | null {
    if (!current.value) return null
    return _findStepInList(current.value.steps, stepId)
  }

  function _findStepInList(steps: Step[], stepId: string): Step | null {
    for (const step of steps) {
      if (step.id === stepId) return step
    }
    return null
  }

  function setWorkflowName(name: string) {
    if (!current.value) return
    current.value.name = name
    dirty.value = true
  }

  return {
    // state
    workflowList,
    current,
    dirty,
    runStates,
    loading,
    saving,
    lastWarnings,
    // actions
    fetchList,
    loadWorkflow,
    saveWorkflow,
    deleteWorkflow,
    cloneWorkflow,
    exportJson,
    importJson,
    addStep,
    removeStep,
    duplicateStep,
    moveStep,
    addEdge,
    removeEdge,
    addAction,
    removeAction,
    renameStep,
    renameAction,
    updateConditionGroup,
    updateActionParams,
    updateStepConfig,
    updateRunCondition,
    toggleStepExpanded,
    findStep,
    setWorkflowName,
  }
})
