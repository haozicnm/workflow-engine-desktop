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
        <button
          :class="['toolbar-btn', recording ? 'recording-active' : '']"
          @click="toggleRecording"
          title="录制浏览器操作"
        >
          {{ recording ? '⏹ 停止录制' : '🔴 录制' }}
        </button>
        <button class="toolbar-btn" @click="pickElement" title="拾取浏览器元素">
          🎯 拾取
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
        ref="wrapperRef"
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

      <!-- 右侧面板：属性 + 预览 -->
      <div class="right-panels">
        <PropertyPanel
          :lg-node="selectedLgNode"
          :output="selectedLgNode ? store.stepOutputs[String(selectedLgNode.id)] : undefined"
          :error="selectedLgNode ? store.nodeStatuses[String(selectedLgNode.id)] === 'error' ? '执行失败' : undefined : undefined"
          :duration="undefined"
          @update-label="onUpdateLabel"
          @update-widget="onUpdateWidget"
          @delete="onDeleteNode"
        />
        <div class="panel-divider"></div>
        <PreviewPanel
          :lg-node="selectedLgNode"
          @update-widget="onUpdateWidget"
        />
      </div>
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
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useRoute } from 'vue-router'
import { LGraph, LGraphCanvas, LiteGraph, LGraphNode } from '@comfyorg/litegraph'
import '@comfyorg/litegraph/style.css'
import { invoke } from '@tauri-apps/api/core'
import type { UnlistenFn } from '@tauri-apps/api/event'

// Dev mode: Tauri invoke/event not available
const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__

async function safeInvoke<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) {
    throw new Error('此功能需要 Tauri 桌面运行时（当前为开发模式）')
  }
  return invoke<T>(cmd, args)
}

async function safeListen<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  if (!isTauri) return () => {}
  const { listen } = await import('@tauri-apps/api/event')
  return listen<T>(event, handler)
}
import { useFlowStore } from '../stores/flowStore'
import { useUndo } from '../composables/useUndo'
import { useAutoSave } from '../composables/useAutoSave'
import NodePalette from '../components/flow/NodePalette.vue'
import PropertyPanel from '../components/flow/PropertyPanel.vue'
import PreviewPanel from '../components/flow/PreviewPanel.vue'
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
const wrapperRef = ref<HTMLDivElement | null>(null)
let graph: LGraph
let canvas: LGraphCanvas
let resizeObserver: ResizeObserver | null = null

// ─── 本地状态 ───
const selectedLgNode = ref<LGraphNode | null>(null)
const isRunning = ref(false)
const recording = ref(false)  // 浏览器录制状态
const logs = ref<{ id: number; time: string; text: string; level: string }[]>([])

// ─── ID 映射：store 用 String(lgId)，通过 find 直接查找 ───
/** 通过 store 的 string ID 查找 LiteGraph 节点 */
function findLgNode(storeId: string): LGraphNode | undefined {
  return graph._nodes?.find((n: LGraphNode) => String(n.id) === storeId)
}

// ─── 标记：是否正在从 store 同步到 graph（避免循环更新） ───
let syncingFromStore = false
let lastSyncTime = 0
let pendingRaf = false  // 防止 raf 堆叠
const SYNC_THROTTLE_MS = 100

