<script setup lang="ts">
import { ref } from 'vue'
import type { WorkflowStep, StepStatus } from '../types/workflow'
import { STEP_COLORS, STEP_LABELS, STEP_ICONS } from '../types/workflow'

const props = defineProps<{
  steps: WorkflowStep[]
  stepStatuses: Record<string, StepStatus>
  breakpoints?: string[]
  searchQuery?: string
}>()

const emit = defineEmits<{
  'edit-step': [id: string]
  'remove-step': [id: string]
  'move-step': [from: number, to: number]
  'add-step': [type: string, index: number]
  'toggle-breakpoint': [stepId: string]
}>()

const dragIndex = ref<number | null>(null)
const dragOverIndex = ref<number | null>(null)
const paletteDragOver = ref(false)

// ─── 搜索匹配 ───
function stepMatchesSearch(step: WorkflowStep): boolean {
  if (!props.searchQuery) return true
  const q = props.searchQuery.toLowerCase()
  return (
    step.name?.toLowerCase().includes(q) ||
    step.type?.toLowerCase().includes(q) ||
    step.id?.toLowerCase().includes(q) ||
    JSON.stringify(step.config || {}).toLowerCase().includes(q)
  )
}

// ─── 数据流：提取配置中的变量引用 ───
function extractRefs(config: Record<string, unknown>): string[] {
  const refs: Set<string> = new Set()
  const str = JSON.stringify(config || {})
  const regex = /\{\{([^}]+)\}\}/g
  let match
  while ((match = regex.exec(str)) !== null) {
    const ref = match[1].trim()
    // 过滤掉循环内置变量
    if (!ref.startsWith('__item') && !ref.startsWith('__index') && !ref.startsWith('__current')) {
      refs.add(ref)
    }
  }
  return [...refs]
}

// ─── 数据流：每种节点类型的典型输出字段 ───
const STEP_OUTPUTS: Record<string, string[]> = {
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

// ─── 拖拽排序 ───

function handleDragStart(e: DragEvent, index: number) {
  dragIndex.value = index
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = 'move'
    e.dataTransfer.setData('text/plain', '')
  }
}

function handleDragOver(e: DragEvent, index: number) {
  e.preventDefault()
  dragOverIndex.value = index
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
}

function handleDrop(e: DragEvent, index: number) {
  e.preventDefault()
  if (dragIndex.value !== null && dragIndex.value !== index) {
    emit('move-step', dragIndex.value, index)
  }
  dragIndex.value = null
  dragOverIndex.value = null
}

function handleDragEnd() {
  dragIndex.value = null
  dragOverIndex.value = null
}

// ─── 从面板拖入 ───

function handleCanvasDragOver(e: DragEvent) {
  e.preventDefault()
  paletteDragOver.value = true
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy'
}

function handleCanvasDragLeave() {
  paletteDragOver.value = false
}

function handleCanvasDrop(e: DragEvent) {
  e.preventDefault()
  paletteDragOver.value = false
  const type = e.dataTransfer?.getData('step-type')
  if (type) emit('add-step', type, props.steps.length)
}

// ─── 工具函数 ───

function moveUp(i: number) { if (i > 0) emit('move-step', i, i - 1) }
function moveDown(i: number) { if (i < props.steps.length - 1) emit('move-step', i, i + 1) }

/** 根据 step id 查找步骤名称 */
function getStepName(id: string | undefined): string {
  if (!id) return ''
  const s = props.steps.find(s => s.id === id)
  return s ? s.name : id
}

