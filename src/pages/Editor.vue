<script setup lang="ts">
import { ref, computed, onMounted, watch, onUnmounted, nextTick } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useWorkflowStore } from '../stores/workflow'
import { invoke } from '@tauri-apps/api/core'
import StepPalette from '../components/StepPalette.vue'
import StepCanvas from '../components/StepCanvas.vue'
import YamlPanel from '../components/YamlPanel.vue'
import VariablePanel from '../components/VariablePanel.vue'
import StepConfigDialog from '../components/step-config/StepConfigDialog.vue'
import RecordingBar from '../components/RecordingBar.vue'
import { useToast } from '../composables/useToast'

const route = useRoute()
const router = useRouter()
const store = useWorkflowStore()
const toast = useToast()

const showConfig = ref(false)
const editingStepId = ref<string | null>(null)
const showYaml = ref(true)
const showSidebar = ref(true)
const showDesc = ref(false)
const searchQuery = ref('')
const showSearch = ref(false)
let searchInputRef = ref<HTMLInputElement | null>(null)
let unlistenStepUpdate: (() => void) | null = null
let unlistenRunUpdate: (() => void) | null = null
let unlistenBreakpoint: (() => void) | null = null
let unlistenVariableUpdate: (() => void) | null = null

// 执行进度追踪
const runProgress = ref({ current: 0, total: 0, stepName: '' })

// 实时变量监视（非调试模式下也显示）
const liveVars = ref<Record<string, any>>({})
const liveStepOutputs = ref<Record<string, any>>({})

// ─── Undo/Redo ───
const undoStack = ref<string[]>([])
const undoIndex = ref(-1)
const isUndoRedoing = ref(false)
const MAX_UNDO = 50

function pushUndoState() {
  if (isUndoRedoing.value) return
  // 去重：内容未变化不压栈
  if (undoStack.value[undoIndex.value] === store.yamlText) return
  // 裁剪未来状态
  if (undoIndex.value < undoStack.value.length - 1) {
    undoStack.value = undoStack.value.slice(0, undoIndex.value + 1)
  }
  undoStack.value.push(store.yamlText)
  if (undoStack.value.length > MAX_UNDO) undoStack.value.shift()
  else undoIndex.value = undoStack.value.length - 1
}

function undo() {
  if (undoIndex.value <= 0) return
  undoIndex.value--
  isUndoRedoing.value = true
  store.parseYaml(undoStack.value[undoIndex.value])
  nextTick(() => { isUndoRedoing.value = false })
}

function redo() {
  if (undoIndex.value >= undoStack.value.length - 1) return
  undoIndex.value++
  isUndoRedoing.value = true
  store.parseYaml(undoStack.value[undoIndex.value])
  nextTick(() => { isUndoRedoing.value = false })
}

// ─── AutoSave ───
let autoSaveTimer: ReturnType<typeof setInterval> | null = null
let beforeUnloadHandler: ((e: BeforeUnloadEvent) => void) | null = null

function startAutoSave(intervalMs = 60000) {
  if (autoSaveTimer) return
  autoSaveTimer = setInterval(async () => {
    if (store.currentId && store.yamlText) {
      await store.saveWorkflow()
    }
  }, intervalMs)
  beforeUnloadHandler = (e: BeforeUnloadEvent) => {
    if (store.yamlText) {
      e.preventDefault()
      e.returnValue = ''
    }
  }
  window.addEventListener('beforeunload', beforeUnloadHandler)
}

function stopAutoSave() {
  if (autoSaveTimer) { clearInterval(autoSaveTimer); autoSaveTimer = null }
  if (beforeUnloadHandler) {
    window.removeEventListener('beforeunload', beforeUnloadHandler)
    beforeUnloadHandler = null
  }
}

