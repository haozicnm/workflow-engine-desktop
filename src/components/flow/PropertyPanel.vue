<template>
  <aside class="property-panel">
    <!-- 未选中节点 -->
    <div v-if="!node" class="panel-empty">
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
          <span class="node-info-type">{{ nodeDef?.label || node.type }}</span>
        </div>
      </div>

      <!-- 节点名称 -->
      <div class="panel-section">
        <label class="section-label">名称</label>
        <input
          :value="node.label"
          class="field-input"
          type="text"
          @change="onLabelChange"
        />
      </div>

      <!-- 参数编辑 -->
      <div class="panel-section">
        <label class="section-label">⚙ 参数</label>

        <div v-for="(value, key) in params" :key="key" class="param-row">
          <label class="param-key">{{ formatLabel(key) }}</label>

          <!-- 下拉选择 -->
          <select
            v-if="isSelectField(key)"
            :value="value"
            class="field-select"
            @change="onParamChange(key, ($event.target as HTMLSelectElement).value)"
          >
            <option
              v-for="opt in selectOptions(key)"
              :key="opt"
              :value="opt"
            >
              {{ opt }}
            </option>
          </select>

          <!-- 数字输入 -->
          <input
            v-else-if="typeof value === 'number'"
            type="number"
            :value="value"
            class="field-input"
            @change="onParamChange(key, Number(($event.target as HTMLInputElement).value))"
          />

          <!-- 布尔 -->
          <input
            v-else-if="typeof value === 'boolean'"
            type="checkbox"
            :checked="value"
            class="field-checkbox"
            @change="onParamChange(key, ($event.target as HTMLInputElement).checked)"
          />

          <!-- 文本 -->
          <input
            v-else
            type="text"
            :value="value"
            class="field-input"
            :placeholder="getPlaceholder(key)"
            @change="onParamChange(key, ($event.target as HTMLInputElement).value)"
          />
        </div>
      </div>

      <!-- 针脚信息 -->
      <div class="panel-section">
        <label class="section-label">📥 输入</label>
        <div
          v-for="pin in nodeDef?.inputs"
          :key="pin.id"
          class="pin-info-row"
        >
          <span class="pin-dot" :style="{ background: getPinColor(pin.type) }"></span>
          <span class="pin-info-label">{{ pin.label }}</span>
          <span class="pin-info-type">{{ pinBadge(pin.type) }}</span>
        </div>
        <span v-if="!nodeDef?.inputs.length" class="no-pins">无输入针脚</span>
      </div>

      <div class="panel-section">
        <label class="section-label">📤 输出</label>
        <div
          v-for="pin in nodeDef?.outputs"
          :key="pin.id"
          class="pin-info-row"
        >
          <span class="pin-dot" :style="{ background: getPinColor(pin.type) }"></span>
          <span class="pin-info-label">{{ pin.label }}</span>
          <span class="pin-info-type">{{ pinBadge(pin.type) }}</span>
        </div>
        <span v-if="!nodeDef?.outputs.length" class="no-pins">无输出针脚</span>
      </div>

      <!-- 输出数据预览 -->
      <div v-if="node.output !== undefined" class="panel-section">
        <label class="section-label">📤 输出预览</label>
        <pre class="data-preview">{{ formatOutput(node.output) }}</pre>
        <div class="data-actions">
          <button class="data-action-btn" @click="copyData(node.output)">📋 复制</button>
          <button class="data-action-btn" @click="expandData = !expandData">
            {{ expandData ? '📉 收起' : '📊 展开' }}
          </button>
        </div>
        <!-- 展开的完整视图 -->
        <pre v-if="expandData" class="data-preview data-preview-full">{{ formatOutput(node.output) }}</pre>
      </div>

      <!-- 错误信息 -->
      <div v-if="node.error" class="panel-section panel-error">
        <label class="section-label">⚠ 错误</label>
        <pre class="error-text">{{ node.error }}</pre>
      </div>

      <!-- 执行元数据 -->
      <div v-if="node.duration !== undefined || node.output !== undefined" class="panel-section panel-meta">
        <label class="section-label">📊 执行详情</label>
        <div class="meta-grid">
          <div v-if="node.duration !== undefined" class="meta-item">
            <span class="meta-key">耗时</span>
            <span class="meta-value">{{ node.duration }}ms</span>
          </div>
          <div v-if="node.output !== undefined" class="meta-item">
            <span class="meta-key">类型</span>
            <span class="meta-value">{{ getDataType(node.output) }}</span>
          </div>
          <div v-if="node.output !== undefined" class="meta-item">
            <span class="meta-key">大小</span>
            <span class="meta-value">{{ getDataSize(node.output) }}</span>
          </div>
        </div>
      </div>
    </template>
  </aside>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import type { FlowNode } from './pinTypes'
