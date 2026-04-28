<template>
  <div class="dag-editor">
    <!-- 工具栏 -->
    <div class="toolbar">
      <div class="toolbar-left">
        <span class="workflow-name">📁 {{ workflowName }}</span>
        <span class="node-count">{{ store.nodes.length }} 节点</span>
        <span class="edge-count">{{ store.edges.length }} 连线</span>
      </div>
      <div class="toolbar-actions">
        <button class="toolbar-btn primary" @click="runAll" :disabled="isRunning || store.nodes.length === 0">
          ▶ 运行
        </button>
        <button class="toolbar-btn" @click="stopAll" :disabled="!isRunning">
          ■ 停止
        </button>
        <button class="toolbar-btn" @click="clearAll">
          🗑 清空
        </button>
      </div>
    </div>

    <!-- 主体三栏 -->
    <div class="editor-body">
      <!-- 左侧: 节点库 -->
      <NodeLibrary />

      <!-- 中央: 画布 -->
      <div
        class="canvas-wrapper"
        @drop="onDrop"
        @dragover.prevent
      >
        <VueFlow
          v-model="elements"
          :default-viewport="{ x: 0, y: 0, zoom: 1 }"
          :min-zoom="0.1"
          :max-zoom="4"
          :snap-to-grid="true"
          :snap-grid="[20, 20]"
          fit-view-on-init
          @node-click="onNodeClick"
          @pane-click="onPaneClick"
          @connect="onConnect"
          @edge-click="onEdgeClick"
        >
          <Background :gap="20" :pattern-color="'#313244'" />
          <Controls />
          <MiniMap
            class="dag-minimap"
            :pannable="true"
            :zoomable="true"
            node-stroke-color="#555"
          />

          <template #node-custom="nodeProps">
            <ComfyNode
              :id="nodeProps.id"
              :label="nodeProps.data.label"
              :icon="nodeProps.data.icon"
              :color="nodeProps.data.color"
              :status="nodeProps.data.status"
              :inputs="nodeProps.data.inputs"
              :outputs="nodeProps.data.outputs"
              :has-params="false"
              :selected="nodeProps.selected"
              :duration="nodeProps.data.duration"
            />
          </template>
        </VueFlow>
      </div>

      <!-- 右侧: 属性面板 -->
      <PropertiesPanel
        :selected-node="selectedNode"
        @update-label="onUpdateLabel"
        @update-config="onUpdateConfig"
      />
    </div>

    <!-- 底部控制台 -->
    <div class="console" v-if="logs.length">
      <div
        v-for="log in logs"
        :key="log.id"
        :class="['log-line', log.level]"
      >
        <span class="log-time">{{ log.time }}</span>
        <span>{{ log.text }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { VueFlow } from '@vue-flow/core'
import { Background } from '@vue-flow/background'
import { Controls } from '@vue-flow/controls'
import { MiniMap } from '@vue-flow/minimap'
import { useDAGStore } from '../stores/dag-store'
import { BASE_NODE_DEFINITIONS } from '../types/dag-node'
import type { DAGNode } from '../types/dag-node'
import ComfyNode from '../components/dag/ComfyNode.vue'
import NodeLibrary from '../components/dag/NodeLibrary.vue'
import PropertiesPanel from '../components/dag/PropertiesPanel.vue'

import '@vue-flow/core/dist/style.css'
import '@vue-flow/core/dist/theme-default.css'
import '@vue-flow/controls/dist/style.css'
import '@vue-flow/minimap/dist/style.css'

const store = useDAGStore()

// ─── 状态 ───
const workflowName = ref('未命名工作流')
const selectedNode = ref<DAGNode | null>(null)
const isRunning = ref(false)
const logs = ref<{ id: string; time: string; text: string; level: string }[]>([])

// ─── Vue Flow 元素 ───
const elements = computed(() => [
  ...store.nodes.map(n => {
    const def = BASE_NODE_DEFINITIONS.find(d => d.type === n.type)
    return {
      id: n.id,
      type: 'custom',
      position: n.position,
      data: {
        label: n.label,
        icon: def?.icon || '📦',
        color: def?.color || '#6c7086',
        status: store.nodeStatuses[n.id] || 'idle',
        inputs: def?.inputs || [],
        outputs: def?.outputs || [],
        duration: n.duration,
      },
    }
  }),
  ...store.edges.map(e => ({
    id: e.id,
    source: e.source,
    target: e.target,
    sourceHandle: e.sourceHandle,
    targetHandle: e.targetHandle,
    animated: true,
    style: { stroke: '#585b70', strokeWidth: 2 },
  })),
])

// ─── 日志 ───
function addLog(text: string, level: string = 'info') {
  const now = new Date()
  const time = now.toLocaleTimeString('zh-CN', { hour12: false })
  logs.value.push({ id: Date.now().toString(), time, text, level })
  if (logs.value.length > 100) logs.value.shift()
}

// ─── 拖放节点 ───
function onDrop(event: DragEvent) {
  const type = event.dataTransfer?.getData('application/dag-node-type')
  if (!type) return

  const def = BASE_NODE_DEFINITIONS.find(d => d.type === type)
  if (!def) return

  const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect()
  const x = event.clientX - bounds.left - 130
  const y = event.clientY - bounds.top

  const id = `node_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`
  store.addNode({
    id,
    type: def.type,
    label: def.label,
    position: { x, y },
    config: { ...def.defaultConfig },
  })

  addLog(`➕ 添加节点: ${def.label}`, 'info')
}

// ─── 节点交互 ───
function onNodeClick({ node }: any) {
  selectedNode.value = store.nodes.find(n => n.id === node.id) || null
}

function onPaneClick() {
  selectedNode.value = null
}

function onUpdateLabel(value: string) {
  if (selectedNode.value) {
    selectedNode.value.label = value
  }
}

function onUpdateConfig(config: Record<string, unknown>) {
  if (selectedNode.value) {
    store.updateNodeConfig(selectedNode.value.id, config)
  }
}

// ─── 连线 ───
function onConnect(connection: any) {
  if (!connection.source || !connection.target) return

  const sourceNode = store.nodes.find(n => n.id === connection.source)
  const targetNode = store.nodes.find(n => n.id === connection.target)
  if (!sourceNode || !targetNode) return

  const sourceDef = BASE_NODE_DEFINITIONS.find(d => d.type === sourceNode.type)
  const targetDef = BASE_NODE_DEFINITIONS.find(d => d.type === targetNode.type)
  const sourcePin = sourceDef?.outputs.find(p => p.id === connection.sourceHandle)
  const targetPin = targetDef?.inputs.find(p => p.id === connection.targetHandle)

  if (!sourcePin || !targetPin) {
    addLog('❌ 针脚不存在', 'error')
    return
  }

  // 类型匹配检查
  if (
    sourcePin.type !== 'any' &&
    targetPin.type !== 'any' &&
    sourcePin.type !== targetPin.type
  ) {
    addLog(`❌ 类型不匹配: ${sourcePin.type} → ${targetPin.type}`, 'error')
    return
  }

  store.addEdge({
    id: `e_${connection.source}_${connection.sourceHandle}_${connection.target}_${connection.targetHandle}`,
    source: connection.source,
    target: connection.target,
    sourceHandle: connection.sourceHandle,
    targetHandle: connection.targetHandle,
  })

  addLog(`🔗 连线: ${sourceNode.label} → ${targetNode.label}`, 'info')
}

function onEdgeClick({ edge }: any) {
  store.removeEdge(edge.id)
  addLog(`✂ 删除连线`, 'info')
}

// ─── 工具栏 ───
function runAll() {
  isRunning.value = true
  store.resetAllStatuses()
  addLog('▶ 开始执行工作流...', 'info')
  // TODO: Phase 2 — invoke Tauri command
}

function stopAll() {
  isRunning.value = false
  addLog('■ 已停止', 'warn')
}

function clearAll() {
  if (store.nodes.length > 0 && !confirm('确定清空所有节点和连线？')) return
  store.clear()
  selectedNode.value = null
  addLog('🗑 画布已清空', 'info')
}
</script>

<style scoped>
.dag-editor {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #11111b;
}

/* ─── 工具栏 ─── */
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 16px;
  background: #181825;
  border-bottom: 1px solid #313244;
  flex-shrink: 0;
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.workflow-name {
  font-size: 13px;
  font-weight: 600;
  color: #cdd6f4;
}

.node-count,
.edge-count {
  font-size: 11px;
  color: #6c7086;
}

.toolbar-actions {
  display: flex;
  gap: 8px;
}

.toolbar-btn {
  padding: 5px 14px;
  border: 1px solid #313244;
  border-radius: 4px;
  background: #1e1e2e;
  color: #cdd6f4;
  font-size: 12px;
  cursor: pointer;
}
.toolbar-btn:hover {
  background: #313244;
}
.toolbar-btn:disabled {
  opacity: 0.4;
  cursor: default;
}
.toolbar-btn.primary {
  background: #89b4fa;
  color: #1e1e2e;
  border-color: #89b4fa;
  font-weight: 600;
}
.toolbar-btn.primary:hover {
  background: #74a8f5;
}

/* ─── 主体 ─── */
.editor-body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.canvas-wrapper {
  flex: 1;
  position: relative;
}

/* ─── 底部控制台 ─── */
.console {
  height: 120px;
  background: #181825;
  border-top: 1px solid #313244;
  overflow-y: auto;
  padding: 8px 16px;
  font-family: monospace;
  font-size: 11px;
  flex-shrink: 0;
}

.log-line {
  padding: 2px 0;
  display: flex;
  gap: 8px;
}

.log-time {
  color: #6c7086;
  opacity: 0.7;
  flex-shrink: 0;
}

.log-line.info { color: #a6adc8; }
.log-line.error { color: #f38ba8; }
.log-line.warn { color: #f9e2af; }
</style>
