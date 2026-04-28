import { ref } from 'vue'
import { useEditorStore } from '../stores/editorStore'

export function useUndo(maxSteps = 50) {
  const editor = useEditorStore()
  const history = ref<string[]>([])
  const historyIndex = ref(-1)
  const isUndoRedo = ref(false)

  function pushState() {
    if (isUndoRedo.value) return
    // Remove future states if we're in the middle
    if (historyIndex.value < history.value.length - 1) {
      history.value = history.value.slice(0, historyIndex.value + 1)
    }
    history.value.push(editor.yamlText)
    // Trim if exceeding max
    if (history.value.length > maxSteps) {
      history.value.shift()
    } else {
      historyIndex.value = history.value.length - 1
    }
  }

  function undo() {
    if (historyIndex.value <= 0) return false
    historyIndex.value--
    isUndoRedo.value = true
    editor.parseYaml(history.value[historyIndex.value])
    isUndoRedo.value = false
    return true
  }

  function redo() {
    if (historyIndex.value >= history.value.length - 1) return false
    historyIndex.value++
    isUndoRedo.value = true
    editor.parseYaml(history.value[historyIndex.value])
    isUndoRedo.value = false
    return true
  }

  function reset() {
    history.value = [editor.yamlText]
    historyIndex.value = 0
  }

  return { pushState, undo, redo, reset, history, historyIndex }
}
