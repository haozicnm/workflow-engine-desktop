<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '../composables/useToast'

const toast = useToast()
const router = useRouter()
const route = useRoute()

interface RunHistoryItem {
  id: string
  workflow_id: string
  workflow_name: string
  status: string
  started_at: string
  finished_at: string | null
  error: string | null
}

interface StepRunInfo {
  id: string
  run_id: string
  step_id: string
  status: string
  started_at: string
  finished_at: string | null
  output: any
  error: string | null
}

interface RunDetail {
  run: RunHistoryItem
  workflow_name: string
  steps: StepRunInfo[]
}

const runs = ref<RunHistoryItem[]>([])
const loading = ref(false)
const expandedId = ref<string | null>(null)
const detailCache = ref<Record<string, RunDetail>>({})
const loadingDetail = ref<string | null>(null)

// 筛选
const filterWorkflowId = ref<string | null>(null)
const workflowList = ref<{ id: string; name: string }[]>([])

onMounted(async () => {
  // 从 query 参数读取筛选
  filterWorkflowId.value = (route.query.workflow_id as string) || null
  await Promise.all([loadRuns(), loadWorkflowList()])
})

async function loadWorkflowList() {
  try {
    const list: any[] = await invoke('workflow_list')
    workflowList.value = list.map(w => ({ id: w.id, name: w.name }))
  } catch (e) {
    console.warn('加载工作流列表失败:', e)
  }
}

async function loadRuns() {
  loading.value = true
  try {
    runs.value = await invoke<RunHistoryItem[]>('run_list', {
      workflowId: filterWorkflowId.value,
      limit: 100,
    })
  } catch (e: any) {
    toast.error('加载运行历史失败: ' + (e.message || e))
  } finally {
    loading.value = false
  }
}

async function toggleExpand(runId: string) {
  if (expandedId.value === runId) {
    expandedId.value = null
    return
  }
  expandedId.value = runId
  if (!detailCache.value[runId]) {
    loadingDetail.value = runId
    try {
      const detail = await invoke<RunDetail>('run_detail', { runId })
      detailCache.value[runId] = detail
    } catch (e: any) {
      toast.error('加载详情失败: ' + (e.message || e))
    } finally {
      loadingDetail.value = null
    }
  }
}

function onFilterChange() {
  // 更新 URL query
  router.replace({
    query: filterWorkflowId.value ? { workflow_id: filterWorkflowId.value } : {},
  })
  loadRuns()
}

function clearFilter() {
  filterWorkflowId.value = null
  onFilterChange()
}

// ─── 工具函数 ───

function formatTime(iso: string): string {
  try {
    const d = new Date(iso)
    const pad = (n: number) => String(n).padStart(2, '0')
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  } catch {
    return iso
  }
}

