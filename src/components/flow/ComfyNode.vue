<template>
  <div
    class="comfy-node"
    :class="[`status-${status}`, { selected }]"
  >
    <!-- 标题栏 -->
    <div class="node-header">
      <span class="status-dot" :style="{ backgroundColor: statusDotColor }"></span>
      <span class="node-icon">{{ icon }}</span>
      <span class="node-label">{{ label }}</span>
      <span v-if="duration !== undefined" class="node-duration">{{ duration }}ms</span>
    </div>

    <!-- 迷你参数行（紧凑模式） -->
    <div v-if="miniParamText" class="node-mini-params">
      <span class="mini-param">{{ miniParamText }}</span>
    </div>

    <div class="node-body">
      <!-- 输入针脚（左侧） -->
      <div v-if="inputs.length" class="pins inputs">
        <div v-for="pin in inputs" :key="pin.id" class="pin-row input-pin">
          <Handle
            type="target"
            :id="pin.id"
            :position="Position.Left"
            class="pin-handle"
            :style="{ backgroundColor: getPinColor(pin.type) }"
          />
          <span class="pin-label">{{ pin.label }}</span>
          <span class="pin-type" :style="{ color: getPinColor(pin.type) }">
            {{ getPinBadge(pin.type) }}
          </span>
        </div>
      </div>

      <!-- 参数区域 -->
      <div v-if="hasParams" class="node-params">
        <slot name="params" />
      </div>

      <!-- 输出针脚（右侧） -->
      <div v-if="outputs.length" class="pins outputs">
        <div
          v-for="pin in outputs"
          :key="pin.id"
          class="pin-row output-pin"
          @mouseenter="onPinHover($event, pin.id)"
          @mouseleave="onPinLeave"
        >
          <span class="pin-type" :style="{ color: getPinColor(pin.type) }">
            {{ getPinBadge(pin.type) }}
          </span>
          <span class="pin-label">{{ pin.label }}</span>
          <Handle
            type="source"
            :id="pin.id"
            :position="Position.Right"
            class="pin-handle"
            :style="{ backgroundColor: getPinColor(pin.type) }"
          />
        </div>
      </div>
    </div>

    <!-- 错误信息 -->
    <div v-if="error" class="node-error">
      <span class="error-icon">⚠</span>
      <span class="error-text">{{ error }}</span>
    </div>

    <!-- 数据预览气泡 -->
    <Teleport to="body">
      <div
        v-if="tooltip.visible"
        class="pin-tooltip"
        :style="tooltipStyle"
      >
        <div class="tooltip-header">{{ tooltip.pinLabel }}</div>
        <pre class="tooltip-data">{{ tooltip.text }}</pre>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, reactive } from 'vue'
import { Handle, Position } from '@vue-flow/core'
import type { PinDefinition, NodeStatus } from './pinTypes'
import { STATUS_COLORS, pinColor, pinBadge } from './pinTypes'

const props = defineProps<{
  id: string
  label: string
  icon: string
  status: NodeStatus
  inputs: PinDefinition[]
  outputs: PinDefinition[]
  hasParams: boolean
  selected: boolean
  duration?: number
  error?: string
  type?: string
  config?: Record<string, unknown>
  stepOutput?: unknown
}>()

const statusDotColor = computed(() => STATUS_COLORS[props.status] || '#484f58')

function getPinColor(type: string): string {
  return pinColor(type)
}

function getPinBadge(type: string): string {
  return pinBadge(type)
}

// ─── 迷你参数行（紧凑模式） ───
const miniParamText = computed(() => {
  const cfg = props.config
  const t = props.type
  if (!cfg || !t) return ''

  switch (t) {
    case 'http': {
      const method = String(cfg.method || 'GET')
      const url = String(cfg.url || '').substring(0, 40)
      return `${method} ${url}${cfg.url && String(cfg.url).length > 40 ? '…' : ''}`
    }
    case 'data': {
      const action = String(cfg.action || '')
      const key = cfg.key ? String(cfg.key) : ''
      const parts = [action]
      if (key) parts.push(key)
      return parts.filter(Boolean).join(' → ')
    }
    case 'script': {
      const script = String(cfg.script || '')
      if (!script) return ''
      return script.length > 40 ? script.substring(0, 40) + '…' : script
    }
    default:
      return ''
  }
})

// ─── 数据预览气泡 ───
const tooltip = reactive({
  visible: false,
  text: '',
  pinLabel: '',
  x: 0,
  y: 0,
})

const tooltipStyle = computed(() => ({
  left: `${tooltip.x}px`,
  top: `${tooltip.y}px`,
}))

function formatTooltipData(data: unknown): string {
  if (data === undefined || data === null) return '(无数据)'
  try {
    const s = typeof data === 'string' ? data : JSON.stringify(data, null, 2)
    return s.length > 200 ? s.substring(0, 200) + '…' : s
  } catch {
    return String(data).substring(0, 200)
  }
}

