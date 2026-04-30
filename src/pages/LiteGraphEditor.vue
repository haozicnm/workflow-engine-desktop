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

        <canvas ref="canvasRef" class="litegraph-canvas"></canvas>
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
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { LGraph, LGraphCanvas, LiteGraph, LGraphNode } from '@comfyorg/litegraph'
import '@comfyorg/litegraph/style.css'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useFlowStore } from '../stores/flowStore'
import { useUndo } from '../composables/useUndo'
import { useAutoSave } from '../composables/useAutoSave'
import NodePalette from '../components/flow/NodePalette.vue'
import PropertyPanel from '../components/flow/PropertyPanel.vue'
import { registerAllNodes } from '../nodes/litegraph-nodes'
import { getNodeDef } from '../components/flow/pinTypes'
import type { FlowNode, NodeStatus } from '../components/flow/pinTypes'

// ─── Props ───
const route = useRoute()
const store = useFlowStore()

// ─── 撤销/重做 + 自动保存 ───
const undoManager = useUndo()
const autoSave = useAutoSave(30000) // 30 秒自动保存

// ─── LiteGraph 引用 ───
const canvasRef = ref<HTMLCanvasElement | null>(null)
let graph: LGraph
let canvas: LGraphCanvas

// ─── 本地状态 ───
const selectedNode = ref<FlowNode | null>(null)
const isRunning = ref(false)
const logs = ref<{ id: number; time: string; text: string; level: string }[]>([])

// ─── ID 映射：store 用 String(lgId)，通过 find 直接查找 ───
/** 通过 store 的 string ID 查找 LiteGraph 节点 */
function findLgNode(storeId: string): LGraphNode | undefined {
  return graph._nodes?.find((n: LGraphNode) => String(n.id) === storeId)
}

// ─── 标记：是否正在从 store 同步到 graph（避免循环更新） ───
let syncingFromStore = false

// ─── 日志 ───
function addLog(text: string, level = 'info') {
  const now = new Date()
  const time = now.toLocaleTimeString('zh-CN', { hour12: false })
  logs.value.push({ id: Date.now(), time, text, level })
  if (logs.value.length > 100) logs.value.shift()
}

// ─── Canvas 初始化 ───
onMounted(() => {
  // 注册所有自定义 LiteGraph 节点
  registerAllNodes()

  // 创建 graph
  graph = new LGraph()

  // 创建 canvas - LiteGraph 直接接管 canvas 元素
  canvas = new LGraphCanvas(canvasRef.value!, graph)

  // 配置 canvas 外观（暗色主题）
  canvas.background_image = ''
  canvas.clear_background = true
  canvas.render_canvas_border = false
  canvas.render_border = false

  // 监听 graph 变化 → 同步到 store
  graph.onAfterChange = () => {
    if (syncingFromStore) return
    syncGraphToStore()
    undoManager.pushState()
  }

  // 监听选中变化 → 更新属性面板
  // LiteGraph uses onSelectionChange on the canvas
  canvas.onSelectionChange = (selectedDict: Record<string, LGraphNode>) => {
    const selected = Object.values(selectedDict)[0]
    if (selected instanceof LGraphNode) {
      selectedNode.value = liteGraphNodeToFlowNode(selected)
    } else {
      selectedNode.value = null
    }
  }

  // 监听连线变化
  graph.onConnectionChange = () => {
    if (syncingFromStore) return
    syncEdgesToStore()
  }

  // 路由参数
  const id = route.params.id
  if (id && typeof id === 'string' && id !== 'new') {
    store.setWorkflowId(id)
    addLog(`📂 加载工作流: ${id}`)
  }

  // 模板数据已通过 Dashboard 预加载
  if (store.nodes.length > 0) {
    loadFromStore()
    if (id !== 'new') {
      addLog(`📋 已加载模板「${store.workflowName}」(${store.nodeCount} 节点)`, 'info')
    }
  } else if (id === 'new') {
    // 自动恢复上次未保存的工作流
    const restored = autoSave.loadAutoSave()
    if (restored) {
      loadFromStore()
      addLog('📂 已恢复上次未保存的工作流', 'info')
    }
  }

  // 初始化撤销系统
  undoManager.init()
  // 启动自动保存
  autoSave.start()
  // 键盘快捷键
  document.addEventListener('keydown', onKeyDown)

  // 监听窗口 resize
  window.addEventListener('resize', onResize)
})

