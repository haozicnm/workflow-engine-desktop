<script setup lang="ts">
import { ref, computed, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps<{
  workflowId: string
}>()

const emit = defineEmits<{
  (e: 'workflow-created', id: string): void
  (e: 'yaml-generated', yaml: string, summary: any[]): void
}>()

// ─── 状态 ───
type RecordingMode = 'idle' | 'browser' | 'desktop' | 'converting'
const mode = ref<RecordingMode>('idle')
const selectedMode = ref<'browser' | 'desktop'>('browser')
const actionCount = ref(0)
const elapsed = ref('00:00')
const error = ref('')
const showPreview = ref(false)
const previewSummary = ref<any[]>([])

let timer: ReturnType<typeof setInterval> | null = null
let seconds = 0

// ─── 计算属性 ───
const isRecording = computed(() => mode.value === 'browser' || mode.value === 'desktop')
const isIdle = computed(() => mode.value === 'idle')
const isConverting = computed(() => mode.value === 'converting')

const statusText = computed(() => {
  switch (mode.value) {
    case 'browser': return '📹 录制中 (浏览器)'
    case 'desktop': return '🖱️ 录制中 (桌面)'
    case 'converting': return '🔄 生成流程中...'
    default: return ''
  }
})

const statusClass = computed(() => {
  if (isRecording.value) return 'recording'
  if (isConverting.value) return 'converting'
  return ''
})

// ─── 录制控制 ───

async function startRecording() {
  error.value = ''
  try {
    const modeParam = selectedMode.value
    // 使用 workflow_validate 的 context 执行录制
    // 直接通过 invoke 调用 run 系统来执行录制节点
    // 这里简化：使用内部 step_test 来触发录制
    const config = {
      action: 'start',
      mode: modeParam,
      headless: modeParam === 'browser',
    }

    const result = await invoke('step_test', {
      stepType: 'recording',
      config: config,
      variables: null,
    })

    if (result && typeof result === 'object') {
      const r = result as any
      if (r.success === false) {
        error.value = r.error || '启动录制失败'
        return
      }
    }

    mode.value = modeParam
    actionCount.value = 0
    seconds = 0
    elapsed.value = '00:00'
    timer = setInterval(() => {
      seconds++
      const m = Math.floor(seconds / 60).toString().padStart(2, '0')
      const s = (seconds % 60).toString().padStart(2, '0')
      elapsed.value = `${m}:${s}`
    }, 1000)
  } catch (e: any) {
    // 桌面录制可能失败（非 Windows），回退到浏览器
    if (selectedMode.value === 'desktop') {
      error.value = '桌面录制不可用（仅支持 Windows），请使用浏览器录制'
    } else {
      error.value = `启动录制失败: ${e}`
    }
  }
}

async function stopRecording() {
  if (timer) { clearInterval(timer); timer = null }

  mode.value = 'converting'

  try {
    const config = {
      action: 'stop',
      workflow_name: generateWorkflowName(),
    }

    const result = await invoke('step_test', {
      stepType: 'recording',
      config: config,
      variables: null,
    })

    if (result && typeof result === 'object') {
      const r = result as any
      if (r.success === false) {
        error.value = r.error || '停止录制失败'
        mode.value = 'idle'
        return
      }

      // 获取动作计数和 YAML
      actionCount.value = r.output?.count || 0
      const yaml = r.output?.yaml || ''
      const summary = r.output?.step_summary || []

      if (yaml) {
        previewSummary.value = summary
        emit('yaml-generated', yaml, summary)
        showPreview.value = true
      }

      mode.value = 'idle'
    } else {
      mode.value = 'idle'
    }
  } catch (e: any) {
    error.value = `停止录制失败: ${e}`
    mode.value = 'idle'
  }
}

function cancelRecording() {
  if (timer) { clearInterval(timer); timer = null }
  mode.value = 'idle'
  actionCount.value = 0
  seconds = 0
  elapsed.value = '00:00'
  error.value = ''
  showPreview.value = false
}

async function applyToWorkflow() {
  // 创建新的工作流
  try {
    const name = generateWorkflowName()

    // 将预览摘要中的步骤信息传给父组件
    emit('workflow-created', name)
    showPreview.value = false
    previewSummary.value = []
  } catch (e: any) {
    error.value = `创建工作流失败: ${e}`
  }
}

function generateWorkflowName(): string {
  const now = new Date()
  const ts = `${now.getFullYear()}${(now.getMonth()+1).toString().padStart(2,'0')}${now.getDate().toString().padStart(2,'0')}_${now.getHours().toString().padStart(2,'0')}${now.getMinutes().toString().padStart(2,'0')}`
  const modeName = selectedMode.value === 'desktop' ? '桌面操作' : '浏览器操作'
  return `${modeName}_${ts}`
}

onUnmounted(() => {
  if (timer) clearInterval(timer)
})
</script>

<template>
  <!-- 录制浮动控制栏 -->
  <div class="recording-bar" :class="statusClass">
    <div class="recording-bar-inner">
      <!-- 左侧：状态 -->
      <div class="recording-status">
        <span v-if="isRecording" class="recording-dot"></span>
        <span v-if="statusText" class="status-label">{{ statusText }}</span>
        <span v-if="isRecording" class="elapsed">{{ elapsed }}</span>
      </div>

      <!-- 中间：模式选择（仅空闲时） -->
      <div v-if="isIdle" class="recording-mode-select">
        <button
          class="mode-btn"
          :class="{ active: selectedMode === 'browser' }"
          @click="selectedMode = 'browser'"
          title="录制浏览器操作"
        >
          🌐 浏览器
        </button>
        <button
          class="mode-btn"
          :class="{ active: selectedMode === 'desktop' }"
          @click="selectedMode = 'desktop'"
          title="录制桌面键鼠操作"
        >
          🖥️ 桌面
        </button>
      </div>

      <!-- 右侧：按钮 -->
      <div class="recording-actions">
        <!-- 开始录制 -->
        <button
          v-if="isIdle"
          class="btn btn-record"
          @click="startRecording"
          title="开始录制"
        >
          ⏺ 开始录制
        </button>

        <!-- 停止录制 -->
        <button
          v-if="isRecording"
          class="btn btn-stop"
          @click="stopRecording"
          title="停止录制并生成流程"
        >
          ⏹ 停止录制
        </button>

        <!-- 取消 -->
        <button
          v-if="isRecording"
          class="btn btn-cancel"
          @click="cancelRecording"
          title="取消录制"
        >
          取消
        </button>

        <!-- 转换中 -->
        <span v-if="isConverting" class="converting-text">
          <span class="spinner"></span> 正在生成工作流...
        </span>

        <span v-if="isRecording" class="action-count">
          已录 {{ actionCount || '...' }} 个操作
        </span>
      </div>
    </div>

    <!-- 错误提示 -->
    <div v-if="error" class="recording-error">
      ⚠️ {{ error }}
      <button class="dismiss-error" @click="error = ''">✕</button>
    </div>

    <!-- 预览面板 -->
    <div v-if="showPreview && previewSummary.length > 0" class="recording-preview">
      <div class="preview-header">
        <span>📋 生成的工作流预览 ({{ previewSummary.length }} 个步骤)</span>
        <button class="btn-close-preview" @click="showPreview = false">✕</button>
      </div>
      <div class="preview-steps">
        <div
          v-for="(step, i) in previewSummary"
          :key="step.id"
          class="preview-step"
        >
          <span class="step-index">{{ i + 1 }}</span>
          <span class="step-type-badge">{{ step.step_type }}</span>
          <span class="step-name">{{ step.name }}</span>
          <span class="step-desc">{{ step.description }}</span>
        </div>
      </div>
      <div class="preview-footer">
        <button class="btn btn-apply" @click="applyToWorkflow">
          ✅ 应用此工作流
        </button>
        <button class="btn btn-discard" @click="showPreview = false">
          暂不使用
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.recording-bar {
  position: relative;
  background: #1a1a2e;
  border-bottom: 2px solid #2a2a4e;
  padding: 0;
  z-index: 100;
  transition: border-color 0.3s;
}

.recording-bar.recording {
  border-bottom-color: #e74c3c;
  background: #1a0a0a;
}

.recording-bar.converting {
  border-bottom-color: #f39c12;
}

.recording-bar-inner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 16px;
  gap: 12px;
}