function onPinHover(event: MouseEvent, pinId: string) {
  const pin = props.outputs.find(p => p.id === pinId)
  if (!pin) return

  const data = props.stepOutput
  if (data === undefined) return

  tooltip.pinLabel = pin.label
  tooltip.text = formatTooltipData(data)
  tooltip.visible = true

  // 定位在鼠标右下方偏移
  tooltip.x = event.clientX + 12
  tooltip.y = event.clientY + 8
}

function onPinLeave() {
  tooltip.visible = false
}
</script>

<style scoped>
.comfy-node {
  width: 240px;
  min-width: 240px;
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 6px;
  font-size: 13px;
  color: #c9d1d9;
  overflow: visible;
  box-shadow: 0 1px 3px rgba(0,0,0,0.3);
  transition: box-shadow 0.15s, border-color 0.15s;
}

.comfy-node.selected {
  border-color: #58a6ff;
  box-shadow: 0 0 0 2px rgba(88, 166, 255, 0.3);
}

/* 执行中呼吸动画 */
.comfy-node.status-running {
  border-color: #58a6ff;
  animation: breathe 2s ease-in-out infinite;
}

@keyframes breathe {
  0%, 100% {
    box-shadow: 0 0 0 0 rgba(88, 166, 255, 0.4);
  }
  50% {
    box-shadow: 0 0 0 4px rgba(88, 166, 255, 0.1);
  }
}

/* 标题栏 */
.node-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 10px;
  background: rgba(255,255,255,0.03);
  border-bottom: 1px solid #21262d;
  font-weight: 600;
  font-size: 12px;
}

.status-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}

/* 执行中脉冲 */
.status-running .status-dot {
  animation: dot-pulse 1s ease-in-out infinite;
}

@keyframes dot-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}

.status-running .node-header {
  background: rgba(88, 166, 255, 0.06);
}

.node-icon {
  font-size: 14px;
  flex-shrink: 0;
}

.node-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.node-duration {
  margin-left: auto;
  font-size: 10px;
  color: #8b949e;
  font-weight: 400;
}

/* ─── 迷你参数行 ─── */
.node-mini-params {
  padding: 2px 10px;
  background: #0d1117;
  border-bottom: 1px solid #21262d;
}

.mini-param {
  font-family: 'SF Mono', 'Cascadia Code', monospace;
  font-size: 9px;
  color: #8b949e;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  display: block;
}

/* 节点主体 */
.node-body {
  /* flexible layout */
}

/* 针脚区域 */
.pins {
  padding: 2px 0;
}

.pin-row {
  display: flex;
  align-items: center;
  gap: 5px;
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
  color: #8b949e;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.input-pin .pin-label {
  flex: 1;
}

.output-pin .pin-label {
  flex: 1;
  text-align: right;
}

.pin-type {
  font-family: 'SF Mono', 'Cascadia Code', monospace;
  font-size: 10px;
  font-weight: 600;
  min-width: 14px;
  text-align: center;
  flex-shrink: 0;
}

.pin-handle {
  width: 8px !important;
  height: 8px !important;
  border: 2px solid #161b22 !important;
  border-radius: 50% !important;
  z-index: 10;
}

/* 参数区域 */
.node-params {
  padding: 4px 10px 6px;
  border-top: 1px solid #21262d;
  font-size: 12px;
}

/* 错误区域 */
.node-error {
  display: flex;
  align-items: flex-start;
  gap: 4px;
  padding: 4px 10px;
  background: rgba(248, 81, 73, 0.1);
  border-top: 1px solid rgba(248, 81, 73, 0.2);
  font-size: 11px;
  color: #f85149;
}

.error-icon {
  flex-shrink: 0;
}

.error-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 移除 Vue Flow 默认连接线外观覆盖 */
:deep(.vue-flow__handle) {
  background: transparent !important;
  border: none !important;
}
</style>

<!-- 非 scoped 样式：数据预览气泡（Teleport 到 body） -->
<style>
.pin-tooltip {
  position: fixed;
  z-index: 9999;
  background: #1a1e24;
  border: 1px solid #30363d;
  border-radius: 6px;
  padding: 8px 10px;
  max-width: 300px;
  min-width: 140px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  pointer-events: none;
  font-size: 11px;
  color: #c9d1d9;
}

.tooltip-header {
  font-size: 10px;
  font-weight: 600;
  color: #8b949e;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 4px;
}

.tooltip-data {
  font-family: 'SF Mono', 'Cascadia Code', monospace;
  font-size: 11px;
  line-height: 1.4;
  white-space: pre-wrap;
  word-break: break-all;
  color: #c9d1d9;
  margin: 0;
  max-height: 160px;
  overflow-y: auto;
}
</style>