onMounted(async () => {
  const id = route.params.id as string
  if (id && id !== 'new') {
    await store.loadWorkflow(id)
  } else {
    store.loadNew()
  }
  // 初始化 undo 栈
  if (store.yamlText) {
    undoStack.value = [store.yamlText]
    undoIndex.value = 0
  }
  setupExecListener()
  startAutoSave(60000)
  // 全局快捷键
  window.addEventListener('keydown', onKeyDown)
  // 加载断点
  if (store.currentId) {
    try {
      breakpoints.value = await invoke('debug_get_breakpoints', { workflowId: store.currentId })
    } catch (e) { /* 新工作流无断点 */ }
  }
})

onUnmounted(() => {
  unlistenStepUpdate?.()
  unlistenRunUpdate?.()
  unlistenBreakpoint?.()
  unlistenVariableUpdate?.()
  stopAutoSave()
  window.removeEventListener('keydown', onKeyDown)
})

// ─── 键盘快捷键 ───

function onKeyDown(e: KeyboardEvent) {
  // Ctrl+Z → Undo
  if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key === 'z') {
    e.preventDefault()
    undo()
  }
  // Ctrl+Shift+Z / Ctrl+Y → Redo
  if ((e.ctrlKey || e.metaKey) && (e.key === 'y' || (e.shiftKey && e.key === 'z'))) {
    e.preventDefault()
    redo()
  }
  // Ctrl+S / Cmd+S → 保存
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    e.preventDefault()
    onSave()
  }
  // Escape → 关闭弹窗
  if (e.key === 'Escape' && showConfig.value) {
    showConfig.value = false
    editingStepId.value = null
  }
  // Escape → 关闭搜索
  if (e.key === 'Escape' && showSearch.value) {
    showSearch.value = false
    searchQuery.value = ''
  }
  // Ctrl+F / Cmd+F → 搜索
  if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
    e.preventDefault()
    showSearch.value = true
    nextTick(() => searchInputRef.value?.focus())
  }
}

// 监听 YAML 变化 → 压入 Undo 栈
watch(() => store.yamlText, () => {
  pushUndoState()
})

// ─── 执行状态监听 ───

// 步骤耗时追踪
const stepTimers = ref<Record<string, number>>({})
const stepDurations = ref<Record<string, number>>({})

async function setupExecListener() {
  try {
    const { listen } = await import('@tauri-apps/api/event')
    unlistenStepUpdate = await listen('step-update', (event: any) => {
      const { step_id, status, output, error } = event.payload
      store.updateStepStatus(step_id, { status, output, error })

      // 耗时追踪
      if (status === 'running') {
        stepTimers.value[step_id] = Date.now()
      } else if (stepTimers.value[step_id]) {
        stepDurations.value[step_id] = Date.now() - stepTimers.value[step_id]
      }

      // 更新进度
      if (status === 'running') {
        const step = store.steps.find(s => s.id === step_id)
        const idx = store.steps.findIndex(s => s.id === step_id)
        runProgress.value = {
          current: idx + 1,
          total: store.steps.length,
          stepName: step?.name || step_id,
        }
      }
    })
    unlistenRunUpdate = await listen('run-update', (event: any) => {
      const { status, error } = event.payload
      store.running = false
      store.runningRunId = null
      isDebugging.value = false
      debugPaused.value = false
      debugVars.value = {}
      debugStepOutputs.value = {}
      debugCurrentStep.value = ''
      liveVars.value = {}
      liveStepOutputs.value = {}
      if (status === 'completed') {
        toast.success('工作流执行完成 ✅')
        runProgress.value = { current: 0, total: 0, stepName: '' }
      } else if (status === 'failed') {
        toast.error('工作流执行失败: ' + (error || '未知错误'))
        runProgress.value = { current: 0, total: 0, stepName: '' }
      }
    })
    // 断点命中事件
    unlistenBreakpoint = await listen('breakpoint-hit', (event: any) => {
      const { step_id, step_name, variables, step_outputs, reason } = event.payload
      debugPaused.value = true
      debugCurrentStep.value = step_id
      debugVars.value = variables || {}
      debugStepOutputs.value = step_outputs || {}
      if (reason === 'step_mode') {
        toast.info(`单步暂停: ${step_name}`)
      } else {
        toast.info(`断点命中: ${step_name}`)
      }
    })
    // 变量实时更新事件
    unlistenVariableUpdate = await listen('variable-update', (event: any) => {
      const { variables, step_outputs } = event.payload
      liveVars.value = variables || {}
      liveStepOutputs.value = step_outputs || {}
    })
  } catch (e) {
    console.warn('无法监听执行事件:', e)
  }
}

