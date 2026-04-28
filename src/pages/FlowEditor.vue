<template>
  <div class="flow-editor">
    <!-- 工具栏 -->
    <header class="toolbar">
      <div class="toolbar-left">
        <span class="workflow-name">📁 {{ store.workflowName }}</span>
        <span class="stat-badge">{{ store.nodeCount }} 节点</span>
        <span class="stat-badge">{{ store.edgeCount }} 连线</span>
        <span v-if="store.dirty" class="stat-badge dirty">● 已修改</span>
      </div>
      <div class="toolbar-center">
        <button
          class="toolbar-btn primary"
          :disabled="isRunning || store.nodes.length === 0"
          @click="runAll"
        >
          ▶ 运行
        </button>
        <button
          class="toolbar-btn"
          :disabled="isRunning || store.nodes.length === 0"
          @click="runSingle"
        >
          ⏯ 单步
        </button>
        <button
          class="toolbar-btn"
          :disabled="!isRunning"
          @click="stopRun"
        >
          ■ 停止
        </button>
      </div>
      <div class="toolbar-right">
        <button class="toolbar-btn" @click="autoLayout" title="自动布局">
          ↻ 布局
        </button>
        <button class="toolbar-btn" @click="fitView" title="适应画布">
          ⊞ 适应
        </button>
        <button class="toolbar-btn" @click="clearCanvas">
          🗑 清空
        </button>
      </div>
    </header>

    <!-- 主体三栏布局 -->
    <div class="editor-body">
      <!-- 左侧：节点库 -->
      <NodePalette @drag-start="onDragStart" />

      <!-- 中央：画布 -->
      <div
        class="canvas-wrapper"
        @drop="onDrop"
        @dragover.prevent
      >
        <!-- 空状态提示 -->
        <div v-if="store.nodes.length === 0" class="empty-canvas">
          <div class="empty-icon">🎨</div>
          <div class="empty-title">空画布</div>
          <div class="empty-hint">从左侧拖节点到此处，或按 <kbd>Ctrl+S</kbd> 保存</div>
        </div>

        <VueFlow
          ref="vueFlowRef"
          v-model="vueFlowElements"
          :default-viewport="{ x: 0, y: 0, zoom: 0.8 }"
          :min-zoom="0.1"
          :max-zoom="4"
          :snap-to-grid="true"
          :snap-grid="[20, 20]"
          fit-view-on-init
          @node-click="onNodeClick"
          @node-drag-stop="onNodeDragStop"
          @pane-click="onPaneClick"
          @connect="onConnect"
          @edge-click="onEdgeClick"
        >
          <Background :gap="20" pattern-color="#21262d" />
          <Controls position="bottom-right" />
          <MiniMap
            class="flow-minimap"
            :pannable="true"
            :zoomable="true"
            :node-stroke-color="'#30363d'"
          />

          <template #node-custom="nodeProps">
            <ComfyNode
              :id="String(nodeProps.id)"
              :label="String(nodeProps.data.label)"
              :icon="String(nodeProps.data.icon)"
              :status="(nodeProps.data.status as NodeStatus) || 'idle'"
              :inputs="(nodeProps.data.inputs as PinDefinition[]) || []"
              :outputs="(nodeProps.data.outputs as PinDefinition[]) || []"
              :has-params="Boolean(nodeProps.data.hasParams)"
              :selected="Boolean(nodeProps.selected)"
              :duration="nodeProps.data.duration as number | undefined"
              :error="nodeProps.data.error as string | undefined"
            />
          </template>
        </VueFlow>
      </div>

      <!-- 右侧：属性面板 -->
      <PropertyPanel
        :node="selectedNode"
        @update-label="onUpdateLabel"
        @update-config="onUpdateConfig"
        @delete="onDeleteNode"
      />
    </div>

    <!-- 底部控制台 -->
    <footer v-if="logs.length > 0" class="console">
      <div class="console-header">
        <span>📋 执行日志</span>
        <button class="console-clear" @click="logs = []">清除</button>
      </div>
      <div class="console-body">
        <div
          v-for="log in logs"
          :key="log.id"
          :class="['log-line', log.level]"
        >
          <span class="log-time">{{ log.time }}</span>
          <span class="log-text">{{ log.text }}</span>
        </div>
      </div>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useRoute } from 'vue-router'
