import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { StepStatus } from '../types/workflow'
import { useEditorStore } from './editorStore'
import { useWorkflowStore } from './workflowStore'

export const useExecutionStore = defineStore('execution', () => {
  const running = ref(false)
  const runningRunId = ref<string | null>(null)
  const stepStatuses = ref<Record<string, StepStatus>>({})

  // ─── Run ───

  async function runWorkflow(): Promise<string | null> {
    const workflowStore = useWorkflowStore()
    const editorStore = useEditorStore()

    if (!workflowStore.currentId) {
      const saved = await workflowStore.saveWorkflow()
      if (!saved) return null
    }
    running.value = true
    stepStatuses.value = {}
    try {
      const runId = await invoke<string>('run_start', { workflowId: workflowStore.currentId })
      runningRunId.value = runId
      return runId
    } catch (e) {
      console.error('执行失败:', e)
      running.value = false
      return null
    }
  }

  // ─── Execution status ───

  function updateStepStatus(stepId: string, status: StepStatus) {
    const existing = stepStatuses.value[stepId]
    if (status.status === 'running') {
      status.startedAt = Date.now()
    } else if (existing?.startedAt) {
      status.duration = Date.now() - existing.startedAt
      status.startedAt = existing.startedAt
    }
    stepStatuses.value[stepId] = status
  }

  function clearStepStatuses() {
    stepStatuses.value = {}
  }

  function getStepStatusesRaw(): Record<string, StepStatus> {
    return stepStatuses.value
  }

  // ─── Validate ───

  async function validateYaml(): Promise<{ valid: boolean; error?: string }> {
    try {
      const editorStore = useEditorStore()
      const result = await invoke<{ valid: boolean; error?: string }>('workflow_validate', { yaml: editorStore.yamlText })
      return result
    } catch (e) {
      return { valid: false, error: String(e) }
    }
  }

  return {
    running, runningRunId, stepStatuses,
    runWorkflow, updateStepStatus, clearStepStatuses, getStepStatusesRaw,
    validateYaml,
  }
})