function calcDuration(started: string, finished: string | null): string {
  if (!finished) return '—'
  try {
    const ms = new Date(finished).getTime() - new Date(started).getTime()
    if (ms < 1000) return `${ms}ms`
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`
    const m = Math.floor(ms / 60000)
    const s = Math.round((ms % 60000) / 1000)
    return `${m}m ${s}s`
  } catch {
    return '—'
  }
}

function statusBadge(status: string): { icon: string; color: string; bg: string } {
  switch (status) {
    case 'completed': return { icon: '✅', color: '#3fb950', bg: '#23863622' }
    case 'failed': return { icon: '❌', color: '#f85149', bg: '#da363322' }
    case 'running': return { icon: '⏳', color: '#58a6ff', bg: '#1f6feb22' }
    case 'pending': return { icon: '⏸', color: '#8b949e', bg: '#21262d' }
    default: return { icon: '❓', color: '#8b949e', bg: '#21262d' }
  }
}

function formatOutput(output: any): string {
  if (!output) return ''
  const s = typeof output === 'string' ? output : JSON.stringify(output, null, 2)
  return s.length > 200 ? s.substring(0, 200) + '...' : s
}

// 统计
const stats = computed(() => {
  const total = runs.value.length
  const completed = runs.value.filter(r => r.status === 'completed').length
  const failed = runs.value.filter(r => r.status === 'failed').length
  const running = runs.value.filter(r => r.status === 'running').length
  return { total, completed, failed, running }
})
</script>

<template>
  <div class="run-history">
    <!-- 顶部 -->
    <div class="rh-header">
      <div class="rh-title">
        <button class="btn btn-sm" @click="router.push('/')">← 返回</button>
        <h2>📊 运行历史</h2>
        <span class="rh-count" v-if="!loading">{{ runs.length }} 条</span>
      </div>
      <div class="rh-actions">
        <!-- 工作流筛选 -->
        <select class="filter-select" v-model="filterWorkflowId" @change="onFilterChange">
          <option :value="null">全部工作流</option>
          <option v-for="wf in workflowList" :key="wf.id" :value="wf.id">{{ wf.name }}</option>
        </select>
        <button v-if="filterWorkflowId" class="btn btn-xs" @click="clearFilter">✕ 清除筛选</button>
      </div>
    </div>

    <!-- 统计 -->
    <div v-if="runs.length > 0" class="stats-bar">
      <span class="stat-item">共 {{ stats.total }}</span>
      <span class="stat-item success">✅ {{ stats.completed }}</span>
      <span class="stat-item danger">❌ {{ stats.failed }}</span>
      <span v-if="stats.running > 0" class="stat-item info">⏳ {{ stats.running }}</span>
    </div>

    <!-- 加载中 -->
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>加载中...</span>
    </div>

    <!-- 空状态 -->
    <div v-else-if="runs.length === 0" class="empty-state">
      <div class="empty-icon">📭</div>
      <div class="empty-text">暂无运行记录</div>
      <div class="empty-hint">执行工作流后，运行记录会出现在这里</div>
    </div>

    <!-- 运行列表 -->
    <div v-else class="run-list">
      <div
        v-for="run in runs"
        :key="run.id"
        class="run-card"
        :class="{ expanded: expandedId === run.id }"
      >
        <!-- 运行摘要（可点击展开） -->
        <div class="run-summary" @click="toggleExpand(run.id)">
          <div class="run-status">
            <span
              class="status-badge"
              :style="{ color: statusBadge(run.status).color, background: statusBadge(run.status).bg }"
            >
              {{ statusBadge(run.status).icon }} {{ run.status }}
            </span>
          </div>
          <div class="run-info">
            <div class="run-wf-name">{{ run.workflow_name }}</div>
            <div class="run-meta">
              <span>🕐 {{ formatTime(run.started_at) }}</span>
              <span>⏱ {{ calcDuration(run.started_at, run.finished_at) }}</span>
            </div>
          </div>
          <div class="run-expand">
            <span class="expand-arrow" :class="{ open: expandedId === run.id }">▸</span>
          </div>
        </div>

        <!-- 错误信息 -->
        <div v-if="run.error" class="run-error-bar">
          ❌ {{ run.error }}
        </div>

        <!-- 展开详情 -->
        <div v-if="expandedId === run.id" class="run-detail">
          <div v-if="loadingDetail === run.id" class="detail-loading">
            <div class="spinner small"></div> 加载步骤详情...
          </div>
          <div v-else-if="detailCache[run.id]" class="step-list">
            <div class="detail-header">
              <span>步骤执行记录</span>
              <span class="step-count">{{ detailCache[run.id].steps.length }} 步</span>
            </div>
            <div
              v-for="(step, idx) in detailCache[run.id].steps"
              :key="step.id"
              class="step-row"
              :class="'status-' + step.status"
            >
              <div class="step-idx">{{ idx + 1 }}</div>
              <div class="step-icon">{{ statusBadge(step.status).icon }}</div>
              <div class="step-name">{{ step.step_id }}</div>
              <div class="step-duration">{{ calcDuration(step.started_at, step.finished_at) }}</div>
              <div v-if="step.error" class="step-error">{{ step.error }}</div>
              <div v-if="step.output" class="step-output">
                <pre>{{ formatOutput(step.output) }}</pre>
              </div>
            </div>
            <div v-if="detailCache[run.id].steps.length === 0" class="no-steps">
              暂无步骤记录
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.run-history { padding: 24px; height: 100%; overflow-y: auto; }

.rh-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; flex-wrap: wrap; gap: 12px; }
.rh-title { display: flex; align-items: center; gap: 12px; }
.rh-title h2 { margin: 0; font-size: 20px; color: #e1e4e8; }
.rh-count { font-size: 12px; color: #6e7681; background: #21262d; padding: 2px 8px; border-radius: 10px; }
.rh-actions { display: flex; align-items: center; gap: 8px; }

.filter-select {
  background: #0d1117; border: 1px solid #30363d; color: #c9d1d9;
  padding: 5px 10px; border-radius: 6px; font-size: 13px; cursor: pointer;
}
.filter-select:focus { outline: none; border-color: #58a6ff; }

.stats-bar { display: flex; gap: 16px; margin-bottom: 16px; }
.stat-item { font-size: 13px; color: #8b949e; }
.stat-item.success { color: #3fb950; }
.stat-item.danger { color: #f85149; }
.stat-item.info { color: #58a6ff; }

.loading-state { display: flex; align-items: center; justify-content: center; gap: 10px; height: 200px; color: #8b949e; }
.spinner { width: 20px; height: 20px; border: 2px solid #30363d; border-top-color: #58a6ff; border-radius: 50%; animation: spin 0.8s linear infinite; }
.spinner.small { width: 14px; height: 14px; border-width: 1.5px; }
@keyframes spin { to { transform: rotate(360deg); } }

.empty-state { display: flex; flex-direction: column; align-items: center; justify-content: center; height: 300px; gap: 12px; color: #6e7681; }
.empty-icon { font-size: 48px; }
.empty-text { font-size: 16px; color: #8b949e; }
.empty-hint { font-size: 13px; }

.run-list { display: flex; flex-direction: column; gap: 8px; }

.run-card {
  background: #161b22; border: 1px solid #30363d; border-radius: 10px;
  overflow: hidden; transition: border-color 0.15s;
}
.run-card:hover { border-color: #484f58; }
.run-card.expanded { border-color: #58a6ff44; }

.run-summary {
  display: flex; align-items: center; gap: 12px;
  padding: 14px 16px; cursor: pointer; transition: background 0.15s;
}
.run-summary:hover { background: #1c2128; }

.run-status { flex-shrink: 0; }
.status-badge {
  font-size: 12px; font-weight: 600; padding: 3px 10px;
  border-radius: 6px; white-space: nowrap;
}

.run-info { flex: 1; min-width: 0; }
.run-wf-name { font-size: 14px; font-weight: 600; color: #e1e4e8; }
.run-meta { display: flex; gap: 16px; margin-top: 4px; font-size: 12px; color: #6e7681; }

.run-expand { flex-shrink: 0; }
.expand-arrow { font-size: 14px; color: #484f58; transition: transform 0.2s; display: inline-block; }
.expand-arrow.open { transform: rotate(90deg); }

.run-error-bar {
  padding: 8px 16px; background: #da363315; color: #f85149;
  font-size: 12px; border-top: 1px solid #da363333;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
}

.run-detail { border-top: 1px solid #21262d; }
.detail-loading { display: flex; align-items: center; gap: 8px; padding: 16px; color: #8b949e; font-size: 13px; }

.step-list { padding: 12px 16px; }
.detail-header {
  display: flex; justify-content: space-between; align-items: center;
  font-size: 12px; color: #6e7681; margin-bottom: 10px;
  text-transform: uppercase; letter-spacing: 0.5px;
}
.step-count { font-size: 11px; background: #21262d; padding: 1px 6px; border-radius: 4px; }

.step-row {
  display: grid; grid-template-columns: 28px 22px 1fr auto;
  align-items: center; gap: 8px; padding: 8px 10px;
  border-radius: 6px; margin-bottom: 4px;
}
.step-row:hover { background: #1c2128; }
.step-row.status-running { border-left: 2px solid #58a6ff; }
.step-row.status-completed { border-left: 2px solid #238636; }
.step-row.status-failed { border-left: 2px solid #da3633; }

.step-idx { font-size: 11px; color: #484f58; text-align: center; }
.step-icon { font-size: 13px; }
.step-name { font-size: 13px; color: #c9d1d9; font-family: 'Cascadia Code', monospace; }
.step-duration { font-size: 11px; color: #6e7681; }

.step-error {
  grid-column: 2 / -1; font-size: 11px; color: #f85149;
  font-family: 'Cascadia Code', monospace;
}
.step-output {
  grid-column: 2 / -1; margin-top: 4px;
}
.step-output pre {
  font-size: 11px; color: #8b949e; background: #0d1117;
  padding: 6px 8px; border-radius: 4px; margin: 0;
  overflow-x: auto; font-family: 'Cascadia Code', monospace;
  max-height: 120px; overflow-y: auto;
}

.no-steps { text-align: center; color: #484f58; font-size: 13px; padding: 12px; }

.btn { padding: 6px 14px; border-radius: 6px; font-size: 13px; font-weight: 500; cursor: pointer; border: 1px solid #30363d; background: #21262d; color: #c9d1d9; transition: all 0.15s; }
.btn:hover { background: #30363d; }
.btn-sm { padding: 5px 12px; font-size: 12px; }
.btn-xs { padding: 3px 8px; font-size: 11px; }
</style>
