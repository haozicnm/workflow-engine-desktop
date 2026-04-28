<template>
  <div class="properties-panel" v-if="selectedNode">
    <div class="panel-title">📋 节点属性</div>

    <!-- 节点名称 -->
    <div class="panel-section">
      <label class="section-label">名称</label>
      <input
        :value="selectedNode.label"
        class="prop-input"
        @change="emit('update-label', ($event.target as HTMLInputElement).value)"
      />
    </div>

    <!-- 节点类型信息 -->
    <div class="panel-section">
      <div class="node-type-info">
        <span>{{ nodeDef?.icon }}</span>
        <span>{{ nodeDef?.label }}</span>
      </div>
    </div>

    <!-- 参数表单 -->
    <div class="panel-section" v-if="paramEntries.length">
      <label class="section-label">⚙ 参数</label>
      <div v-for="[key, value] in paramEntries" :key="key" class="param-row">
        <label class="param-label">{{ key }}</label>
        <input
          v-if="typeof value === 'boolean'"
          type="checkbox"
          :checked="value"
          class="prop-checkbox"
          @change="emit('update-config', { [key]: ($event.target as HTMLInputElement).checked })"
        />
        <input
          v-else-if="typeof value === 'number'"
          type="number"
          :value="value"
          class="prop-input"
          @change="emit('update-config', { [key]: Number(($event.target as HTMLInputElement).value) })"
        />
        <select
          v-else-if="isSelectParam(key)"
          :value="value"
          class="prop-select"
          @change="emit('update-config', { [key]: ($event.target as HTMLSelectElement).value })"
        >
          <option v-for="opt in selectOptions(key)" :key="opt" :value="opt">{{ opt }}</option>
        </select>
        <input
          v-else
          type="text"
          :value="value"
          class="prop-input"
          @change="emit('update-config', { [key]: ($event.target as HTMLInputElement).value })"
        />
      </div>
    </div>

    <!-- 输出预览 -->
    <div class="panel-section" v-if="selectedNode.output !== undefined">
      <label class="section-label">📤 输出预览</label>
      <pre class="output-preview">{{ JSON.stringify(selectedNode.output, null, 2) }}</pre>
    </div>

    <!-- 错误信息 -->
    <div class="panel-section error-section" v-if="selectedNode.error">
      <label class="section-label">⚠ 错误</label>
      <pre class="error-text">{{ selectedNode.error }}</pre>
    </div>
  </div>

  <div class="properties-panel empty" v-else>
    <span class="empty-hint">选中节点查看属性</span>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { DAGNode } from '../../types/dag-node'
import { BASE_NODE_DEFINITIONS } from '../../types/dag-node'

const props = defineProps<{
  selectedNode: DAGNode | null
}>()

const emit = defineEmits<{
  'update-label': [value: string]
  'update-config': [config: Record<string, unknown>]
}>()

const nodeDef = computed(() =>
  props.selectedNode
    ? BASE_NODE_DEFINITIONS.find(d => d.type === props.selectedNode!.type)
    : null
)

const paramEntries = computed<[string, unknown][]>(() => {
  if (!props.selectedNode) return []
  return Object.entries(props.selectedNode.config)
})

// Select-type params (method, format, etc.)
const SELECT_PARAMS: Record<string, string[]> = {
  method: ['GET', 'POST', 'PUT', 'DELETE', 'PATCH'],
  format: ['json', 'yaml', 'csv', 'xml', 'text'],
}

function isSelectParam(key: string): boolean {
  return key in SELECT_PARAMS
}

function selectOptions(key: string): string[] {
  return SELECT_PARAMS[key] || []
}
</script>

<style scoped>
.properties-panel {
  width: 260px;
  height: 100%;
  background: #181825;
  border-left: 1px solid #313244;
  padding: 12px;
  overflow-y: auto;
  color: #cdd6f4;
  font-size: 13px;
}

.properties-panel.empty {
  display: flex;
  align-items: center;
  justify-content: center;
}

.empty-hint {
  color: #6c7086;
  font-size: 13px;
}

.panel-title {
  font-weight: 700;
  font-size: 14px;
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid #313244;
}

.panel-section {
  margin-bottom: 12px;
}

.section-label {
  display: block;
  font-size: 11px;
  font-weight: 600;
  color: #a6adc8;
  margin-bottom: 4px;
}

.node-type-info {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: #a6adc8;
}

.prop-input,
.prop-select {
  width: 100%;
  padding: 5px 8px;
  background: #1e1e2e;
  border: 1px solid #313244;
  border-radius: 4px;
  color: #cdd6f4;
  font-size: 12px;
  outline: none;
}
.prop-input:focus,
.prop-select:focus {
  border-color: #89b4fa;
}

.prop-checkbox {
  width: 16px;
  height: 16px;
  accent-color: #89b4fa;
  cursor: pointer;
}

.param-row {
  margin-bottom: 8px;
}

.param-label {
  display: block;
  font-size: 11px;
  margin-bottom: 2px;
  color: #9399b2;
}

.output-preview {
  background: #11111b;
  padding: 8px;
  border-radius: 4px;
  font-size: 11px;
  max-height: 200px;
  overflow: auto;
  font-family: monospace;
  white-space: pre-wrap;
  word-break: break-all;
}

.error-section {
  border-top: 1px solid #313244;
  padding-top: 8px;
}

.error-text {
  color: #f38ba8;
  font-size: 11px;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
