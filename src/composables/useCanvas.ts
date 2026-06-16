// useCanvas.ts — Canvas graph editor state management
import { ref, computed, watch, type Ref } from 'vue'
import type { Step, Edge, NodePosition } from '../types/types'

export interface DraggingEdge {
  from: string
  fromPort: string
  mouseX: number
  mouseY: number
}

export function useCanvas(
  steps: Ref<Step[]>,
  edges: Ref<Edge[]>
) {
  // ─── Positions ───
  // Auto-layout: arrange steps vertically with spacing
  const nodePositions = ref<Map<string, NodePosition>>(new Map())
  const nodeWidth = 200
  const nodeHeight = 80
  const horizontalSpacing = 80
  const verticalSpacing = 40

  // ─── View state ───
  const zoom = ref(1)
  const panX = ref(0)
  const panY = ref(0)
  const selectedNode = ref<string | null>(null)
  const draggingEdge = ref<DraggingEdge | null>(null)
  const isPanning = ref(false)
  const panStart = ref({ x: 0, y: 0 })

  // ─── Layout computation ───
  function autoLayout() {
    const stepList = steps.value
    const edgeList = edges.value || []
    const positions = new Map<string, NodePosition>()

    if (edgeList.length === 0) {
      // Linear layout: vertical stack
      stepList.forEach((step, i) => {
        positions.set(step.id, { x: 100, y: 40 + i * (nodeHeight + verticalSpacing) })
      })
    } else {
      // Topological layout: group nodes by topological layers
      const levels = computeLevels(stepList, edgeList)
      levels.forEach((levelSteps, levelIndex) => {
        const totalWidth = levelSteps.length * (nodeWidth + horizontalSpacing) - horizontalSpacing
        const startX = Math.max(100, (totalWidth < 800 ? 800 - totalWidth : 0) / 2 + 100)
        levelSteps.forEach((stepId, i) => {
          positions.set(stepId, {
            x: startX + i * (nodeWidth + horizontalSpacing),
            y: 40 + levelIndex * (nodeHeight + verticalSpacing * 2),
          })
        })
      })
    }

    nodePositions.value = positions
  }

  // ─── Re-layout on step/edge change ───
  watch(() => steps.value, () => autoLayout(), { deep: true })
  watch(() => edges.value, () => autoLayout(), { deep: true })

  // ─── Position management ───
  function getPosition(stepId: string): NodePosition {
    return nodePositions.value.get(stepId) || { x: 100, y: 100 }
  }

  function updateNodePosition(stepId: string, x: number, y: number) {
    const pos = nodePositions.value.get(stepId)
    if (pos) {
      pos.x = x
      pos.y = y
    } else {
      nodePositions.value.set(stepId, { x, y })
    }
    // Force reactivity
    nodePositions.value = new Map(nodePositions.value)
  }

  // ─── Edge management ───
  const edgeList = computed(() => edges.value || [])

  function hasEdge(from: string, to: string): boolean {
    return edgeList.value.some(e => e.from === from && e.to === to)
  }

  function removeEdge(index: number) {
    const current = [...edges.value]
    if (index >= 0 && index < current.length) {
      current.splice(index, 1)
      // Since edges is a Ref, we need to mutate through the parent
      // We just return — caller handles the mutation
    }
  }

  // ─── Edge dragging ───
  function startDraggingEdge(stepId: string, port: string, x: number, y: number) {
    draggingEdge.value = { from: stepId, fromPort: port, mouseX: x, mouseY: y }
  }

  function updateDraggingEdge(x: number, y: number) {
    if (draggingEdge.value) {
      draggingEdge.value = { ...draggingEdge.value, mouseX: x, mouseY: y }
    }
  }

  function finishDraggingEdge(_targetStepId: string, _targetPort?: string) {
    draggingEdge.value = null
  }

  function cancelDraggingEdge() {
    draggingEdge.value = null
  }

  // ─── Selection ───
  function selectNode(id: string | null) {
    selectedNode.value = id
  }

  // ─── Zoom / Pan ───
  function handleWheel(e: WheelEvent) {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault()
      const delta = e.deltaY > 0 ? 0.9 : 1.1
      zoom.value = Math.max(0.2, Math.min(3, zoom.value * delta))
    } else {
      panX.value -= e.deltaX
      panY.value -= e.deltaY
    }
  }

  function startPan(e: MouseEvent) {
    isPanning.value = true
    panStart.value = { x: e.clientX - panX.value, y: e.clientY - panY.value }
  }

  function doPan(e: MouseEvent) {
    if (!isPanning.value) return
    panX.value = e.clientX - panStart.value.x
    panY.value = e.clientY - panStart.value.y
  }

  function endPan() {
    isPanning.value = false
  }

  function setPan(x: number, y: number) {
    panX.value = x
    panY.value = y
  }

  function setZoom(z: number) {
    zoom.value = Math.max(0.2, Math.min(3, z))
  }

  function resetView() {
    zoom.value = 1
    panX.value = 0
    panY.value = 0
  }

  // ─── SVG coordinate helpers ───
  function screenToCanvas(clientX: number, clientY: number, svgRect: DOMRect) {
    return {
      x: (clientX - svgRect.left - panX.value) / zoom.value,
      y: (clientY - svgRect.top - panY.value) / zoom.value,
    }
  }

  return {
    // State
    nodePositions,
    zoom,
    panX,
    panY,
    selectedNode,
    draggingEdge,
    isPanning,
    // Computed
    edgeList,
    // Methods
    autoLayout,
    getPosition,
    updateNodePosition,
    hasEdge,
    removeEdge,
    startDraggingEdge,
    updateDraggingEdge,
    finishDraggingEdge,
    cancelDraggingEdge,
    selectNode,
    setPan,
    setZoom,
    resetView,
    handleWheel,
    startPan,
    doPan,
    endPan,
    screenToCanvas,
    // Constants
    nodeWidth,
    nodeHeight,
  }
}

// ─── Topological sort ───
function computeLevels(steps: Step[], edges: Edge[]): string[][] {
  const adj = new Map<string, string[]>()
  const inDeg = new Map<string, number>()

  for (const s of steps) {
    adj.set(s.id, [])
    inDeg.set(s.id, 0)
  }
  for (const e of edges) {
    if (adj.has(e.from)) {
      adj.get(e.from)!.push(e.to)
    }
    if (inDeg.has(e.to)) {
      inDeg.set(e.to, (inDeg.get(e.to) || 0) + 1)
    }
  }

  const levels: string[][] = []
  const queue: string[] = []

  for (const [id, deg] of inDeg) {
    if (deg === 0) queue.push(id)
  }

  while (queue.length > 0) {
    const level: string[] = [...queue]
    levels.push(level)
    const nextQueue: string[] = []
    for (const id of queue) {
      for (const neighbor of adj.get(id) || []) {
        const newDeg = (inDeg.get(neighbor) || 1) - 1
        inDeg.set(neighbor, newDeg)
        if (newDeg === 0) {
          nextQueue.push(neighbor)
        }
      }
    }
    queue.length = 0
    queue.push(...nextQueue)
  }

  return levels
}
