<template>
  <aside class="property-panel">
    <!-- 未选中节点 -->
    <div v-if="!lgNode" class="panel-empty">
      <svg width="32" height="32" viewBox="0 0 16 16" fill="currentColor" class="empty-icon">
        <path d="M2 4a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V4zm10 1H4v7h8V5z"/>
      </svg>
      <span>选中节点查看属性</span>
    </div>

    <!-- 选中节点 -->
    <template v-else>
      <div class="panel-header">
        <span class="panel-title">📋 节点属性</span>
        <button class="btn-delete" title="删除节点" @click="$emit('delete')">🗑</button>
      </div>

      <!-- 节点类型信息 -->
      <div class="panel-section">
        <div class="node-info">
          <span class="node-info-icon">{{ nodeDef?.icon || '📦' }}</span>
          <span class="node-info-type">{{ nodeDef?.label || lgNode.type }}</span>
        </div>
      </div>

      <!-- 节点名称 -->
      <div class="panel-section">
        <label class="section-label">名称</label>
        <input
          :value="lgNode.title"
          class="field-input"
          type="text"
          @change="onLabelChange"
        />
      </div>

      <!-- 参数 — 由 LiteGraph widgets 驱动 -->
      <div v-if="widgets.length > 0" class="panel-section">
        <label class="section-label">⚙ 参数</label>

        <div v-for="w in widgets" :key="w.name" class="param-row">
          <label class="param-key">{{ w.label || formatKey(w.name) }}</label>

          <!-- combo → 下拉 -->
          <select
            v-if="w.type === 'combo'"
            :value="w.value"
            class="field-select"
            @change="onWidgetChange(w, ($event.target as HTMLSelectElement).value)"
          >
            <option
              v-for="opt in comboOptions(w)"
              :key="opt"
              :value="opt"
            >{{ opt }}</option>
          </select>

          <!-- toggle → 复选框 -->
          <input
            v-else-if="w.type === 'toggle'"
            type="checkbox"
            :checked="w.value"
            class="field-checkbox"
            @change="onWidgetChange(w, ($event.target as HTMLInputElement).checked)"
          />

          <!-- number → 数字输入 -->
          <input
            v-else-if="w.type === 'number' || w.type === 'slider'"
            type="number"
            :value="w.value"
            :min="w.options?.min"
            :max="w.options?.max"
            :step="w.options?.step2 ?? w.options?.step ?? 1"
            class="field-input"
            @change="onWidgetChange(w, Number(($event.target as HTMLInputElement).value))"
          />

          <!-- text → 多行文本 -->
          <textarea
            v-else-if="w.type === 'text'"
            :value="w.value"
            class="field-textarea"
            rows="3"
            @change="onWidgetChange(w, ($event.target as HTMLTextAreaElement).value)"
          />

          <!-- string / 默认 → 单行输入 -->
          <input
            v-else
            type="text"
            :value="w.value"
            class="field-input"
            :placeholder="widgetPlaceholder(w)"
            @change="onWidgetChange(w, ($event.target as HTMLInputElement).value)"
          />
        </div>
      </div>

      <!-- 针脚信息 -->
      <div class="panel-section">
        <label class="section-label">📥 输入</label>
        <div
          v-for="(pin, idx) in lgNode.inputs || []"
          :key="idx"
          class="pin-info-row"
        >
          <span class="pin-dot" :style="{ background: getPinColor(pin.type || pin.label || 'default') }"></span>
          <span class="pin-info-label">{{ pin.name || pin.label }}</span>
          <span class="pin-info-type">{{ pin.type || 'any' }}</span>
        </div>
        <span v-if="!(lgNode.inputs?.length)" class="no-pins">无输入针脚</span>
      </div>

      <div class="panel-section">
        <label class="section-label">📤 输出</label>
        <div
          v-for="(pin, idx) in (lgNode.outputs || [])"
          :key="idx"
          class="pin-info-row"
        >
          <span class="pin-dot" :style="{ background: getPinColor(pin.type || pin.label || 'default') }"></span>
          <span class="pin-info-label">{{ pin.name || pin.label }}</span>
          <span class="pin-info-type">{{ pin.type || 'any' }}</span>
        </div>
        <span v-if="!(lgNode.outputs?.length)" class="no-pins">无输出针脚</span>
      </div>

      <!-- 输出数据预览 -->
      <div v-if="nodeOutput !== undefined" class="panel-section">
        <label class="section-label">📤 输出预览</label>
        <pre class="data-preview">{{ formatOutput(nodeOutput) }}</pre>
        <div class="data-actions">
          <button class="data-action-btn" @click="copyData(nodeOutput)">📋 复制</button>
          <button class="data-action-btn" @click="expandData = !expandData">
            {{ expandData ? '📉 收起' : '📊 展开' }}
          </button>
        </div>
        <pre v-if="expandData" class="data-preview data-preview-full">{{ formatOutput(nodeOutput) }}</pre>
      </div>

      <!-- 错误信息 -->
      <div v-if="nodeError" class="panel-section panel-error">
        <label class="section-label">⚠ 错误</label>
        <pre class="error-text">{{ nodeError }}</pre>
      </div>

      <!-- 执行元数据 -->
      <div v-if="nodeDuration !== undefined || nodeOutput !== undefined" class="panel-section panel-meta">
        <label class="section-label">📊 执行详情</label>
        <div class="meta-grid">
          <div v-if="nodeDuration !== undefined" class="meta-item">
            <span class="meta-key">耗时</span>
            <span class="meta-value">{{ nodeDuration }}ms</span>
          </div>
          <div v-if="nodeOutput !== undefined" class="meta-item">
            <span class="meta-key">类型</span>
            <span class="meta-value">{{ getDataType(nodeOutput) }}</span>
          </div>
          <div v-if="nodeOutput !== undefined" class="meta-item">
            <span class="meta-key">大小</span>
            <span class="meta-value">{{ getDataSize(nodeOutput) }}</span>
          </div>
        </div>
      </div>
    </template>
  </aside>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import type { LGraphNode, IBaseWidget, IComboWidget } from '@comfyorg/litegraph'