// ─── 步骤配置弹窗 ───

function onEditStep(id: string) {
  editingStepId.value = id
  showConfig.value = true
}

function onConfigSave(updates: { name: string; type: string; config: Record<string, any> }) {
  if (editingStepId.value) {
    store.updateStep(editingStepId.value, updates)
    toast.success('步骤已更新')
  }
  showConfig.value = false
  editingStepId.value = null
}

function onConfigClose() {
  showConfig.value = false
  editingStepId.value = null
}

// ─── YAML 双向同步 ───

let yamlDebounce: ReturnType<typeof setTimeout> | null = null

function onYamlChange(text: string) {
  if (yamlDebounce) clearTimeout(yamlDebounce)
  yamlDebounce = setTimeout(() => {
    store.parseYaml(text)
  }, 500)
}

function onSyncToYaml() {
  store.syncToYaml()
}

function onSyncFromYaml() {
  store.parseYaml(store.yamlText)
}

// ─── 工具栏操作 ───

async function onSave() {
  const ok = await store.saveWorkflow()
  if (ok) {
    toast.success('保存成功 💾')
  } else {
    toast.error('保存失败')
  }
}

async function onRun() {
  const runId = await store.runWorkflow()
  if (runId) {
    toast.info('工作流已启动 ⚡')
  } else {
    toast.error('启动失败')
  }
}

// ─── 调试控制 ───
const isDebugging = ref(false)
const debugPaused = ref(false)
const debugVars = ref<Record<string, any>>({})
const debugStepOutputs = ref<Record<string, any>>({})
const debugCurrentStep = ref('')
const breakpoints = ref<string[]>([])

async function onDebugRun() {
  // 以调试模式启动（step_mode=true）
  const runId = await store.runWorkflow()
  if (runId) {
    isDebugging.value = true
    debugPaused.value = false
    // 启动单步模式
    try { await invoke('debug_step', { runId }) } catch (e) { console.warn(e) }
    toast.info('调试模式已启动 🔧')
  }
}

async function onDebugStep() {
  // 单步执行
  if (!store.runningRunId) return
  try {
    await invoke('debug_step', { runId: store.runningRunId })
    debugPaused.value = false
  } catch (e: any) {
    toast.error('单步执行失败: ' + e)
  }
}

async function onDebugContinue() {
  // 继续执行
  if (!store.runningRunId) return
  try {
    await invoke('debug_continue', { runId: store.runningRunId })
    debugPaused.value = false
  } catch (e: any) {
    toast.error('继续执行失败: ' + e)
  }
}

async function onDebugStop() {
  if (!store.runningRunId) return
  try {
    await invoke('run_cancel', { runId: store.runningRunId })
    isDebugging.value = false
    debugPaused.value = false
    toast.info('调试已停止')
  } catch (e: any) {
    toast.error('停止失败: ' + e)
  }
}

async function onToggleBreakpoint(stepId: string) {
  if (!store.currentId) return
  try {
    const bps: string[] = await invoke('debug_get_breakpoints', { workflowId: store.currentId })
    if (bps.includes(stepId)) {
      await invoke('debug_remove_breakpoint', { workflowId: store.currentId, stepId })
      toast.info('断点已移除')
    } else {
      await invoke('debug_set_breakpoint', { workflowId: store.currentId, stepId })
      toast.success('断点已设置')
    }
    // 刷新断点列表
    breakpoints.value = await invoke('debug_get_breakpoints', { workflowId: store.currentId })
  } catch (e: any) {
    console.warn('断点操作失败:', e)
  }
}

