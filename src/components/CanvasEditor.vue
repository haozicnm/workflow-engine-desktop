<script setup lang="ts">
// components/CanvasEditor.vue — Canvas 图编辑器主组件
//
// 渲染步骤卡片为可拖拽节点，支持端口间连线、缩放平移、执行高亮

import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Workflow, Step, Edge, StepRunState } from '../types/types'
import { useCanvas } from '../composables/useCanvas'
import CanvasNode from './CanvasNode.vue'
import CanvasEdge from './CanvasEdge.vue'
import Button from './ui/button/Button.vue'

const { t } = useI18n()

const props = defineProps<{
  workflow: Workflow | null | undefined
  runStates: Record<string, StepRunState>
}>()

const emit = defineEmits<{
  'add-edge': [from: string, to: string]
  'remove-edge': [from: string, to: string]
  'update-node-position': [id: string, x: number, y: number]
}>()

// ─── 响应式数据 ───
const stepRef = ref<Step[]>(props.workflow?.steps || [])
const edgeRef = ref<Edge[]>(props.workflow?.edges || [])

watch(() => props.workflow?.steps, (val) => { stepRef.value = val || [] }, { deep: true })
watch(() => props.workflow?.edges, (val) => { edgeRef.value = val || [] }, { deep: true })

const canvas = useCanvas(stepRef, edgeRef)

// 回写到 workflow
watch(edgeRef, (val) => {
  if (props.workflow) {
    props.workflow.edges = val
  }
}, { deep: true })

// SVG 容器
const svgContainer = ref<HTMLElement | null>(null)
const selectedEdgeIdx = ref<number | null>(null)

// Delete 键删除选中的边
function onKeyDown(e: KeyboardEvent) {
  if (e.key === 'Delete' || e.key === 'Backspace') {
    if (selectedEdgeIdx.value !== null) {
      const line = edgeLines.value[selectedEdgeIdx.value]
      if (line) {
        emit('remove-edge', line.fromId, line.toId)
        selectedEdgeIdx.value = null
      }
    }
  }
}

// ─── 拖拽节点 ───
let dragTarget: string | null = null
let dragOffsetX = 0
let dragOffsetY = 0

function onNodeDragStart(stepId: string, e: MouseEvent) {
  dragTarget = stepId
  const pos = canvas.nodePositions.value.get(stepId)
  if (pos) {
    const rect = svgContainer.value?.getBoundingClientRect()
    if (!rect) return
    dragOffsetX = e.clientX - pos.x * canvas.zoom.value - canvas.panX.value
    dragOffsetY = e.clientY - pos.y * canvas.zoom.value - canvas.panY.value
  }
  window.addEventListener('mousemove', onNodeDragMove)
  window.addEventListener('mouseup', onNodeDragEnd)
}

function onNodeDragMove(e: MouseEvent) {
  if (!dragTarget) return
  const x = (e.clientX - dragOffsetX) / canvas.zoom.value
  const y = (e.clientY - dragOffsetY) / canvas.zoom.value
  canvas.updateNodePosition(dragTarget, Math.max(0, x), Math.max(0, y))
  emit('update-node-position', dragTarget, Math.max(0, x), Math.max(0, y))
}

function onNodeDragEnd() {
  dragTarget = null
  window.removeEventListener('mousemove', onNodeDragMove)
  window.removeEventListener('mouseup', onNodeDragEnd)
}

// ─── 端口连线拖拽 ───
let portDragData: { stepId: string; portLabel: string } | null = null

function onPortDragStart(stepId: string, portLabel: string, _e: MouseEvent) {
  portDragData = { stepId, portLabel }
  const pos = canvas.nodePositions.value.get(stepId)
  if (pos) {
    const portX = (pos.x + canvas.nodeWidth / 2) * canvas.zoom.value + canvas.panX.value
    const portY = (pos.y + canvas.nodeHeight / 2) * canvas.zoom.value + canvas.panY.value
    canvas.startDraggingEdge(stepId, portLabel, portX, portY)
  }
  window.addEventListener('mousemove', onPortDragMove)
  window.addEventListener('mouseup', onPortDragEnd)
}