function configSummary(step: WorkflowStep): string {
  const c = step.config
  switch (step.type) {
    case 'http': return `${c.action || 'GET'} ${c.url || ''}`
    case 'data': return `${c.action || ''} ${c.key || ''}`
    case 'script': return (c.script || '').substring(0, 60)
    case 'condition': {
      if (c.op) {
        return `${c.left || ''} ${c.op} ${c.right || ''}`
      }
      const p = [`if ${(c.script || '').substring(0, 40)}`]
      if (c.true_next) p.push(`✅→${getStepName(c.true_next)}`)
      if (c.false_next) p.push(`❌→${getStepName(c.false_next)}`)
      return p.join(' ')
    }
    case 'loop': {
      const items = c.items || []
      const body = c.body || []
      return `遍历 ${Array.isArray(items) ? items.length : '…'} 项 · 循环体 ${body.length} 步`
    }
    case 'while': {
      const items = c.items || []
      const body = c.body || []
      const cond = c.condition || {}
      return `条件: ${cond.op || 'not_empty'}${cond.check ? ' @' + cond.check : ''} · 循环体 ${body.length} 步`
    }
    case 'web_scrape': return `${c.url || ''} · ${(c.extract || []).length} 规则`
    case 'map': return `映射 ${c.source || ''}`
    case 'parallel': return `${(c.branches || []).length} 个分支`
    case 'browser': return `${c.action || ''} ${c.params?.url || c.params?.selector || ''}`
    case 'notify': return `${c.notify_type || 'system'}: ${c.title || ''}`
    case 'approval': return c.message || ''
    case 'excel': return `${c.action || ''} ${c.path || ''}`
    case 'word': return `${c.action || ''} ${c.path || ''}`
    default: return JSON.stringify(c).substring(0, 60)
  }
}

function getStatusClass(stepId: string): string {
  const s = props.stepStatuses[stepId]
  return s ? `status-${s.status}` : ''
}

function getStatusIcon(stepId: string): string {
  const s = props.stepStatuses[stepId]
  if (!s) return ''
  return s.status === 'running' ? '⏳' : s.status === 'completed' ? '✅' : '❌'
}

/** 是否有分支/展开内容 */
function hasExpansion(step: WorkflowStep): boolean {
  if (step.type === 'parallel') return (step.config.branches || []).length > 0
  if (step.type === 'loop') return (step.config.body || []).length > 0
  if (step.type === 'while') return (step.config.body || []).length > 0
  if (step.type === 'condition') return !!(step.config.true_next || step.config.false_next)
  return false
}

function hasBranch(step: WorkflowStep): boolean {
  return step.type === 'condition' || step.type === 'loop' || step.type === 'while' || step.type === 'parallel'
}
</script>

