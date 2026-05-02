<template>
  <!-- ═══════════ 叠加层架构（Canvas 全屏 + UI 浮层）—— 已验证可用 ═══════════ -->
  <div class="editor-app">
    <!-- Canvas 全屏 -->
    <canvas ref="canvasRef" class="editor-canvas" />

    <!-- 空画布提示 -->
    <div v-if="store.nodes.length === 0" class="empty-canvas">
      <div class="empty-icon">🎨</div>
      <div class="empty-title">空画布</div>
      <div class="empty-hint">双击空白画布搜索节点，或右键查看更多操作</div>
    </div>

    <!-- UI 浮层 -->
    <div class="ui-overlay">
      <div class="overlay-top">
        <WorkflowTabs @add="onTabAdd" />
        <TopMenuSection
          :name="store.workflowName" :node-count="store.nodeCount" :edge-count="store.edgeCount"
          :dirty="store.dirty" :running="isRunning" :recording="recording"
          :disable-run="store.nodes.length === 0"
          @run="runAll" @step="runSingle" @stop="stopRun"
          @record="toggleRecording" @pick="pickElement"
          @import="importWorkflow" @export="exportWorkflow"
          @clear="clearCanvas" @save="onSaveWorkflow"
        />
      </div>

      <div class="overlay-main">
        <div class="overlay-left">
          <SideToolbar
            :show-console="showConsole"
            @toggle-dashboard="showDashboard = !showDashboard"
            @toggle-palette="showPalette = !showPalette"
            @toggle-history="showHistory = !showHistory"
            @toggle-console="showConsole = !showConsole"
            @toggle-settings="showSettings = !showSettings"
            @toggle-schedule="showSchedule = !showSchedule"
          />
        </div>
        <div class="overlay-center" />
      </div>

      <div v-if="showConsole && logs.length > 0" class="overlay-bottom">
        <div class="console-header">
          <span>📋 执行日志</span>
          <button class="console-clear" @click="logs = []">清除</button>
        </div>
        <div class="console-body">
          <div v-for="log in logs" :key="log.id" :class="['log-line', log.level]">
            <span class="log-time">{{ log.time }}</span>
            <span class="log-text">{{ log.text }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- MiniMap -->
    <MiniMap v-if="canvasReady" :canvas="canvas" :graph="graph" :visible="showMiniMap" />

    <!-- 右键菜单 -->
    <CanvasContextMenu
      :visible="contextMenuVisible" :x="contextMenuPos.x" :y="contextMenuPos.y"
      :items="contextMenuItems" @close="contextMenuVisible = false"
    />

    <!-- 搜索弹窗 -->
    <CanvasSearchPopover
      :visible="searchVisible" :x="searchPos.x" :y="searchPos.y"
      :graph="graph" @close="searchVisible = false" @node-added="onSearchNodeAdded"
    />
  </div>

  <!-- Import -->
  <input ref="importInputRef" type="file" accept=".json" style="display:none" @change="onImportFile" />

  <!-- 节点库 -->
  <FloatingPanel :visible="showPalette" title="节点库" :width="240" :height="450" @close="showPalette = false">
    <NodePalette @add-node="onAddNodeFromPalette" />
  </FloatingPanel>

  <!-- 属性面板 -->
  <FloatingPanel
    :visible="!!selectedLgNode" :title="selectedLgNode?.title || selectedLgNode?.type || '属性'"
    :width="300" :height="420" @close="onDeselectNode"
  >
    <PropertyPanel :key="widgetVersion" :lg-node="selectedLgNode"
      :output="selectedLgNode ? store.stepOutputs[String(selectedLgNode.id)] : undefined"
      :error="selectedLgNode ? store.nodeStatuses[String(selectedLgNode.id)] === 'error' ? '执行失败' : undefined : undefined"
      :duration="undefined"
      @update-label="onUpdateLabel" @update-widget="onUpdateWidget" @delete="onDeleteNode"
    />
  </FloatingPanel>

  <!-- 工作流列表 -->
  <FloatingPanel :visible="showDashboard" title="📋 工作流" :width="720" :height="560" @close="showDashboard = false">
    <Dashboard @open-workflow="onOpenWorkflow" @create-from-template="onCreateFromTemplate" @navigate="onDashboardNavigate" />
  </FloatingPanel>

  <!-- 设置 -->
  <FloatingPanel :visible="showSettings" title="⚙️ 设置" :width="560" :height="500" @close="showSettings = false">
    <Settings />
  </FloatingPanel>

  <!-- 运行历史 -->
  <FloatingPanel :visible="showHistory" title="📊 运行历史" :width="640" :height="500" @close="showHistory = false">
    <RunHistory />
  </FloatingPanel>

  <!-- 定时计划 -->
  <FloatingPanel :visible="showSchedule" title="📅 定时计划" :width="560" :height="420" @close="showSchedule = false">
    <ScheduleSection :schedules="scheduleList" :loading="scheduleLoading"
      @toggle-schedule="onToggleSchedule" @delete-schedule="onDeleteSchedule"
      @edit-schedule="(s: any) => scheduleDialogRef?.open(workflowListForSchedule, s)"
      @new-schedule="scheduleDialogRef?.open(workflowListForSchedule)"
    />
    <ScheduleDialog ref="scheduleDialogRef" @saved="loadSchedules" />
  </FloatingPanel>

  <!-- Preview -->
  <Teleport to="body">
    <div v-if="previewVisible" class="preview-overlay" @mousedown.self="previewVisible = false">
      <div ref="previewPopupRef" class="preview-popup" :style="previewStyle" @mousedown.stop>
        <div class="preview-popup-header" @mousedown="onPreviewDragStart">
          <span>{{ previewTitle }}</span>
          <div class="preview-popup-actions"><button @click="previewVisible = false">✕</button></div>
        </div>
        <div class="preview-popup-body"><PreviewPanel :lg-node="selectedLgNode" @update-widget="onUpdateWidget" /></div>
        <div class="preview-resize-handle" @mousedown="onPreviewResizeStart"></div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
/* ═══════════ 叠加层架构（Canvas 全屏 + UI 浮层） ═══════════ */

.editor-app {
  position: relative;
  width: 100vw; height: 100vh;
  overflow: hidden;
  background: var(--color-bg);
}

/* Canvas 全视口背景 */
.editor-canvas {
  position: fixed;
  inset: 0;
  z-index: 0;
  display: block;
  background: var(--color-bg);
  touch-action: none;
  user-select: none;
  outline: none;
}

/* 空画布提示 */
.empty-canvas {
  position: fixed;
  top: 50%; left: 50%;
  transform: translate(-50%, -50%);
  text-align: center;
  z-index: 10;
  pointer-events: none;
  user-select: none;
}
.empty-icon { font-size: 48px; margin-bottom: 12px; opacity: 0.4; }
.empty-title { font-size: 18px; font-weight: 600; color: #8b949e; margin-bottom: 6px; }
.empty-hint { font-size: 13px; color: #484f58; }

/* UI 浮层 */
.ui-overlay {
  position: fixed;
  inset: 0;
  z-index: 999;
  pointer-events: none;
  display: flex;
  flex-direction: column;
}

.overlay-top {
  pointer-events: auto;
  flex-shrink: 0;
}

.overlay-main {
  flex: 1;
  display: flex;
  min-height: 0;
}

.overlay-left {
  pointer-events: auto;
  display: flex;
  flex-direction: row;
  flex-shrink: 0;
}

.overlay-center {
  flex: 1;
  pointer-events: none;
  min-width: 0;
}

/* 底部控制台 */
.overlay-bottom {
  pointer-events: auto;
  height: 140px;
  background: var(--color-surface);
  border-top: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
}

.console-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 4px 12px; border-bottom: 1px solid #21262d;
  font-size: 11px; font-weight: 600; color: #8b949e; flex-shrink: 0;
}
.console-clear {
  padding: 1px 8px; background: none; border: 1px solid #30363d;
  border-radius: 4px; color: #8b949e; font-size: 10px; cursor: pointer;
}
.console-clear:hover { background: #21262d; color: #c9d1d9; }
.console-body {
  flex: 1; overflow-y: auto; padding: 4px 12px;
  font-family: monospace; font-size: 11px; line-height: 1.5;
}
.log-line { display: flex; gap: 8px; padding: 1px 0; }
.log-time { color: #484f58; flex-shrink: 0; }
.log-text { word-break: break-all; }
.log-line.info { color: #8b949e; }
.log-line.error { color: #f85149; }
.log-line.warn { color: #d29922; }
.log-line.success { color: #3fb950; }

/* Preview */
.preview-overlay {
  position: fixed; inset: 0; z-index: 9999;
  background: rgba(0,0,0,0.3); pointer-events: auto;
}
.preview-popup {
  position: fixed; background: #161b22; border: 1px solid #30363d;
  border-radius: 8px; box-shadow: 0 8px 32px rgba(0,0,0,0.6);
  display: flex; flex-direction: column; overflow: hidden;
  min-width: 320px; min-height: 200px;
}
.preview-popup-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 6px 10px; background: #21262d; border-bottom: 1px solid #30363d;
  cursor: move; font-size: 12px; font-weight: 600; color: #c9d1d9; flex-shrink: 0; user-select: none;
}
.preview-popup-actions button {
  background: none; border: none; color: #8b949e;
  cursor: pointer; font-size: 14px; padding: 2px 6px; border-radius: 4px;
}
.preview-popup-actions button:hover { background: #30363d; color: #f85149; }
.preview-popup-body { flex: 1; overflow: auto; padding: 0; }
.preview-resize-handle {
  position: absolute; bottom: 0; right: 0; width: 16px; height: 16px;
  cursor: nwse-resize;
  background: linear-gradient(135deg, transparent 50%, #30363d 50%);
  border-radius: 0 0 8px 0;
}
</style>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick, toRaw } from 'vue'
import { LGraph, LGraphCanvas, LiteGraph, LGraphNode } from '@comfyorg/litegraph'
import '@comfyorg/litegraph/style.css'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { safeInvoke, safeListen } from '../utils/tauri'
import { useFlowStore } from '../stores/flowStore'
import { useTabStore } from '../stores/tabStore'
import { useUndo } from '../composables/useUndo'
import { useAutoSave } from '../composables/useAutoSave'
import NodePalette from '../components/flow/NodePalette.vue'
import PropertyPanel from '../components/flow/PropertyPanel.vue'
import PreviewPanel from '../components/flow/PreviewPanel.vue'
import SideToolbar from '../components/SideToolbar.vue'
import TopMenuSection from '../components/TopMenuSection.vue'
import WorkflowTabs from '../components/WorkflowTabs.vue'
import FloatingPanel from '../components/FloatingPanel.vue'
import ScheduleSection from '../components/ScheduleSection.vue'
import ScheduleDialog from '../components/ScheduleDialog.vue'
import Dashboard from './Dashboard.vue'
import Settings from './Settings.vue'
import RunHistory from './RunHistory.vue'
import MiniMap from '../components/MiniMap.vue'
import CanvasContextMenu from '../components/CanvasContextMenu.vue'
import CanvasSearchPopover from '../components/CanvasSearchPopover.vue'
import type { ContextMenuItem } from '../components/CanvasContextMenu.vue'
import { registerAllNodes } from '../nodes/litegraph-nodes'
import { getNodeDef } from '../components/flow/pinTypes'
import type { FlowNode, NodeStatus, NodeDefinition } from '../components/flow/pinTypes'

// ─── Props ───
const store = useFlowStore()
const tabStore = useTabStore()

// ─── 撤销/重做 + 自动保存 ───
const undoManager = useUndo()
const autoSave = useAutoSave(30000) // 30 秒自动保存

// ─── LiteGraph 引用 ───
const canvasRef = ref<HTMLCanvasElement | null>(null)
let graph: LGraph
let canvas: LGraphCanvas
/** 防止卸载后 raf 回调执行 */
let _canvasMounted = false
let _resizeObserver: ResizeObserver | null = null

// ─── 本地状态 ───
const selectedLgNode = ref<LGraphNode | null>(null)
const isRunning = ref(false)
const recording = ref(false)  // 浏览器录制状态
const widgetVersion = ref(0)  // 递增触发 PropertyPanel 重渲染
const importInputRef = ref<HTMLInputElement | null>(null)
const previewVisible = ref(false)  // 预览弹窗
const previewPopupRef = ref<HTMLDivElement | null>(null)
const previewPos = ref({ x: 100, y: 100 })
const previewSize = ref({ w: 560, h: 420 })
const previewTitle = ref('预览')
let previewDragging = false, previewDragOff = { x: 0, y: 0 }
let previewResizing = false, previewResizeStart = { x: 0, y: 0, w: 0, h: 0 }

const previewStyle = computed(() => ({
  left: `${previewPos.value.x}px`,
  top: `${previewPos.value.y}px`,
  width: `${previewSize.value.w}px`,
  height: `${previewSize.value.h}px`,
}))

function onPreviewDragStart(e: MouseEvent) {
  previewDragging = true
  previewDragOff = { x: e.clientX - previewPos.value.x, y: e.clientY - previewPos.value.y }
  document.addEventListener('mousemove', onPreviewDrag)
  document.addEventListener('mouseup', onPreviewDragEnd)
}
function onPreviewDrag(e: MouseEvent) {
  if (!previewDragging) return
  previewPos.value = { x: e.clientX - previewDragOff.x, y: e.clientY - previewDragOff.y }
}
function onPreviewDragEnd() {
  previewDragging = false
  document.removeEventListener('mousemove', onPreviewDrag)
  document.removeEventListener('mouseup', onPreviewDragEnd)
}

function onPreviewResizeStart(e: MouseEvent) {
  e.preventDefault()
  e.stopPropagation()
  previewResizing = true
  previewResizeStart = { x: e.clientX, y: e.clientY, w: previewSize.value.w, h: previewSize.value.h }
  document.addEventListener('mousemove', onPreviewResize)
  document.addEventListener('mouseup', onPreviewResizeEnd)
}
function onPreviewResize(e: MouseEvent) {
  if (!previewResizing) return
  previewSize.value = {
    w: Math.max(320, previewResizeStart.w + e.clientX - previewResizeStart.x),
    h: Math.max(200, previewResizeStart.h + e.clientY - previewResizeStart.y),
  }
}
function onPreviewResizeEnd() {
  previewResizing = false
  document.removeEventListener('mousemove', onPreviewResize)
  document.removeEventListener('mouseup', onPreviewResizeEnd)
}

// 选中文件/浏览器节点时自动弹出预览
const previewTypes = new Set(['browser_navigate', 'browser_click', 'browser_extract', 'browser_screenshot',
  'browser_evaluate', 'browser', 'excel', 'word', 'file', 'file_save', 'http', 'web_scrape'])
watch(selectedLgNode, (node) => {
  if (node && previewTypes.has(node.type)) {
    previewTitle.value = node.title || node.type
    previewVisible.value = true
  }
})

// 标签页切换：保存当前 → 加载目标
watch(() => tabStore.activeTabId, (newId, oldId) => {
  if (!newId || newId === oldId || !graph) return
  if (oldId) saveCurrentTab()
  loadTabData(newId)
})

// 名称变化同步到 tab
watch(() => store.workflowName, (name) => {
  tabStore.renameTab(tabStore.activeTabId || '', name)
})

// dirty 状态同步到 tab
watch(() => store.dirty, (d) => {
  tabStore.setDirty(tabStore.activeTabId || '', d)
})

// 标签关闭时清理缓存
watch(() => tabStore.tabs.length, (len, oldLen) => {
  if (len < (oldLen ?? 0)) {
    // 有标签被删除，清理不在 store 中的缓存
    const activeIds = new Set(tabStore.tabs.map(t => t.id))
    for (const key of tabDataCache.keys()) {
      if (!activeIds.has(key)) tabDataCache.delete(key)
    }
  }
})
const logs = ref<{ id: number; time: string; text: string; level: string }[]>([])

// ─── 鼠标位置追踪（供 ghost placement 用） ───
let _lastMousePos = { x: 0, y: 0 }

// ─── 面板状态 ───
const showConsole = ref(false)
const showPalette = ref(true)
const showDashboard = ref(false)
const showSettings = ref(false)
const showHistory = ref(false)
const showSchedule = ref(false)

// ─── P0：MiniMap / 右键菜单 / 搜索弹窗 ───
const canvasReady = ref(false)  // initCanvasReal 完成后设为 true
const showMiniMap = ref(true)
const contextMenuVisible = ref(false)
const contextMenuPos = ref({ x: 0, y: 0 })
const searchVisible = ref(false)
const searchPos = ref({ x: 0, y: 0 })

// ─── 右键菜单项 ───
const contextMenuItems = computed<ContextMenuItem[]>(() => {
  const items: ContextMenuItem[] = [
    { label: '🔍 搜索节点', action: () => {
      searchVisible.value = true
      searchPos.value = { ...contextMenuPos.value }
    }},
    { label: 'separator', action: () => {}, divider: true },
    { label: '📋 复制', action: () => {
      // TODO: 复制选中节点
      canvas?.copyToClipboard?.()
      addLog('📋 已复制', 'info')
    }, disabled: !selectedLgNode.value },
    { label: '📌 粘贴', action: () => {
      // paste at menu position
      const rect = canvasRef.value?.getBoundingClientRect()
      if (!rect) return
      const worldPos = canvas?.convertOffsetToCanvas?.(
        [contextMenuPos.value.x - rect.left, contextMenuPos.value.y - rect.top], [0, 0]
      )
      canvas?.pasteFromClipboard?.(worldPos)
      addLog('📌 已粘贴', 'info')
    }},
    { label: '🗑 删除选中', action: () => {
      if (selectedLgNode.value) onDeleteNode(selectedLgNode.value)
    }, disabled: !selectedLgNode.value },
    { label: 'separator', action: () => {}, divider: true },
    { label: '📐 适应视图', action: () => { fitView() }},
    { label: '🗑 清空画布', action: () => { clearCanvas() }},
  ]
  return items
})

// ─── 标签页数据缓存 ───
const tabDataCache = new Map<string, { nodes: any[]; edges: any[]; name: string }>()

/** 保存当前画布到标签页缓存 */
function saveCurrentTab() {
  const tid = tabStore.activeTabId
  if (!tid) return
  tabDataCache.set(tid, {
    nodes: store.nodes.map(n => ({ ...n })),
    edges: store.edges.map(e => ({ ...e })),
    name: store.workflowName,
  })
}

/** 从标签页缓存加载到画布 */
function loadTabData(tabId: string) {
  const data = tabDataCache.get(tabId)
  if (data) {
    store.$patch({
      workflowName: data.name,
      nodes: data.nodes,
      edges: data.edges,
    })
  } else {
    store.$patch({ workflowName: '未命名', nodes: [], edges: [] })
  }
  // 重新渲染画布
  loadFromStore()
  undoManager.init()  // 重置撤销历史
}

function onTabAdd() {
  saveCurrentTab()
  const newId = tabStore.addTab('工作流 ' + tabStore.tabCount)
  tabDataCache.set(newId, { nodes: [], edges: [], name: '工作流 ' + tabStore.tabCount })
  loadTabData(newId)
}

// ─── Dashboard 浮层事件处理 ───

/** 加载已有工作流到画布 */
async function onOpenWorkflow(id?: string) {
  showDashboard.value = false
  if (!id) {
    // 新建空工作流
    saveCurrentTab()
    const newId = tabStore.addTab('工作流 ' + tabStore.tabCount)
    tabDataCache.set(newId, { nodes: [], edges: [], name: '工作流 ' + tabStore.tabCount })
    loadTabData(newId)
    return
  }
  // 加载已有工作流：workflow_get → YAML → import_workflow → JSON
  try {
    const meta = await safeInvoke<any>('workflow_get', { id })
    if (!meta) { addLog(`❌ 工作流 ${id} 不存在`, 'error'); return }
    const parsed = await safeInvoke<any>('import_workflow', { yamlContent: meta.yaml })
    if (!parsed?.nodes) { addLog(`❌ 工作流 ${id} 解析失败`, 'error'); return }
    store.load({
      name: meta.name || '未命名',
      nodes: parsed.nodes,
      edges: parsed.edges || [],
    })
    store.setWorkflowId(id)
    loadFromStore()
    undoManager.init()
    addLog(`📂 已加载「${meta.name || id}」`)
  } catch (e: unknown) {
    addLog(`❌ 加载失败: ${(e as Error).message || e}`, 'error')
  }
}

/** 从模板创建工作流 */
function onCreateFromTemplate(tpl: { id: string; name: string; nodes: unknown[]; edges: unknown[] }) {
  showDashboard.value = false
  store.load({
    name: tpl.name,
    nodes: tpl.nodes as any[],
    edges: tpl.edges as any[],
  })
  store.templateSource = tpl.id
  loadFromStore()
  undoManager.init()
}

/** Dashboard 内导航请求（历史/设置）→ 关闭 Dashboard 打开对应面板 */
function onDashboardNavigate(path: string) {
  showDashboard.value = false
  if (path === '/history') { showHistory.value = true }
  if (path === '/settings') { showSettings.value = true }
}

// ─── 定时计划 ───
const scheduleList = ref<any[]>([])
const scheduleLoading = ref(false)
const scheduleDialogRef = ref<InstanceType<typeof ScheduleDialog> | null>(null)
const workflowListForSchedule = ref<{ id: string; name: string }[]>([])

async function loadSchedules() {
  scheduleLoading.value = true
  try {
    scheduleList.value = await safeInvoke<any[]>('schedule_list')
  } catch (e) {
    console.warn('加载计划失败:', e)
  } finally { scheduleLoading.value = false }
}

async function onToggleSchedule(s: any) {
  try {
    await safeInvoke('schedule_update', { id: s.id, enabled: !s.enabled })
    s.enabled = !s.enabled
    addLog(`计划已${s.enabled ? '启用' : '禁用'}`, 'info')
  } catch (e: unknown) {
    addLog(`❌ ${(e as Error).message || e}`, 'error')
  }
}

async function onDeleteSchedule(s: any) {
  if (!confirm(`确定删除此定时计划？`)) return
  try {
    await safeInvoke('schedule_delete', { id: s.id })
    scheduleList.value = scheduleList.value.filter(x => x.id !== s.id)
    addLog('计划已删除', 'info')
  } catch (e: unknown) {
    addLog(`❌ ${(e as Error).message || e}`, 'error')
  }
}

// 监听 schedule 面板打开时加载数据
watch(showSchedule, (v) => {
  if (v) {
    loadSchedules()
    // 加载工作流列表供 schedule dialog 用
    safeInvoke<any[]>('workflow_list').then(list => {
      workflowListForSchedule.value = (list || []).map((w: any) => ({ id: w.id, name: w.name }))
    }).catch(() => {})
  }
})

// ─── 保存工作流 ───
async function onSaveWorkflow() {
  if (store.nodes.length === 0) {
    addLog('⚠ 画布为空，无需保存', 'warn')
    return
  }
  try {
    const id = store.workflowId
    if (id) {
      // 更新已有工作流
      const yaml = JSON.stringify({ name: store.workflowName, nodes: store.nodes, edges: store.edges })
      await safeInvoke('workflow_save_yaml', { id, yaml })
      addLog(`💾 已保存「${store.workflowName}」`)
    } else {
      // 新建工作流
      const yaml = JSON.stringify({ name: store.workflowName, nodes: store.nodes, edges: store.edges })
      const newId = await safeInvoke<string>('workflow_create', { name: store.workflowName, description: '' })
      if (newId) {
        await safeInvoke('workflow_save_yaml', { id: newId, yaml })
        store.setWorkflowId(newId)
        addLog(`💾 已保存「${store.workflowName}」`)
      }
    }
    store.dirty = false
  } catch (e: unknown) {
    addLog(`❌ 保存失败: ${(e as Error).message || e}`, 'error')
  }
}

// ─── ID 映射：store 用 String(lgId)，通过 find 直接查找 ───
/** 浅比较两个 record 的键值是否不同 */
function shallowDiff(a: Record<string, unknown>, b: Record<string, unknown>): boolean {
  const keysA = Object.keys(a)
  const keysB = Object.keys(b)
  if (keysA.length !== keysB.length) return true
  for (const k of keysA) {
    if (a[k] !== b[k]) return true
  }
  return false
}
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
        if (!_canvasMounted) return
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

  // ⚠️ 等待 DOM 就绪后初始化 — 全视口 canvas，无需 wrapper 尺寸
  nextTick(() => {
    requestAnimationFrame(() => {
      if (!canvasRef.value) return
      initCanvas()
    })
  })

  function initCanvas() {
    const c = canvasRef.value!
    // 全视口：先清固定分辨率 → 取 CSS 实际尺寸 → DPI 缩放
    c.width = c.height = NaN as unknown as number
    const { width, height } = c.getBoundingClientRect()
    if (width === 0 || height === 0) {
      // 极端 case：再等一帧
      requestAnimationFrame(() => {
        if (!canvasRef.value) return
        const b = canvasRef.value.getBoundingClientRect()
        if (b.width > 0 && b.height > 0) {
          initCanvasReal(b.width, b.height)
        }
      })
      return
    }
    initCanvasReal(width, height)
  }

  function initCanvasReal(w: number, h: number) {
    const c = canvasRef.value!
    // 设置 canvas 分辨率（DPI 缩放）
    const dpr = Math.max(window.devicePixelRatio || 1, 1)
    c.width = Math.round(w * dpr)
    c.height = Math.round(h * dpr)
    c.getContext('2d')?.scale(dpr, dpr)

    // 创建 LGraphCanvas
    canvas = new LGraphCanvas(c, graph)

    // 配置 canvas 外观（暗色主题 + 网格线）
    canvas.background_image = ''
    canvas.clear_background = true
    canvas.clear_background_color = '#0d1117'
    canvas.render_canvas_border = false
    canvas.render_border = false
    canvas.node_title_color = '#e6edf3'

    // 画布网格背景
    canvas.onDrawBackground = (ctx: CanvasRenderingContext2D) => {
      const ds = (canvas as any).ds
      const scale = ds?.scale || 1
      const offset = ds?.offset || [0, 0]
      const gridSize = 40 * scale
      const alpha = Math.max(0.08, Math.min(0.15, scale * 0.06))

      ctx.strokeStyle = `rgba(255, 255, 255, ${alpha})`
      ctx.lineWidth = 1

      const startX = Math.floor(-offset[0] / gridSize) * gridSize
      const startY = Math.floor(-offset[1] / gridSize) * gridSize
      const endX = offset[0] + canvasRef.value!.width / scale + gridSize
      const endY = offset[1] + canvasRef.value!.height / scale + gridSize

      ctx.beginPath()
      for (let x = startX; x <= endX; x += gridSize) {
        ctx.moveTo(x, startY)
        ctx.lineTo(x, endY)
      }
      for (let y = startY; y <= endY; y += gridSize) {
        ctx.moveTo(startX, y)
        ctx.lineTo(endX, y)
      }
      ctx.stroke()
    }

    // 显式开启节点拖拽（WebView2 中默认可能不生效）
    canvas.allow_dragnodes = true
    canvas.allow_dragcanvas = true
    canvas.allow_interaction = true
    // 防止 WebView2 手势冲突
    canvasRef.value!.style.touchAction = 'none'

    // Canvas resize — 采用 ComfyUI 方式：先清 NaN → 取 CSS 尺寸 → 设 DPI 缩放分辨率
    const resizeHandler = () => {
      if (!canvasRef.value || !canvas) return
      const c = canvasRef.value
      c.width = c.height = NaN as unknown as number  // 清固定分辨率，让 CSS 100% 生效
      const { width, height } = c.getBoundingClientRect()
      if (width === 0 || height === 0) return
      const dpr = Math.max(window.devicePixelRatio || 1, 1)
      c.width = Math.round(width * dpr)
      c.height = Math.round(height * dpr)
      c.getContext('2d')?.scale(dpr, dpr)
      canvas.draw(true, true)
    }
    _resizeObserver = new ResizeObserver(resizeHandler)
    _resizeObserver.observe(canvasRef.value!)
    // 初始化时立即触发一次（确保尺寸正确）
    resizeHandler()

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

    // 启动时恢复自动保存或从 store 加载
    if (store.nodes.length > 0) {
      loadFromStore()
      addLog(`📋 已加载「${store.workflowName}」(${store.nodeCount} 节点)`, 'info')
    } else {
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
    _canvasMounted = true
    canvasReady.value = true

    // Canvas mousemove — 追踪鼠标位置供 ghost placement 使用
    canvasRef.value!.addEventListener('mousemove', (e) => {
      _lastMousePos = { x: e.clientX, y: e.clientY }
    })

    // 右键菜单
    canvasRef.value!.addEventListener('contextmenu', (e: MouseEvent) => {
      e.preventDefault()
      contextMenuPos.value = { x: e.clientX, y: e.clientY }
      contextMenuVisible.value = true
    })

    // 双击空白画布 — 搜索弹窗
    canvasRef.value!.addEventListener('dblclick', (e: MouseEvent) => {
      const target = e.target as HTMLElement
      if (target.tagName !== 'CANVAS') return
      setTimeout(() => {
        if (!selectedLgNode.value) {
          searchPos.value = { x: e.clientX, y: e.clientY }
          searchVisible.value = true
        }
      }, 10)
    })

    // 初始化第一个标签页
    const tid = tabStore.ensureTab()
    if (!tabDataCache.has(tid)) {
      tabDataCache.set(tid, { nodes: [], edges: [], name: store.workflowName || '工作流 1' })
    }
  }
})

onUnmounted(() => {
  _canvasMounted = false
  autoSave.stop()
  document.removeEventListener('keydown', onKeyDown)
  if (_resizeObserver) {
    _resizeObserver.disconnect()
    _resizeObserver = null
  }
  // 清理 graph/canvas 回调
  if (graph) {
    graph.on_change = undefined
    graph.onAfterChange = undefined
    graph.onConnectionChange = undefined
  }
  if (canvas) {
    canvas.onSelectionChange = undefined
    canvas.onDrawBackground = undefined
  }
  // 清理 DAG 事件监听器
  cleanupDagListeners()
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
        // 浅比较 config 键值（避免 JSON.stringify 大对象性能问题）
        const configChanged = shallowDiff(existing.config, config)

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
    if (store.edges.length > 0) store.edges.forEach(e => store.removeEdge(e.id))
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
  // 兼容端口名不匹配：先精确匹配，失败则回退到第一个输出/输入端口
  for (const edge of store.edges) {
    const sourceNode = oldToNew.get(edge.source)
    const targetNode = oldToNew.get(edge.target)
    if (!sourceNode || !targetNode) {
      addLog(`⚠ 连线跳过: 节点 ${edge.source}→${edge.target} 不存在`, 'warn')
      continue
    }

    let sourceSlot = sourceNode.outputs?.findIndex((o: any) => o.name === edge.sourceHandle)
    let targetSlot = targetNode.inputs?.findIndex((i: any) => i.name === edge.targetHandle)

    // fallback: 端口名不匹配时用第一个可用端口（LiteGraph 节点通常只有一个）
    if (sourceSlot < 0 && sourceNode.outputs && sourceNode.outputs.length > 0) {
      sourceSlot = 0
    }
    if (targetSlot < 0 && targetNode.inputs && targetNode.inputs.length > 0) {
      targetSlot = 0
    }

    if (sourceSlot >= 0 && targetSlot >= 0) {
      sourceNode.connect(sourceSlot, targetNode, targetSlot)
    } else {
      addLog(`⚠ 连线失败: ${sourceNode.type}[${edge.sourceHandle}]→${targetNode.type}[${edge.targetHandle}] 无可匹配端口`, 'warn')
    }
  }

  // 不再需要 oldToNew 映射 — syncGraphToStore 会统一用 LiteGraph 整数 ID 重写 store
  syncingFromStore = false
  // 重新同步 store，将 LiteGraph 生成的整数 ID 写回
  syncGraphToStore()
  store.dirty = false
  // 确保交互属性
  canvas.allow_dragnodes = true
  canvas.allow_dragcanvas = true
  canvas.allow_interaction = true
  // 强制刷新画布，清除可能的残留渲染（解决隐形窗口/鬼影连线问题）
  canvas.setDirty(true, true)
  // 自适应视图使所有节点可见
  nextTick(() => fitView())
}

// ─── Ghost placement（对齐 ComfyUI：双击节点库 → 创建 ghost 节点跟随鼠标 → 点击落位） ───
function onAddNodeFromPalette(def: NodeDefinition) {
  showPalette.value = false
  if (!canvasRef.value || !canvas) return

  const node = LiteGraph.createNode(def.type)
  if (!node) {
    addLog(`⚠ 无法创建节点: ${def.label}`, 'error')
    return
  }

  node.title = def.label
  // 设置默认 config 到 widgets
  if (def.defaultConfig && node.widgets) {
    for (const w of node.widgets) {
      if (def.defaultConfig[w.name] !== undefined) {
        w.value = def.defaultConfig[w.name]
      }
    }
  }

  // Ghost placement：节点跟随鼠标，点击落位，Esc 取消
  const event = new MouseEvent('mousemove', {
    clientX: _lastMousePos.x,
    clientY: _lastMousePos.y,
  })
  graph.add(node, { ghost: true, dragEvent: event })
  addLog(`👻 放置节点: ${def.label}（点击画布落位，Esc 取消）`)
}

// ─── 搜索弹窗添加节点 ───
function onSearchNodeAdded(def: NodeDefinition) {
  if (!canvasRef.value || !canvas) return

  const node = LiteGraph.createNode(def.type)
  if (!node) {
    addLog(`⚠ 无法创建节点: ${def.label}`, 'error')
    return
  }

  node.title = def.label
  if (def.defaultConfig && node.widgets) {
    for (const w of node.widgets) {
      if (def.defaultConfig[w.name] !== undefined) {
        w.value = def.defaultConfig[w.name]
      }
    }
  }

  // 非 ghost 模式：直接放到搜索弹窗位置（双击位置）
  const rect = canvasRef.value.getBoundingClientRect()
  const worldPos = canvas.convertOffsetToCanvas?.(
    [searchPos.value.x - rect.left, searchPos.value.y - rect.top],
    [0, 0]
  )
  if (worldPos) {
    node.pos = [worldPos[0], worldPos[1]]
  }
  graph.add(node)
  addLog(`✅ 添加节点: ${def.label}`)
}

// ─── 属性面板回调 ───
function onUpdateLabel(node: LGraphNode, label: string) {
  const raw = toRaw(node)
  raw.title = label
  graph.setDirtyCanvas(true)
  store.updateNodeLabel(String(raw.id), label)
}

function onUpdateWidget(node: LGraphNode, widgetName: string, value: unknown) {
  // toRaw 绕过 Vue reactive proxy，ComfyUI LiteGraph 用 ES private 字段
  const raw = toRaw(node)
  const widget = raw.widgets?.find(w => w.name === widgetName)
  if (widget) {
    widget.value = value
    graph.setDirtyCanvas(true)
  }
  store.updateNodeConfig(String(raw.id), { [widgetName]: value })
  widgetVersion.value++
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

function onDeselectNode() {
  canvas.deselectAllNodes()
  selectedLgNode.value = null
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

// ─── 导入导出 ───
function importWorkflow() {
  importInputRef.value?.click()
}

async function onImportFile(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return

  try {
    const text = await file.text()
    const data = JSON.parse(text)
    store.load(data)
    loadFromStore()
    addLog(`📥 已导入: ${data.name || file.name}`, 'info')
    fitView()
  } catch (e: any) {
    addLog(`❌ 导入失败: ${e.message}`, 'error')
  } finally {
    input.value = ''  // 允许重复导入同一文件
  }
}

function exportWorkflow() {
  syncGraphToStore()
  const json = JSON.stringify(store.toJSON(), null, 2)
  const blob = new Blob([json], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `${store.workflowName || 'workflow'}.json`
  a.click()
  URL.revokeObjectURL(url)
  addLog('📤 工作流已导出')
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
    exportWorkflow()
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
    onDeselectNode()
    return
  }
}
</script>

<style scoped>
/* ═══════════ 叠加层架构（对齐 ComfyUI LiteGraphCanvasSplitterOverlay） ═══════════ */

/* 层 0：全屏容器 */
.editor-app {
  position: relative;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  background: var(--color-bg);
}

/* 层 0：Canvas 全视口背景 */
.editor-canvas {
  position: fixed;
  inset: 0;
  z-index: 0;
  display: block;
  background: var(--color-bg);
  touch-action: none;
  user-select: none;
  outline: none;
}

/* 层 999：UI 浮层 — 默认穿透事件到 canvas */
.ui-overlay {
  position: fixed;
  inset: 0;
  z-index: 999;
  pointer-events: none;
  display: flex;
  flex-direction: column;
}

/* ─── 各 UI 区域：pointer-events: auto 恢复交互 ─── */
.overlay-top {
  pointer-events: auto;
  flex-shrink: 0;
}

.overlay-main {
  flex: 1;
  display: flex;
  min-height: 0;
}

.overlay-left {
  pointer-events: auto;
  display: flex;
  flex-direction: row;
  flex-shrink: 0;
}

.overlay-center {
  flex: 1;
  pointer-events: none;  /* 穿透到 canvas */
  min-width: 0;
}

/* ─── 空画布提示（浮在 canvas 上方，z-index 在 ui-overlay 之下） ─── */
.empty-canvas {
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  text-align: center;
  z-index: 10;
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
.overlay-bottom {
  pointer-events: auto;
  height: 140px;
  background: var(--color-surface);
  border-top: 1px solid var(--color-border);
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

/* ═══════════ 预览弹窗 ═══════ */
.preview-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  background: rgba(0, 0, 0, 0.3);
  pointer-events: auto;
}

.preview-popup {
  position: fixed;
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 8px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 320px;
  min-height: 200px;
}

.preview-popup-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  background: #21262d;
  border-bottom: 1px solid #30363d;
  cursor: move;
  font-size: 12px;
  font-weight: 600;
  color: #c9d1d9;
  flex-shrink: 0;
  user-select: none;
}

.preview-popup-actions button {
  background: none;
  border: none;
  color: #8b949e;
  cursor: pointer;
  font-size: 14px;
  padding: 2px 6px;
  border-radius: 4px;
}
.preview-popup-actions button:hover { background: #30363d; color: #f85149; }

.preview-popup-body {
  flex: 1;
  overflow: auto;
  padding: 0;
}

.preview-resize-handle {
  position: absolute;
  bottom: 0;
  right: 0;
  width: 16px;
  height: 16px;
  cursor: nwse-resize;
  background: linear-gradient(135deg, transparent 50%, #30363d 50%);
  border-radius: 0 0 8px 0;
}
</style>