import { getNodeDef, pinColor, pinBadge } from './pinTypes'

const props = defineProps<{
  node: FlowNode | null
}>()

const emit = defineEmits<{
  'update-label': [id: string, label: string]
  'update-config': [id: string, config: Record<string, unknown>]
  'delete': [id: string]
}>()

const expandData = ref(false)

const nodeDef = computed(() => {
  if (!props.node) return undefined
  return getNodeDef(props.node.type)
})

const params = computed<Record<string, unknown>>(() => {
  return props.node ? { ...props.node.config } : {}
})

// ─── 下拉字段映射 ───
const SELECT_FIELDS: Record<string, string[]> = {
  method: ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD'],
  format: ['json', 'yaml', 'csv', 'txt', 'xml'],
  encoding: ['utf-8', 'gbk', 'ascii', 'base64'],
}

const PLACEHOLDER_MAP: Record<string, string> = {
  url: 'https://api.example.com/data',
  expression: '$.data.items[*]',
  template: '你好 {{name}}，今天 {{weather}}',
  path: './output/data.json',
  output_key: 'result',
  target_field: '$',
}

function isSelectField(key: string): boolean {
  return key in SELECT_FIELDS
}

function selectOptions(key: string): string[] {
  return SELECT_FIELDS[key] || []
}

function getPlaceholder(key: string): string {
  return PLACEHOLDER_MAP[key] || ''
}

function formatLabel(key: string): string {
  // snake_case → Title Case
  return key.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase())
}

function formatOutput(data: unknown): string {
  try {
    return JSON.stringify(data, null, 2)
  } catch {
    return String(data)
  }
}

// ─── 事件处理 ───
function onLabelChange(event: Event) {
  const target = event.target as HTMLInputElement
  if (props.node) {
    emit('update-label', props.node.id, target.value)
  }
}

function onParamChange(key: string, value: unknown) {
  if (props.node) {
    emit('update-config', props.node.id, { [key]: value })
  }
}

function getPinColor(type: string): string {
  return pinColor(type)
}

// ─── 数据预览工具函数 ───
function copyData(data: unknown) {
  const text = typeof data === 'string' ? data : JSON.stringify(data, null, 2)
  navigator.clipboard.writeText(text).catch(() => {})
}

function getDataType(data: unknown): string {
  if (data === null) return 'null'
  if (data === undefined) return 'undefined'
  if (Array.isArray(data)) return `array[${data.length}]`
  if (typeof data === 'object') {
    const keys = Object.keys(data as Record<string, unknown>)
    return `object{${keys.length}}`
  }
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
  width: 260px;
  min-width: 260px;
  height: 100%;
  background: #0d1117;
  border-left: 1px solid #21262d;
  overflow-y: auto;
  color: #c9d1d9;
  font-size: 13px;
}

/* ─── 未选中 ─── */
.panel-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  height: 100%;
  color: #484f58;
  font-size: 13px;
}

.empty-icon {
  opacity: 0.4;
}

