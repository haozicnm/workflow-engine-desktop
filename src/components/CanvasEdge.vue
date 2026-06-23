<script setup lang="ts">
import { computed, ref } from 'vue'

// 数据流动画 CSS（全局注入一次）
if (!document.getElementById('edge-flow-animation')) {
  const style = document.createElement('style')
  style.id = 'edge-flow-animation'
  style.textContent = `
    @keyframes edge-flow { to { stroke-dashoffset: -20; } }
    .animate-flow { animation: edge-flow 0.8s linear infinite; }
  `
  document.head.appendChild(style)
}

const props = defineProps<{
  from: { x: number; y: number }
  to: { x: number; y: number }
  fromPort?: string
  toPort?: string
  selected?: boolean
  active?: boolean
}>()

const emit = defineEmits<{
  'remove': []
  'select': []
}>()

const isHovered = ref(false)

// 条件边颜色：true=绿色，false=红色
const edgeColor = computed(() => {
  if (props.selected) return 'var(--primary)'
  if (isHovered.value) return 'var(--primary)'
  if (props.fromPort === 'true') return 'var(--success, #22c55e)'
  if (props.fromPort === 'false') return 'var(--danger, #ef4444)'
  return 'var(--border)'
})

// Bezier curve: offset control points horizontally
const d = computed(() => {
  const dx = Math.abs(props.to.x - props.from.x)
  const offset = Math.max(50, dx * 0.4)
  const cx1 = props.from.x + offset
  const cy1 = props.from.y
  const cx2 = props.to.x - offset
  const cy2 = props.to.y
  return `M ${props.from.x},${props.from.y} C ${cx1},${cy1} ${cx2},${cy2} ${props.to.x},${props.to.y}`
})

// Arrow marker at the end
const arrowPath = computed(() => {
  const dx = props.to.x - props.from.x
  const dy = props.to.y - props.from.y
  const len = Math.sqrt(dx * dx + dy * dy)
  if (len === 0) return ''
  const ux = dx / len
  const uy = dy / len
  const size = 8
  // Position the arrow slightly before the end point
  const tipX = props.to.x - ux * 7
  const tipY = props.to.y - uy * 7
  const leftX = tipX - ux * size + uy * size * 0.5
  const leftY = tipY - uy * size - ux * size * 0.5
  const rightX = tipX - ux * size - uy * size * 0.5
  const rightY = tipY - uy * size + ux * size * 0.5
  return `M ${tipX},${tipY} L ${leftX},${leftY} L ${rightX},${rightY} Z`
})
</script>

<template>
  <g
    class="canvas-edge cursor-pointer"
    @mouseenter="isHovered = true"
    @mouseleave="isHovered = false"
  >
    <!-- Invisible wider hit area -->
    <path
      :d
      fill="none"
      stroke="transparent"
      stroke-width="12"
      class="cursor-pointer"
      @click.stop="emit('select')"
    />
    <!-- Visible line -->
    <path
      :d
      fill="none"
      :stroke="edgeColor"
      :stroke-width="props.selected ? 3 : 2"
      class="transition-colors"
    />
    <!-- Data flow animation (active edges) -->
    <path
      v-if="props.active"
      :d
      fill="none"
      :stroke="props.fromPort === 'false' ? 'var(--danger, #ef4444)' : 'var(--primary)'"
      stroke-width="3"
      stroke-dasharray="8 12"
      stroke-linecap="round"
      class="animate-flow opacity-70"
    />
    <!-- Arrow head -->
    <path
      :d="arrowPath"
      :fill="isHovered ? 'var(--primary)' : 'var(--border)'"
      class="transition-colors"
    />
    <!-- Delete button (visible on hover) -->
    <g v-if="isHovered" transform="translate(0, -16)" class="cursor-pointer">
      <circle
        :cx="(from.x + to.x) / 2"
        :cy="(from.y + to.y) / 2"
        r="10"
        class="fill-destructive stroke-destructive-foreground"
      />
      <text
        :x="(from.x + to.x) / 2"
        :y="(from.y + to.y) / 2 + 4"
        text-anchor="middle"
        class="fill-destructive-foreground text-[12px] font-bold pointer-events-none select-none"
        @click.stop="emit('remove')"
      >✕</text>
    </g>
  </g>
</template>