onUnmounted(() => {
  autoSave.stop()
  document.removeEventListener('keydown', onKeyDown)
  window.removeEventListener('resize', onResize)
})

// ─── Canvas resize ───
function onResize() {
  if (canvas) {
    canvas.resize()
  }
}

// ─── LiteGraph 节点 → FlowNode 转换 ───
function liteGraphNodeToFlowNode(node: LGraphNode): FlowNode {
  // 收集 widgets 的值作为 config
  const config: Record<string, unknown> = {}
  if (node.widgets) {
    for (const w of node.widgets) {
      config[w.name] = w.value
    }
  }

  return {
    id: String(node.id),
    type: node.type || '',
    label: node.title || node.type || '',
    position: { x: node.pos[0], y: node.pos[1] },
    config,
    status: (store.nodeStatuses[String(node.id)] as NodeStatus) || 'idle',
  }
}

// ─── 从 LiteGraph graph 同步节点到 store ───
function syncGraphToStore() {
  syncingFromStore = true

  const lgNodes = graph._nodes || []
  const storeNodeIds = new Set(store.nodes.map(n => n.id))

  for (let i = 0; i < lgNodes.length; i++) {
    const ln = lgNodes[i]
    if (!ln) continue
    const nodeId = String(ln.id)

    // 收集 widgets 的值为 config
    const config: Record<string, unknown> = {}
    if (ln.widgets) {
      for (const w of ln.widgets) {
        config[w.name] = w.value
      }
    }

    if (storeNodeIds.has(nodeId)) {
      // 更新已有节点
      const existing = store.getNode(nodeId)
      if (existing) {
        const posChanged =
          existing.position.x !== ln.pos[0] ||
          existing.position.y !== ln.pos[1]
        const labelChanged = existing.label !== (ln.title || ln.type || '')
        const configChanged = JSON.stringify(existing.config) !== JSON.stringify(config)

        if (posChanged) {
          store.updateNodePosition(nodeId, { x: ln.pos[0], y: ln.pos[1] })
        }
        if (labelChanged) {
          store.updateNodeLabel(nodeId, ln.title || ln.type || '')
        }
        if (configChanged) {
          store.updateNodeConfig(nodeId, config)
        }
      }
    } else {
      // 新增节点
      store.addNode({
        id: nodeId,
        type: ln.type || '',
        label: ln.title || ln.type || '',
        position: { x: ln.pos[0], y: ln.pos[1] },
        config,
      })
    }
  }

  // 删除在 store 中但不在 graph 中的节点
  const graphNodeIds = new Set(lgNodes.map((n: LGraphNode) => String(n.id)))
  for (const sn of store.nodes) {
    if (!graphNodeIds.has(sn.id)) {
      store.removeNode(sn.id)
    }
  }

  // 同步连线
  syncEdgesToStore()

  syncingFromStore = false
}

