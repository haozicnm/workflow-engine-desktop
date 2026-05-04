<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { safeInvoke, safeListen } from '../utils/tauri'
import { useToast } from '../composables/useToast'

interface WorkflowItem {
  id: string
  name: string
  description: string
  enabled: boolean
  created_at: string
  updated_at: string
}

const emit = defineEmits<{
  'open-workflow': [id?: string]
  'open-settings': []
  'open-history': []
}>()

const toast = useToast()
const workflows = ref<WorkflowItem[]>([])
const loading = ref(false)
const deletingId = ref<string | null>(null)
const exportingId = ref<string | null>(null)

// ─── 搜索 ───
const searchQuery = ref('')

const filteredWorkflows = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return workflows.value
  return workflows.value.filter(w =>
    w.name.toLowerCase().includes(q) || w.description.toLowerCase().includes(q)
  )
})

let unlistenRunUpdate: (() => void) | null = null

onMounted(async () => {
  await loadList()
  try {
    unlistenRunUpdate = await safeListen('run-update', (event: { payload: { status: string; error?: string } }) => {
      const { status, error } = event.payload
      if (status === 'completed') toast.success('工作流执行完成 ✅')
      else if (status === 'failed') toast.error('工作流执行失败: ' + (error || '未知错误'))
    })
  } catch (e) { console.warn('无法监听执行事件:', e) }
})

onUnmounted(() => { unlistenRunUpdate?.() })

async function loadList() {
  loading.value = true
  try {
    workflows.value = await safeInvoke<WorkflowItem[]>('workflow_list')
  } catch (e: unknown) {
    toast.error('获取工作流列表失败: ' + ((e as Error).message || e))
  } finally { loading.value = false }
}

// ─── 操作 ───

function onEdit(item: WorkflowItem) {
  emit('open-workflow', item.id)
}

function onNewWorkflow() {
  emit('open-workflow', undefined)
}

async function onRun(item: WorkflowItem) {
  try {
    await safeInvoke<string>('run_start', { workflowId: item.id })
    toast.info(`「${item.name}」已启动`)
  } catch (e: unknown) {
    toast.error('执行失败: ' + ((e as Error).message || e))
  }
}

async function onDelete(item: WorkflowItem) {
  if (!confirm(`确定删除「${item.name}」？此操作不可撤销。`)) return
  deletingId.value = item.id
  try {
    await safeInvoke('workflow_delete', { id: item.id })
    workflows.value = workflows.value.filter(w => w.id !== item.id)
    toast.success(`已删除「${item.name}」`)
  } catch (e: unknown) {
    toast.error('删除失败: ' + ((e as Error).message || e))
  } finally { deletingId.value = null }
}