import { VueFlow, type VueFlowInstance } from '@vue-flow/core'
import { Background } from '@vue-flow/background'
import { Controls } from '@vue-flow/controls'
import { MiniMap } from '@vue-flow/minimap'
import { useFlowStore } from '../stores/flowStore'
import { useUndo } from '../composables/useUndo'
import { useAutoSave } from '../composables/useAutoSave'
import ComfyNode from '../components/flow/ComfyNode.vue'
import NodePalette from '../components/flow/NodePalette.vue'
import PropertyPanel from '../components/flow/PropertyPanel.vue'
import { getNodeDef } from '../components/flow/pinTypes'
import type { FlowNode, FlowEdge, NodeStatus, PinDefinition } from '../components/flow/pinTypes'

import '@vue-flow/core/dist/style.css'
import '@vue-flow/core/dist/theme-default.css'
import '@vue-flow/controls/dist/style.css'
import '@vue-flow/minimap/dist/style.css'

// ─── Props ───
const route = useRoute()
const store = useFlowStore()

// ─── 撤销/重做 + 自动保存 ───
const undoManager = useUndo()
const autoSave = useAutoSave(30000) // 30 秒自动保存

// ─── Vue Flow 引用 ───
const vueFlowRef = ref<VueFlowInstance | null>(null)

// ─── 本地状态 ───
const selectedNode = ref<FlowNode | null>(null)
const isRunning = ref(false)
const logs = ref<{ id: number; time: string; text: string; level: string }[]>([])

// ─── Vue Flow 元素（计算属性） ───
const vueFlowElements = computed(() => [
  ...store.nodes.map(n => {
    const def = getNodeDef(n.type)
    return {
      id: n.id,
      type: 'custom',
      position: n.position,
      data: {
        label: n.label,
        icon: def?.icon || '📦',
        color: def?.color || '#8b949e',
        status: store.nodeStatuses[n.id] || 'idle',
        inputs: def?.inputs || [],
        outputs: def?.outputs || [],
        hasParams: Object.keys(n.config).length > 0,
        duration: n.duration,
        error: n.error,
      },
    }
  }),
  ...store.edges.map(e => ({
    id: e.id,
    source: e.source,
    target: e.target,
    sourceHandle: e.sourceHandle,
    targetHandle: e.targetHandle,
    style: { stroke: '#30363d', strokeWidth: 2 },
    animated: false,
  })),
])

// ─── 日志 ───
function addLog(text: string, level = 'info') {
  const now = new Date()
  const time = now.toLocaleTimeString('zh-CN', { hour12: false })
  logs.value.push({ id: Date.now(), time, text, level })
  if (logs.value.length > 100) logs.value.shift()
}

// ─── 拖放节点 ───
const _dragOffset = ref({ x: 0, y: 0 })

function onDragStart(_def: unknown, event: DragEvent) {
  // 记录拖拽起始作为偏移参考
  _dragOffset.value = { x: event.clientX, y: event.clientY }
}

function onDrop(event: DragEvent) {
  const type = event.dataTransfer?.getData('application/flow-node-type')
  if (!type) return

  const def = getNodeDef(type)
  if (!def) return

  // 计算画布坐标
  const canvas = event.currentTarget as HTMLElement
  const bounds = canvas.getBoundingClientRect()
  const x = event.clientX - bounds.left - 120
  const y = event.clientY - bounds.top - 30

  const id = store.generateId()
  store.addNode({
    id,
    type: def.type,
    label: def.label,
    position: { x, y },
    config: { ...def.defaultConfig },
  })

  addLog(`➕ 添加节点: ${def.label}`)
  undoManager.pushState()
}

// ─── 节点交互 ───
function onNodeClick({ node }: { node: { id: string } }) {
  const found = store.nodes.find(n => n.id === node.id) || null
  selectedNode.value = found
}

function onNodeDragStop({ node }: { node: { id: string; position: { x: number; y: number } } }) {
  store.updateNodePosition(node.id, node.position)
  undoManager.pushState()
}

function onPaneClick() {
  selectedNode.value = null
}

function onUpdateLabel(id: string, label: string) {
  store.updateNodeLabel(id, label)
}

function onUpdateConfig(id: string, config: Record<string, unknown>) {
  store.updateNodeConfig(id, config)
  // 同步回 selectedNode
  if (selectedNode.value && selectedNode.value.id === id) {
    selectedNode.value.config = { ...selectedNode.value.config, ...config }
  }
}

function onDeleteNode(id: string) {
  const node = store.getNode(id)
  store.removeNode(id)
  if (selectedNode.value?.id === id) {
    selectedNode.value = null
  }
  addLog(`🗑 删除节点: ${node?.label || id}`)
  undoManager.pushState()
}