async function onValidate() {
  const result = await store.validateYaml()
  if (result.valid) {
    toast.success('YAML 格式有效 ✅')
  } else {
    toast.error('YAML 格式无效: ' + result.error)
  }
}

function goBack() {
  router.push('/')
}

// ─── 计算属性 ───

const editingStep = computed(() => {
  if (!editingStepId.value) return null
  return store.steps.find(s => s.id === editingStepId.value) || null
})

// 步骤搜索过滤
function matchesSearch(step: any): boolean {
  if (!searchQuery.value) return true
  const q = searchQuery.value.toLowerCase()
  return (
    step.name?.toLowerCase().includes(q) ||
    step.type?.toLowerCase().includes(q) ||
    step.id?.toLowerCase().includes(q) ||
    JSON.stringify(step.config || {}).toLowerCase().includes(q)
  )
}

// ─── 变量面板 ───

const showVars = ref(true)

function copyRef(text: string) {
  navigator.clipboard.writeText(text)
  toast.success('已复制: ' + text)
}

function truncate(s: string, len: number) {
  return s.length > len ? s.substring(0, len) + '...' : s
}

function formatDuration(ms: number): string {
  if (ms < 1000) return ms + 'ms'
  return (ms / 1000).toFixed(1) + 's'
}

function getStatusIcon(stepId: string): string {
  const s = store.stepStatuses[stepId]
  if (!s) return ''
  switch (s.status) {
    case 'running': return '⚡'
    case 'completed': return '✅'
    case 'failed': return '❌'
    default: return ''
  }
}

// ─── 数据流增强 ───

// 每种节点类型的典型输出字段
const STEP_OUTPUTS_LOCAL: Record<string, string[]> = {
  http: ['status', 'body'],
  data: ['result'],
  script: ['result'],
  condition: ['result'],
  loop: ['count', 'results', 'collected', 'table'],
  while: ['count', 'results', 'collected', 'table'],
  map: ['result'],
  parallel: ['results'],
  browser: ['result'],
  web_scrape: ['items', 'total_items'],
  notify: ['sent'],
  approval: ['approved'],
  excel: ['headers', 'rows', 'row_count'],
  word: ['text', 'paragraphs'],
}

// 步骤间数据流连接
const dataFlow = computed(() => {
  const flows: { from: string; to: string; field: string }[] = []
  for (const step of store.steps) {
    const str = JSON.stringify(step.config || {})
    const regex = /\{\{(?:step_)([^}]+)\}\}/g
    let match
    while ((match = regex.exec(str)) !== null) {
      const ref = match[1].trim()
      const parts = ref.split('.')
      const fromId = parts[0]
      const field = parts.slice(1).join('.') || '(全部)'
      flows.push({ from: fromId, to: step.id, field })
    }
  }
  return flows
})

function getFlowStepName(id: string): string {
  const s = store.steps.find(s => s.id === id)
  return s ? s.name : id
}

// ─── 录制 → 工作流 ───

function onRecordingYaml(yaml: string, summary: any[]) {
  // 录制停止后生成的 YAML 和步骤概要
  store.yamlText = yaml
  store.parseYaml(yaml)
  toast.success(`录制完成！已转换为 ${summary.length} 个步骤的工作流`)
}

async function onRecordingWorkflow(name: string) {
  // 用户确认应用录制的工作流
  await store.saveWorkflow()
  toast.success(`工作流「${name}」已保存`)
}
</script>