import { getNodeDef, pinColor } from './pinTypes'

const props = defineProps<{
  /** LiteGraph 节点引用（从画布直接传入） */
  lgNode: LGraphNode | null
  /** 节点输出数据（从 store 同步） */
  output?: unknown
  /** 节点错误信息 */
  error?: string
  /** 节点执行耗时 (ms) */
  duration?: number
}>()

const emit = defineEmits<{
  'update-label': [node: LGraphNode, label: string]
  'update-widget': [node: LGraphNode, widgetName: string, value: unknown]
  'delete': [node: LGraphNode]
}>()

const expandData = ref(false)

const nodeDef = computed(() => {
  if (!props.lgNode || !props.lgNode.type) return undefined
  return getNodeDef(props.lgNode.type)
})

const widgets = computed<IBaseWidget[]>(() => {
  return props.lgNode?.widgets ?? []
})

const nodeOutput = computed(() => props.output)
const nodeError = computed(() => props.error)
const nodeDuration = computed(() => props.duration)

// ─── Widget helpers ───

function comboOptions(widget: IBaseWidget): string[] {
  const opts = widget.options?.values
  if (!opts) return []
  if (typeof opts === 'function') return (opts as () => string[])()
  if (Array.isArray(opts)) return opts as string[]
  if (typeof opts === 'object') return Object.keys(opts as Record<string, string>)
  return []
}

function formatKey(name: string): string {
  return name.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase())
}

function widgetPlaceholder(widget: IBaseWidget): string {
  const defaults: Record<string, string> = {
    url: 'https://api.example.com/data',
    path: './output/data.json',
    expression: '$.data.items[*]',
    template: '你好 {{name}}',
    key: 'variable_name',
    value: 'value or expression',
    items: '{{previous.output}}',
  }
  return defaults[widget.name] || ''
}

// ─── 事件处理 ───

function onLabelChange(event: Event) {
  const target = event.target as HTMLInputElement
  if (props.lgNode) {
    emit('update-label', props.lgNode, target.value)
  }
}

function onWidgetChange(widget: IBaseWidget, value: unknown) {
  if (props.lgNode) {
    emit('update-widget', props.lgNode, widget.name, value)
  }
}

// ─── 针脚颜色 ───

function getPinColor(type: string): string {
  return pinColor(type)
}

// ─── 输出预览工具 ───

function formatOutput(data: unknown): string {
  try { return JSON.stringify(data, null, 2) }
  catch { return String(data) }
}