// ─── 连线 ───
function onConnect(connection: { source: string; target: string; sourceHandle: string; targetHandle: string }) {
  if (!connection.source || !connection.target) return

  const sourceNode = store.getNode(connection.source)
  const targetNode = store.getNode(connection.target)
  if (!sourceNode || !targetNode) return

  const sourceDef = getNodeDef(sourceNode.type)
  const targetDef = getNodeDef(targetNode.type)
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

  const id = `e_${connection.source}_${connection.sourceHandle}_${connection.target}_${connection.targetHandle}`
  store.addEdge({
    id,
    source: connection.source,
    target: connection.target,
    sourceHandle: connection.sourceHandle,
    targetHandle: connection.targetHandle,
  })

  addLog(`🔗 连线: ${sourceNode.label} → ${targetNode.label}`)
  undoManager.pushState()
}

function onEdgeClick({ edge }: { edge: FlowEdge }) {
  store.removeEdge(edge.id)
  addLog('✂ 删除连线')
  undoManager.pushState()
}

// ─── 工具栏操作 ───
function runAll() {
  if (store.nodes.length === 0) return
  isRunning.value = true
  store.resetAllStatuses()
  addLog('▶ 全量运行开始...', 'info')
  // TODO: P2 集成 Tauri 后端执行引擎
  simulateRun()
}

function runSingle() {
  if (store.nodes.length === 0) return
  addLog('⏯ 单步调试模式（待实现）', 'info')
}

function stopRun() {
  isRunning.value = false
  addLog('■ 运行已停止', 'warn')
}

// 模拟运行（P1 占位）
async function simulateRun() {
  const nodeIds = store.nodes.map(n => n.id)
  for (const id of nodeIds) {
    store.setNodeStatus(id, 'running')
    addLog(`⏳ 执行节点: ${store.getNode(id)?.label || id}`)
    await new Promise(r => setTimeout(r, 600 + Math.random() * 800))

    store.setNodeStatus(id, 'success')
    const node = store.getNode(id)
    if (node) {
      node.output = { status: 'ok', simulated: true }
      node.duration = Math.round(600 + Math.random() * 800)
      node.error = undefined
    }
    addLog(`✅ 完成: ${store.getNode(id)?.label || id} (${node?.duration}ms)`)
  }
  isRunning.value = false
  addLog('✅ 工作流执行完成', 'info')
}

function fitView() {
  vueFlowRef.value?.fitView({ padding: 0.2 })
}

function autoLayout() {
  const ns = store.nodes
  if (ns.length === 0) return

  // 简单水平布局
  const SPACING_X = 320
  const SPACING_Y = 200
  const COLS = Math.max(1, Math.ceil(Math.sqrt(ns.length)))

  ns.forEach((n, i) => {
    const col = i % COLS
    const row = Math.floor(i / COLS)
    store.updateNodePosition(n.id, {
      x: col * SPACING_X + 60,
      y: row * SPACING_Y + 60,
    })
  })

  // 延迟 fitView 等节点位置更新
  setTimeout(() => {
    vueFlowRef.value?.fitView({ padding: 0.15 })
  }, 50)
  addLog('↻ 自动布局完成')
}

function clearCanvas() {
  if (store.nodes.length > 0 && !confirm('确定清空画布上所有节点和连线？')) return
  store.clear()
  selectedNode.value = null
  addLog('🗑 画布已清空')
  undoManager.pushState()
}

// ─── 路由参数监听 ───
watch(
  () => route.params.id,
  (id) => {
    if (id && typeof id === 'string') {
      store.setWorkflowId(id)
    }
  },
  { immediate: true }
)

onMounted(() => {
  const id = route.params.id
  if (id && typeof id === 'string' && id !== 'new') {
    store.setWorkflowId(id)
    addLog(`📂 加载工作流: ${id}`)
  }

  // 初始化撤销系统
  undoManager.init()
  // 启动自动保存
  autoSave.start()
  // 尝试恢复上次未保存的工作流
  if (id === 'new' && store.nodes.length === 0) {
    const restored = autoSave.loadAutoSave()
    if (restored) {
      addLog('📂 已恢复上次未保存的工作流', 'info')
    }
  }
  // 键盘快捷键
  document.addEventListener('keydown', onKeyDown)
})

onUnmounted(() => {
  autoSave.stop()
  document.removeEventListener('keydown', onKeyDown)
})