/* ─── 头部 ─── */
.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border-bottom: 1px solid #21262d;
  position: sticky;
  top: 0;
  background: #0d1117;
  z-index: 1;
}

.panel-title {
  font-weight: 700;
  font-size: 13px;
}

.btn-delete {
  padding: 2px 6px;
  background: none;
  border: 1px solid transparent;
  border-radius: 4px;
  color: #8b949e;
  font-size: 12px;
  cursor: pointer;
}

.btn-delete:hover {
  background: rgba(248, 81, 73, 0.1);
  border-color: rgba(248, 81, 73, 0.3);
  color: #f85149;
}

/* ─── 区块 ─── */
.panel-section {
  padding: 8px 12px;
  border-bottom: 1px solid #21262d;
}

.section-label {
  display: block;
  font-size: 11px;
  font-weight: 600;
  color: #8b949e;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 6px;
}

/* ─── 节点信息 ─── */
.node-info {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.node-info-icon {
  font-size: 18px;
}

.node-info-type {
  color: #8b949e;
}

/* ─── 表单字段 ─── */
.field-input,
.field-select {
  width: 100%;
  padding: 5px 8px;
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 4px;
  color: #c9d1d9;
  font-size: 12px;
  outline: none;
  transition: border-color 0.15s;
}

.field-input:focus,
.field-select:focus {
  border-color: #58a6ff;
}

.field-input::placeholder {
  color: #484f58;
}

.field-checkbox {
  width: 16px;
  height: 16px;
  accent-color: #58a6ff;
  cursor: pointer;
}

/* ─── 参数行 ─── */
.param-row {
  margin-bottom: 8px;
}

.param-row:last-child {
  margin-bottom: 0;
}

.param-key {
  display: block;
  font-size: 11px;
  margin-bottom: 3px;
  color: #8b949e;
}

/* ─── 针脚信息 ─── */
.pin-info-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 0;
  font-size: 12px;
}

.pin-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.pin-info-label {
  flex: 1;
  color: #c9d1d9;
}

.pin-info-type {
  font-family: monospace;
  font-size: 10px;
  color: #8b949e;
  background: #161b22;
  padding: 1px 5px;
  border-radius: 3px;
}

.no-pins {
  color: #484f58;
  font-size: 11px;
}

/* ─── 数据预览 ─── */
.data-preview {
  background: #161b22;
  padding: 8px;
  border: 1px solid #30363d;
  border-radius: 4px;
  font-size: 11px;
  max-height: 180px;
  overflow: auto;
  font-family: 'SF Mono', 'Cascadia Code', monospace;
  white-space: pre-wrap;
  word-break: break-all;
  color: #c9d1d9;
  line-height: 1.4;
  margin: 0;
}

.data-preview-full {
  max-height: 400px;
  margin-top: 8px;
}

/* ─── 数据操作按钮 ─── */
.data-actions {
  display: flex;
  gap: 6px;
  margin-top: 6px;
}

.data-action-btn {
  padding: 2px 8px;
  background: #21262d;
  border: 1px solid #30363d;
  border-radius: 4px;
  color: #8b949e;
  font-size: 10px;
  cursor: pointer;
  transition: background 0.1s, color 0.1s;
}

.data-action-btn:hover {
  background: #30363d;
  color: #c9d1d9;
}

/* ─── 执行元数据 ─── */
.panel-meta {
  background: rgba(88, 166, 255, 0.03);
}

.meta-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 6px;
}

.meta-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.meta-key {
  font-size: 10px;
  color: #8b949e;
  text-transform: uppercase;
}

.meta-value {
  font-size: 12px;
  color: #c9d1d9;
  font-family: 'SF Mono', 'Cascadia Code', monospace;
}

/* ─── 错误 ─── */
.panel-error {
  border-top: 1px solid rgba(248, 81, 73, 0.2);
  background: rgba(248, 81, 73, 0.04);
}

.error-text {
  color: #f85149;
  font-size: 11px;
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0;
}
</style>
