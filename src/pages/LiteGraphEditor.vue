<template>
  <div class="editor-app">
    <!-- 顶栏：logo + 名称 + 运行 + 保存 -->
    <div class="topbar-simple">
      <div class="topbar-left">
        <span class="topbar-logo" @click="$emit('back')" title="返回首页">WorkFlow</span>
        <span class="topbar-sep">|</span>
        <span class="workflow-name" @click="onRenameWorkflow" title="点击重命名">
          {{ store.workflowName || '未命名工作流' }}
        </span>
      </div>
      <div class="topbar-actions">
        <button class="btn-run" @click="runAll" :disabled="isRunning">
          {{ isRunning ? '⏸ 运行中...' : '▶ 运行' }}
        </button>
        <button class="btn-save" @click="onSaveWorkflow">💾 保存</button>
      </div>
    </div>

    <!-- Canvas 画布 -->
    <div class="grid-canvas">
      <canvas ref="canvasRef" class="editor-canvas" />
      <div class="canvas-overlay">
        <div v-if="store.nodes.length === 0" class="empty-canvas">
          <p>双击画布或右键添加节点</p>
        </div>
      </div>
      <!-- 右键菜单 -->
      <CanvasContextMenu
        :visible="contextMenuVisible"
        :x="contextMenuPos.x" :y="contextMenuPos.y"
        :items="contextMenuItems"
        @close="contextMenuVisible = false"
      />
      <!-- 搜索弹窗 -->
      <CanvasSearchPopover
        :visible="searchVisible"
        :x="searchPos.x" :y="searchPos.y"
        :graph="graph"
        @close="searchVisible = false"
        @node-added="onSearchNodeAdded"
      />
      <!-- 预览窗口（右侧浮出，选中节点时显示） -->
      <div v-if="selectedLgNode" class="preview-right">
        <PreviewPanel :lg-node="selectedLgNode" @update-widget="onUpdateWidget" />
      </div>
    </div>

    <!-- 控制台（右下角内嵌面板，可折叠） -->
    <div class="console-float" :class="{ collapsed: consoleCollapsed }">
      <div class="console-header" @click="consoleCollapsed = !consoleCollapsed">
        <span>📟 控制台</span>
        <span class="console-badge">{{ logs.length }}</span>
        <button class="console-clear" @click.stop="logs = []" title="清除">🗑</button>
        <button class="console-toggle" title="折叠">{{ consoleCollapsed ? '▲' : '▼' }}</button>
      </div>
      <div v-if="!consoleCollapsed" class="console-body">
        <div v-if="logs.length === 0" class="console-hint">等待操作…</div>
        <div v-for="log in logs" :key="log.id" :class="['log-line', log.level]">
          <span class="log-time">{{ log.time }}</span><span class="log-text">{{ log.text }}</span>
        </div>
      </div>
    </div>

    <!-- v4.0: 容器节点 action 选择器 -->
    <Teleport to="body">
      <div
        v-if="actionPickerVisible"
        class="action-picker-overlay"
        @mousedown.prevent.stop
        @click="actionPickerVisible = false"
      >
        <div
          class="action-picker-popup"
          :style="{ left: actionPickerPos.x + 'px', top: actionPickerPos.y + 'px' }"
          @click.stop
        >
          <div class="action-picker-header">添加动作</div>
          <div class="action-picker-list">
            <div
              v-for="action in availableActions"
              :key="action.type"
              class="action-picker-item"
              @click="onAddContainerAction(action)"
            >
              <span class="action-picker-icon">{{ action.icon }}</span>
              <div>
                <div class="action-picker-label">{{ action.label }}</div>
                <div class="action-picker-desc">{{ action.desc }}</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- v4.0: Action 参数编辑面板 -->
    <Teleport to="body">
      <div
        v-if="actionEditorVisible"
        class="action-editor-overlay"
        @mousedown.prevent.stop
        @click="actionEditorVisible = false"
      >
        <div
          class="action-editor-panel"
          :style="{ left: Math.min(actionEditorPos.x, window.innerWidth - 280) + 'px', top: Math.min(actionEditorPos.y + 8, window.innerHeight - 320) + 'px' }"
          @click.stop
        >
          <div class="ae-header">
            <span>⚙️ 动作参数</span>
            <button class="ae-close" @click="actionEditorVisible = false">×</button>
          </div>
          <div class="ae-body">
            <div
              v-for="param in actionEditorParams"
              :key="param.key"
              class="ae-field"
            >
              <label class="ae-label">{{ param.label }}</label>
              <input
                v-if="param.type === 'text'"
                class="ae-input"
                :placeholder="param.ph || ''"
                v-model="actionEditorForm[param.key]"
              />
              <input
                v-else-if="param.type === 'number'"
                class="ae-input"
                type="number"
                v-model.number="actionEditorForm[param.key]"
              />
              <select
                v-else-if="param.type === 'select'"
                class="ae-select"
                v-model="actionEditorForm[param.key]"
              >
                <option v-for="v in param.vals" :key="v" :value="v">{{ v }}</option>
              </select>
              <label v-else-if="param.type === 'toggle'" class="ae-toggle">
                <input type="checkbox" v-model="actionEditorForm[param.key]" />
                <span class="ae-toggle-slider"></span>
              </label>
            </div>
            <div v-if="actionEditorParams.length === 0" class="ae-empty">
              此动作无额外参数
            </div>
          </div>
          <div class="ae-footer">
            <button class="ae-btn ae-btn-del" @click="deleteActionFromEditor">🗑 删除</button>
            <button class="ae-btn ae-btn-save" @click="saveActionEditor">💾 保存</button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.editor-app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #0d1117;
}

.topbar-simple {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 16px;
  background: #161b22;
  border-bottom: 1px solid #30363d;
  z-index: 10;
  flex-shrink: 0;
}

.workflow-name {
  font-size: 15px;
  font-weight: 600;
  color: #e6edf3;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
  transition: background 0.15s;
}
.workflow-name:hover {
  background: #21262d;
}

.topbar-left {
  display: flex;
  align-items: center;
  gap: 8px;
}
.topbar-logo {
  font-size: 15px;
  font-weight: 700;
  color: #58a6ff;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
  transition: background 0.15s;
  letter-spacing: -0.3px;
}
.topbar-logo:hover {
  background: rgba(88, 166, 255, 0.1);
}
.topbar-sep {
  color: #30363d;
  font-size: 16px;
  user-select: none;
}

.topbar-actions {
  display: flex;
  gap: 8px;
}

.btn-run {
  padding: 4px 16px;
  background: #238636;
  color: #fff;
  border: none;
  border-radius: 4px;
  font-size: 13px;
  cursor: pointer;
}
.btn-run:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.btn-run:hover:not(:disabled) {
  background: #2ea043;
}

.btn-save {
  padding: 4px 12px;
  background: #21262d;
  color: #c9d1d9;
  border: 1px solid #30363d;
  border-radius: 4px;
  font-size: 13px;
  cursor: pointer;
}
.btn-save:hover {
  background: #30363d;
}