// ─── 键盘快捷键 ───
function onKeyDown(e: KeyboardEvent) {
  // Ctrl+Z / Cmd+Z — 撤销
  if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key === 'z') {
    e.preventDefault()
    if (undoManager.undo()) {
      addLog('↩ 撤销')
    }
    return
  }
  // Ctrl+Shift+Z / Cmd+Shift+Z — 重做
  if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'z') {
    e.preventDefault()
    if (undoManager.redo()) {
      addLog('↪ 重做')
    }
    return
  }
  // Ctrl+S / Cmd+S — 保存（导出 JSON）
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    e.preventDefault()
    const data = store.toJSON()
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${store.workflowName || 'workflow'}.json`
    a.click()
    URL.revokeObjectURL(url)
    store.dirty = false
    addLog('💾 工作流已导出')
    return
  }
  // Delete / Backspace — 删除选中节点
  if ((e.key === 'Delete' || e.key === 'Backspace') && selectedNode.value) {
    // 排除输入框内的按键
    if (document.activeElement?.tagName === 'INPUT' || document.activeElement?.tagName === 'TEXTAREA') return
    e.preventDefault()
    onDeleteNode(selectedNode.value.id)
    return
  }
  // Escape — 取消选择
  if (e.key === 'Escape') {
    selectedNode.value = null
    return
  }
}
</script>

<style scoped>
.flow-editor {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #0d1117;
  color: #c9d1d9;
}

/* ─── 工具栏 ─── */
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  background: #161b22;
  border-bottom: 1px solid #21262d;
  flex-shrink: 0;
  gap: 12px;
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.workflow-name {
  font-size: 13px;
  font-weight: 600;
  color: #c9d1d9;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.stat-badge {
  font-size: 10px;
  color: #8b949e;
  background: #21262d;
  padding: 2px 7px;
  border-radius: 10px;
  white-space: nowrap;
}

.stat-badge.dirty {
  color: #d29922;
}

.toolbar-center {
  display: flex;
  gap: 6px;
}

.toolbar-right {
  display: flex;
  gap: 6px;
}

.toolbar-btn {
  padding: 4px 12px;
  border: 1px solid #30363d;
  border-radius: 6px;
  background: #21262d;
  color: #c9d1d9;
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
  transition: background 0.1s, border-color 0.1s;
}

.toolbar-btn:hover {
  background: #30363d;
}

.toolbar-btn:disabled {
  opacity: 0.4;
  cursor: default;
}

.toolbar-btn.primary {
  background: #238636;
  border-color: #2ea043;
  color: #fff;
  font-weight: 600;
}

.toolbar-btn.primary:hover:not(:disabled) {
  background: #2ea043;
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
  background: #0d1117;
}

.canvas-wrapper :deep(.vue-flow__pane) {
  cursor: default;
}

/* ─── 空画布提示 ─── */
.empty-canvas {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  text-align: center;
  z-index: 5;
  pointer-events: none;
  user-select: none;
}

.empty-icon {
  font-size: 48px;
  margin-bottom: 12px;
  opacity: 0.4;
}

.empty-title {
  font-size: 18px;
  font-weight: 600;
  color: #8b949e;
  margin-bottom: 6px;
}

.empty-hint {
  font-size: 13px;
  color: #484f58;
}

.empty-hint kbd {
  display: inline-block;
  padding: 2px 6px;
  background: #21262d;
  border: 1px solid #30363d;
  border-radius: 4px;
  font-family: inherit;
  font-size: 11px;
}

.flow-minimap {
  background: #161b22 !important;
  border: 1px solid #30363d !important;
}

/* ─── 底部控制台 ─── */
.console {
  height: 140px;
  background: #161b22;
  border-top: 1px solid #21262d;
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
}

.console-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 12px;
  border-bottom: 1px solid #21262d;
  font-size: 11px;
  font-weight: 600;
  color: #8b949e;
  flex-shrink: 0;
}

.console-clear {
  padding: 1px 8px;
  background: none;
  border: 1px solid #30363d;
  border-radius: 4px;
  color: #8b949e;
  font-size: 10px;
  cursor: pointer;
}

.console-clear:hover {
  background: #21262d;
  color: #c9d1d9;
}

.console-body {
  flex: 1;
  overflow-y: auto;
  padding: 4px 12px;
  font-family: 'SF Mono', 'Cascadia Code', monospace;
  font-size: 11px;
  line-height: 1.5;
}

.log-line {
  display: flex;
  gap: 8px;
  padding: 1px 0;
}

.log-time {
  color: #484f58;
  flex-shrink: 0;
}

.log-text {
  word-break: break-all;
}

.log-line.info { color: #8b949e; }
.log-line.error { color: #f85149; }
.log-line.warn { color: #d29922; }
.log-line.success { color: #3fb950; }
</style>