function onPortDragMove(e: MouseEvent) {
  if (!portDragData || !svgContainer.value) return
  const rect = svgContainer.value.getBoundingClientRect()
  const mx = (e.clientX - rect.left - canvas.panX.value) / canvas.zoom.value
  const my = (e.clientY - rect.top - canvas.panY.value) / canvas.zoom.value
  canvas.updateDraggingEdge(mx, my)
}

function onPortDragEnd(e: MouseEvent) {
  if (portDragData) {
    // 检查是否落在某个节点的输入端
    const target = findPortTarget(e.clientX, e.clientY)
    if (target && target.stepId !== portDragData.stepId) {
      canvas.finishDraggingEdge(target.stepId, target.portLabel)
      emit('add-edge', portDragData.stepId, target.stepId)
    } else {
      canvas.cancelDraggingEdge()
    }
  }
  portDragData = null
  window.removeEventListener('mousemove', onPortDragMove)
  window.removeEventListener('mouseup', onPortDragEnd)
}

function findPortTarget(clientX: number, clientY: number): { stepId: string; portLabel: string } | null {
  const els = document.querySelectorAll('[data-port-target]')
  for (const el of els) {
    const rect = el.getBoundingClientRect()
    if (clientX >= rect.left && clientX <= rect.right && clientY >= rect.top && clientY <= rect.bottom) {
      return {
        stepId: el.getAttribute('data-step-id') || '',
        portLabel: el.getAttribute('data-port-label') || 'in',
      }
    }
  }
  return null
}

// ─── 画布平移 ───
let isPanning = false
let panStartX = 0
let panStartY = 0

function onCanvasMouseDown(e: MouseEvent) {
  if (e.target === e.currentTarget || (e.target as HTMLElement)?.closest('.canvas-bg')) {
    isPanning = true
    panStartX = e.clientX - canvas.panX.value
    panStartY = e.clientY - canvas.panY.value
    canvas.selectNode(null)
    window.addEventListener('mousemove', onCanvasMouseMove)
    window.addEventListener('mouseup', onCanvasMouseUp)
  }
}

function onCanvasMouseMove(e: MouseEvent) {
  if (!isPanning) return
  canvas.setPan(e.clientX - panStartX, e.clientY - panStartY)
}

function onCanvasMouseUp() {
  isPanning = false
  window.removeEventListener('mousemove', onCanvasMouseMove)
  window.removeEventListener('mouseup', onCanvasMouseUp)
}

// ─── 滚轮缩放 ───
function onCanvasWheel(e: WheelEvent) {
  e.preventDefault()
  const delta = e.deltaY > 0 ? -0.1 : 0.1
  canvas.setZoom(canvas.zoom.value + delta)
}

// ─── 连线数据 ───
interface EdgeLine {
  fromId: string
  toId: string
  from: { x: number; y: number }
  to: { x: number; y: number }
}

const edgeLines = computed<EdgeLine[]>(() => {
  const positions = canvas.nodePositions.value
  return (edgeRef.value || []).map((edge) => {
    const fromPos = positions.get(edge.from)
    const toPos = positions.get(edge.to)
    if (!fromPos || !toPos) return null
    return {
      fromId: edge.from,
      toId: edge.to,
      from: {
        x: fromPos.x + canvas.nodeWidth / 2,
        y: fromPos.y + canvas.nodeHeight / 2,
      },
      to: {
        x: toPos.x,
        y: toPos.y + canvas.nodeHeight / 2,
      },
    }
  }).filter((x): x is EdgeLine => x !== null)
})

// ─── 视图控制 ───
const zoomPercent = computed(() => Math.round(canvas.zoom.value * 100))

function zoomIn() { canvas.setZoom(canvas.zoom.value + 0.1) }
function zoomOut() { canvas.setZoom(canvas.zoom.value - 0.1) }
function resetView() { canvas.resetView() }
</script>

