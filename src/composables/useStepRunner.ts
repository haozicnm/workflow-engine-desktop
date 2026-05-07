import { ref } from 'vue'
import { useWorkflowStore } from '../stores/workflowStore'
import type { Workflow, StepRunState } from '../types/types'
import { safeInvoke, safeListen } from '../utils/tauri'

export function useStepRunner() {
  const store = useWorkflowStore()
  const runId = ref<string | null>(null)
  const isRunning = ref(false)

  let unlistenStep: (() => void) | null = null
  let unlistenRun: (() => void) | null = null
  const stepStartTimes: Record<string, number> = {} // T14: track per-step start time

  /**
   * Run a workflow: save first, then call backend run_start, listen for events.
   */
  async function runWorkflow(wf: Workflow): Promise<void> {
    if (isRunning.value) return

    // 1. Save workflow first (need an id for the backend)
    if (!wf.id || store.dirty) {
      const ok = await store.saveWorkflow()
      if (!ok) {
        console.error('[StepRunner] 保存工作流失败，无法运行')
        return
      }
    }
    if (!wf.id) {
      console.error('[StepRunner] 工作流无 ID')
      return
    }

    // 2. Clear previous run states
    store.runStates = {}
    Object.keys(stepStartTimes).forEach(k => delete stepStartTimes[k])
    isRunning.value = true

    try {
      // 3. Listen for step-update events
      unlistenStep = await safeListen<{
        run_id: string
        step_id: string
        step_name: string
        total_steps: number
        status: string
        output?: unknown
        error?: string | null
      }>('step-update', (event) => {
        const { step_id, status, output, error } = event.payload

        // Map backend status to frontend status
        const statusMap: Record<string, StepRunState['status']> = {
          running: 'running',
          completed: 'success',
          failed: 'error',
          ignored: 'success', // ignored errors are treated as success
        }
        const frontendStatus = statusMap[status] || 'idle'

        // Update runStates
        const existing = store.runStates[step_id]
        if (existing) {
          existing.status = frontendStatus
          if (output !== undefined && output !== null) {
            existing.output = output
          }
          if (error) {
            existing.error = error
          }
          if (frontendStatus === 'success' || frontendStatus === 'error') {
            // T14: Calculate duration from frontend-tracked start time
            const start = stepStartTimes[step_id]
            if (start) {
              existing.duration = Date.now() - start
              delete stepStartTimes[step_id]
            }
          }
          // Trigger reactivity
          store.runStates = { ...store.runStates }
        } else {
          // First time seeing this step — initialize
          store.runStates[step_id] = {
            status: frontendStatus,
            actionStates: {},
            output: output ?? undefined,
            error: error ?? undefined,
          }
          // T14: Record start time
          if (frontendStatus === 'running') {
            stepStartTimes[step_id] = Date.now()
          }
          store.runStates = { ...store.runStates }
        }
      })

      // 4. Listen for run-update events
      unlistenRun = await safeListen<{
        run_id: string
        workflow_name?: string
        status: string
        error?: string
      }>('run-update', (event) => {
        const { status, error } = event.payload

        if (status === 'completed') {
          // 工作流执行完成
          isRunning.value = false
          cleanup()
        } else if (status === 'failed') {
          console.error('[StepRunner] ❌ 工作流执行失败:', error)
          isRunning.value = false
          cleanup()
        } else if (status === 'cancelled') {
          // 工作流已取消
          isRunning.value = false
          cleanup()
        }
        // 'running', 'paused' — just ignore, keep listening
      })

      // 5. Start execution
      const id = await safeInvoke<string>('run_start', { workflowId: wf.id })
      if (!id) {
        console.error('[StepRunner] run_start 返回空 ID')
        isRunning.value = false
        cleanup()
        return
      }
      runId.value = id
      // 运行已启动

    } catch (e) {
      console.error('[StepRunner] 启动失败:', e)
      isRunning.value = false
      cleanup()
    }
  }

  /**
   * Stop the running workflow.
   */
  async function stopWorkflow() {
    if (!runId.value) return
    try {
      await safeInvoke('run_cancel', { runId: runId.value })
      // 取消请求已发送
    } catch (e) {
      console.error('[StepRunner] 取消失败:', e)
    }
  }

  function cleanup() {
    unlistenStep?.()
    unlistenRun?.()
    unlistenStep = null
    unlistenRun = null
    runId.value = null
  }

  return {
    runWorkflow,
    stopWorkflow,
    isRunning,
    runId,
  }
}
