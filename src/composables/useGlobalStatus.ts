import { reactive, readonly } from 'vue'
import { useToast } from './useToast'
import { safeInvoke } from '../utils/tauri'

export interface RunningWorkflow {
  id: string
  name: string
  startedAt: number
  currentStep?: string
  progress?: { done: number; total: number }
}

export interface ScheduledWorkflow {
  id: string
  workflowId: string
  workflowName: string
  cronExpr: string
  enabled: boolean
  nextRun: string | null
}

interface GlobalStatusState {
  runningWorkflows: Map<string, RunningWorkflow>
  scheduledWorkflows: ScheduledWorkflow[]
  schedulesLoaded: boolean
  ipcOnline: boolean
  apiOnline: boolean
}

const state = reactive<GlobalStatusState>({
  runningWorkflows: new Map(),
  scheduledWorkflows: [],
  schedulesLoaded: false,
  ipcOnline: false,
  apiOnline: false,
})

let scheduleRefreshTimer: ReturnType<typeof setInterval> | null = null

export function useGlobalStatus() {
  const toast = useToast()

  function registerRun(id: string, name: string) {
    state.runningWorkflows.set(id, {
      id,
      name,
      startedAt: Date.now(),
    })
    // Force reactivity on Map changes
    state.runningWorkflows = new Map(state.runningWorkflows)
  }

  function updateRunProgress(id: string, stepName: string, done: number, total: number) {
    const entry = state.runningWorkflows.get(id)
    if (entry) {
      entry.currentStep = stepName
      entry.progress = { done, total }
      state.runningWorkflows = new Map(state.runningWorkflows)
    }
  }

  function unregisterRun(id: string) {
    state.runningWorkflows.delete(id)
    state.runningWorkflows = new Map(state.runningWorkflows)
  }

  async function refreshSchedules() {
    try {
      const all = await safeInvoke<{ id: string; workflow_id: string; workflow_name: string; cron_expr: string; enabled: boolean; next_run: string | null }[]>('schedule_list')
      state.scheduledWorkflows = (all || [])
        .filter(s => s.enabled)
        .map(s => ({
          id: s.id,
          workflowId: s.workflow_id,
          workflowName: s.workflow_name,
          cronExpr: s.cron_expr,
          enabled: s.enabled,
          nextRun: s.next_run,
        }))
      state.schedulesLoaded = true
    } catch (e) {
      toast.error('Failed to load schedules')
    }
  }

  async function refreshIpcStatus() {
    try {
      const ok = await safeInvoke<boolean>('check_ipc')
      state.ipcOnline = ok === true
    } catch (e) {
      state.ipcOnline = false
    }
  }

  async function refreshApiStatus() {
    try {
      const resp = await fetch('/api/health')
      state.apiOnline = resp.status === 200
    } catch (e) {
      state.apiOnline = false
    }
  }

  function startSchedulePolling() {
    if (scheduleRefreshTimer) return
    refreshSchedules()
    scheduleRefreshTimer = setInterval(refreshSchedules, 30_000)
  }

  function stopSchedulePolling() {
    if (scheduleRefreshTimer) {
      clearInterval(scheduleRefreshTimer)
      scheduleRefreshTimer = null
    }
  }

  return {
    state: readonly(state) as unknown as typeof state,
    registerRun,
    updateRunProgress,
    unregisterRun,
    refreshSchedules,
    refreshIpcStatus,
    refreshApiStatus,
    startSchedulePolling,
    stopSchedulePolling,
  }
}
