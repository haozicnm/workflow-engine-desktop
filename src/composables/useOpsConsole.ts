import { ref, readonly, shallowRef } from 'vue'

export interface OpLogEntry {
  time: string
  source: 'gui' | 'agent'
  category: 'invoke' | 'event'
  name: string
  status: 'start' | 'ok' | 'fail'
  detail?: string
  elapsed?: number
}

// ─── Module-level state (accessible outside Vue components) ───
const _logs: OpLogEntry[] = []
let _visible = true
let _subscribers: Array<() => void> = []

function _notify() {
  for (const s of _subscribers) s()
}

/** Module-level add — usable from tauri.ts without Vue dependency */
export function addOp(entry: Omit<OpLogEntry, 'time'>) {
  _logs.push({ ...entry, time: new Date().toLocaleTimeString() })
  if (_logs.length > 1000) _logs.shift()
  _notify()
}

export function clearOpsLog() {
  _logs.length = 0
  _notify()
}

export function toggleOpsPanel() {
  _visible = !_visible
  _notify()
}

// ─── Vue composable ───
export function useOpsConsole() {
  const logs = shallowRef<OpLogEntry[]>([..._logs])
  const visible = ref(_visible)

  const sub = () => {
    logs.value = [..._logs]
    visible.value = _visible
  }
  _subscribers.push(sub)

  // Cleanup on component unmount (caller should handle)
  const unsubscribe = () => {
    _subscribers = _subscribers.filter(s => s !== sub)
  }

  function addOpVue(entry: Omit<OpLogEntry, 'time'>) {
    addOp(entry)
  }

  function clearLogs() {
    clearOpsLog()
  }

  function toggle() {
    toggleOpsPanel()
  }

  return {
    logs: readonly(logs),
    visible: readonly(visible),
    addOp: addOpVue,
    clearLogs,
    toggle,
    unsubscribe,
  }
}