<template>
  <div class="editor">
    <!-- 工具栏 -->
    <div class="editor-toolbar">
      <div class="toolbar-left">
        <button class="btn btn-sm" @click="goBack" title="返回首页">← 返回</button>
        <input
          v-model="store.workflowName"
          class="wf-name-input"
          placeholder="工作流名称"
          @change="store.syncToYaml"
        />
        <button class="btn btn-xs" @click="showDesc = !showDesc" title="编辑描述">
          📝 {{ showDesc ? '隐藏描述' : '描述' }}
        </button>

        <!-- 执行进度 -->
        <span v-if="store.running" class="run-progress">
          ⚡ 步骤 {{ runProgress.current }}/{{ runProgress.total }}: {{ runProgress.stepName }}
        </span>
      </div>
      <div class="toolbar-right">
        <button class="btn btn-xs" @click="showSidebar = !showSidebar">
          {{ showSidebar ? '◀ 隐藏面板' : '▶ 显示面板' }}
        </button>
        <button class="btn btn-xs" @click="showYaml = !showYaml">
          {{ showYaml ? '📝 隐藏YAML' : '📝 显示YAML' }}
        </button>
        <button class="btn btn-xs" @click="onValidate">✔ 验证</button>
        <button class="btn btn-xs" @click="router.push(store.currentId ? `/history?workflow_id=${store.currentId}` : '/history')" title="运行历史">📊 历史</button>
        <button class="btn btn-sm btn-primary" @click="onSave" :disabled="store.saving" title="Ctrl+S">
          {{ store.saving ? '保存中...' : '💾 保存' }}
        </button>
        <button class="btn btn-sm btn-success" @click="onRun" :disabled="store.running || !store.currentId">
          {{ store.running ? '执行中...' : '▶ 执行' }}
        </button>
        <button class="btn btn-sm btn-warning" @click="onDebugRun" :disabled="store.running || !store.currentId" title="调试模式启动">
          🔧 调试
        </button>
        <template v-if="isDebugging && store.running">
          <button v-if="debugPaused" class="btn btn-xs btn-primary" @click="onDebugStep" title="执行当前步骤后暂停">
            ⏭ 单步
          </button>
          <button v-if="debugPaused" class="btn btn-xs btn-success" @click="onDebugContinue" title="继续执行到下一个断点">
            ▶ 继续
          </button>
          <button class="btn btn-xs btn-danger" @click="onDebugStop" title="停止调试">
            ⏹ 停止
          </button>
        </template>
      </div>
    </div>

    <!-- 录制控制栏 -->
    <RecordingBar
      :workflow-id="store.currentId || 'new'"
      @yaml-generated="onRecordingYaml"
      @workflow-created="onRecordingWorkflow"
    />

    <!-- 描述编辑栏 -->
    <div v-if="showDesc" class="desc-bar">
      <textarea
        v-model="store.workflowDesc"
        class="desc-input"
        placeholder="工作流描述（可选）"
        rows="2"
        @change="store.syncToYaml"
      ></textarea>
    </div>

    <!-- 搜索栏 -->
    <div v-if="showSearch" class="search-bar">
      <span class="search-icon">🔍</span>
      <input
        ref="searchInputRef"
        v-model="searchQuery"
        class="search-input"
        placeholder="搜索步骤名称、类型... (Esc 关闭)"
        @keyup.escape="showSearch = false; searchQuery = ''"
      />
      <span v-if="searchQuery" class="search-count">
        {{ steps.value.filter(s => matchesSearch(s)).length }}/{{ store.steps.length }}
      </span>
      <button class="btn btn-xs" @click="showSearch = false; searchQuery = ''">✕</button>
    </div>

    <!-- 编辑区域 -->
    <div class="editor-body">
      <!-- 左侧：步骤面板 -->
      <div v-if="showSidebar" class="editor-sidebar">
        <StepPalette @add-step="store.addStep" />

        <!-- 全局变量编辑面板 -->
        <VariablePanel />

        <!-- 变量面板 -->
        <div class="var-panel">
          <div class="var-title" @click="showVars = !showVars">
            📋 变量与引用
            <span class="toggle-arrow" :class="{ open: showVars }">▸</span>
          </div>
          <div v-if="showVars" class="var-body">
            <!-- 数据流 -->
            <div v-if="dataFlow.length > 0" class="var-section">
              <div class="var-section-title">📊 数据流</div>
              <div v-for="(flow, idx) in dataFlow" :key="idx" class="dataflow-item">
                <span class="df-from">{{ getFlowStepName(flow.from) }}</span>
                <span class="df-arrow">→</span>
                <span class="df-to">{{ getFlowStepName(flow.to) }}</span>
                <span class="df-field">{{ flow.field }}</span>
              </div>
            </div>
            <!-- 步骤输出引用 -->
            <div class="var-section">
              <div class="var-section-title">步骤输出</div>
              <div v-if="store.steps.length === 0" class="var-empty">暂无步骤</div>
              <div v-else v-for="s in store.steps" :key="s.id" class="var-item var-item-step"
                   @click="copyRef(`{{step_${s.id}}}`)">
                <div class="var-step-row">
                  <span class="var-key">{{ s.name }}</span>
                  <span v-if="getStatusIcon(s.id)" class="var-status">{{ getStatusIcon(s.id) }}</span>
                </div>
                <div class="var-step-meta">
                  <span class="var-type">{{ s.type }}</span>
                  <span v-if="stepDurations[s.id]" class="var-duration">{{ formatDuration(stepDurations[s.id]) }}</span>
                  <span v-else-if="store.stepStatuses[s.id]?.status === 'running'" class="var-duration running">运行中...</span>
                </div>
                <!-- 可用输出字段 -->
                <div v-if="STEP_OUTPUTS_LOCAL[s.type]" class="var-fields">
                  <span v-for="field in STEP_OUTPUTS_LOCAL[s.type]" :key="field" class="var-field-chip"
                        @click.stop="copyRef(`{{step_${s.id}.${field}}}`)">
                    {{ field }}
                  </span>
                </div>
                <div v-if="store.stepStatuses[s.id]?.error" class="var-error">
                  {{ truncate(store.stepStatuses[s.id].error!, 60) }}
                </div>
              </div>
            </div>
            <!-- 🔧 调试变量监视 -->
            <div v-if="isDebugging && debugPaused" class="var-section debug-section">
              <div class="var-section-title">🔧 调试监视</div>
              <div class="debug-current" v-if="debugCurrentStep">
                当前步骤: <strong>{{ debugCurrentStep }}</strong>
              </div>
              <div v-if="Object.keys(debugVars).length > 0" class="debug-group">
                <div class="debug-group-title">运行时变量</div>
                <div v-for="(v, k) in debugVars" :key="k" class="var-item"
                     @click="copyRef(`{{${k}}}`)">
                  <span class="var-key">{{ k }}</span>
                  <span class="var-val">{{ truncate(JSON.stringify(v), 40) }}</span>
                </div>
              </div>
              <div v-if="Object.keys(debugStepOutputs).length > 0" class="debug-group">
                <div class="debug-group-title">步骤输出</div>
                <div v-for="(v, k) in debugStepOutputs" :key="k" class="var-item"
                     @click="copyRef(`{{step_${k}}}`)">
                  <span class="var-key">{{ k }}</span>
                  <span class="var-val">{{ truncate(JSON.stringify(v), 40) }}</span>
                </div>
              </div>
            </div>
            <!-- ⚡ 实时变量监视（非调试执行时也显示） -->
            <div v-if="store.running && !debugPaused && (Object.keys(liveStepOutputs).length > 0 || Object.keys(liveVars).length > 0)" class="var-section live-section">
              <div class="var-section-title">⚡ 实时变量</div>
              <div v-if="Object.keys(liveVars).length > 0" class="debug-group">
                <div class="debug-group-title">全局变量</div>
                <div v-for="(v, k) in liveVars" :key="k" class="var-item"
                     @click="copyRef(`{{${k}}}`)">
                  <span class="var-key">{{ k }}</span>
                  <span class="var-val">{{ truncate(JSON.stringify(v), 40) }}</span>
                </div>
              </div>
              <div v-if="Object.keys(liveStepOutputs).length > 0" class="debug-group">
                <div class="debug-group-title">步骤输出</div>
                <div v-for="(v, k) in liveStepOutputs" :key="k" class="var-item"
                     @click="copyRef(`{{step_${k}}}`)">
                  <span class="var-key">{{ k }}</span>
                  <span class="var-val">{{ truncate(JSON.stringify(v), 40) }}</span>
                </div>
              </div>
            </div>
            <div class="var-hint">💡 点击复制引用到剪贴板</div>
          </div>
        </div>
      </div>

      <!-- 中间：画布 -->
      <div class="editor-canvas">
        <StepCanvas
          :steps="store.steps"
          :step-statuses="store.stepStatuses"
          :breakpoints="breakpoints"
          :search-query="searchQuery"
          @edit-step="onEditStep"
          @remove-step="store.removeStep"
          @move-step="store.moveStep"
          @add-step="(type, idx) => store.addStep(type, idx)"
          @toggle-breakpoint="onToggleBreakpoint"
        />
      </div>

      <!-- 右侧：YAML 面板 -->
      <div v-if="showYaml" class="editor-yaml">
        <YamlPanel
          :yaml-text="store.yamlText"
          :error="store.yamlError"
          @change="onYamlChange"
          @sync-to-yaml="onSyncToYaml"
          @sync-from-yaml="onSyncFromYaml"
        />
      </div>
    </div>

    <!-- 配置弹窗 -->
    <StepConfigDialog
      v-if="showConfig"
      :step="editingStep"
      :all-steps="store.steps"
      @save="onConfigSave"
      @close="onConfigClose"
    />
  </div>
