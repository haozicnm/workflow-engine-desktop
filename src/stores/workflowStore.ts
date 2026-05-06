import { defineStore } from 'pinia'
import { ref } from 'vue'
import { safeInvoke } from '../utils/tauri'
import {
  type Workflow, type Step, type ContainerType, type StepRunState,
  newStep, newAction, newWorkflow, serializeWorkflow, deserializeWorkflow,
} from '../types/workflow'

// ─── Backend response types ───

export interface WorkflowListItem {
  id: string
  name: string
  description: string
  enabled: boolean
  created_at: string
  updated_at: string
}

export interface WorkflowFull {
  id: string
  name: string
  description: string
  enabled: boolean
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

  // ─── List ───

  async function fetchList() {
    loading.value = true
    try {
      workflowList.value = await safeInvoke<WorkflowListItem[]>('workflow_list') || []
    } catch (e) {
      console.error('获取工作流列表失败:', e)
    } finally {
      loading.value = false
    }
  }

  // ─── Load ───

  /** 确保所有 step 都有 actions 数组（兼容旧格式/损坏数据） */
  function normalizeSteps(steps: Step[]) {
    for (const step of steps) {
      if (!Array.isArray(step.actions)) step.actions = []
      if (step.thenSteps) normalizeSteps(step.thenSteps)
      if (step.elseSteps) normalizeSteps(step.elseSteps)
    }
  }

  async function loadWorkflow(id: string) {
    loading.value = true
    try {
      const wf = await safeInvoke<WorkflowFull | null>('workflow_get', { id })
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
        // 确保所有 step 都有 actions 数组（兼容旧格式）
        normalizeSteps(parsed.steps)
        current.value = parsed
        dirty.value = false
        runStates.value = {}
      }
    } catch (e) {
      console.error('加载工作流失败:', e)
    } finally {
      loading.value = false
    }
  }


  // ─── Save ───

  async function saveWorkflow(): Promise<boolean> {
    if (!current.value) return false
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
      // Update metadata
      await safeInvoke('workflow_update', {
        id: current.value.id,
        name: current.value.name,
        description: current.value.description || '',
        enabled: true,
      })
      // Save workflow in new format (backend v5 parser handles it)
      const yaml = serializeWorkflow(current.value)
      await safeInvoke('workflow_save_yaml', {
        id: current.value.id,
        yaml,
      })
      dirty.value = false
      return true
    } catch (e) {
      console.error('保存失败:', e)
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
      console.error('删除失败:', e)
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
      console.error('克隆失败:', e)
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
      const text = await file.text()
      const wf = deserializeWorkflow(text)
      if (!wf || !Array.isArray(wf.steps)) {
        throw new Error('无效的工作流 JSON')
      }
      return wf
    } catch (e) {
      console.error('导入失败:', e)
      return null
    }
  }

  // ─── Step Operations ───

  function addStep(containerType: ContainerType) {
    if (!current.value) return
    current.value.steps.push(newStep(containerType))
    dirty.value = true
  }

  function removeStep(stepId: string) {
    if (!current.value) return
    current.value.steps = current.value.steps.filter(s => s.id !== stepId)
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
    step.actions.push(newAction(actionType, step.type, step.actions))
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

  function updateConditionGroup(stepId: string, group: import('../types/workflow').LogicConditionGroup) {
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
    action.params = { ...action.params, ...params }
    dirty.value = true
  }

  // ─── UI State ───

  function toggleStepExpanded(stepId: string) {
    const step = findStep(stepId)
    if (step) {
      step.expanded = !step.expanded
    }
  }

  // ─── Helpers ───

  function findStep(stepId: string): Step | null {
    if (!current.value) return null
    return _findStepInList(current.value.steps, stepId)
  }

  function _findStepInList(steps: Step[], stepId: string): Step | null {
    for (const step of steps) {
      if (step.id === stepId) return step
      if (step.type === 'logic') {
        if (step.thenSteps) {
          const found = _findStepInList(step.thenSteps, stepId)
          if (found) return found
        }
        if (step.elseSteps) {
          const found = _findStepInList(step.elseSteps, stepId)
          if (found) return found
        }
      }
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
    moveStep,
    addAction,
    removeAction,
    renameStep,
    renameAction,
    updateConditionGroup,
    updateActionParams,
    toggleStepExpanded,
    findStep,
    setWorkflowName,
  }
})
