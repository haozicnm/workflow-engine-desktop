<template>
  <div
    v-if="visible"
    class="minimap-container"
  >
    <canvas
      ref="canvasRef"
      :width="width"
      :height="height"
      class="minimap-canvas"
      @mousedown.prevent="onMouseDown"
      @mousemove.prevent="onMouseMove"
      @mouseup="onMouseUp"
      @mouseleave="onMouseUp"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, computed } from 'vue'
import type { LGraphCanvas, LGraph } from '@comfyorg/litegraph'
import { getNodeDef } from './flow/pinTypes'

const props = defineProps<{
  canvas: LGraphCanvas | null
  graph: LGraph | null
  visible: boolean
}>()

const MINIMAP_W = 200
const MINIMAP_H = 140
const width = MINIMAP_W
const height = MINIMAP_H

const canvasRef = ref<HTMLCanvasElement | null>(null)
let _rafId = 0
let _mounted = false

// ─── 交互状态 ───
let _dragging = false

// ─── 计算所有节点包围盒 ───
function getNodesBounds() {
  if (!props.graph?._nodes?.length) return null
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
  for (const node of props.graph._nodes) {
    if (!node) continue
    const [x, y] = node.pos
    const [w, h] = node.size || [140, 80]
    minX = Math.min(minX, x)
    minY = Math.min(minY, y)
    maxX = Math.max(maxX, x + w)
    maxY = Math.max(maxY, y + h)
  }
  const pad = 40
  return { x: minX - pad, y: minY - pad, w: maxX - minX + pad * 2, h: maxY - minY + pad * 2 }
}

// ─── 绘制循环 ───
function drawMiniMap() {
  if (!_mounted) return
  const c = canvasRef.value
  if (!c) return
  const ctx = c.getContext('2d')
  if (!ctx) return
  if (!props.visible || !props.canvas || !props.graph) {
    _rafId = requestAnimationFrame(drawMiniMap)
    return
  }

  const bounds = getNodesBounds()
  if (!bounds || bounds.w <= 0 || bounds.h <= 0) {
    ctx.clearRect(0, 0, MINIMAP_W, MINIMAP_H)
    _rafId = requestAnimationFrame(drawMiniMap)
    return
  }

  // 清空
  ctx.clearRect(0, 0, MINIMAP_W, MINIMAP_H)

  // 缩放比（保持纵横比）
  const pad = 8
  const drawW = MINIMAP_W - pad * 2
  const drawH = MINIMAP_H - pad * 2
  const scale = Math.min(drawW / bounds.w, drawH / bounds.h)

  const offsetX = pad + (drawW - bounds.w * scale) / 2
  const offsetY = pad + (drawH - bounds.h * scale) / 2

  // 绘制节点
  const nodes = props.graph._nodes || []
  for (const node of nodes) {
    if (!node) continue
    const [nx, ny] = node.pos
    const [nw, nh] = node.size || [140, 80]
    const x = offsetX + (nx - bounds.x) * scale
    const y = offsetY + (ny - bounds.y) * scale
    const w = Math.max(4, nw * scale)
    const h = Math.max(2, nh * scale)

    const def = getNodeDef(node.type)
    const color = def?.color || '#30363d'

    ctx.fillStyle = color
    ctx.fillRect(x, y, w, h)
    ctx.strokeStyle = '#ffffff11'
    ctx.lineWidth = 0.5
    ctx.strokeRect(x, y, w, h)
  }

  // 绘制 viewport 矩形
  const ds = props.canvas.ds
  const vpScale = ds?.scale || 1
  const vpOffset = ds?.offset || [0, 0]
  const vpw = (props.canvas.canvas?.width || MINIMAP_W) / vpScale
  const vph = (props.canvas.canvas?.height || MINIMAP_H) / vpScale
  const vpx = offsetX + (-vpOffset[0] - bounds.x) * scale
  const vpy = offsetY + (-vpOffset[1] - bounds.y) * scale
  const vpW = Math.max(3, vpw * scale)
  const vpH = Math.max(3, vph * scale)

  ctx.strokeStyle = '#58a6ff'
  ctx.lineWidth = 1.5
  ctx.strokeRect(vpx, vpy, vpW, vpH)
  ctx.fillStyle = 'rgba(88, 166, 255, 0.08)'
  ctx.fillRect(vpx, vpy, vpW, vpH)

  _rafId = requestAnimationFrame(drawMiniMap)
}

// ─── 点击跳转 ───
function moveCanvasTo(minimapX: number, minimapY: number) {
  if (!props.canvas) return
  const bounds = getNodesBounds()
  if (!bounds) return

  const pad = 8
  const drawW = MINIMAP_W - pad * 2
  const drawH = MINIMAP_H - pad * 2
  const scale = Math.min(drawW / bounds.w, drawH / bounds.h)
  const offsetX = pad + (drawW - bounds.w * scale) / 2
  const offsetY = pad + (drawH - bounds.h * scale) / 2

  const worldX = (minimapX - offsetX) / scale + bounds.x
  const worldY = (minimapY - offsetY) / scale + bounds.y

  // 跳转画布中心到该位置
  props.canvas.adjustPan(
    -worldX + (props.canvas.canvas?.width || MINIMAP_W) / (props.canvas.ds?.scale || 1) / 2,
    -worldY + (props.canvas.canvas?.height || MINIMAP_H) / (props.canvas.ds?.scale || 1) / 2
  )
}

function onMouseDown(e: MouseEvent) {
  _dragging = true
  const rect = canvasRef.value?.getBoundingClientRect()
  if (!rect) return
  moveCanvasTo(e.clientX - rect.left, e.clientY - rect.top)
}

function onMouseMove(e: MouseEvent) {
  if (!_dragging) return
  const rect = canvasRef.value?.getBoundingClientRect()
  if (!rect) return
  moveCanvasTo(e.clientX - rect.left, e.clientY - rect.top)
}

function onMouseUp() {
  _dragging = false
}

onMounted(() => {
  _mounted = true
  _rafId = requestAnimationFrame(drawMiniMap)
})

onUnmounted(() => {
  _mounted = false
  cancelAnimationFrame(_rafId)
})
</script>

<style scoped>
.minimap-container {
  position: fixed;
  right: 8px;
  bottom: 54px;
  z-index: 1000;
  pointer-events: auto;
}

.minimap-canvas {
  display: block;
  background: #0d1117cc;
  border: 1px solid #30363d;
  border-radius: 6px;
  cursor: crosshair;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
  /* DPI aware: 物理像素 = CSS像素 × dpr */
}
</style>
