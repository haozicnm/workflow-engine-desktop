import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import yaml from 'js-yaml'
import type { WorkflowStep, StepTypeInfo } from '../types/workflow'
import { STEP_TYPES, defaultStep, generateId, createDefaultYaml as _createDefaultYaml } from '../types/workflow'

export const useEditorStore = defineStore('editor', () => {
  const workflowName = ref('')
  const workflowDesc = ref('')
  const steps = ref<WorkflowStep[]>([])
  const yamlText = ref('')
  const yamlError = ref('')
  const variables = ref<Record<string, unknown>>({})

  const stepTypes = computed<StepTypeInfo[]>(() => STEP_TYPES)

  // ─── Utils exposed for workflowStore ───

  function setMetadata(name: string, desc: string) {
    workflowName.value = name
    workflowDesc.value = desc
  }

  function createDefaultYaml(): string {
    return _createDefaultYaml()
  }

  // ─── YAML parse ───

  function parseYaml(text: string) {
    yamlText.value = text
    yamlError.value = ''
    try {
      const data = yaml.load(text, { schema: yaml.JSON_SCHEMA })
      if (!data || typeof data !== 'object') {
        throw new Error('YAML 格式无效：应为对象')
      }
      const obj = data as Record<string, unknown>
      workflowName.value = (obj.name as string) || '未命名'
      workflowDesc.value = (obj.description as string) || ''
      variables.value = (obj.variables as Record<string, unknown>) || {}
      steps.value = ((obj.steps || []) as WorkflowStep[]).map((s: WorkflowStep) => ({
        id: s.id || generateId(),
        name: s.name || '未命名步骤',
        type: s.type || 'http',
        config: (s.config || {}) as Record<string, unknown>,
        next: s.next,
        timeout: s.timeout,
        retry: s.retry,
      }))
    } catch (e: unknown) {
      yamlError.value = (e as Error).message || 'YAML 解析失败'
    }
  }

  // ─── Sync: Steps → YAML ───

  function syncToYaml() {
    const data: Record<string, unknown> = {
      name: workflowName.value,
      description: workflowDesc.value,
      version: '1.0',
      variables: Object.keys(variables.value).length > 0 ? variables.value : undefined,
      steps: steps.value.map(s => {
        const step: Record<string, unknown> = {
          id: s.id,
          name: s.name,
          type: s.type,
          config: s.config,
        }
        if (s.next) step.next = s.next
        if (s.timeout) step.timeout = s.timeout
        if (s.retry) step.retry = s.retry
        return step
      }),
    }
    yamlText.value = yaml.dump(data, { lineWidth: 120, noRefs: true })
    yamlError.value = ''
  }

  // ─── Steps CRUD ───

  function addStep(type: string, index?: number): WorkflowStep {
    const step = defaultStep(type)
    if (index !== undefined && index >= 0) {
      steps.value.splice(index, 0, step)
    } else {
      steps.value.push(step)
    }
    syncToYaml()
    return step
  }

  function removeStep(id: string) {
    steps.value = steps.value.filter(s => s.id !== id)
    syncToYaml()
  }

  function updateStep(id: string, updates: Partial<WorkflowStep>) {
    const idx = steps.value.findIndex(s => s.id === id)
    if (idx >= 0) {
      steps.value[idx] = { ...steps.value[idx], ...updates }
      syncToYaml()
    }
  }

  function moveStep(fromIndex: number, toIndex: number) {
    if (fromIndex === toIndex) return
    const item = steps.value.splice(fromIndex, 1)[0]
    steps.value.splice(toIndex, 0, item)
    syncToYaml()
  }

  // ─── Load new ───

  function loadNew() {
    workflowName.value = '新工作流'
    workflowDesc.value = ''
    steps.value = [defaultStep('http')]
    syncToYaml()
  }

  return {
    workflowName, workflowDesc, steps, yamlText, yamlError,
    stepTypes, variables,
    setMetadata, createDefaultYaml,
    parseYaml, syncToYaml,
    addStep, removeStep, updateStep, moveStep,
    loadNew,
  }
})
