// ─── FlowEditor AutoSave — 定时保存到 localStorage ───
import { onUnmounted } from 'vue'
import { useFlowStore } from '../stores/flowStore'

const STORAGE_KEY = 'flow-editor-autosave'

export function useAutoSave(intervalMs = 30000) {
  const store = useFlowStore()
  let timer: ReturnType<typeof setInterval> | null = null

  function save() {
    if (!store.dirty) return
    try {
      const data = store.toJSON()
      localStorage.setItem(STORAGE_KEY, JSON.stringify(data))
      store.dirty = false
    } catch {
      // localStorage may be full or unavailable
    }
  }

  function start() {
    if (timer) return
    timer = setInterval(save, intervalMs)
  }

  function stop() {
    if (timer) {
      clearInterval(timer)
      timer = null
    }
  }

  function loadAutoSave(): boolean {
    try {
      const raw = localStorage.getItem(STORAGE_KEY)
      if (!raw) return false
      const data = JSON.parse(raw)
      if (data.nodes && data.edges) {
        store.load({ name: data.name || '恢复的工作流', nodes: data.nodes, edges: data.edges })
        return true
      }
    } catch {
      // corrupted data
    }
    return false
  }

  function clearAutoSave() {
    localStorage.removeItem(STORAGE_KEY)
  }

  onUnmounted(stop)

  return { start, stop, save, loadAutoSave, clearAutoSave }
}