// ─── 同步连线到 store ───
function syncEdgesToStore() {
  const links = (graph as any)._links || []
  const storeEdgeIds = new Set(store.edges.map(e => e.id))

  for (const link of links) {
    const sourceId = String(link.origin_id)
    const targetId = String(link.target_id)
    const sourceSlot = link.origin_slot
    const targetSlot = link.target_slot

    // 获取 pin 名称
    const srcNode = graph._nodes?.find((n: LGraphNode) => String(n.id) === sourceId)
    const tgtNode = graph._nodes?.find((n: LGraphNode) => String(n.id) === targetId)

    const sourceHandle = srcNode?.outputs?.[sourceSlot]?.name || `out_${sourceSlot}`
    const targetHandle = tgtNode?.inputs?.[targetSlot]?.name || `in_${targetSlot}`

    const edgeId = `e_${sourceId}_${sourceHandle}_${targetId}_${targetHandle}`

    if (!storeEdgeIds.has(edgeId)) {
      store.addEdge({
        id: edgeId,
        source: sourceId,
        target: targetId,
        sourceHandle,
        targetHandle,
      })
    }
  }

  // 删除不在 graph 中的连线
  const linkIds = new Set(
    links.map((l: any) => {
      const sId = String(l.origin_id)
      const tId = String(l.target_id)
      const sNode = graph._nodes?.find((n: LGraphNode) => String(n.id) === sId)
      const tNode = graph._nodes?.find((n: LGraphNode) => String(n.id) === tId)
      return `e_${sId}_${sNode?.outputs?.[l.origin_slot]?.name || `out_${l.origin_slot}`}_${tId}_${tNode?.inputs?.[l.target_slot]?.name || `in_${l.target_slot}`}`
    })
  )

  for (const edge of store.edges) {
    if (!linkIds.has(edge.id)) {
      store.removeEdge(edge.id)
    }
  }
}

// ─── 从 store 加载到 LiteGraph ───
function loadFromStore() {
  syncingFromStore = true
  graph.clear()

  // 旧 store ID → 新 LiteGraph 节点的映射（LiteGraph 会分配新整数 ID）
  const oldToNew = new Map<string, LGraphNode>()

  // 添加节点
  for (const sn of store.nodes) {
    const node = LiteGraph.createNode(sn.type)
    if (!node) {
      addLog(`⚠ 未知节点类型: ${sn.type}`, 'warn')
      continue
    }
    oldToNew.set(sn.id, node)
    node.pos = [sn.position.x, sn.position.y]
    node.title = sn.label
    // 设置 widget 值
    if (sn.config && node.widgets) {
      for (const w of node.widgets) {
        if (sn.config[w.name] !== undefined) {
          w.value = sn.config[w.name]
        }
      }
    }
    graph.add(node)
  }

  // 添加连线（使用 ID 映射）
  for (const edge of store.edges) {
    const sourceNode = oldToNew.get(edge.source)
    const targetNode = oldToNew.get(edge.target)
    if (!sourceNode || !targetNode) continue

    const sourceSlot = sourceNode.outputs?.findIndex((o: any) => o.name === edge.sourceHandle)
    const targetSlot = targetNode.inputs?.findIndex((i: any) => i.name === edge.targetHandle)

    if (sourceSlot >= 0 && targetSlot >= 0) {
      sourceNode.connect(sourceSlot, targetNode, targetSlot)
    }
  }

  // 不再需要 oldToNew 映射 — syncGraphToStore 会统一用 LiteGraph 整数 ID 重写 store
  syncingFromStore = false
  // 重新同步 store，将 LiteGraph 生成的整数 ID 写回
  syncGraphToStore()
  store.dirty = false
}

// ─── 保存：从 graph 同步到 store 再导出 JSON ───
function saveToJSON() {
  syncGraphToStore()
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
}

// ─── 拖放节点 ───
function onDragStart(_def: unknown, event: DragEvent) {
  // NodePalette 已设置 dataTransfer
}

