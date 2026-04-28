<template>
  <div
    class="comfy-node"
    :class="[`status-${status}`, { selected }]"
    :style="{ borderColor: color }"
  >
    <!-- 状态指示灯 + 标题栏 -->
    <div class="node-header" :style="{ backgroundColor: color + '18' }">
      <span class="status-dot" :style="{ backgroundColor: statusDotColor }"></span>
      <span class="node-icon">{{ icon }}</span>
      <span class="node-label">{{ label }}</span>
      <span v-if="duration !== undefined" class="node-duration">{{ duration }}ms</span>
    </div>

    <!-- 输入针脚 (左侧) -->
    <div v-if="inputs.length" class="pins-section inputs">
      <div v-for="pin in inputs" :key="pin.id" class="pin-row input-pin">
        <Handle
          type="target"
          :id="pin.id"
          :position="Position.Left"
          :style="{ backgroundColor: pinColor(pin.type) }"
        />
        <span class="pin-label">{{ pin.label }}</span>
        <span class="pin-type-badge" :style="{ color: pinColor(pin.type) }">
          {{ pinTypeBadge(pin.type) }}
        </span>
      </div>
    </div>

    <!-- 参数区（可选） -->
    <div v-if="hasParams" class="node-params">
      <slot name="params" />
    </div>

    <!-- 输出针脚 (右侧) -->
    <div v-if="outputs.length" class="pins-section outputs">
      <div
        v-for="pin in outputs"
        :key="pin.id"
        class="pin-row output-pin"
        @mouseenter="$emit('preview', pin.id)"
        @mouseleave="$emit('preview-hide')"
      >
        <span class="pin-type-badge" :style="{ color: pinColor(pin.type) }">
          {{ pinTypeBadge(pin.type) }}
        </span>
        <span class="pin-label">{{ pin.label }}</span>
        <Handle
          type="source"
          :id="pin.id"
          :position="Position.Right"
          :style="{ backgroundColor: pinColor(pin.type) }"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Handle, Position } from '@vue-flow/core'
import type { PinDefinition, DAGNodeStatus } from '../../types/dag-node'
import { PIN_TYPE_COLORS, STATUS_COLORS } from '../../types/dag-node'

const props = defineProps<{
  id: string
  label: string
  icon: string
  color: string
  status: DAGNodeStatus
  inputs: PinDefinition[]
  outputs: PinDefinition[]
  hasParams: boolean
  selected: boolean
  duration?: number
}>()

const statusDotColor = computed(() => STATUS_COLORS[props.status])

function pinColor(type: string): string {
  return (PIN_TYPE_COLORS as Record<string, string>)[type] || '#9CA3AF'
}

function pinTypeBadge(type: string): string {
  const map: Record<string, string> = {
    string: 'Aa', number: '#', boolean: '◉', object: '{}',
    array: '[]', file: '📄', any: '*', error: '!',
  }
  return map[type] || type
}
</script>

<style scoped>
.comfy-node {
  width: 260px;
  background: #1e1e2e;
  border: 2px solid #3B82F6;
  border-radius: 8px;
  font-size: 13px;
  color: #cdd6f4;
  overflow: hidden;
  transition: box-shadow 0.2s;
}
.comfy-node.selected {
  box-shadow: 0 0 0 2px #89b4fa;
}
.comfy-node.status-running {
  animation: breathe 1.5s ease-in-out infinite;
}
@keyframes breathe {
  0%, 100% { box-shadow: 0 0 4px rgba(59, 130, 246, 0.3); }
  50% { box-shadow: 0 0 14px rgba(59, 130, 246, 0.6); }
}

.node-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  font-weight: 600;
  font-size: 12px;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.node-duration {
  margin-left: auto;
  font-size: 11px;
  opacity: 0.6;
}

.pins-section {
  padding: 4px 0;
}

.pin-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 2px 8px;
  font-size: 11px;
  position: relative;
}

.input-pin {
  padding-left: 2px;
}

.output-pin {
  justify-content: flex-end;
  padding-right: 2px;
}

.pin-label {
  opacity: 0.8;
}

.pin-type-badge {
  font-family: monospace;
  font-size: 10px;
  font-weight: 600;
  min-width: 16px;
  text-align: center;
}

.node-params {
  padding: 6px 10px;
  border-top: 1px solid #313244;
}
</style>
