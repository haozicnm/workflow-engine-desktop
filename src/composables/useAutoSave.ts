import { onMounted, onUnmounted, ref } from 'vue'
import { useWorkflowStore } from '../stores/workflowStore'
import { useEditorStore } from '../stores/editorStore'

export function useAutoSave(intervalMs = 60000) {
  const workflowStore = useWorkflowStore()
  const editor = useEditorStore()
  const lastSaved = ref<number>(Date.now())
  let timer: ReturnType<typeof setInterval> | null = null
  let beforeUnloadHandler: ((e: BeforeUnloadEvent) => void) | null = null

  function start() {
    if (timer) return
    timer = setInterval(async () => {
      // Only save if yaml has changed since last save
      if (workflowStore.currentId) {
        await workflowStore.saveWorkflow()
        lastSaved.value = Date.now()
      }
    }, intervalMs)

    beforeUnloadHandler = (e: BeforeUnloadEvent) => {
      if (editor.yamlText) {
        e.preventDefault()
        e.returnValue = '' // Chrome requires this
      }
    }
    window.addEventListener('beforeunload', beforeUnloadHandler)
  }

  function stop() {
    if (timer) {
      clearInterval(timer)
      timer = null
    }
    if (beforeUnloadHandler) {
      window.removeEventListener('beforeunload', beforeUnloadHandler)
      beforeUnloadHandler = null
    }
  }

  onUnmounted(stop)

  return { start, stop, lastSaved }
}