function onDrop(event: DragEvent) {
  const nodeType = event.dataTransfer?.getData('application/flow-node-type')
  if (!nodeType) return

  const def = getNodeDef(nodeType)
  if (!def) return

  const rect = canvasRef.value!.getBoundingClientRect()
  // 使用 LiteGraph 的坐标转换（考虑缩放和平移）
  // convertOffsetToCanvas(pos: Point, out: Point): Point
  const canvasCoords = canvas.convertOffsetToCanvas(
    { x: event.clientX - rect.left, y: event.clientY - rect.top },
    { x: 0, y: 0 }
  )

  const node = LiteGraph.createNode(nodeType)
  if (!node) {
    addLog(`⚠ 无法创建节点: ${def.label}`, 'error')
    return
  }

  node.pos = [canvasCoords.x, canvasCoords.y]
  node.title = def.label

  // 设置默认 config 到 widgets
  if (def.defaultConfig && node.widgets) {
    for (const w of node.widgets) {
      if (def.defaultConfig[w.name] !== undefined) {
        w.value = def.defaultConfig[w.name]
      }
    }
  }

  graph.add(node)
  // LiteGraph 已分配整数 ID，syncGraphToStore 会生成 store ID 并建立映射
  addLog(`➕ 添加节点: ${def.label}`)
  undoManager.pushState()
}

// ─── 属性面板回调 ───
function onUpdateLabel(id: string, label: string) {
  const lgNode = findLgNode(id)
  if (lgNode) {
    lgNode.title = label
  }
  store.updateNodeLabel(id, label)
}

function onUpdateConfig(id: string, config: Record<string, unknown>) {
  const lgNode = findLgNode(id)
  if (lgNode && lgNode.widgets) {
    for (const w of lgNode.widgets) {
      if (config[w.name] !== undefined) {
        w.value = config[w.name]
      }
    }
  }
  store.updateNodeConfig(id, config)
  // 同步回 selectedNode
  if (selectedNode.value && selectedNode.value.id === id) {
    selectedNode.value.config = { ...selectedNode.value.config, ...config }
  }
}

function onDeleteNode(id: string) {
  const lgNode = findLgNode(id)
  if (lgNode) {
    graph.remove(lgNode)
  } else {
    store.removeNode(id)
  }
  if (selectedNode.value?.id === id) {
    selectedNode.value = null
  }
  addLog(`🗑 删除节点: ${id}`)
  undoManager.pushState()
}

// ─── 工具栏操作 ───
function runAll() {
  if (store.nodes.length === 0) return
  isRunning.value = true
  store.resetAllStatuses()
  addLog('▶ DAG 执行开始...', 'info')
  runDagFlow()
}

function runSingle() {
  if (store.nodes.length === 0) return
  addLog('⏯ 单步调试模式（待实现）', 'info')
}

async function stopRun() {
  if (currentRunId.value) {
    try {
      await invoke('dag_run_cancel', { runId: currentRunId.value })
      addLog('■ 已发送取消指令', 'warn')
    } catch (e) {
      addLog(`■ 取消失败: ${e}`, 'error')
    }
  }
  isRunning.value = false
  currentRunId.value = null
}

// ─── 真实 DAG 执行 ───
const currentRunId = ref<string | null>(null)
let dagEventUnlisteners: UnlistenFn[] = []

async function runDagFlow() {
  syncGraphToStore()

  const workflowJson = {
    name: store.workflowName || '未命名',
    description: '',
    nodes: store.nodes.map(n => ({
      id: n.id,
      type: n.type,
      label: n.label,
      position: n.position,
      config: n.config,
    })),
    edges: store.edges.map(e => ({
      id: e.id,
      source: e.source,
      target: e.target,
      sourceHandle: e.sourceHandle,
      targetHandle: e.targetHandle,
    })),
    variables: {},
  }

  try {
    await setupDagListeners()
    const runId = await invoke<string>('dag_run_start', { workflowJson })
    currentRunId.value = runId
    addLog(`🚀 DAG 运行已启动: ${runId.slice(0, 8)}...`, 'info')
  } catch (e) {
    addLog(`❌ DAG 启动失败: ${e}`, 'error')
    isRunning.value = false
    cleanupDagListeners()
  }
}

