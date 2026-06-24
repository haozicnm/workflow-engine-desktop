<script setup lang="ts">
import { computed } from 'vue'
import type { Step, StepRunState, NodePosition } from '../types/types'
import ActionIcon from './ActionIcon.vue'
import { getContainerDef } from '../types/node-registry'

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
    case 'running': return 'border-info bg-info/10'
    case 'success': return 'border-success bg-success/10'
    case 'error': return 'border-danger bg-danger/10'
    default: return 'border-border bg-card'
  }
})

const statusDot = computed(() => {
  if (!props.runState) return ''
  switch (props.runState.status) {
    case 'running': return 'bg-info animate-pulse'
    case 'success': return 'bg-success'
    case 'error': return 'bg-danger'
    default: return 'bg-muted-foreground'
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

// 动态多端口：从 schema 获取输出端口定义
const outputPorts = computed(() => {
  if (props.step.type === 'condition') return null // condition 用硬编码 true/false
  try {
    const def = getContainerDef(props.step.type)
    const outputs = (def as unknown as Record<string, unknown>)?.outputs as Array<Record<string, string>> | undefined
    if (outputs && outputs.length > 1) {
      return outputs
    }
  } catch { /* 非容器节点，无输出端口定义 */ }
  return null
})
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
      :class="selected ? 'fill-primary/10 stroke-primary' : 'fill-transparent stroke-transparent'"
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
        class="h-full rounded-md border-2 flex flex-col overflow-hidden text-xs transition-colors select-none"
        :class="statusColor"
      >
        <!-- Header -->
        <div class="flex items-center gap-1.5 px-2.5 py-1.5 border-b border-border/50 bg-muted/30">
          <span v-if="statusDot" class="w-2 h-2 rounded-full shrink-0" :class="statusDot" />
          <ActionIcon :name="step.type" cls="w-3.5 h-3.5 shrink-0" />
          <span class="font-medium truncate flex-1">{{ step.label || step.id }}</span>
          <span class="text-[10px] text-muted-foreground truncate max-w-[60px]">{{ step.type }}</span>
          <!-- 断点标记 -->
          <span v-if="step.breakpoint" class="w-2 h-2 rounded-full bg-red-500 shrink-0 animate-pulse" title="断点" />
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
      class="fill-hairline-strong stroke-hairline-strong hover:stroke-primary hover:fill-primary/20 cursor-crosshair transition-colors"
      stroke-width="2"
      data-port-target
      :data-step-id="step.id"
      data-port-label="in"
      @mousedown="onPortMouseDown('in', $event)"
    />

    <!-- Output port(s) (right side) -->
    <template v-if="step.type === 'condition'">
      <!-- True port (green, upper) -->
      <circle
        :cx="position.x + nodeWidth"
        :cy="position.y + nodeHeight * 0.3"
        r="5"
        class="fill-success/80 stroke-success hover:fill-success cursor-crosshair transition-colors"
        stroke-width="2"
        @mousedown="onPortMouseDown('true', $event)"
      />
      <text
        :x="position.x + nodeWidth + 10"
        :y="position.y + nodeHeight * 0.3 + 4"
        class="fill-success text-[10px] select-none pointer-events-none"
      >✓</text>
      <!-- False port (red, lower) -->
      <circle
        :cx="position.x + nodeWidth"
        :cy="position.y + nodeHeight * 0.7"
        r="5"
        class="fill-danger/80 stroke-danger hover:fill-danger cursor-crosshair transition-colors"
        stroke-width="2"
        @mousedown="onPortMouseDown('false', $event)"
      />
      <text
        :x="position.x + nodeWidth + 10"
        :y="position.y + nodeHeight * 0.7 + 4"
        class="fill-danger text-[10px] select-none pointer-events-none"
      >✗</text>
    </template>
    <template v-else-if="outputPorts">
      <!-- Dynamic multi-port (from schema) -->
      <g v-for="(port, idx) in outputPorts" :key="port.name || port.label">
        <circle
          :cx="position.x + nodeWidth"
          :cy="position.y + nodeHeight * (idx + 1) / (outputPorts.length + 1)"
          r="5"
          class="fill-hairline-strong stroke-hairline-strong hover:stroke-primary hover:fill-primary/20 cursor-crosshair transition-colors"
          stroke-width="2"
          data-port-target
          :data-step-id="step.id"
          :data-port-label="port.name || port.label"
          @mousedown="onPortMouseDown(port.name || port.label, $event)"
        />
        <text
          :x="position.x + nodeWidth + 8"
          :y="position.y + nodeHeight * (idx + 1) / (outputPorts.length + 1) + 4"
          class="fill-muted-foreground text-[9px] select-none pointer-events-none"
        >{{ port.name || port.label }}</text>
      </g>
    </template>
    <template v-else>
      <!-- Single output port -->
      <circle
        :cx="position.x + nodeWidth"
        :cy="position.y + nodeHeight / 2"
        r="5"
        class="fill-hairline-strong stroke-hairline-strong hover:stroke-primary hover:fill-primary/20 cursor-crosshair transition-colors"
        stroke-width="2"
        data-port-target
        :data-step-id="step.id"
        data-port-label="out"
        @mousedown="onPortMouseDown('out', $event)"
      />
    </template>
  </g>
</template>
