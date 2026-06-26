import { ref, watch, onUnmounted, computed } from 'vue'
import { useWorkflowStore } from '../stores/workflowStore'

export interface LogEntry {
  time: string
  stepId: string
  stepName: string
  status: string
  message: string
  level: 'info' | 'error' | 'warn'
}

/**
 * P1 composable: AutoSave, Undo/Redo, Step Search, Log Panel
 */
export function useEditorEnhancements() {
  const store = useWorkflowStore()

  // ─── AutoSave ───
  let autoSaveTimer: ReturnType<typeof setTimeout> | null = null
  const autoSaveEnabled = ref(true)
  const lastSavedAt = ref<string>('')

  function scheduleAutoSave() {
    if (!autoSaveEnabled.value) return
    if (autoSaveTimer) clearTimeout(autoSaveTimer)
    autoSaveTimer = setTimeout(async () => {
      if (store.dirty && store.current) {
        const ok = await store.saveWorkflow()
        if (ok) {
          lastSavedAt.value = new Date().toLocaleTimeString()
        }
      }
    }, 3000)
  }

  // Watch dirty → schedule autosave
  const stopAutoSaveWatch = watch(
    () => store.dirty,
    (dirty) => { if (dirty) scheduleAutoSave() },
  )

  // ─── Undo/Redo ───
  const MAX_HISTORY = 50
  const undoStack = ref<string[]>([]) // serialized snapshots
  const redoStack = ref<string[]>([])
  const canUndo = computed(() => undoStack.value.length > 0)
  const canRedo = computed(() => redoStack.value.length > 0)
  let skipSnapshot = false

  function snapshot() {
    if (skipSnapshot || !store.current) return
    // 序列化完整状态：steps + edges + variables + name
    const snap = JSON.stringify({
      steps: store.current.steps,
      edges: store.current.edges || [],
      variables: store.current.variables || {},
      name: store.current.name,
    })
    if (undoStack.value.length && undoStack.value[undoStack.value.length - 1] === snap) return
    undoStack.value.push(snap)
    if (undoStack.value.length > MAX_HISTORY) undoStack.value.shift()
    redoStack.value = []
  }

  function undo() {
    if (!undoStack.value.length || !store.current) return
    const currentSnap = JSON.stringify({
      steps: store.current.steps,
      edges: store.current.edges || [],
      variables: store.current.variables || {},
      name: store.current.name,
    })
    redoStack.value.push(currentSnap)
    const prev = JSON.parse(undoStack.value.pop()!)
    skipSnapshot = true
    store.current.steps = prev.steps
    if (prev.edges) store.current.edges = prev.edges
    if (prev.variables) store.current.variables = prev.variables
    if (prev.name) store.current.name = prev.name
    store.dirty = true
    skipSnapshot = false
  }

  function redo() {
    if (!redoStack.value.length || !store.current) return
    const currentJson = JSON.stringify(store.current.steps)
    undoStack.value.push(currentJson)
    const next = redoStack.value.pop()!
    skipSnapshot = true
    store.current.steps = JSON.parse(next)
    store.dirty = true
    skipSnapshot = false
  }

  // Watch steps changes → snapshot
  const stopUndoWatch = watch(
    () => store.current?.steps,
    () => { if (!skipSnapshot) snapshot() },
    { deep: true },
  )

  // ─── Step Search ───
  const searchQuery = ref('')
  const searchVisible = ref(false)

  function toggleSearch() {
    searchVisible.value = !searchVisible.value
    if (!searchVisible.value) searchQuery.value = ''
  }

  // ─── Log Panel ───
  const logs = ref<LogEntry[]>([])
  const logPanelVisible = ref(false)

  function addLog(entry: LogEntry) {
    logs.value.push(entry)
    // Cap at 500
    if (logs.value.length > 500) logs.value.shift()
  }

  function clearLogs() {
    logs.value = []
  }

  // Cleanup
  onUnmounted(() => {
    stopAutoSaveWatch()
    stopUndoWatch()
    if (autoSaveTimer) clearTimeout(autoSaveTimer)
  })

  return {
    // AutoSave
    autoSaveEnabled,
    lastSavedAt,
    scheduleAutoSave,
    // Undo/Redo
    canUndo,
    canRedo,
    undo,
    redo,
    snapshot,
    // Search
    searchQuery,
    searchVisible,
    toggleSearch,
    // Logs
    logs,
    logPanelVisible,
    addLog,
    clearLogs,
  }
}