function copyData(data: unknown) {
  const text = typeof data === 'string' ? data : JSON.stringify(data, null, 2)
  navigator.clipboard.writeText(text).catch(() => {})
}

function getDataType(data: unknown): string {
  if (data === null) return 'null'
  if (data === undefined) return 'undefined'
  if (Array.isArray(data)) return `array[${data.length}]`
  if (typeof data === 'object') return `object{${Object.keys(data as object).length}}`
  return typeof data
}

function getDataSize(data: unknown): string {
  const str = typeof data === 'string' ? data : JSON.stringify(data)
  const bytes = new TextEncoder().encode(str).length
  if (bytes < 1024) return `${bytes}B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`
}
</script>

<style scoped>
.property-panel {
  width: 260px; min-width: 260px; height: 100%;
  background: #0d1117; border-left: 1px solid #21262d;
  overflow-y: auto; color: #c9d1d9; font-size: 13px;
}
.panel-empty {
  display: flex; flex-direction: column; align-items: center;
  justify-content: center; gap: 10px; height: 100%;
  color: #484f58; font-size: 13px;
}
.empty-icon { opacity: 0.4; }
.panel-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 10px 12px; border-bottom: 1px solid #21262d;
  position: sticky; top: 0; background: #0d1117; z-index: 1;
}
.panel-title { font-weight: 700; font-size: 13px; }
.btn-delete {
  padding: 2px 6px; background: none; border: 1px solid transparent;
  border-radius: 4px; color: #8b949e; font-size: 12px; cursor: pointer;
}
.btn-delete:hover { background: rgba(248,81,73,0.1); border-color: rgba(248,81,73,0.3); color: #f85149; }
.panel-section { padding: 8px 12px; border-bottom: 1px solid #21262d; }
.section-label {
  display: block; font-size: 11px; font-weight: 600; color: #8b949e;
  text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 6px;
}
.node-info { display: flex; align-items: center; gap: 8px; font-size: 13px; }
.node-info-icon { font-size: 18px; }
.node-info-type { color: #8b949e; }

.param-row { margin-bottom: 6px; }
.param-row:last-child { margin-bottom: 0; }
.param-key {
  display: block; font-size: 11px; color: #8b949e; margin-bottom: 3px;
  text-transform: capitalize;
}
.field-input, .field-select, .field-textarea {
  width: 100%; padding: 5px 8px; background: #161b22;
  border: 1px solid #30363d; border-radius: 4px; color: #c9d1d9;
  font-size: 12px; outline: none; transition: border-color 0.15s;
}
.field-textarea { resize: vertical; font-family: monospace; }
.field-input:focus, .field-select:focus, .field-textarea:focus { border-color: #58a6ff; }
.field-checkbox { margin-left: 4px; accent-color: #58a6ff; }

.pin-info-row {
  display: flex; align-items: center; gap: 6px; padding: 3px 0;
  font-size: 12px;
}
.pin-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
.pin-info-label { flex: 1; color: #c9d1d9; }
.pin-info-type { color: #6e7681; font-size: 11px; }
.no-pins { color: #484f58; font-size: 12px; font-style: italic; }

.data-preview {
  margin: 0; padding: 8px; background: #161b22; border: 1px solid #30363d;
  border-radius: 4px; font-size: 11px; font-family: monospace;
  max-height: 150px; overflow: auto; white-space: pre-wrap; word-break: break-all;
}
.data-preview-full { max-height: 400px; }
.data-actions { display: flex; gap: 6px; margin-top: 6px; }
.data-action-btn {
  padding: 3px 8px; background: #21262d; border: 1px solid #30363d;
  border-radius: 4px; color: #c9d1d9; font-size: 11px; cursor: pointer;
}
.data-action-btn:hover { background: #30363d; }

.panel-error { background: rgba(248,81,73,0.06); }
.error-text {
  margin: 0; padding: 6px; background: rgba(248,81,73,0.1);
  border-radius: 4px; font-size: 11px; color: #f85149;
  white-space: pre-wrap; word-break: break-all;
}
.panel-meta { background: rgba(88,166,255,0.04); }
.meta-grid { display: flex; flex-wrap: wrap; gap: 8px; }
.meta-item { display: flex; flex-direction: column; gap: 2px; }
.meta-key { font-size: 10px; color: #6e7681; text-transform: uppercase; }
.meta-value { font-size: 12px; color: #c9d1d9; font-weight: 600; }
</style>