function throttledSync() {
  if (syncingFromStore) return
  const now = Date.now()
  if (now - lastSyncTime < SYNC_THROTTLE_MS) {
    // 只排一个 raf，避免堆叠
    if (!pendingRaf) {
      pendingRaf = true
      requestAnimationFrame(() => {
        pendingRaf = false
        if (!syncingFromStore) {
          syncGraphToStore()
          undoManager.pushState()
        }
      })
    }
    return
  }
  lastSyncTime = now
  syncGraphToStore()
  undoManager.pushState()
}

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

  // ⚠️ 等 CSS flex 布局完成后再设尺寸、初始化 LGraphCanvas
  // onMounted 时 wrapper 的 clientWidth/Height 可能还是 0
  nextTick(() => {
    requestAnimationFrame(() => {
      if (!canvasRef.value || !wrapperRef.value) return
      const w = wrapperRef.value.clientWidth
      const h = wrapperRef.value.clientHeight
      if (w === 0 || h === 0) {
        // 极端情况：再等一帧
        requestAnimationFrame(() => {
          const w2 = wrapperRef.value!.clientWidth
          const h2 = wrapperRef.value!.clientHeight
          if (w2 > 0 && h2 > 0) initCanvas(w2, h2)
        })
        return
      }
      initCanvas(w, h)
    })
  })

  function initCanvas(w: number, h: number) {
    // CSS 用 absolute 定位铺满 wrapper，属性设内部坐标系
    canvasRef.value!.width = w
    canvasRef.value!.height = h

    // 创建 canvas
    canvas = new LGraphCanvas(canvasRef.value!, graph)

    // 配置 canvas 外观（暗色主题）
    canvas.background_image = ''
    canvas.clear_background = true
    canvas.clear_background_color = '#0d1117'  // 覆盖默认 #222，匹配画布背景
    canvas.render_canvas_border = false
    canvas.render_border = false
    canvas.node_title_color = '#e6edf3'

    // 监听 graph 变化 → 同步到 store
    graph.on_change = () => throttledSync()
    graph.onAfterChange = () => throttledSync()

    // 监听选中变化 → 更新属性面板
    canvas.onSelectionChange = (selectedDict: Record<string, LGraphNode>) => {
      const selected = Object.values(selectedDict)[0]
      selectedLgNode.value = selected instanceof LGraphNode ? selected : null
    }

    // 监听连线变化
    graph.onConnectionChange = () => throttledSync()

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

    // ResizeObserver 动态跟踪 wrapper 尺寸变化
    if (wrapperRef.value) {
      resizeObserver = new ResizeObserver(() => {
        if (canvasRef.value && wrapperRef.value) {
          const w2 = wrapperRef.value.clientWidth
          const h2 = wrapperRef.value.clientHeight
          if (canvasRef.value.width !== w2 || canvasRef.value.height !== h2) {
            canvasRef.value.width = w2
            canvasRef.value.height = h2
            canvas.resize(w2, h2)
          }
        }
      })
      resizeObserver.observe(wrapperRef.value)
    }
  }
})