.grid-canvas {
  flex: 1;
  position: relative;
  overflow: hidden;
  min-height: 0;
}

.editor-canvas {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  display: block;
  background: #0d1117;
  touch-action: none;
  user-select: none;
  outline: none;
}

.canvas-overlay {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.empty-canvas {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  pointer-events: auto;
  color: #484f58;
  font-size: 14px;
  text-align: center;
}

.preview-right {
  position: absolute;
  right: 0;
  top: 0;
  bottom: 0;
  width: 320px;
  background: #161b22;
  border-left: 1px solid #30363d;
  overflow-y: auto;
  z-index: 50;
}

/* ─── 控制台（右下角浮动内嵌面板）─── */
.console-float {
  position: absolute;
  right: 12px;
  bottom: 12px;
  width: 420px;
  max-height: 300px;
  background: rgba(13, 17, 23, 0.92);
  border: 1px solid #30363d;
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  z-index: 100;
  backdrop-filter: blur(8px);
  box-shadow: 0 4px 16px rgba(0,0,0,0.4);
  transition: max-height 0.2s;
  overflow: hidden;
}
.console-float.collapsed {
  max-height: 32px;
}

.console-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 10px;
  background: rgba(22, 27, 34, 0.9);
  font-size: 11px;
  color: #8b949e;
  cursor: pointer;
  flex-shrink: 0;
  user-select: none;
  border-radius: 8px 8px 0 0;
}
.console-header:hover {
  background: rgba(33, 38, 45, 0.9);
}
.console-badge {
  font-size: 10px;
  background: #30363d;
  color: #8b949e;
  padding: 1px 6px;
  border-radius: 8px;
  margin-left: auto;
}
.console-toggle {
  background: none;
  border: none;
  color: #8b949e;
  cursor: pointer;
  font-size: 10px;
  padding: 0 4px;
}
.console-toggle:hover { color: #e6edf3; }
.console-clear {
  padding: 1px 6px;
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
  padding: 6px 10px;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
  font-size: 11px;
  color: #c9d1d9;
  max-height: 250px;
  line-height: 1.5;
}

.console-hint {
  color: #484f58;
  font-style: italic;
  padding: 8px 0;
  text-align: center;
}

.log-line {
  display: flex;
  gap: 8px;
  padding: 1px 0;
}
.log-time { color: #484f58; flex-shrink: 0; }
.log-text { word-break: break-all; }
.log-line.info { color: #8b949e; }
.log-line.error { color: #f85149; }
.log-line.warn { color: #d29922; }
.log-line.success { color: #3fb950; }
</style>

<!-- v4.0: 容器节点 action picker（非 scoped — Teleport to body） -->
<style>
.action-picker-overlay {
  position: fixed;
  inset: 0;
  z-index: 10000;
}
.action-picker-popup {
  position: fixed;
  z-index: 10001;
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 8px;
  min-width: 240px;
  max-height: 360px;
  overflow-y: auto;
  box-shadow: 0 8px 24px rgba(0,0,0,0.5);
}
.action-picker-header {
  padding: 8px 12px;
  font-size: 12px;
  font-weight: 600;
  color: #8b949e;
  border-bottom: 1px solid #21262d;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.action-picker-list {
  padding: 4px;
}
.action-picker-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.12s;
}
.action-picker-item:hover {
  background: #1f6feb;
}
.action-picker-icon {
  font-size: 16px;
  width: 24px;
  text-align: center;
  flex-shrink: 0;
}
.action-picker-label {
  font-size: 13px;
  color: #e6edf3;
  font-weight: 500;
}
.action-picker-desc {
  font-size: 11px;
  color: #8b949e;
  margin-top: 1px;
}

/* ─── v4.0: Action 参数编辑面板 ─── */
.action-editor-overlay {
  position: fixed;
  inset: 0;
  z-index: 10002;
}
.action-editor-panel {
  position: fixed;
  z-index: 10003;
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 8px;
  width: 260px;
  box-shadow: 0 8px 24px rgba(0,0,0,0.5);
}
.ae-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  font-size: 12px;
  font-weight: 600;
  color: #8b949e;
  border-bottom: 1px solid #21262d;
}
.ae-close {
  background: none;
  border: none;
  color: #8b949e;
  font-size: 16px;
  cursor: pointer;
  padding: 0 4px;
}
.ae-close:hover { color: #e6edf3; }
.ae-body {
  padding: 8px 12px;
  max-height: 200px;
  overflow-y: auto;
}
.ae-field {
  margin-bottom: 8px;
}
.ae-label {
  display: block;
  font-size: 11px;
  color: #8b949e;
  margin-bottom: 3px;
}
.ae-input, .ae-select {
  width: 100%;
  padding: 5px 8px;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 4px;
  color: #e6edf3;
  font-size: 12px;
  outline: none;
  box-sizing: border-box;
}
.ae-input:focus, .ae-select:focus {
  border-color: #58a6ff;
}
.ae-select {
  cursor: pointer;
}
.ae-toggle {
  display: flex;
  align-items: center;
  cursor: pointer;
}
.ae-toggle input[type="checkbox"] {
  accent-color: #1f6feb;
  width: 14px;
  height: 14px;
}
.ae-empty {
  font-size: 12px;
  color: #484f58;
  text-align: center;
  padding: 8px;
}
.ae-footer {
  display: flex;
  justify-content: flex-end;
  gap: 6px;
  padding: 8px 12px;
  border-top: 1px solid #21262d;
}
.ae-btn {
  padding: 4px 12px;
  border-radius: 4px;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  border: 1px solid #30363d;
  transition: all 0.12s;
}
.ae-btn-save {
  background: #1f6feb;
  color: #fff;
  border-color: #388bfd;
}
.ae-btn-save:hover { background: #388bfd; }
.ae-btn-del {
  background: transparent;
  color: #f85149;
  border-color: #30363d;
}
.ae-btn-del:hover {
  background: #da3633;
  color: #fff;
  border-color: #f85149;
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
import PreviewPanel from '../components/flow/PreviewPanel.vue'
import CanvasContextMenu from '../components/CanvasContextMenu.vue'
import CanvasSearchPopover from '../components/CanvasSearchPopover.vue'
import type { ContextMenuItem } from '../components/CanvasContextMenu.vue'
import { registerAllNodes, BROWSER_ACTIONS, WORD_ACTIONS, EXCEL_ACTIONS } from '../nodes/litegraph-nodes'
import type { NodeStatus, NodeDefinition } from '../components/flow/pinTypes'

// ─── Props ───
const props = defineProps<{
  initialWorkflowId?: string
}>()

const emit = defineEmits<{
  'back': []
}>()

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
const widgetVersion = ref(0)  // 递增触发 PropertyPanel 重渲染（保留给 PreviewPanel 使用）
const breakpoints = ref(new Set<string>())  // v3: 断点集合

// ─── 鼠标位置追踪（供搜索弹窗 placement 用） ───
let _lastMousePos = { x: 0, y: 0 }

// ─── 面板状态 ───
const consoleCollapsed = ref(false)  // 控制台折叠状态（默认展开）

// ─── 容器节点 action 选择器 ───
const actionPickerVisible = ref(false)
const actionPickerPos = ref({ x: 0, y: 0 })
const actionPickerNodeId = ref<string | null>(null)
// 存储容器节点的 action 区域（用于点击检测）
const _containerActionRects = new WeakMap<object, { type: 'action'; actionId: string } | { type: 'add'; nodeId: string }>()

// ─── v4.0: Action 参数编辑器 ───
const actionEditorVisible = ref(false)
const actionEditorPos = ref({ x: 0, y: 0 })
const actionEditorNodeId = ref<string | null>(null)
const actionEditorActionId = ref<string | null>(null)
const actionEditorForm = ref<Record<string, any>>({})

// 参数 schema：每个 action type → 可编辑的参数字段
const ACTION_PARAMS: Record<string, Array<{ key: string; label: string; type: 'text'|'select'|'number'|'toggle'; ph?: string; vals?: string[]; def?: any }>> = {
  navigate: [
    { key: 'url', label: 'URL', type: 'text', ph: 'https://...' },
    { key: 'wait_until', label: '等待策略', type: 'select', vals: ['load','domcontentloaded','networkidle'] },
  ],
  wait: [
    { key: 'selector', label: '选择器', type: 'text', ph: '#element' },
    { key: 'timeout', label: '超时(ms)', type: 'number', def: 5000 },
  ],
  input: [
    { key: 'selector', label: '选择器', type: 'text', ph: '#input' },
    { key: 'value', label: '填写值', type: 'text', ph: '留空=从连线传入' },
    { key: 'clear', label: '先清空', type: 'toggle', def: true },
  ],
  fill: [
    { key: 'selector', label: '选择器', type: 'text', ph: '#input' },
    { key: 'value', label: '填写值', type: 'text', ph: '留空=从连线传入' },
    { key: 'clear', label: '先清空', type: 'toggle', def: true },
  ],
  click: [
    { key: 'selector', label: '选择器', type: 'text', ph: '#button' },
  ],
  extract: [
    { key: 'selector', label: '选择器', type: 'text', ph: 'body' },
    { key: 'mode', label: '提取模式', type: 'select', vals: ['text','html','attribute:value'] },
  ],
  screenshot: [
    { key: 'full_page', label: '全页截图', type: 'toggle' },
  ],
  evaluate: [
    { key: 'script', label: 'JS 代码', type: 'text', ph: 'document.title' },
  ],
  scroll: [
    { key: 'x', label: 'X 偏移', type: 'number', def: 0 },
    { key: 'y', label: 'Y 偏移', type: 'number', def: 0 },
  ],
  get_title: [],
  pdf: [],
  read: [
    { key: 'range', label: '范围(Excel)', type: 'text', ph: 'A1:B10 (仅 Excel)' },
  ],
  write: [
    { key: 'range', label: '范围', type: 'text', ph: 'A1 或 A1:B10' },
    { key: 'value', label: '值', type: 'text', ph: '留空=从连线传入' },
  ],
  filter: [
    { key: 'column', label: '列', type: 'text', ph: 'A' },
    { key: 'op', label: '操作', type: 'select', vals: ['contains','equals','gt','gte','lt','lte','is_empty','not_empty'] },
    { key: 'value', label: '筛选值', type: 'text', ph: '留空=从连线传入' },
  ],
  sort: [
    { key: 'column', label: '列', type: 'text', ph: 'A' },
    { key: 'order', label: '排序', type: 'select', vals: ['asc','desc'] },
  ],
  create: [
    { key: 'headers', label: '表头(Excel)', type: 'text', ph: '列A,列B' },
    { key: 'title', label: '标题(Word)', type: 'text', ph: '文档标题' },
  ],
  append: [
    { key: 'value', label: '追加数据', type: 'text', ph: '留空=从连线传入' },
  ],
  formula: [
    { key: 'cell', label: '单元格', type: 'text', ph: 'A1' },
    { key: 'formula', label: '公式', type: 'text', ph: 'SUM(B1:B10)' },
  ],
  replace: [
    { key: 'old_text', label: '查找文本', type: 'text' },
    { key: 'new_text', label: '替换为', type: 'text', ph: '留空=从连线传入' },
  ],
  merge: [
    { key: 'files', label: '文件列表', type: 'text', ph: '留空=从连线传入' },
  ],
  insert_table: [
    { key: 'data', label: '表格数据', type: 'text', ph: '留空=从连线传入' },
  ],
  csv: [
    { key: 'direction', label: '方向', type: 'select', vals: ['csv_to_xlsx', 'xlsx_to_csv'] },
    { key: 'output', label: '输出路径', type: 'text', ph: 'output.csv 或 output.xlsx' },
    { key: 'delimiter', label: '分隔符', type: 'select', vals: [',', ';', '\\t'] },
  ],
}

function openActionEditor(node: any, actionId: string, clientX: number, clientY: number) {
  const actions = (node.properties as any)?.actions || []
  const action = actions.find((a: any) => a.id === actionId)
  if (!action) return
  actionEditorNodeId.value = String(node.id)
  actionEditorActionId.value = actionId
  actionEditorPos.value = { x: clientX, y: clientY }
  // 初始化表单为当前 action.config 的值
  const form: Record<string, any> = {}
  const params = ACTION_PARAMS[action.type] || []
  for (const p of params) {
    form[p.key] = action.config?.[p.key] !== undefined ? action.config[p.key] : (p.def ?? (p.type === 'toggle' ? false : ''))
  }
  actionEditorForm.value = form
  actionEditorVisible.value = true
}

function saveActionEditor() {
  if (!actionEditorNodeId.value || !actionEditorActionId.value || !graph) return
  const node = graph._nodes?.find((n: any) => String(n.id) === actionEditorNodeId.value)
  if (!node) return
  const actions = (node.properties as any)?.actions || []
  const action = actions.find((a: any) => a.id === actionEditorActionId.value)
  if (!action) return
  action.config = { ...(action.config || {}), ...actionEditorForm.value }
  ;(node as any).rebuildPorts?.()
  graph.setDirtyCanvas(true, true)
  actionEditorVisible.value = false
}

function deleteActionFromEditor() {
  if (!actionEditorNodeId.value || !actionEditorActionId.value || !graph) return
  const node = graph._nodes?.find((n: any) => String(n.id) === actionEditorNodeId.value)
  if (!node) return
  ;(node as any).removeAction?.(actionEditorActionId.value)
  actionEditorVisible.value = false
}

// 根据选中容器节点类型返回可用动作列表
const availableActions = computed(() => {
  if (!actionPickerNodeId.value || !graph) return []
  const node = graph._nodes?.find((n: any) => String(n.id) === actionPickerNodeId.value)
  if (!node) return []
  const t = (node as any).type || ''
  if (t === 'browser_container') return BROWSER_ACTIONS
  if (t === 'excel_container') return EXCEL_ACTIONS
  if (t === 'word_container') return WORD_ACTIONS
  return []
})

function onAddContainerAction(actionDef: { type: string; label: string; icon: string }) {
  if (!actionPickerNodeId.value || !graph) return
  const node = graph._nodes?.find((n: any) => String(n.id) === actionPickerNodeId.value)
  if (!node) return
  ;(node as any).addAction?.(actionDef.type, actionDef.label)
  actionPickerVisible.value = false
  addLog(`➕ ${actionDef.label} → ${node.title}`, 'info')
}

// 当前编辑 action 的参数列表
const actionEditorParams = computed(() => {
  if (!actionEditorNodeId.value || !actionEditorActionId.value || !graph) return []
  const node = graph._nodes?.find((n: any) => String(n.id) === actionEditorNodeId.value)
  if (!node) return []
  const actions = (node.properties as any)?.actions || []
  const action = actions.find((a: any) => a.id === actionEditorActionId.value)
  if (!action) return []
  return ACTION_PARAMS[action.type] || []
})

// ─── 右键菜单 / 搜索弹窗 ───
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
      canvas?.copyToClipboard?.()
      addLog('📋 已复制', 'info')
    }, disabled: !selectedLgNode.value },
    { label: '📌 粘贴', action: () => {
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
    { label: '▶ 从此运行', action: () => {
      if (!selectedLgNode.value) return
      runFromNode(String(selectedLgNode.value.id))
    }, disabled: !selectedLgNode.value || isRunning.value },
    { label: selectedLgNode.value && breakpoints.value.has(String(selectedLgNode.value.id))
      ? '🔴 取消断点' : '🔵 设置断点', action: () => {
      if (!selectedLgNode.value) return
      const id = String(selectedLgNode.value.id)
      if (breakpoints.value.has(id)) breakpoints.value.delete(id)
      else breakpoints.value.add(id)
      breakpoints.value = new Set(breakpoints.value)  // 触发响应式
      syncNodeStatuses()
      addLog(breakpoints.value.has(id) ? `🔴 断点: ${selectedLgNode.value.title}` : `🔵 取消断点: ${selectedLgNode.value.title}`)
    }, disabled: !selectedLgNode.value },
    { label: 'separator', action: () => {}, divider: true },
    { label: '📟 控制台', action: () => { consoleCollapsed.value = !consoleCollapsed.value } },
    { label: 'separator', action: () => {}, divider: true },
    { label: '📐 适应视图', action: () => { fitView() }},
    { label: '🗑 清空画布', action: () => { clearCanvas() }},
  ]
  return items
})

const logs = ref<{ id: number; time: string; text: string; level: string }[]>([])

// ─── 节点状态可视化：同步 flowStore.nodeStatuses 到 LiteGraph 节点渲染 ───
const STATUS_COLORS: Record<string, string> = {
  idle:    '#2a3040',
  queued:  '#d29922',
  running: '#58a6ff',
  success: '#3fb950',
  error:   '#f85149',
  warning: '#d29922',
  paused:  '#8b949e',
}

function syncNodeStatuses() {
  if (!graph) return
  const lgNodes = graph._nodes || []
  for (const lgNode of lgNodes) {
    if (!lgNode || !lgNode.id) continue
    const status = store.nodeStatuses[String(lgNode.id)] || 'idle'
    lgNode.color = STATUS_COLORS[status] || STATUS_COLORS.idle
    if (status === 'running') {
      lgNode.boxcolor = '#1f6feb'
    } else if (status === 'error') {
      lgNode.boxcolor = '#da3633'
    } else if (status === 'success') {
      lgNode.boxcolor = '#238636'
    } else {
      lgNode.boxcolor = '#2a3040'
    }
    ;(lgNode as any)._hasBreakpoint = breakpoints.value.has(String(lgNode.id))
  }
  graph.setDirtyCanvas?.(true)
}

// 监听状态变化 → 更新节点外观
watch(() => store.nodeStatuses, () => {
  syncNodeStatuses()
  const lgNodes = graph?._nodes || []
  for (const lgNode of lgNodes) {
    if (!lgNode?.id) continue
    const status = store.nodeStatuses[String(lgNode.id)]
    if (status === 'success') {
      const outLinks = (lgNode.outputs || []).flatMap((_o: any, slotIdx: number) => 
        lgNode.getOutputLinks?.(slotIdx) || []
      )
      for (const link of outLinks) {
        startDataFlowAnimation(link)
      }
    }
  }
}, { deep: true })

// ─── 数据流动画系统 ───
interface FlowParticle {
  link: any
  progress: number
  startTime: number
  duration: number
}

const flowParticles: FlowParticle[] = []
let flowAnimFrame: number | null = null

function startDataFlowAnimation(link: any) {
  flowParticles.push({
    link,
    progress: 0,
    startTime: performance.now(),
    duration: 600,
  })
  if (!flowAnimFrame) {
    flowAnimFrame = requestAnimationFrame(renderFlowParticles)
  }
}

function renderFlowParticles(now: number) {
  if (!canvas) return
  const active = flowParticles.filter(p => {
    p.progress = Math.min(1, (now - p.startTime) / p.duration)
    return p.progress < 1
  })
  if (active.length === 0) {
    flowParticles.length = 0
    flowAnimFrame = null
    return
  }
  flowParticles.length = 0
  flowParticles.push(...active)
  const ctx = canvas.ctx
  const ds = (canvas as any).ds
  if (!ctx || !ds) {
    flowAnimFrame = requestAnimationFrame(renderFlowParticles)
    return
  }
  for (const p of flowParticles) {
    const alpha = 1 - p.progress
    const size = 4 + (1 - p.progress) * 3
    const [sx, sy] = p.link._pos || [0, 0]
    const [ex, ey] = p.link._pos_end || [0, 0]
    const x = sx + (ex - sx) * p.progress
    const y = sy + (ey - sy) * p.progress
    ctx.save()
    ctx.globalAlpha = alpha * 0.8
    ctx.fillStyle = '#3fb950'
    ctx.shadowColor = '#3fb950'
    ctx.shadowBlur = 6
    ctx.beginPath()
    ctx.arc(x, y, size, 0, Math.PI * 2)
    ctx.fill()
    ctx.restore()
  }
  canvas.setDirty?.(true, false)
  flowAnimFrame = requestAnimationFrame(renderFlowParticles)
}

// 监听输出变化 → 在节点上显示数据摘要
watch(() => store.stepOutputs, () => {
  if (!graph) return
  const lgNodes = graph._nodes || []
  for (const lgNode of lgNodes) {
    if (!lgNode || !lgNode.id) continue
    const output = store.stepOutputs[String(lgNode.id)]
    if (output) {
      ;(lgNode as any)._outputBadge = summarizeOutput(output)
    }
  }
  graph.setDirtyCanvas?.(true)
}, { deep: true })

function summarizeOutput(data: unknown): string {
  if (data === null || data === undefined) return 'null'
  if (typeof data === 'string') return data.length > 40 ? data.slice(0, 40) + '...' : data
  if (typeof data === 'number') return String(data)
  if (Array.isArray(data)) return `[${data.length} 条]`
  if (typeof data === 'object') {
    const keys = Object.keys(data as object)
    return `{${keys.slice(0, 3).join(', ')}${keys.length > 3 ? '...' : ''}}`
  }
  return String(data).slice(0, 40)
}

/** v3: roundRect polyfill for WebView2 compatibility */
function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  ctx.beginPath()
  ctx.moveTo(x + r, y)
  ctx.lineTo(x + w - r, y)
  ctx.arcTo(x + w, y, x + w, y + r, r)
  ctx.lineTo(x + w, y + h - r)
  ctx.arcTo(x + w, y + h, x + w - r, y + h, r)
  ctx.lineTo(x + r, y + h)
  ctx.arcTo(x, y + h, x, y + h - r, r)
  ctx.lineTo(x, y + r)
  ctx.arcTo(x, y, x + r, y, r)
  ctx.closePath()
}

// ─── Canvas 初始化 ───
onMounted(() => {
  registerAllNodes()
  graph = new LGraph()

  nextTick(() => {
    requestAnimationFrame(() => {
      if (!canvasRef.value) return
      initCanvas()
    })
  })

  function initCanvas() {
    const c = canvasRef.value!
    c.width = c.height = NaN as unknown as number
    const width = c.offsetWidth, height = c.offsetHeight
    if (width === 0 || height === 0) {
      requestAnimationFrame(() => {
        if (!canvasRef.value) return
        const b = canvasRef.value
        if (b.offsetWidth > 0 && b.offsetHeight > 0) {
          initCanvasReal(b.offsetWidth, b.offsetHeight)
        }
      })
      return
    }
    initCanvasReal(width, height)
  }

  function initCanvasReal(w: number, h: number) {
    const c = canvasRef.value!
    const dpr = Math.max(window.devicePixelRatio || 1, 1)
    c.width = Math.round(w * dpr)
    c.height = Math.round(h * dpr)
    c.getContext('2d')?.scale(dpr, dpr)

    canvas = new LGraphCanvas(c, graph)

    canvas.background_image = ''
    canvas.clear_background = true
    canvas.clear_background_color = '#0d1117'
    canvas.render_canvas_border = false
    canvas.render_border = false
    canvas.node_title_color = '#e6edf3'

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

    canvas.allow_dragnodes = true
    canvas.allow_dragcanvas = true
    canvas.allow_interaction = true
    canvasRef.value!.style.touchAction = 'none'

    const _origShowSearchBox = canvas.showSearchBox.bind(canvas)
    canvas.showSearchBox = (e: MouseEvent, opts?: any) => {
      if (opts) return _origShowSearchBox(e, opts)
    }
    const _origProcessContextMenu = canvas.processContextMenu.bind(canvas)
    canvas.processContextMenu = (node: any, event: MouseEvent) => {
      if (node) return _origProcessContextMenu(node, event)
    }

    const resizeHandler = () => {
      if (!canvasRef.value || !canvas) return
      const c = canvasRef.value
      c.width = c.height = NaN as unknown as number
      const width = c.offsetWidth, height = c.offsetHeight
      if (width === 0 || height === 0) return
      const dpr = Math.max(window.devicePixelRatio || 1, 1)
      c.width = Math.round(width * dpr)
      c.height = Math.round(height * dpr)
      c.getContext('2d')?.scale(dpr, dpr)
      canvas.draw(true, true)
    }
    _resizeObserver = new ResizeObserver(resizeHandler)
    _resizeObserver.observe(canvasRef.value!)
    resizeHandler()

    graph.on_change = () => throttledSync()
    graph.onAfterChange = () => throttledSync()

    canvas.onSelectionChange = (selectedDict: Record<string, LGraphNode>) => {
      const selected = Object.values(selectedDict)[0]
      selectedLgNode.value = selected instanceof LGraphNode ? selected : null
    }

    graph.onConnectionChange = () => throttledSync()

    // v3: 断点指示器 + 输出徽章
    const origDraw = canvas.draw.bind(canvas)
    canvas.draw = function(force: boolean, skipLinks: boolean) {
      origDraw(force, skipLinks)
      const ctx2 = canvas.ctx
      if (!ctx2) return
      const ds = (canvas as any).ds
      const scale = ds?.scale || 1
      const offset = ds?.offset || [0, 0]
      for (const node of graph._nodes || []) {
        const [nx, ny] = node.pos || [0, 0]
        const [nw, nh] = node.size || [200, 60]
        const sx = nx * scale + offset[0]
        const sy = ny * scale + offset[1]
        const sw = nw * scale
        const sh = nh * scale
        if ((node as any)._hasBreakpoint) {
          ctx2.save()
          ctx2.strokeStyle = 'rgba(248, 81, 73, 0.5)'
          ctx2.lineWidth = 2
          ctx2.strokeRect(sx - 2, sy - 2, sw + 4, sh + 4)
          ctx2.restore()
          ctx2.save()
          ctx2.fillStyle = '#f85149'
          ctx2.shadowColor = '#f85149'
          ctx2.shadowBlur = 6
          ctx2.beginPath()
          ctx2.arc(sx + 10, sy + 10, 5, 0, Math.PI * 2)
          ctx2.fill()
          ctx2.restore()
        }
        const badge = (node as any)._outputBadge as string | undefined
        if (badge) {
          const fontSize = Math.max(9, Math.min(11, 11 * scale))
          ctx2.save()
          ctx2.font = `${fontSize}px monospace`
          const metrics = ctx2.measureText(badge)
          const padX = 4, padY = 2
          const bw = metrics.width + padX * 2
          const bh = fontSize + padY * 2
          const bx = sx + sw - bw - 4
          const by = sy + sh - bh - 2
          ctx2.fillStyle = 'rgba(22, 27, 34, 0.85)'
          ctx2.strokeStyle = '#30363d'
          ctx2.lineWidth = 1
          roundRect(ctx2, bx, by, bw, bh, 3)
          ctx2.fill()
          ctx2.stroke()
          ctx2.fillStyle = '#58a6ff'
          ctx2.textBaseline = 'middle'
          ctx2.fillText(badge, bx + padX, by + bh / 2)
          ctx2.restore()
        }

        // v4.0: 容器节点 action 列表 + "+" 按钮
        const nodeType = (node as any).type || ''
        if (nodeType === 'browser_container' || nodeType === 'excel_container' || nodeType === 'word_container') {
          const actions = ((node as any).properties?.actions || []) as any[]
          const actionH = Math.max(14, 16 * scale)
          const actionGap = 4 * scale
          const startY = sy + sh + 6 * scale

          // 重置该节点所有区域标记
          const oldRects = (node as any)._actionRects || []
          for (const r of oldRects) _containerActionRects.delete(r)

          // 渲染每个 action pill
          ctx2.save()
          let ay = startY
          for (const action of actions) {
            const label = (action.label || action.type || '?') as string
            const fontSize2 = Math.max(9, Math.min(11, 11 * scale))
            ctx2.font = `${fontSize2}px sans-serif`
            const textW = ctx2.measureText(label).width
            const pillW = textW + 16 * scale
            const pillH = actionH
            // 背景
            ctx2.fillStyle = 'rgba(22, 27, 34, 0.9)'
            ctx2.strokeStyle = '#30363d'
            ctx2.lineWidth = 1
            roundRect(ctx2, sx + 4, ay, pillW, pillH, 3)
            ctx2.fill()
            ctx2.stroke()
            // 图标 + 文本
            ctx2.fillStyle = '#8b949e'
            ctx2.fillText('▸ ' + label, sx + 8, ay + pillH * 0.7)
            // 存储点击区域
            const rectKey = {}
            ;(node as any)._actionRects = [...((node as any)._actionRects || []), rectKey]
            _containerActionRects.set(rectKey, { type: 'action', actionId: action.id })
            // 存储像素位置用于 hit test
            ;(rectKey as any)._pos = { x: sx + 4, y: ay, w: pillW, h: pillH }
            ay += pillH + actionGap
          }

          // "+" 按钮
          const plusSize = Math.max(14, 16 * scale)
          const plusX = sx + 4
          const plusY = ay
          ctx2.fillStyle = 'rgba(31, 111, 235, 0.25)'
          ctx2.strokeStyle = '#1f6feb'
          ctx2.lineWidth = 1.5
          roundRect(ctx2, plusX, plusY, plusSize, plusSize, 3)
          ctx2.fill()
          ctx2.stroke()
          // "+" 文字
          const plusFontSize = Math.max(10, Math.min(14, 14 * scale))
          ctx2.font = `bold ${plusFontSize}px sans-serif`
          ctx2.fillStyle = '#58a6ff'
          ctx2.textAlign = 'center'
          ctx2.textBaseline = 'middle'
          ctx2.fillText('+', plusX + plusSize / 2, plusY + plusSize / 2)
          ctx2.textAlign = 'left'
          ctx2.restore()

          // 存储 "+" 点击区域
          const plusKey = {}
          ;(node as any)._actionRects = [...((node as any)._actionRects || []), plusKey]
          _containerActionRects.set(plusKey, { type: 'add', nodeId: String(node.id) })
          ;(plusKey as any)._pos = { x: plusX, y: plusY, w: plusSize, h: plusSize }
        }
      }
    }

    // 启动时恢复自动保存或从 store 加载
    if (store.nodes.length > 0) {
      loadFromStore()
      addLog(`📋 已加载「${store.workflowName}」(${store.nodeCount} 节点)`, 'info')
      document.title = `${store.workflowName} — Workflow Engine`
    } else if (props.initialWorkflowId) {
      loadWorkflowById(props.initialWorkflowId)
    } else {
      const restored = autoSave.loadAutoSave()
      if (restored) {
        loadFromStore()
        addLog('📂 已恢复上次未保存的工作流', 'info')
      }
    }

    undoManager.init()
    autoSave.start()
    document.addEventListener('keydown', onKeyDown)
    _canvasMounted = true

    canvasRef.value!.addEventListener('mousemove', (e) => {
      _lastMousePos = { x: e.clientX, y: e.clientY }
    })

    canvasRef.value!.addEventListener('contextmenu', (e: MouseEvent) => {
      e.preventDefault()
      contextMenuPos.value = { x: e.clientX, y: e.clientY }
      contextMenuVisible.value = true
    })

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

    // v4.0: 容器节点 action 点击检测
    canvasRef.value!.addEventListener('click', (e: MouseEvent) => {
      if (!canvas || !graph) return
      const rect = canvasRef.value?.getBoundingClientRect()
      if (!rect) return
      const mx = e.clientX - rect.left
      const my = e.clientY - rect.top
      const ds = (canvas as any).ds
      const scale = ds?.scale || 1
      const offset = ds?.offset || [0, 0]
      for (const node of graph._nodes || []) {
        const ra = (node as any)._actionRects as any[] | undefined
        if (!ra) continue
        for (const rk of ra) {
          const hit = (rk as any)._pos as { x: number; y: number; w: number; h: number } | undefined
          if (!hit) continue
          if (mx >= hit.x && mx <= hit.x + hit.w && my >= hit.y && my <= hit.y + hit.h) {
            const meta = _containerActionRects.get(rk)
            if (meta?.type === 'add') {
              actionPickerNodeId.value = meta.nodeId
              actionPickerPos.value = { x: hit.x + rect.left, y: hit.y + hit.h + 4 + rect.top }
              actionPickerVisible.value = true
              e.stopPropagation()
              e.preventDefault()
            } else if (meta?.type === 'action') {
              // 点击 action pill → 弹出参数编辑面板
              openActionEditor(node, meta.actionId, e.clientX, e.clientY)
              e.stopPropagation()
              e.preventDefault()
            }
            return
          }
        }
      }
    })

    // 初始化标签页
    tabStore.ensureTab()
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
  if (graph) {
    graph.on_change = undefined
    graph.onAfterChange = undefined
    graph.onConnectionChange = undefined
  }
  if (canvas) {
    canvas.onSelectionChange = undefined
    canvas.onDrawBackground = undefined
  }
  cleanupDagListeners()
})

// ─── 标记：是否正在从 store 同步到 graph（避免循环更新） ───
let syncingFromStore = false
let lastSyncTime = 0
let pendingRaf = false
const SYNC_THROTTLE_MS = 100

function throttledSync() {
  if (syncingFromStore) return
  const now = Date.now()
  if (now - lastSyncTime < SYNC_THROTTLE_MS) {
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

// ─── 从 LiteGraph graph 同步节点到 store ───
function syncGraphToStore() {
  syncingFromStore = true
  const lgNodes = graph._nodes || []
  const storeNodeIds = new Set(store.nodes.map(n => n.id))

  for (let i = 0; i < lgNodes.length; i++) {
    const ln = lgNodes[i]
    if (!ln) continue
    const nodeId = String(ln.id)
    const config: Record<string, unknown> = {}
    if (ln.widgets) {
      for (const w of ln.widgets) {
        config[w.name] = w.value
      }
    }
    if (storeNodeIds.has(nodeId)) {
      const existing = store.getNode(nodeId)
      if (existing) {
        const posChanged = existing.position.x !== ln.pos[0] || existing.position.y !== ln.pos[1]
        const labelChanged = existing.label !== (ln.title || ln.type || '')
        const configChanged = shallowDiff(existing.config, config)
        if (posChanged) store.updateNodePosition(nodeId, { x: ln.pos[0], y: ln.pos[1] })
        if (labelChanged) store.updateNodeLabel(nodeId, ln.title || ln.type || '')
        if (configChanged) store.updateNodeConfig(nodeId, config)
      }
    } else {
      store.addNode({
        id: nodeId,
        type: ln.type || '',
        label: ln.title || ln.type || '',
        position: { x: ln.pos[0], y: ln.pos[1] },
        config,
      })
    }
  }

  const graphNodeIds = new Set(lgNodes.map((n: LGraphNode) => String(n.id)))
  for (const sn of store.nodes) {
    if (!graphNodeIds.has(sn.id)) store.removeNode(sn.id)
  }

  syncEdgesToStore()
  syncingFromStore = false
}

function shallowDiff(a: Record<string, unknown>, b: Record<string, unknown>): boolean {
  const keysA = Object.keys(a)
  const keysB = Object.keys(b)
  if (keysA.length !== keysB.length) return true
  for (const k of keysA) {
    if (a[k] !== b[k]) return true
  }
  return false
}

// ─── 同步连线到 store ───
function syncEdgesToStore() {
  const linkValues = [...((graph as any)._links?.values() || [])]
  if (linkValues.length === 0) {
    if (store.edges.length > 0) store.edges.forEach(e => store.removeEdge(e.id))
    return
  }
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
  for (const edge of store.edges) {
    if (!linkIds.has(edge.id)) store.removeEdge(edge.id)
  }
}

// ─── 从 store 加载到 LiteGraph ───
function loadFromStore() {
  syncingFromStore = true
  graph.clear()
  const oldToNew = new Map<string, LGraphNode>()
  for (const sn of store.nodes) {
    const node = LiteGraph.createNode(sn.type)
    if (!node) {
      addLog(`⚠ 未知节点类型: ${sn.type}`, 'warn')
      continue
    }
    oldToNew.set(sn.id, node)
    node.pos = [sn.position.x, sn.position.y]
    node.title = sn.label
    if (sn.config && node.widgets) {
      for (const w of node.widgets) {
        if (sn.config[w.name] !== undefined) w.value = sn.config[w.name]
      }
    }
    if ((sn as any).actions && sn.type?.endsWith('_container')) {
      (node.properties as any).actions = (sn as any).actions
      ;(node as any).rebuildPorts?.()
    }
    graph.add(node)
  }
  for (const edge of store.edges) {
    const sourceNode = oldToNew.get(edge.source)
    const targetNode = oldToNew.get(edge.target)
    if (!sourceNode || !targetNode) {
      addLog(`⚠ 连线跳过: 节点 ${edge.source}→${edge.target} 不存在`, 'warn')
      continue
    }
    let sourceSlot = sourceNode.outputs?.findIndex((o: any) => o.name === edge.sourceHandle)
    let targetSlot = targetNode.inputs?.findIndex((i: any) => i.name === edge.targetHandle)
    if (sourceSlot < 0 && sourceNode.outputs && sourceNode.outputs.length > 0) sourceSlot = 0
    if (targetSlot < 0 && targetNode.inputs && targetNode.inputs.length > 0) targetSlot = 0
    if (sourceSlot >= 0 && targetSlot >= 0) {
      sourceNode.connect(sourceSlot, targetNode, targetSlot)
    } else {
      addLog(`⚠ 连线失败: ${sourceNode.type}[${edge.sourceHandle}]→${targetNode.type}[${edge.targetHandle}] 无可匹配端口`, 'warn')
    }
  }
}

// ─── v3: 按 ID 加载工作流 ───
async function loadWorkflowById(id: string) {
  try {
    const result = await safeInvoke<{ name: string; yaml: string }>('workflow_get', { id })
    if (result && result.yaml) {
      const data = JSON.parse(result.yaml)
      store.load({ name: data.name || result.name, nodes: data.nodes || [], edges: data.edges || [], variables: data.variables || {} })
      store.setWorkflowId(id)
      loadFromStore()
      addLog(`📋 已打开「${data.name || result.name}」(${store.nodeCount} 节点)`, 'info')
      fitView()
    }
  } catch (e: any) {
    addLog(`❌ 打开工作流失败: ${e}`, 'error')
  }
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
      if (def.defaultConfig[w.name] !== undefined) w.value = def.defaultConfig[w.name]
    }
  }
  const rect = canvasRef.value.getBoundingClientRect()
  const worldPos = canvas.convertOffsetToCanvas?.(
    [searchPos.value.x - rect.left, searchPos.value.y - rect.top], [0, 0]
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
  const raw = toRaw(node)
  const widget = raw.widgets?.find(w => w.name === widgetName)
  if (widget) {
    widget.value = value
    graph.setDirtyCanvas(true)
  }
  store.updateNodeConfig(String(raw.id), { [widgetName]: value })
  widgetVersion.value++
}

function onContainerUpdate() {
  if (!selectedLgNode.value) return
  const raw = toRaw(selectedLgNode.value)
  const actions = (raw.properties as any)?.actions
  if (actions) {
    store.updateNodeConfig(String(raw.id), { actions: [...actions] })
    graph.setDirtyCanvas(true)
  }
  widgetVersion.value++
}

function onDeleteNode(node: LGraphNode) {
  graph.remove(node)
  if (selectedLgNode.value === node) selectedLgNode.value = null
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
  consoleCollapsed.value = false
  store.resetAllStatuses()
  addLog('▶ DAG 执行开始...', 'info')
  runDagFlow(false)
}

function runSingle() {
  if (store.nodes.length === 0) return
  isRunning.value = true
  consoleCollapsed.value = false
  store.resetAllStatuses()
  addLog('⏯ 单步调试模式启动...', 'info')
  runDagFlow(true)
}

async function runFromNode(nodeId: string) {
  syncGraphToStore()
  const reachable = new Set<string>()
  const queue = [nodeId]
  reachable.add(nodeId)
  while (queue.length > 0) {
    const cur = queue.shift()!
    for (const edge of store.edges) {
      if (edge.source === cur && !reachable.has(edge.target)) {
        reachable.add(edge.target)
        queue.push(edge.target)
      }
    }
  }
  const filteredNodes = store.nodes.filter(n => reachable.has(n.id))
  const filteredEdges = store.edges.filter(e => reachable.has(e.source) && reachable.has(e.target))
  if (filteredNodes.length === 0) {
    addLog('⚠ 未找到下游节点', 'warn')
    return
  }
  isRunning.value = true
  consoleCollapsed.value = false
  store.resetAllStatuses()
  addLog(`▶ 从节点 [${nodeId}] 开始执行 (${filteredNodes.length} 节点)...`, 'info')

  const workflowJson = {
    name: store.workflowName || '未命名',
    description: '',
    nodes: filteredNodes.map(n => ({
      id: n.id, type: n.type, label: n.label, position: n.position, config: n.config,
    })),
    edges: filteredEdges.map(e => ({
      id: e.id, source: e.source, target: e.target, sourceHandle: e.sourceHandle, targetHandle: e.targetHandle,
    })),
    variables: {},
    breakpoints: Array.from(breakpoints.value),
  }

  try {
    await setupDagListeners()
    const runId = await safeInvoke<string>('dag_run_start', { workflowJson, stepMode: false })
    currentRunId.value = runId
    addLog(`🚀 DAG 运行已启动: ${runId.slice(0, 8)}... (从此运行)`, 'info')
  } catch (e) {
    addLog(`❌ DAG 启动失败: ${e}`, 'error')
    isRunning.value = false
    cleanupDagListeners()
  }
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
const stepStartTimes = new Map<string, number>()

async function runDagFlow(stepMode: boolean) {
  syncGraphToStore()
  const workflowJson = {
    name: store.workflowName || '未命名',
    description: '',
    nodes: store.nodes.map(n => ({
      id: n.id, type: n.type, label: n.label, position: n.position, config: n.config,
    })),
    edges: store.edges.map(e => ({
      id: e.id, source: e.source, target: e.target, sourceHandle: e.sourceHandle, targetHandle: e.targetHandle,
    })),
    variables: {},
    breakpoints: Array.from(breakpoints.value),
  }

  try {
    await setupDagListeners()
    const runId = await safeInvoke<string>('dag_run_start', { workflowJson, stepMode })
    currentRunId.value = runId
    addLog(`🚀 DAG 运行已启动: ${runId.slice(0, 8)}...${stepMode ? ' (单步模式)' : ''}`, 'info')
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
      if (status === 'running') {
        stepStartTimes.set(step_id, performance.now())
        addLog(`⏳ [${current_step}/${total_steps}] ${step_name}`, 'info')
      } else if (status === 'success') {
        const start = stepStartTimes.get(step_id)
        const ms = start ? Math.round(performance.now() - start) : 0
        const duration = ms >= 1000 ? `${(ms / 1000).toFixed(1)}s` : `${ms}ms`
        addLog(`✅ [${current_step}/${total_steps}] ${step_name} (${duration})`, 'success')
        stepStartTimes.delete(step_id)
      }
    }
  )
  dagEventUnlisteners.push(u1)

  const u1b = await safeListen<{ step_id: string; output: unknown }>(
    'step-output',
    (event) => { store.setNodeOutput(event.payload.step_id, event.payload.output) }
  )
  dagEventUnlisteners.push(u1b)

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

  const u3 = await safeListen<{ step_id: string; step_name: string; error?: string }>(
    'breakpoint-hit',
    (event) => {
      const { step_id, step_name } = event.payload
      addLog(`🔴 断点: ${step_name}`, 'warn')
      store.setNodeStatus(step_id, 'paused')
    }
  )
  dagEventUnlisteners.push(u3)

  const u4 = await safeListen<{ step_id: string; step_name: string; error?: string }>(
    'step-error',
    (event) => {
      const { step_id, step_name, error } = event.payload
      const start = stepStartTimes.get(step_id)
      const ms = start ? Math.round(performance.now() - start) : 0
      const duration = ms >= 1000 ? `${(ms / 1000).toFixed(1)}s` : `${ms}ms`
      addLog(`❌ ${step_name} (${duration}): ${error || '未知错误'}`, 'error')
      stepStartTimes.delete(step_id)
    }
  )
  dagEventUnlisteners.push(u4)
}

function cleanupDagListeners() {
  for (const unlisten of dagEventUnlisteners) unlisten()
  dagEventUnlisteners = []
}

function fitView() {
  const lgNodes = graph._nodes
  if (!canvas || !lgNodes || lgNodes.length === 0) return
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

// ─── 保存工作流 ───
async function onSaveWorkflow() {
  if (store.nodes.length === 0) {
    addLog('⚠ 画布为空，无需保存', 'warn')
    return
  }
  try {
    const id = store.workflowId
    if (id) {
      const yaml = JSON.stringify({ name: store.workflowName, nodes: store.nodes, edges: store.edges })
      await safeInvoke('workflow_save_yaml', { id, yaml })
      addLog(`💾 已保存「${store.workflowName}」`)
    } else {
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

// ─── 重命名工作流 ───
function onRenameWorkflow() {
  const name = prompt('输入工作流名称', store.workflowName || '未命名工作流')
  if (name && name.trim()) {
    store.workflowName = name.trim()
    document.title = `${name.trim()} — Workflow Engine`
    addLog(`✏ 已重命名为「${name.trim()}」`)
  }
}

// ─── 键盘快捷键 ───
function onKeyDown(e: KeyboardEvent) {
  // Ctrl+Z / Cmd+Z — 撤销
  if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key === 'z') {
    e.preventDefault()
    if (undoManager.undo()) {
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
  // Ctrl+S / Cmd+S — 保存工作流
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    e.preventDefault()
    onSaveWorkflow()
    return
  }
  // Ctrl+C — 复制选中节点
  if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key === 'c') {
    if (document.activeElement?.tagName === 'INPUT' || document.activeElement?.tagName === 'TEXTAREA') return
    e.preventDefault()
    canvas?.copyToClipboard?.()
    addLog('📋 已复制', 'info')
    return
  }
  // Ctrl+V — 粘贴节点
  if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key === 'v') {
    if (document.activeElement?.tagName === 'INPUT' || document.activeElement?.tagName === 'TEXTAREA') return
    e.preventDefault()
    canvas?.pasteFromClipboard?.()
    addLog('📌 已粘贴', 'info')
    return
  }
  // Ctrl+Enter / Cmd+Enter — 运行工作流
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
    e.preventDefault()
    runAll()
    return
  }
  // Ctrl+. — 单步执行
  if ((e.ctrlKey || e.metaKey) && e.key === '.') {
    e.preventDefault()
    runSingle()
    return
  }
  // Ctrl+F — 搜索节点
  if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
    e.preventDefault()
    const rect = canvasRef.value?.getBoundingClientRect()
    searchPos.value = rect ? { x: rect.left + rect.width / 2 - 120, y: rect.top + 100 } : { x: 200, y: 120 }
    searchVisible.value = true
    return
  }
  // Space — 缩放适配
  if (e.key === ' ' && !(document.activeElement?.tagName === 'INPUT' || document.activeElement?.tagName === 'TEXTAREA')) {
    e.preventDefault()
    fitView()
    return
  }
  // F — 聚焦选中节点
  if (e.key === 'f' && !e.ctrlKey && !e.metaKey && selectedLgNode.value) {
    if (document.activeElement?.tagName === 'INPUT' || document.activeElement?.tagName === 'TEXTAREA') return
    e.preventDefault()
    canvas?.centerOnNode?.(selectedLgNode.value)
    return
  }
  // Delete / Backspace — 删除选中节点
  if ((e.key === 'Delete' || e.key === 'Backspace') && selectedLgNode.value) {
    if (document.activeElement?.tagName === 'INPUT' || document.activeElement?.tagName === 'TEXTAREA') return
    e.preventDefault()
    onDeleteNode(selectedLgNode.value)
    return
  }
  // Escape — 三级：停止运行 > 取消选择 > 返回首页
  if (e.key === 'Escape') {
    if (isRunning.value) {
      e.preventDefault()
      stopRun()
      return
    }
    if (selectedLgNode.value) {
      onDeselectNode()
      return
    }
    emit('back')
    return
  }
  // ` — 折叠/展开控制台
  if (e.key === '`') {
    e.preventDefault()
    consoleCollapsed.value = !consoleCollapsed.value
    return
  }
}
</script>
