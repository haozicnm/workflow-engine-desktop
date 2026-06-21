<script setup lang="ts">
import { computed } from 'vue'
import type { Step } from '../types/types'

const props = defineProps<{
  steps: Step[]
  positions: Map<string, { x: number; y: number }>
  nodeWidth: number
  nodeHeight: number
  canvasWidth: number
  canvasHeight: number
  panX: number
  panY: number
  zoom: number
}>()

const emit = defineEmits<{
  'navigate': [x: number, y: number]
}>()

// 迷你地图尺寸
const MAP_W = 160
const MAP_H = 100

// 计算所有节点的边界框
const bounds = computed(() => {
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
  for (const [, pos] of props.positions) {
    minX = Math.min(minX, pos.x)
    minY = Math.min(minY, pos.y)
    maxX = Math.max(maxX, pos.x + props.nodeWidth)
    maxY = Math.max(maxY, pos.y + props.nodeHeight)
  }
  if (minX === Infinity) return { minX: 0, minY: 0, maxX: 100, maxY: 100 }
  // 添加 padding
  const pad = 50
  return { minX: minX - pad, minY: minY - pad, maxX: maxX + pad, maxY: maxY + pad }
})

const scaleX = computed(() => MAP_W / (bounds.value.maxX - bounds.value.minX))
const scaleY = computed(() => MAP_H / (bounds.value.maxY - bounds.value.minY))
const scale = computed(() => Math.min(scaleX.value, scaleY.value))

// 当前视口矩形
const viewportRect = computed(() => {
  const x = (-props.panX / props.zoom - bounds.value.minX) * scale.value
  const y = (-props.panY / props.zoom - bounds.value.minY) * scale.value
  const w = (props.canvasWidth / props.zoom) * scale.value
  const h = (props.canvasHeight / props.zoom) * scale.value
  return { x, y, w, h }
})

function onMapClick(e: MouseEvent) {
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
  const clickX = e.clientX - rect.left
  const clickY = e.clientY - rect.top
  // 将点击位置转换回画布坐标
  const canvasX = clickX / scale.value + bounds.value.minX
  const canvasY = clickY / scale.value + bounds.value.minY
  emit('navigate', canvasX, canvasY)
}
</script>

<template>
  <div
    class="absolute bottom-3 left-3 w-[160px] h-[100px] bg-background/80 backdrop-blur border border-border rounded-lg overflow-hidden cursor-crosshair z-10"
    @click="onMapClick"
  >
    <svg width="160" height="100" class="pointer-events-none">
      <!-- 节点点 -->
      <rect
        v-for="step in steps"
        :key="step.id"
        :x="(positions.get(step.id)?.x || 0 - bounds.minX) * scale"
        :y="(positions.get(step.id)?.y || 0 - bounds.minY) * scale"
        :width="nodeWidth * scale"
        :height="nodeHeight * scale"
        rx="1"
        class="fill-primary/30 stroke-primary/50"
        stroke-width="0.5"
      />
      <!-- 当前视口 -->
      <rect
        :x="viewportRect.x"
        :y="viewportRect.y"
        :width="viewportRect.w"
        :height="viewportRect.h"
        class="fill-primary/10 stroke-primary"
        stroke-width="1"
        rx="1"
      />
    </svg>
  </div>
</template>