onUnmounted(() => {
  resizeObserver?.disconnect()
  autoSave.stop()
  document.removeEventListener('keydown', onKeyDown)
})

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
  const linkValues = [...((graph as any)._links?.values() || [])]
  if (linkValues.length === 0) {
    // 快速路径：无 link 时只清理 store 中的旧边
    if (store.edges.length > 0) store.clearEdges?.() || store.edges.forEach(e => store.removeEdge(e.id))
    return
  }

  // 预建 nodeById 索引，后续 O(1) 查找替代 O(n)
  const nodeById = new Map<string, LGraphNode>()
  for (const n of (graph._nodes || [])) {
    if (n) nodeById.set(String(n.id), n)
  }

  const storeEdgeIds = new Set(store.edges.map(e => e.id))
  const linkIds = new Set<string>()

  for (const link of linkValues) {
    const sourceId = String(link.origin_id)
    const targetId = String(link.target_id)
    const srcNode = nodeById.get(sourceId)
    const tgtNode = nodeById.get(targetId)

    const sourceHandle = srcNode?.outputs?.[link.origin_slot]?.name || `out_${link.origin_slot}`
    const targetHandle = tgtNode?.inputs?.[link.target_slot]?.name || `in_${link.target_slot}`
    const edgeId = `e_${sourceId}_${sourceHandle}_${targetId}_${targetHandle}`
    linkIds.add(edgeId)

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
  for (const edge of store.edges) {
    if (!linkIds.has(edge.id)) store.removeEdge(edge.id)
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
  // ComfyUI LiteGraph: DragAndScale 用数组 [x,y] 格式，不是 {x,y} 对象
  const canvasCoords = canvas.convertOffsetToCanvas(
    [event.clientX - rect.left, event.clientY - rect.top],
    [0, 0]
  )
  // 返回的是数组 [x, y]
  const [cx, cy] = canvasCoords

  const node = LiteGraph.createNode(nodeType)
  if (!node) {
    addLog(`⚠ 无法创建节点: ${def.label}`, 'error')
    return
  }

  node.pos = [cx, cy]
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
  addLog(`➕ 添加节点: ${def.label}`)
}

// ─── 属性面板回调 ───
function onUpdateLabel(node: LGraphNode, label: string) {
  node.title = label
  graph.setDirtyCanvas(true)
  store.updateNodeLabel(String(node.id), label)
}

function onUpdateWidget(node: LGraphNode, widgetName: string, value: unknown) {
  const widget = node.widgets?.find(w => w.name === widgetName)
  if (widget) {
    widget.value = value
    graph.setDirtyCanvas(true)
  }
  store.updateNodeConfig(String(node.id), { [widgetName]: value })
}

function onDeleteNode(node: LGraphNode) {
  graph.remove(node)
  if (selectedLgNode.value === node) {
    selectedLgNode.value = null
  }
  store.removeNode(String(node.id))
  addLog(`🗑 删除节点: ${node.title || node.type}`)
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
      await safeInvoke('dag_run_cancel', { runId: currentRunId.value })
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
    const runId = await safeInvoke<string>('dag_run_start', { workflowJson })
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

  const u1 = await safeListen<{ step_id: string; step_name: string; status: string; current_step: number; total_steps: number }>(
    'step-status-update',
    (event) => {
      const { step_id, step_name, status, current_step, total_steps } = event.payload
      store.setNodeStatus(step_id, status as NodeStatus)
      if (status === 'running') addLog(`⏳ [${current_step}/${total_steps}] ${step_name}`)
      else if (status === 'success') addLog(`✅ [${current_step}/${total_steps}] ${step_name}`)
    }
  )
  dagEventUnlisteners.push(u1)

  const u2 = await safeListen<{ run_id: string; status: string }>(
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

  const u3 = await safeListen<{ step_id: string; step_name: string }>(
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

function clearCanvas() {
  if (store.nodes.length > 0 && !confirm('确定清空画布上所有节点和连线？')) return
  graph.clear()
  store.clear()
  selectedLgNode.value = null
  addLog('🗑 画布已清空')
  undoManager.pushState()
}

// ─── 浏览器录制 ───

async function toggleRecording() {
  if (recording.value) {
    // 停止录制
    try {
      const result = await safeInvoke<{ name: string; nodes: unknown[]; edges: unknown[] }>('browser_recording_stop')
      recording.value = false
      if (result && result.nodes && result.nodes.length > 0) {
        store.load(result)
        loadFromStore()
        addLog(`🎬 录制已停止，生成 ${result.nodes.length} 个节点`)
        fitView()
      } else {
        addLog('🎬 录制已停止，未捕获到操作')
      }
    } catch (e: any) {
      recording.value = false
      addLog(`❌ 停止录制失败: ${e}`, 'error')
    }
  } else {
    try {
      await safeInvoke('browser_recording_start')
      recording.value = true
      addLog('🔴 录制已开始 — 请在浏览器中操作')
    } catch (e: any) {
      addLog(`❌ 开始录制失败: ${e}`, 'error')
    }
  }
}

async function pickElement() {
  try {
    const result = await safeInvoke<{ selector?: string }>('browser_pick_element')
    if (result?.selector) {
      // 如果选中了节点且有 selector widget，自动填入
      if (selectedLgNode.value) {
        const selWidget = selectedLgNode.value.widgets?.find(
          w => w.name === 'selector'
        )
        if (selWidget) {
          selWidget.value = result.selector
          graph.setDirtyCanvas(true)
          store.updateNodeConfig(String(selectedLgNode.value.id), { selector: result.selector })
        }
      }
      addLog(`🎯 已拾取元素: ${result.selector}`)
    }
  } catch (e: any) {
    addLog(`❌ 元素拾取失败: ${e}`, 'error')
  }
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
  if ((e.key === 'Delete' || e.key === 'Backspace') && selectedLgNode.value) {
    // 排除输入框内的按键
    if (document.activeElement?.tagName === 'INPUT' || document.activeElement?.tagName === 'TEXTAREA') return
    e.preventDefault()
    onDeleteNode(selectedLgNode.value)
    return
  }
  // Escape — 取消选择
  if (e.key === 'Escape') {
    canvas.deselectAllNodes()
    selectedLgNode.value = null
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

.toolbar-btn.recording-active {
  background: #da3633;
  border-color: #f85149;
  color: #fff;
  animation: pulse 1.5s ease-in-out infinite;
}

.toolbar-btn.btn-pick {
  background: #1f6feb;
  border-color: #388bfd;
  color: #fff;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
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

/* ─── 右侧双面板 ─── */
.right-panels {
  display: flex;
  flex-direction: column;
  min-width: 280px;
  max-width: 340px;
  border-left: 1px solid #30363d;
  overflow: hidden;
}

.right-panels > :first-child {
  flex: 1 1 auto;
  min-height: 0;
  overflow: auto;
}

.right-panels > :last-child {
  flex: 0 0 auto;
}

.panel-divider {
  height: 1px;
  background: #21262d;
  flex-shrink: 0;
}

/* 覆盖 LiteGraph 默认 .lgraphcanvas 的 max-height 限制 */
.canvas-wrapper .litegraph-canvas {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  max-height: none !important;
  background: #0d1117 !important;
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