<template>
  <div class="flex flex-col h-full bg-background">
    <!-- 工具栏 -->
    <div class="flex items-center gap-2 px-3 py-2 border-b shrink-0">
      <Button variant="ghost" size="icon" @click="zoomIn">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/><line x1="11" y1="8" x2="11" y2="14"/><line x1="8" y1="11" x2="14" y2="11"/></svg>
      </Button>
      <Button variant="ghost" size="icon" @click="zoomOut">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/><line x1="8" y1="11" x2="14" y2="11"/></svg>
      </Button>
      <span class="text-xs text-muted-foreground min-w-[3rem] text-center">{{ zoomPercent }}%</span>
      <Button variant="ghost" size="icon" @click="resetView">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M15 3h6v6"/><path d="M9 21H3v-6"/><path d="M21 3l-7 7"/><path d="M3 21l7-7"/></svg>
      </Button>
      <div class="flex-1" />
      <span class="text-xs text-muted-foreground">{{ t('editor.stepsCount', { count: stepRef.length }) }} · {{ t('editor.edgesCount', { count: edgeRef.length }) }}</span>
    </div>

    <!-- 画布 -->
    <div
      ref="svgContainer"
      class="flex-1 overflow-hidden relative cursor-grab active:cursor-grabbing"
      @mousedown="onCanvasMouseDown"
      @wheel.prevent="onCanvasWheel"
      @keydown="onKeyDown"
      tabindex="0"
    >
      <!-- SVG 连线层 -->
      <svg
        class="absolute inset-0 w-full h-full pointer-events-none"
        :style="{
          transform: `translate(${canvas.panX.value}px, ${canvas.panY.value}px)`,
        }"
      >
        <defs>
          <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto">
            <polygon points="0 0, 10 3.5, 0 7" fill="var(--border)" />
          </marker>
        </defs>
        <g :style="{ transform: `scale(${canvas.zoom.value})`, transformOrigin: '0 0' }">
          <!-- 已连接的连线 -->
          <template v-for="(line, i) in edgeLines" :key="'edge-' + i">
            <CanvasEdge
              v-if="line"
              :from="line.from"
              :to="line.to"
              :selected="selectedEdgeIdx === i"
              @select="selectedEdgeIdx = i"
              @remove="emit('remove-edge', line.fromId, line.toId)"
            />
          </template>

          <!-- 拖拽中的临时连线 -->
          <template v-if="canvas.draggingEdge.value">
            <line
              v-if="canvas.draggingEdge.value"
              :x1="0" :y1="0"
              :x2="(canvas.draggingEdge.value.mouseX || 0)"
              :y2="(canvas.draggingEdge.value.mouseY || 0)"
              stroke="var(--primary)"
              stroke-width="2"
              stroke-dasharray="5,5"
            />
          </template>
        </g>
      </svg>

      <!-- 节点层 -->
      <div
        class="absolute inset-0"
        :style="{
          transform: `translate(${canvas.panX.value}px, ${canvas.panY.value}px) scale(${canvas.zoom.value})`,
          transformOrigin: '0 0',
        }"
      >
        <CanvasNode
          v-for="step in stepRef"
          :key="step.id"
          :step="step"
          :position="canvas.getPosition(step.id)"
          :node-width="canvas.nodeWidth"
          :node-height="canvas.nodeHeight"
          :selected="canvas.selectedNode.value === step.id"
          :run-state="runStates[step.id]"
          @start-drag="(id, e) => onNodeDragStart(id, e)"
          @select="canvas.selectNode($event)"
          @start-edge="(id, port, e) => onPortDragStart(id, port, e)"
        />
      </div>

      <!-- 空状态 -->
      <div v-if="!stepRef.length" class="absolute inset-0 flex items-center justify-center text-muted-foreground text-sm pointer-events-none">
        {{ t('editor.canvasEmpty') }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.canvas-bg {
  cursor: grab;
}
.canvas-bg:active {
  cursor: grabbing;
}
</style>
