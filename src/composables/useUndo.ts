// ─── FlowEditor Undo/Redo — 快照 nodes + edges 状态 ───
import { ref, watch } from 'vue'
import { useFlowStore } from '../stores/flowStore'
import type { FlowNode, FlowEdge } from '../components/flow/pinTypes'

interface Snapshot {
  nodes: FlowNode[]
  edges: FlowEdge[]
  workflowName: string
}

export function useUndo(maxSteps = 50) {
  const store = useFlowStore()
  const history = ref<Snapshot[]>([])
  const historyIndex = ref(-1)
  const isUndoRedo = ref(false)

  function takeSnapshot(): Snapshot {
    return {
      nodes: JSON.parse(JSON.stringify(store.nodes)),
      edges: JSON.parse(JSON.stringify(store.edges)),
      workflowName: store.workflowName,
    }
  }

  function restoreSnapshot(snap: Snapshot) {
    isUndoRedo.value = true
    store.load({
      name: snap.workflowName,
      nodes: snap.nodes,
      edges: snap.edges,
    })
    isUndoRedo.value = false
  }

  function pushState() {
    if (isUndoRedo.value) return
    if (historyIndex.value < history.value.length - 1) {
      history.value = history.value.slice(0, historyIndex.value + 1)
    }
    history.value.push(takeSnapshot())
    if (history.value.length > maxSteps) {
      history.value.shift()
    } else {
      historyIndex.value = history.value.length - 1
    }
  }

  function undo(): boolean {
    if (historyIndex.value <= 0) return false
    historyIndex.value--
    restoreSnapshot(history.value[historyIndex.value])
    return true
  }

  function redo(): boolean {
    if (historyIndex.value >= history.value.length - 1) return false
    historyIndex.value++
    restoreSnapshot(history.value[historyIndex.value])
    return true
  }

  function init() {
    history.value = [takeSnapshot()]
    historyIndex.value = 0
  }

  return { pushState, undo, redo, init, history, historyIndex }
}