<template>
  <div
    class="canvas"
    :class="{ 'drag-active': paletteDragOver }"
    @dragover="handleCanvasDragOver"
    @dragleave="handleCanvasDragLeave"
    @drop="handleCanvasDrop"
  >
    <!-- 空状态 -->
    <div v-if="steps.length === 0" class="empty-canvas">
      <div class="empty-icon">📋</div>
      <div class="empty-text">从左侧点击或拖入步骤类型</div>
      <div class="empty-hint">支持拖拽排序，点击步骤编辑配置</div>
    </div>

    <!-- 步骤列表 -->
    <div v-else class="steps-list">
      <!-- 开始节点 -->
      <div class="flow-node start">
        <div class="flow-dot start-dot"></div>
        <span>开始</span>
      </div>
      <div class="flow-line"></div>

      <template v-for="(step, index) in steps" :key="step.id">
        <!-- 步骤卡片 -->
        <div
          class="step-card"
          :class="{
            dragging: dragIndex === index,
            'drag-over': dragOverIndex === index,
            branch: hasBranch(step),
            expanded: hasExpansion(step),
            [getStatusClass(step.id)]: true,
            'search-no-match': props.searchQuery && !stepMatchesSearch(step),
          }"
          :style="{ '--accent': STEP_COLORS[step.type] || '#666' }"
          draggable="true"
          @dragstart="handleDragStart($event, index)"
          @dragover="handleDragOver($event, index)"
          @drop.stop="handleDrop($event, index)"
          @dragend="handleDragEnd"
        >
          <!-- 主行 -->
          <div class="step-main">
            <div class="step-index" :class="{ 'has-breakpoint': breakpoints?.includes(step.id) }"
                 @click.stop="emit('toggle-breakpoint', step.id)" title="点击切换断点">
              {{ breakpoints?.includes(step.id) ? '●' : index + 1 }}
            </div>
            <div class="step-icon">
              {{ STEP_ICONS[step.type] || '📦' }}
              <span v-if="stepStatuses[step.id]" class="status-indicator">{{ getStatusIcon(step.id) }}</span>
            </div>
            <div class="step-info" @click="emit('edit-step', step.id)">
              <div class="step-name">{{ step.name }}</div>
              <div class="step-badges">
                <span class="type-badge">{{ STEP_LABELS[step.type] || step.type }}</span>
                <span v-if="step.timeout" class="meta-badge">⏱ {{ step.timeout }}s</span>
                <span v-if="step.retry" class="meta-badge">🔁 ×{{ step.retry.max }}</span>
              </div>
              <div class="step-config">{{ configSummary(step) }}</div>
              <!-- 数据流指示器 -->
              <div v-if="extractRefs(step.config).length > 0 || (STEP_OUTPUTS[step.type] || []).length > 0" class="step-dataflow">
                <span v-for="ref in extractRefs(step.config)" :key="ref" class="flow-badge in" :title="'读取: ' + ref">
                  📥 {{ ref.replace(/^step_/, '').replace(/^output\./, '') }}
                </span>
                <span v-for="out in (STEP_OUTPUTS[step.type] || [])" :key="out" class="flow-badge out" :title="step.name + ' 输出: ' + out">
                  📤 {{ out }}
                </span>
              </div>
              <div v-if="stepStatuses[step.id]?.error" class="step-error">
                ❌ {{ stepStatuses[step.id].error }}
              </div>
            </div>
            <div class="step-actions">
              <button class="ab" @click.stop="moveUp(index)" :disabled="index === 0" title="上移">▲</button>
              <button class="ab" @click.stop="moveDown(index)" :disabled="index === steps.length - 1" title="下移">▼</button>
              <button class="ab" @click.stop="emit('edit-step', step.id)" title="编辑">✏️</button>
              <button class="ab danger" @click.stop="emit('remove-step', step.id)" title="删除">🗑</button>
            </div>
          </div>

          <!-- ═══ 展开区：并行分支 ═══ -->
          <div v-if="step.type === 'parallel' && (step.config.branches || []).length > 0" class="expansion parallel-expansion">
            <div class="branch-grid" :style="{ gridTemplateColumns: `repeat(${Math.min(step.config.branches.length, 4)}, 1fr)` }">
              <div v-for="(branch, bIdx) in step.config.branches" :key="bIdx" class="branch-col">
                <div class="branch-col-header">
                  <span class="branch-col-icon">🔀</span>
                  <span>分支 {{ bIdx + 1 }}</span>
                  <span class="branch-col-count">{{ (branch || []).length }} 步</span>
                </div>
                <div v-if="branch && branch.length > 0">
                  <div v-for="sub in branch" :key="sub.id" class="sub-step-row">
                    <span class="sub-icon">{{ STEP_ICONS[sub.type] || '📦' }}</span>
                    <span class="sub-name">{{ sub.name }}</span>
                    <span class="sub-type-tag">{{ STEP_LABELS[sub.type] || sub.type }}</span>
                  </div>
                </div>
                <div v-else class="sub-empty">空分支</div>
              </div>
            </div>
          </div>

          <!-- ═══ 展开区：循环体 ═══ -->
          <div v-if="(step.type === 'loop' || step.type === 'while') && (step.config.body || []).length > 0" class="expansion loop-expansion">
            <div class="loop-header-label">
              <span>{{ step.type === 'while' ? '🔁' : '🔄' }}</span>
              {{ step.type === 'while' ? 'While循环体' : '循环体' }}
              <span class="loop-count">{{ (step.config.body || []).length }} 步</span>
              <span v-if="step.type === 'while' && step.config.condition" class="while-cond-badge">
                {{ step.config.condition.op || 'not_empty' }}{{ step.config.condition.check ? ' · ' + step.config.condition.check : '' }}
              </span>
            </div>
            <div class="loop-body-list">
              <div v-for="sub in step.config.body" :key="sub.id" class="sub-step-row">
                <span class="sub-icon">{{ STEP_ICONS[sub.type] || '📦' }}</span>
                <span class="sub-name">{{ sub.name }}</span>
                <span class="sub-type-tag">{{ STEP_LABELS[sub.type] || sub.type }}</span>
              </div>
            </div>
          </div>

          <!-- ═══ 展开区：条件分支 ═══ -->
          <div v-if="step.type === 'condition' && (step.config.true_next || step.config.false_next)" class="expansion condition-expansion">
            <div class="cond-grid">
              <div class="cond-branch cond-true">
                <div class="cond-branch-label">
                  <span>✅</span>
                  <span>True</span>
                </div>
                <div class="cond-target">
                  → {{ getStepName(step.config.true_next) || '下一步' }}
                </div>
              </div>
              <div class="cond-divider"></div>
              <div class="cond-branch cond-false">
                <div class="cond-branch-label">
                  <span>❌</span>
                  <span>False</span>
                </div>
                <div class="cond-target">
                  → {{ getStepName(step.config.false_next) || '结束' }}
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 连接线 -->
        <div v-if="index < steps.length - 1" class="flow-line" :class="{ branch: hasBranch(step) }">
          <span v-if="step.type === 'condition'" class="flow-label condition-label">条件分支</span>
          <span v-if="step.type === 'loop'" class="flow-label loop-label">🔄 循环</span>
          <span v-if="step.type === 'while'" class="flow-label while-label">🔁 While</span>
          <span v-if="step.type === 'parallel'" class="flow-label parallel-label">⚡ 并行</span>
        </div>
      </template>

      <!-- 结束节点 -->
      <div class="flow-line"></div>
      <div class="flow-node end">
        <div class="flow-dot end-dot"></div>
        <span>结束</span>
      </div>
    </div>

    <!-- 画布拖入提示 -->
    <div v-if="paletteDragOver" class="drop-hint">📥 松开添加到末尾</div>

    <!-- 底部 -->
    <div v-if="steps.length > 0" class="canvas-footer">
      {{ steps.length }} 个步骤 · 拖拽排序 · 点击编辑
    </div>
  </div>
