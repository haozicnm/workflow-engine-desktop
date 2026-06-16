<script setup lang="ts">
import { computed } from 'vue'
import type { Step, StepRunState, NodePosition } from '../types/types'
import ActionIcon from './ActionIcon.vue'

const props = defineProps<{
  step: Step
  position: NodePosition
  runState?: StepRunState
  selected: boolean
  nodeWidth: number
  nodeHeight: number
}>()

const emit = defineEmits<{
  'select': [id: string]
  'start-drag': [id: string, e: MouseEvent]
  'start-edge': [id: string, port: string, e: MouseEvent]
}>()

const statusColor = computed(() => {
  if (!props.runState) return 'border-border bg-card'
  switch (props.runState.status) {
    case 'running': return 'border-blue-500 bg-blue-50 dark:bg-blue-950'
    case 'success': return 'border-green-500 bg-green-50 dark:bg-green-950'
    case 'error': return 'border-red-500 bg-red-50 dark:bg-red-950'
    default: return 'border-border bg-card'
  }
})

const statusDot = computed(() => {
  if (!props.runState) return ''
  switch (props.runState.status) {
    case 'running': return 'bg-blue-500 animate-pulse'
    case 'success': return 'bg-green-500'
    case 'error': return 'bg-red-500'
    default: return 'bg-gray-300'
  }
})

function onMouseDown(e: MouseEvent) {
  if (e.button !== 0) return
  emit('select', props.step.id)
  emit('start-drag', props.step.id, e)
}

function onPortMouseDown(port: string, e: MouseEvent) {
  e.stopPropagation()
  emit('start-edge', props.step.id, port, e)
}
</script>

<template>
  <g
    class="canvas-node cursor-grab active:cursor-grabbing"
    :class="{ 'z-10': selected }"
    @mousedown="onMouseDown"
  >
    <!-- Selection highlight -->
    <rect
      :x="position.x - 4"
      :y="position.y - 4"
      :width="nodeWidth + 8"
      :height="nodeHeight + 8"
      rx="10"
      ry="10"
      :class="selected ? 'fill-blue-500/10 stroke-blue-500' : 'fill-transparent stroke-transparent'"
      :stroke-width="selected ? 2 : 0"
    />

    <!-- Node card body -->
    <foreignObject
      :x="position.x"
      :y="position.y"
      :width="nodeWidth"
      :height="nodeHeight"
    >
      <div
        class="h-full rounded-lg border-2 flex flex-col overflow-hidden text-xs transition-colors select-none"
        :class="statusColor"
      >
        <!-- Header -->
        <div class="flex items-center gap-1.5 px-2.5 py-1.5 border-b border-border/50 bg-muted/30">
          <span v-if="statusDot" class="w-2 h-2 rounded-full shrink-0" :class="statusDot" />
          <ActionIcon :name="step.type" cls="w-3.5 h-3.5 shrink-0" />
          <span class="font-medium truncate flex-1">{{ step.label || step.id }}</span>
          <span class="text-[10px] text-muted-foreground truncate max-w-[60px]">{{ step.type }}</span>
        </div>
        <!-- Body -->
        <div class="flex-1 px-2.5 py-1 text-muted-foreground truncate">
          {{ step.actions?.length ? `${step.actions.length} actions` : '' }}
        </div>
      </div>
    </foreignObject>

    <!-- Input port (left side) -->
    <circle
      :cx="position.x"
      :cy="position.y + nodeHeight / 2"
      r="5"
      class="fill-background stroke-muted-foreground hover:stroke-blue-500 hover:fill-blue-100 cursor-crosshair transition-colors"
      stroke-width="2"
      @mousedown="onPortMouseDown('in', $event)"
    />

    <!-- Output port (right side) -->
    <circle
      :cx="position.x + nodeWidth"
      :cy="position.y + nodeHeight / 2"
      r="5"
      class="fill-background stroke-muted-foreground hover:stroke-blue-500 hover:fill-blue-100 cursor-crosshair transition-colors"
      stroke-width="2"
      @mousedown="onPortMouseDown('out', $event)"
    />
  </g>
</template>