/* 状态区 */
.recording-status {
  display: flex;
  align-items: center;
  gap: 8px;
}

.recording-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: #e74c3c;
  animation: pulse 1.2s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.5; transform: scale(1.2); }
}

.status-label {
  font-size: 13px;
  color: #ccc;
  font-weight: 500;
}

.elapsed {
  font-family: 'Courier New', monospace;
  font-size: 13px;
  color: #e74c3c;
  background: rgba(231, 76, 60, 0.1);
  padding: 2px 8px;
  border-radius: 4px;
}

/* 模式选择 */
.recording-mode-select {
  display: flex;
  gap: 4px;
}

.mode-btn {
  background: transparent;
  border: 1px solid #333;
  color: #888;
  padding: 4px 12px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.2s;
}

.mode-btn:hover {
  border-color: #555;
  color: #aaa;
}

.mode-btn.active {
  background: #2563eb;
  border-color: #2563eb;
  color: white;
}

/* 按钮 */
.recording-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.btn {
  padding: 5px 14px;
  border-radius: 5px;
  border: none;
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  transition: all 0.2s;
}

.btn-record {
  background: #e74c3c;
  color: white;
}
.btn-record:hover {
  background: #c0392b;
}

.btn-stop {
  background: #2ecc71;
  color: white;
}
.btn-stop:hover {
  background: #27ae60;
}