</template>

<style scoped>
.canvas {
  min-height: 100%;
  position: relative;
  padding-bottom: 40px;
}
.canvas.drag-active {
  background: #1f6feb08;
  outline: 2px dashed #1f6feb44;
  outline-offset: -4px;
  border-radius: 8px;
}

/* ─── 流程线 ─── */
.flow-node {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.5px;
  padding: 4px 0;
}
.flow-node.start { color: #3fb950; }
.flow-node.end { color: #8b949e; }
.flow-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
}
.start-dot { background: #3fb950; box-shadow: 0 0 6px #3fb95066; }
.end-dot { background: #484f58; }

.flow-line {
  width: 2px;
  height: 20px;
  background: #30363d;
  margin-left: 20px;
  position: relative;
}
.flow-line.branch {
  height: 28px;
  background: repeating-linear-gradient(
    to bottom,
    #30363d 0px,
    #30363d 4px,
    transparent 4px,
    transparent 8px
  );
}
.flow-label {
  position: absolute;
  left: 10px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 10px;
  font-weight: 600;
  white-space: nowrap;
  padding: 1px 6px;
  border-radius: 4px;
}
.condition-label { color: #f59e0b; background: #f59e0b15; }
.loop-label { color: #06b6d4; background: #06b6d415; }
.while-label { color: #0891b2; background: #0891b215; }
.parallel-label { color: #f97316; background: #f9731615; }

/* ─── 数据流指示器 ─── */
.step-dataflow {
  display: flex; flex-wrap: wrap; gap: 3px; margin-top: 4px;
}
.flow-badge {
  font-size: 9px; font-weight: 500;
  padding: 1px 5px; border-radius: 3px;
  white-space: nowrap; cursor: default;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
}
.flow-badge.in {
  color: #79c0ff; background: #1f6feb18; border: 1px solid #1f6feb22;
}
.flow-badge.out {
  color: #7ee787; background: #23863618; border: 1px solid #23863622;
}

.while-cond-badge {
  font-size: 10px; color: #0891b2; background: #0891b218;
  padding: 0 5px; border-radius: 3px; margin-left: 4px; font-weight: 400;
}

/* ─── 空状态 ─── */
.empty-canvas {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 300px;
  gap: 8px;
}
.empty-icon { font-size: 48px; }
.empty-text { color: #8b949e; font-size: 15px; }
.empty-hint { color: #6e7681; font-size: 12px; }

/* ─── 步骤卡片 ─── */
.steps-list {
  padding: 0 16px;
}

.step-card {
  background: #161b22;
  border: 1px solid #30363d;
  border-left: 3px solid var(--accent);
  border-radius: 8px;
  transition: all 0.15s;
  user-select: none;
  overflow: hidden;
}
.step-card:hover {
  border-color: var(--accent);
  background: #1c2128;
  box-shadow: 0 2px 8px rgba(0,0,0,0.2);
}
.step-card.dragging { opacity: 0.4; }
.step-card.drag-over {
  border-color: #58a6ff;
  box-shadow: 0 0 0 1px #58a6ff, 0 0 12px #1f6feb33;
  transform: scale(1.01);
}
.step-card.branch {
  border-left-style: dashed;
}
.step-card.expanded {
  border-left-width: 4px;
}
.step-card.status-running {
  border-color: #58a6ff;
  animation: runningPulse 1.5s infinite;
}
.step-card.status-completed { border-color: #238636; }
.step-card.status-failed { border-color: #da3633; background: #da363308; }

@keyframes runningPulse {
  0%, 100% { background: #1c2128; }
  50% { background: #1f6feb08; }
}

/* ─── 主行 ─── */
.step-main {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
}

.step-index {
  width: 22px; height: 22px;
  display: flex; align-items: center; justify-content: center;
  font-size: 10px; font-weight: 700;
  color: var(--accent);
  background: color-mix(in srgb, var(--accent) 12%, transparent);
  border-radius: 50%; flex-shrink: 0;
  cursor: pointer; transition: all 0.15s;
}
.step-index:hover { transform: scale(1.15); }
.step-index.has-breakpoint {
  background: #ef4444; color: white;
  box-shadow: 0 0 6px #ef444488;
  animation: bp-pulse 2s infinite;
}
@keyframes bp-pulse {
  0%, 100% { box-shadow: 0 0 6px #ef444488; }
  50% { box-shadow: 0 0 12px #ef4444cc; }
}

.step-icon {
  font-size: 20px; flex-shrink: 0; position: relative;
}
.status-indicator {
  position: absolute; top: -5px; right: -5px; font-size: 11px;
}

.step-info { flex: 1; min-width: 0; cursor: pointer; }
.step-name {
  font-weight: 600; font-size: 13px; color: #e1e4e8;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.step-badges { display: flex; gap: 4px; margin-top: 3px; flex-wrap: wrap; }
.type-badge {
  font-size: 10px; font-weight: 600;
  color: var(--accent);
  background: color-mix(in srgb, var(--accent) 15%, transparent);
  padding: 1px 6px; border-radius: 4px;
}
.meta-badge {
  font-size: 10px; color: #6e7681; background: #21262d;
  padding: 1px 5px; border-radius: 4px;
}
.step-config {
  font-size: 11px; color: #6e7681;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  margin-top: 3px;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
}
.step-error {
  font-size: 11px; color: #f85149; margin-top: 3px;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}

.step-actions { display: flex; gap: 2px; flex-shrink: 0; }
.ab {
  background: none; border: 1px solid transparent;
  cursor: pointer; font-size: 12px; padding: 3px 5px;
  border-radius: 4px; opacity: 0.4; transition: all 0.15s; color: #c9d1d9;
}
.ab:hover:not(:disabled) { opacity: 1; background: #21262d; border-color: #30363d; }
.ab:disabled { opacity: 0.15; cursor: not-allowed; }
.ab.danger:hover { background: #da363322; border-color: #da363344; }

/* ─── 展开区通用 ─── */
.expansion {
  padding: 0 14px 12px;
  border-top: 1px solid #21262d;
}

/* ─── 子步骤行 ─── */
.sub-step-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 8px;
  border-radius: 4px;
  font-size: 12px;
  transition: background 0.1s;
}
.sub-step-row:hover { background: #21262d; }
.sub-icon { font-size: 14px; flex-shrink: 0; }
.sub-name {
  flex: 1; color: #c9d1d9; min-width: 0;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.sub-type-tag {
  font-size: 10px; color: #6e7681; background: #21262d;
  padding: 0 5px; border-radius: 3px; flex-shrink: 0;
}
.sub-empty {
  font-size: 11px; color: #484f58; text-align: center; padding: 8px;
}

/* ─── 并行分支 ─── */
.parallel-expansion { padding-top: 10px; }
.branch-grid {
  display: grid;
  gap: 8px;
}
.branch-col {
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 6px;
  padding: 8px;
  min-width: 0;
}
.branch-col-header {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  font-weight: 600;
  color: #8b949e;
  margin-bottom: 6px;
  padding-bottom: 4px;
  border-bottom: 1px solid #21262d;
}
.branch-col-icon { font-size: 13px; }
.branch-col-count {
  margin-left: auto;
  font-size: 10px;
  color: #484f58;
  font-weight: 400;
}

/* ─── 循环体 ─── */
.loop-expansion { padding-top: 10px; }
.loop-header-label {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  font-weight: 600;
  color: #06b6d4;
  margin-bottom: 6px;
}
.loop-count {
  font-size: 10px;
  color: #484f58;
  font-weight: 400;
  margin-left: 4px;
}
.loop-body-list {
  margin-left: 8px;
  padding-left: 12px;
  border-left: 2px solid #06b6d444;
}

/* ─── 条件分支 ─── */
.condition-expansion { padding-top: 10px; }
.cond-grid {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  gap: 0;
  align-items: stretch;
}
.cond-branch {
  background: #0d1117;
  border-radius: 6px;
  padding: 10px;
}
.cond-true { border: 1px solid #23863644; }
.cond-false { border: 1px solid #da363344; }
.cond-divider {
  width: 1px;
  background: #30363d;
  margin: 0 8px;
}
.cond-branch-label {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  font-weight: 700;
  margin-bottom: 4px;
}
.cond-true .cond-branch-label { color: #3fb950; }
.cond-false .cond-branch-label { color: #f85149; }
.cond-target {
  font-size: 12px;
  color: #c9d1d9;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
}

/* ─── 拖入提示 ─── */
.drop-hint {
  position: fixed; bottom: 60px; left: 50%; transform: translateX(-50%);
  background: #1f6feb; color: #fff;
  padding: 8px 20px; border-radius: 8px;
  font-size: 13px; font-weight: 500; z-index: 10;
  box-shadow: 0 4px 12px #1f6feb44; pointer-events: none;
}

/* ─── 搜索不匹配淡化 ─── */
.step-card.search-no-match {
  opacity: 0.25;
  pointer-events: auto;
}

/* ─── 底部 ─── */
.canvas-footer {
  text-align: center; color: #484f58; font-size: 11px; padding: 16px;
}
</style>