</template>

<style scoped>
.editor { display: flex; flex-direction: column; height: 100%; }
.editor-toolbar {
  display: flex; justify-content: space-between; align-items: center;
  padding: 8px 16px; background: #161b22; border-bottom: 1px solid #30363d;
  flex-shrink: 0; gap: 12px;
}
.toolbar-left, .toolbar-right { display: flex; align-items: center; gap: 8px; }
.wf-name-input {
  background: #0d1117; border: 1px solid #30363d; color: #e1e4e8;
  padding: 4px 10px; border-radius: 6px; font-size: 14px; font-weight: 600; width: 200px;
}
.wf-name-input:focus { outline: none; border-color: #58a6ff; }

.run-progress {
  font-size: 12px; font-weight: 600; color: #58a6ff;
  background: #1f6feb15; padding: 3px 10px; border-radius: 6px;
  animation: pulse 1.5s infinite;
}
@keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.6; } }

.desc-bar {
  padding: 8px 16px; background: #161b22; border-bottom: 1px solid #21262d;
}
.desc-input {
  width: 100%; background: #0d1117; border: 1px solid #30363d; color: #c9d1d9;
  padding: 6px 10px; border-radius: 6px; font-size: 12px; resize: none; box-sizing: border-box;
}
.desc-input:focus { outline: none; border-color: #58a6ff; }

/* ─── 搜索栏 ─── */
.search-bar {
  display: flex; align-items: center; gap: 8px;
  padding: 6px 16px; background: #161b22; border-bottom: 1px solid #58a6ff33;
}
.search-icon { font-size: 14px; flex-shrink: 0; }
.search-input {
  flex: 1; background: #0d1117; border: 1px solid #30363d; color: #c9d1d9;
  padding: 5px 10px; border-radius: 6px; font-size: 13px;
  outline: none; max-width: 400px;
}
.search-input:focus { border-color: #58a6ff; }
.search-count { font-size: 11px; color: #6e7681; flex-shrink: 0; }

.editor-body { display: flex; flex: 1; overflow: hidden; }
.editor-sidebar { width: 200px; flex-shrink: 0; border-right: 1px solid #30363d; overflow-y: auto; }
.editor-canvas { flex: 1; overflow-y: auto; padding: 16px; background: #0d1117; }
.editor-yaml { width: 360px; flex-shrink: 0; border-left: 1px solid #30363d; overflow: hidden; }

/* ─── 变量面板 ─── */
.var-panel { border-top: 1px solid #30363d; }
.var-title {
  display: flex; justify-content: space-between; align-items: center;
  padding: 8px 12px; font-size: 11px; font-weight: 600; color: #8b949e;
  cursor: pointer; user-select: none;
}
.var-title:hover { background: #21262d; }
.toggle-arrow { transition: transform 0.2s; }
.toggle-arrow.open { transform: rotate(90deg); }
.var-body { padding: 0 12px 8px; }
.var-section { margin-bottom: 8px; }
.var-section-title { font-size: 10px; color: #6e7681; text-transform: uppercase; margin-bottom: 4px; letter-spacing: 0.5px; }
.var-empty { font-size: 11px; color: #484f58; padding: 4px 0; }
.var-item {
  display: flex; justify-content: space-between; align-items: center;
  padding: 3px 6px; border-radius: 4px; cursor: pointer; font-size: 11px;
}
.var-item:hover { background: #21262d; }
.var-key { color: #c9d1d9; font-family: monospace; }
.var-val { color: #6e7681; font-size: 10px; max-width: 80px; overflow: hidden; text-overflow: ellipsis; }
.var-type { color: #6e7681; font-size: 10px; }
.var-duration { color: #3fb950; font-size: 10px; font-family: monospace; }
.var-duration.running { color: #d29922; animation: pulse 1.5s infinite; }
.var-status { font-size: 12px; }
.var-step-row { display: flex; justify-content: space-between; align-items: center; }
.var-step-meta { display: flex; gap: 6px; align-items: center; margin-top: 1px; }
.var-item-step { flex-direction: column; align-items: flex-start; gap: 1px; }
.var-error { font-size: 10px; color: #f85149; margin-top: 2px; word-break: break-all; }
.var-hint { font-size: 10px; color: #484f58; text-align: center; padding-top: 4px; border-top: 1px solid #21262d; }

/* ─── 输出字段 chips ─── */
.var-fields { display: flex; flex-wrap: wrap; gap: 2px; margin-top: 3px; padding-left: 4px; }
.var-field-chip {
  font-size: 9px; color: #79c0ff; background: #1f6feb18; border: 1px solid #1f6feb22;
  padding: 0 5px; border-radius: 3px; cursor: pointer; font-family: 'Cascadia Code', 'Fira Code', monospace;
  transition: background 0.15s;
}
.var-field-chip:hover { background: #1f6feb44; border-color: #1f6feb55; }

/* ─── 数据流连接 ─── */
.dataflow-item {
  display: flex; align-items: center; gap: 3px;
  font-size: 10px; padding: 2px 4px; border-radius: 3px;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
}
.dataflow-item:hover { background: #21262d; }
.df-from { color: #79c0ff; }
.df-arrow { color: #484f58; }
.df-to { color: #c9d1d9; }
.df-field { color: #6e7681; font-size: 9px; margin-left: 2px; }

/* ─── 调试监视面板 ─── */
.debug-section { background: #1a1e24; border-radius: 6px; padding: 6px 8px; margin-top: 4px; border: 1px solid #f59e0b33; }
.debug-current { font-size: 11px; color: #f59e0b; margin-bottom: 4px; }
.debug-group { margin-top: 4px; }
.debug-group-title { font-size: 10px; color: #8b949e; font-weight: 600; margin-bottom: 2px; text-transform: uppercase; }
/* ─── 实时变量监视 ─── */
.live-section { background: #161b22; border-radius: 6px; padding: 6px 8px; margin-top: 4px; border: 1px solid #58a6ff33; animation: live-pulse 2s infinite; }
@keyframes live-pulse { 0%, 100% { border-color: #58a6ff22; } 50% { border-color: #58a6ff66; } }
</style>
