// Backward-compatible combined store facade
// Gradually migrate to importing from specific stores and types
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import yaml from 'js-yaml'
import type { WorkflowStep, WorkflowListItem } from '../types/workflow'
import { STEP_TYPES, STEP_COLORS, STEP_LABELS, STEP_ICONS, generateId, defaultStep, createDefaultYaml } from '../types/workflow'
import type { StepStatus } from '../types/workflow'

// Re-export types
export type { WorkflowStep, WorkflowData, StepStatus } from '../types/workflow'
export { STEP_TYPES, STEP_COLORS, STEP_LABELS, STEP_ICONS, generateId, defaultStep, createDefaultYaml } from '../types/workflow'

// ─── Combined store for backward compatibility ───
// All state + actions from the 3-domain split are available through this single store.

export const useWorkflowStore = defineStore('workflow', () => {
  // ── workflowStore domain ──
  const workflowList = ref<WorkflowListItem[]>([])
  const currentId = ref<string | null>(null)
  const loading = ref(false)
  const saving = ref(false)

  // ── editorStore domain ──
  const workflowName = ref('')
  const workflowDesc = ref('')
  const steps = ref<WorkflowStep[]>([])
  const yamlText = ref('')
  const yamlError = ref('')
  const variables = ref<Record<string, unknown>>({})
  const stepTypes = computed(() => STEP_TYPES)

  // ── executionStore domain ──
  const running = ref(false)
  const runningRunId = ref<string | null>(null)
  const stepStatuses = ref<Record<string, StepStatus>>({})

  // ═══ workflowStore methods ═══

  async function fetchList() {
    loading.value = true
    try { workflowList.value = await invoke<WorkflowListItem[]>('workflow_list') }
    catch (e) { console.error('获取工作流列表失败:', e) }
    finally { loading.value = false }
  }

  async function loadWorkflow(id: string) {
    loading.value = true
    try {
      const wf = await invoke<{ id: string; name: string; description: string; yaml: string | null } | null>('workflow_get', { id })
      if (wf) {
        currentId.value = wf.id
        workflowName.value = wf.name
        workflowDesc.value = wf.description || ''
        const content = wf.yaml || createDefaultYaml()
        parseYaml(content)
      }
    } catch (e) { console.error('加载工作流失败:', e) }
    finally { loading.value = false }
  }

  function loadNew() {
    currentId.value = null
    workflowName.value = '新工作流'
    workflowDesc.value = ''
    steps.value = [defaultStep('http')]
    stepStatuses.value = {}
    syncToYaml()
  }

  async function saveWorkflow(): Promise<boolean> {
    saving.value = true
    try {
      if (!currentId.value) {
        const id = await invoke<string>('workflow_create', { name: workflowName.value, description: workflowDesc.value })
        currentId.value = id
      }
      await invoke('workflow_update', { id: currentId.value, name: workflowName.value, description: workflowDesc.value, enabled: true })
      await invoke('workflow_save_yaml', { id: currentId.value, yaml: yamlText.value })
      return true
    } catch (e) { console.error('保存失败:', e); return false }
    finally { saving.value = false }
  }

  async function cloneWorkflow(id: string): Promise<string | null> {
    try {
      const wf = await invoke<{ id: string; name: string; description: string; yaml: string | null } | null>('workflow_get', { id })
      if (!wf) return null
      const newId = await invoke<string>('workflow_create', { name: wf.name + '（副本）', description: wf.description })
      if (wf.yaml) await invoke('workflow_save_yaml', { id: newId, yaml: wf.yaml })
      return newId
    } catch (e) { console.error('克隆失败:', e); return null }
  }

  function exportYaml(yamlContent?: string, name?: string) {
    const content = yamlContent || yamlText.value
    const blob = new Blob([content], { type: 'text/yaml' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = (name || workflowName.value || 'workflow') + '.yaml'
    a.click()
    URL.revokeObjectURL(url)
  }

  async function importYaml(file: File): Promise<{ name: string; yaml: string } | null> {
    try {
      const text = await file.text()
      const data = yaml.load(text, { schema: yaml.JSON_SCHEMA })
      if (!data || typeof data !== 'object' || !(data as Record<string, unknown>).steps) throw new Error('无效的工作流 YAML')
      return { name: (data as Record<string, string>).name || file.name.replace(/\.ya?ml$/i, ''), yaml: text }
    } catch (e) { console.error('导入失败:', e); return null }
  }

  // ═══ editorStore methods ═══

  function parseYaml(text: string) {
    yamlText.value = text
    yamlError.value = ''
    try {
      const data = yaml.load(text, { schema: yaml.JSON_SCHEMA })
      if (!data || typeof data !== 'object') throw new Error('YAML 格式无效：应为对象')
      const obj = data as Record<string, unknown>
      workflowName.value = (obj.name as string) || '未命名'
      workflowDesc.value = (obj.description as string) || ''
      variables.value = (obj.variables as Record<string, unknown>) || {}
      steps.value = ((obj.steps || []) as WorkflowStep[]).map((s: WorkflowStep) => ({
        id: s.id || generateId(), name: s.name || '未命名步骤', type: s.type || 'http',
        config: (s.config || {}) as Record<string, unknown>, next: s.next, timeout: s.timeout, retry: s.retry,
      }))
    } catch (e: unknown) { yamlError.value = (e as Error).message || 'YAML 解析失败' }
  }

  function syncToYaml() {
    const data: Record<string, unknown> = {
      name: workflowName.value, description: workflowDesc.value, version: '1.0',
      variables: Object.keys(variables.value).length > 0 ? variables.value : undefined,
      steps: steps.value.map(s => {
        const step: Record<string, unknown> = { id: s.id, name: s.name, type: s.type, config: s.config }
        if (s.next) step.next = s.next; if (s.timeout) step.timeout = s.timeout; if (s.retry) step.retry = s.retry
        return step
      }),
    }
    yamlText.value = yaml.dump(data, { lineWidth: 120, noRefs: true })
    yamlError.value = ''
  }

  function addStep(type: string, index?: number): WorkflowStep {
    const step = defaultStep(type)
    if (index !== undefined && index >= 0) steps.value.splice(index, 0, step)
    else steps.value.push(step)
    syncToYaml()
    return step
  }

  function removeStep(id: string) {
    steps.value = steps.value.filter(s => s.id !== id)
    delete stepStatuses.value[id]
    syncToYaml()
  }

  function updateStep(id: string, updates: Partial<WorkflowStep>) {
    const idx = steps.value.findIndex(s => s.id === id)
    if (idx >= 0) { steps.value[idx] = { ...steps.value[idx], ...updates }; syncToYaml() }
  }

  function moveStep(fromIndex: number, toIndex: number) {
    if (fromIndex === toIndex) return
    const item = steps.value.splice(fromIndex, 1)[0]
    steps.value.splice(toIndex, 0, item)
    syncToYaml()
  }

  // ═══ executionStore methods ═══

  async function runWorkflow(): Promise<string | null> {
    if (!currentId.value) { const s = await saveWorkflow(); if (!s) return null }
    running.value = true; stepStatuses.value = {}
    try {
      const runId = await invoke<string>('run_start', { workflowId: currentId.value })
      runningRunId.value = runId; return runId
    } catch (e) { console.error('执行失败:', e); running.value = false; return null }
  }

  function updateStepStatus(stepId: string, status: StepStatus) {
    const existing = stepStatuses.value[stepId]
    if (status.status === 'running') status.startedAt = Date.now()
    else if (existing?.startedAt) { status.duration = Date.now() - existing.startedAt; status.startedAt = existing.startedAt }
    stepStatuses.value[stepId] = status
  }

  function clearStepStatuses() { stepStatuses.value = {} }

  async function validateYaml(): Promise<{ valid: boolean; error?: string }> {
    try {
      const r = await invoke<{ valid: boolean; error?: string }>('workflow_validate', { yaml: yamlText.value })
      return r
    } catch (e) { return { valid: false, error: String(e) } }
  }

  return {
    workflowList, currentId, workflowName, workflowDesc,
    steps, yamlText, yamlError, loading, saving, running, runningRunId,
    stepStatuses, stepTypes, variables,
    fetchList, loadWorkflow, loadNew,
    parseYaml, syncToYaml,
    addStep, removeStep, updateStep, moveStep,
    saveWorkflow, runWorkflow, validateYaml,
    updateStepStatus, clearStepStatuses,
    cloneWorkflow, exportYaml, importYaml,
  }
})