async function onExport(item: WorkflowItem) {
  exportingId.value = item.id
  try {
    const wf = await safeInvoke<{ yaml: string | null }>('workflow_get', { id: item.id })
    if (wf?.yaml) {
      const blob = new Blob([wf.yaml], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url; a.download = item.name + '.json'; a.click()
      URL.revokeObjectURL(url)
      toast.success(`已导出「${item.name}」`)
    }
  } catch (e: unknown) {
    toast.error('导出失败: ' + ((e as Error).message || e))
  } finally { exportingId.value = null }
}

function formatDate(d: string) {
  if (!d) return ''
  return new Date(d).toLocaleDateString('zh-CN')
}

function onSettings() {
  emit('open-settings')
}

function onHistory() {
  emit('open-history')
}
</script>

<template>
  <div class="dashboard-v4">
    <!-- 顶栏 -->
    <div class="dash-topbar">
      <div class="dash-brand">
        <span class="dash-logo">WorkFlow</span>
      </div>
      <div class="dash-search-wrap">
        <input
          type="text"
          v-model="searchQuery"
          placeholder="搜索工作流..."
          class="dash-search-input"
        />
      </div>
      <div class="dash-top-actions">
        <button class="dash-btn dash-btn-ghost" @click="onHistory">📋 历史</button>
        <button class="dash-btn dash-btn-ghost" @click="onSettings">⚙️ 设置</button>
        <button class="dash-btn dash-btn-primary" @click="onNewWorkflow">＋ 新建</button>
      </div>
    </div>

    <!-- 加载状态 -->
    <div v-if="loading" class="dash-loading">加载中...</div>

    <!-- 空状态 -->
    <div v-else-if="filteredWorkflows.length === 0" class="dash-empty">
      <p v-if="searchQuery">未找到匹配的工作流</p>
      <p v-else>还没有工作流，点击「＋ 新建」创建</p>
    </div>

    <!-- 卡片列表 -->
    <div v-else class="dash-cards">
      <div
        v-for="item in filteredWorkflows"
        :key="item.id"
        class="dash-card"
      >
        <div class="card-header">
          <span class="card-name">{{ item.name }}</span>
        </div>
        <div class="card-desc" v-if="item.description">{{ item.description }}</div>
        <div class="card-meta">
          <span>{{ formatDate(item.updated_at) }}</span>
          <span :class="item.enabled ? 'status-on' : 'status-off'">
            {{ item.enabled ? '启用' : '禁用' }}
          </span>
        </div>
        <div class="card-actions">
          <button class="card-btn" @click="onRun(item)" title="运行">▶ 运行</button>
          <button class="card-btn" @click="onEdit(item)" title="编辑">✏️ 编辑</button>
          <button class="card-btn" @click="onExport(item)" :disabled="exportingId === item.id" title="导出">
            💾 {{ exportingId === item.id ? '导出中...' : '导出' }}
          </button>
          <button class="card-btn card-btn-danger" @click="onDelete(item)" :disabled="deletingId === item.id" title="删除">
            🗑 {{ deletingId === item.id ? '删除中...' : '删除' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dashboard-v4 {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: #0d1117;
}

/* ─── 顶栏 ─── */
.dash-topbar {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 12px 24px;
  background: #161b22;
  border-bottom: 1px solid #30363d;
  flex-shrink: 0;
}
.dash-brand {
  display: flex;
  align-items: center;
  gap: 4px;
}
.dash-logo {
  font-size: 18px;
  font-weight: 700;
  color: #58a6ff;
  letter-spacing: -0.5px;
}
.dash-search-wrap {
  flex: 1;
  max-width: 400px;
}
.dash-search-input {
  width: 100%;
  padding: 6px 12px;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 6px;
  color: #e6edf3;
  font-size: 13px;
  outline: none;
}
.dash-search-input:focus {
  border-color: #58a6ff;
}
.dash-search-input::placeholder {
  color: #484f58;
}
.dash-top-actions {
  display: flex;
  gap: 8px;
  margin-left: auto;
}

/* ─── 按钮 ─── */
.dash-btn {
  padding: 6px 14px;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all 0.15s;
  white-space: nowrap;
}
.dash-btn-primary {
  background: #1f6feb;
  color: #fff;
  border-color: #388bfd;
}
.dash-btn-primary:hover {
  background: #388bfd;
}
.dash-btn-ghost {
  background: transparent;
  color: #8b949e;
  border-color: #30363d;
}
.dash-btn-ghost:hover {
  background: #21262d;
  color: #e6edf3;
}

/* ─── 状态 ─── */
.dash-loading, .dash-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #8b949e;
  font-size: 14px;
}

/* ─── 卡片列表 ─── */
.dash-cards {
  flex: 1;
  overflow-y: auto;
  padding: 20px 24px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.dash-card {
  background: #161b22;
  border: 1px solid #21262d;
  border-radius: 8px;
  padding: 14px 18px;
  transition: border-color 0.15s;
}
.dash-card:hover {
  border-color: #30363d;
}
.card-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}
.card-name {
  font-size: 15px;
  font-weight: 600;
  color: #e6edf3;
}
.card-desc {
  font-size: 12px;
  color: #8b949e;
  margin-bottom: 4px;
  line-height: 1.4;
}
.card-meta {
  display: flex;
  gap: 12px;
  font-size: 11px;
  color: #484f58;
  margin-bottom: 8px;
}
.status-on { color: #3fb950; }
.status-off { color: #f85149; }

/* ─── 卡片操作按钮 ─── */
.card-actions {
  display: flex;
  gap: 6px;
}
.card-btn {
  padding: 4px 10px;
  border-radius: 4px;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  border: 1px solid #30363d;
  background: #21262d;
  color: #c9d1d9;
  transition: all 0.12s;
}
.card-btn:hover:not(:disabled) {
  background: #30363d;
}
</style>