.btn-cancel {
  background: transparent;
  border: 1px solid #555;
  color: #999;
}
.btn-cancel:hover {
  background: #333;
  color: #ccc;
}

.btn-apply {
  background: #2563eb;
  color: white;
  padding: 6px 16px;
}
.btn-apply:hover {
  background: #1d4ed8;
}

.btn-discard {
  background: transparent;
  border: 1px solid #444;
  color: #888;
  padding: 6px 16px;
}

.action-count {
  font-size: 12px;
  color: #666;
  margin-left: 8px;
}

.converting-text {
  color: #f39c12;
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.spinner {
  display: inline-block;
  width: 14px;
  height: 14px;
  border: 2px solid #f39c12;
  border-top-color: transparent;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* 错误提示 */
.recording-error {
  background: rgba(231, 76, 60, 0.1);
  color: #e74c3c;
  padding: 6px 16px;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.dismiss-error {
  background: none;
  border: none;
  color: #e74c3c;
  cursor: pointer;
  font-size: 14px;
}

/* 预览面板 */
.recording-preview {
  border-top: 1px solid #2a2a4e;
  background: #1e1e36;
  padding: 12px 16px;
}

.preview-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
  color: #ccc;
  font-size: 13px;
  font-weight: 500;
}

.btn-close-preview {
  background: none;
  border: none;
  color: #666;
  cursor: pointer;
  font-size: 16px;
}

.preview-steps {
  max-height: 200px;
  overflow-y: auto;
}

.preview-step {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 0;
  border-bottom: 1px solid #222;
  font-size: 12px;
}

.step-index {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: #2563eb;
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  flex-shrink: 0;
}

.step-type-badge {
  background: #333;
  color: #aaa;
  padding: 1px 6px;
  border-radius: 3px;
  font-size: 10px;
  text-transform: uppercase;
  flex-shrink: 0;
}

.step-name {
  color: #ddd;
  font-weight: 500;
  flex-shrink: 0;
  min-width: 0;
}

.step-desc {
  color: #666;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preview-footer {
  margin-top: 10px;
  display: flex;
  gap: 8px;
}
</style>