async function setupDagListeners() {
  cleanupDagListeners()

  const u1 = await listen<{ step_id: string; step_name: string; status: string; current_step: number; total_steps: number }>(
    'step-status-update',
    (event) => {
      const { step_id, step_name, status, current_step, total_steps } = event.payload
      store.setNodeStatus(step_id, status as NodeStatus)
      if (status === 'running') addLog(`⏳ [${current_step}/${total_steps}] ${step_name}`)
      else if (status === 'success') addLog(`✅ [${current_step}/${total_steps}] ${step_name}`)
    }
  )
  dagEventUnlisteners.push(u1)

  const u2 = await listen<{ run_id: string; status: string }>(
    'run-update',
    (event) => {
      if (event.payload.status === 'completed') {
        addLog('✅ DAG 工作流执行完成', 'info')
      } else if (event.payload.status === 'cancelled') {
        addLog('■ DAG 工作流已取消', 'warn')
      }
      isRunning.value = false
      currentRunId.value = null
      cleanupDagListeners()
    }
  )
  dagEventUnlisteners.push(u2)

  const u3 = await listen<{ step_id: string; step_name: string }>(
    'breakpoint-hit',
    (event) => {
      const { step_id, step_name } = event.payload
      addLog(`🔴 断点: ${step_name}`, 'warn')
      store.setNodeStatus(step_id, 'paused')
    }
  )
  dagEventUnlisteners.push(u3)
}

function cleanupDagListeners() {
  for (const unlisten of dagEventUnlisteners) unlisten()
  dagEventUnlisteners = []
}

function fitView() {
  const lgNodes = graph._nodes
  if (!canvas || !lgNodes || lgNodes.length === 0) return

  // 计算所有节点的包围盒
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
  for (const node of lgNodes) {
    if (!node) continue
    const pos = node.pos
    const size = node.size
    minX = Math.min(minX, pos[0])
    minY = Math.min(minY, pos[1])
    maxX = Math.max(maxX, pos[0] + (size[0] || 200))
    maxY = Math.max(maxY, pos[1] + (size[1] || 100))
  }

  canvas.animateToBounds(
    { x: minX - 20, y: minY - 20, width: maxX - minX + 40, height: maxY - minY + 40 },
    { zoom: 0.9 }
  )
}

function autoLayout() {
  const lgNodes = graph._nodes || []
  if (lgNodes.length === 0) return

  // 简单水平布局
  const SPACING_X = 320
  const SPACING_Y = 200
  const COLS = Math.max(1, Math.ceil(Math.sqrt(lgNodes.length)))

  for (let i = 0; i < lgNodes.length; i++) {
    const ln = lgNodes[i]
    if (!ln) continue
    const col = i % COLS
    const row = Math.floor(i / COLS)
    ln.pos = [col * SPACING_X + 60, row * SPACING_Y + 60]
  }

  // 标记 graph 为脏，触发重绘
  graph.setDirtyCanvas(true)
  setTimeout(() => {
    fitView()
  }, 50)
  addLog('↻ 自动布局完成')
}

function clearCanvas() {
  if (store.nodes.length > 0 && !confirm('确定清空画布上所有节点和连线？')) return
  graph.clear()
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

// ─── 键盘快捷键 ───
function onKeyDown(e: KeyboardEvent) {
  // Ctrl+Z / Cmd+Z — 撤销
  if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key === 'z') {
    e.preventDefault()
    if (undoManager.undo()) {
      // 撤销后同步到 LiteGraph
      loadFromStore()
      addLog('↩ 撤销')
    }
    return
  }
  // Ctrl+Shift+Z / Cmd+Shift+Z — 重做
  if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'z') {
    e.preventDefault()
    if (undoManager.redo()) {
      loadFromStore()
      addLog('↪ 重做')
    }
    return
  }
  // Ctrl+S / Cmd+S — 保存（导出 JSON）
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    e.preventDefault()
    saveToJSON()
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
    canvas.deselectAllNodes()
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
  z-index: 10;
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
  overflow: hidden;
}

.litegraph-canvas {
  width: 100%;
  height: 100%;
  display: block;
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
