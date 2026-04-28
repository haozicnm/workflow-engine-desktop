<script setup lang="ts">
import { useRouter } from 'vue-router'

interface WorkflowItem {
  id: string
  name: string
  description: string
  enabled: boolean
  created_at: string
  updated_at: string
}

const props = defineProps<{
  workflows: WorkflowItem[]
  loading: boolean
  deleting: string | null
  cloning: string | null
  searchQuery: string
}>()

const emit = defineEmits<{
  'open-editor': [id?: string]
  'toggle-enabled': [item: WorkflowItem]
  'delete-workflow': [item: WorkflowItem]
  'run-workflow': [item: WorkflowItem]
  'clone-workflow': [item: WorkflowItem]
  'export-workflow': [item: WorkflowItem]
}>()

const router = useRouter()

function formatDate(iso: string): string {
  try {
    const d = new Date(iso)
    const pad = (n: number) => String(n).padStart(2, '0')
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
  } catch {
    return iso
  }
}

function openEditor(id?: string) {
  emit('open-editor', id)
}
</script>

<template>
  <!-- 加载中 -->
  <div v-if="loading" class="loading-state">
    <div class="spinner"></div>
    <span>加载中...</span>
  </div>

  <!-- 空状态 -->
  <div v-else-if="workflows.length === 0" class="empty-state">
    <div class="empty-icon">📭</div>
    <div class="empty-text">还没有工作流</div>
    <div class="empty-hint">点击「新建工作流」或「导入 YAML」开始</div>
    <button class="btn btn-primary" @click="openEditor()">＋ 新建工作流</button>
  </div>

  <!-- 无搜索结果 -->
  <div v-else-if="workflows.length === 0 && searchQuery.trim()" class="empty-state">
    <div class="empty-icon">🔍</div>
    <div class="empty-text">没有匹配的工作流</div>
    <div class="empty-hint">尝试其他关键词</div>
  </div>

  <!-- 列表 -->
  <div v-else class="wf-grid">
    <div
      v-for="wf in workflows"
      :key="wf.id"
      class="wf-card"
      :class="{ disabled: !wf.enabled }"
    >
      <div class="wf-card-header">
        <div class="wf-name" @click="openEditor(wf.id)">{{ wf.name }}</div>
        <span class="status-dot" :class="wf.enabled ? 'on' : 'off'" :title="wf.enabled ? '已启用' : '已禁用'"></span>
      </div>
      <div v-if="wf.description" class="wf-desc">{{ wf.description }}</div>
      <div class="wf-meta">
        <span class="meta-item">🕐 {{ formatDate(wf.updated_at) }}</span>
      </div>
      <div class="wf-actions">
        <button class="action-btn" @click="openEditor(wf.id)">✏️ 编辑</button>
        <button class="action-btn run" @click="emit('run-workflow', wf)" :disabled="!wf.enabled">▶ 执行</button>
        <button class="action-btn" @click="router.push(`/history?workflow_id=${wf.id}`)">📊 历史</button>
        <button class="action-btn" @click="emit('toggle-enabled', wf)">{{ wf.enabled ? '⏸ 禁用' : '▶ 启用' }}</button>
        <button class="action-btn" @click="emit('clone-workflow', wf)" :disabled="cloning === wf.id">
          📋 {{ cloning === wf.id ? '克隆中...' : '克隆' }}
        </button>
        <button class="action-btn" @click="emit('export-workflow', wf)">💾 导出</button>
        <button class="action-btn danger" @click="emit('delete-workflow', wf)" :disabled="deleting === wf.id">
          🗑 {{ deleting === wf.id ? '删除中...' : '删除' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.loading-state { display: flex; align-items: center; justify-content: center; gap: 10px; height: 200px; color: #8b949e; }
.spinner { width: 20px; height: 20px; border: 2px solid #30363d; border-top-color: #58a6ff; border-radius: 50%; animation: spin 0.8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

.empty-state { display: flex; flex-direction: column; align-items: center; justify-content: center; height: 300px; gap: 12px; color: #6e7681; }
.empty-icon { font-size: 48px; }
.empty-text { font-size: 16px; color: #8b949e; }
.empty-hint { font-size: 13px; color: #6e7681; }

.wf-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(340px, 1fr)); gap: 16px; margin-bottom: 24px; }

.wf-card { background: #161b22; border: 1px solid #30363d; border-radius: 10px; padding: 16px; transition: all 0.15s; }
.wf-card:hover { border-color: #484f58; background: #1c2128; }
.wf-card.disabled { opacity: 0.6; }
.wf-card-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
.wf-name { font-size: 15px; font-weight: 600; color: #e1e4e8; cursor: pointer; transition: color 0.15s; }
.wf-name:hover { color: #58a6ff; }
.status-dot { display: inline-block; width: 8px; height: 8px; border-radius: 50%; }
.status-dot.on { background: #3fb950; box-shadow: 0 0 4px #3fb95080; }
.status-dot.off { background: #484f58; }
.wf-desc { font-size: 12px; color: #8b949e; margin-bottom: 10px; line-height: 1.5; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
.wf-meta { display: flex; gap: 12px; margin-bottom: 12px; }
.meta-item { font-size: 11px; color: #6e7681; }
.wf-actions { display: flex; gap: 6px; flex-wrap: wrap; }
.action-btn { background: #21262d; border: 1px solid #30363d; color: #c9d1d9; padding: 4px 10px; border-radius: 6px; font-size: 12px; cursor: pointer; transition: all 0.15s; }
.action-btn:hover:not(:disabled) { background: #30363d; border-color: #484f58; }
.action-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.action-btn.run { background: #238636; border-color: #2ea043; color: #fff; }
.action-btn.run:hover:not(:disabled) { background: #2ea043; }
.action-btn.danger:hover:not(:disabled) { background: #da363322; border-color: #da363344; color: #f85149; }

.btn { padding: 6px 14px; border-radius: 6px; font-size: 13px; font-weight: 500; cursor: pointer; border: 1px solid #30363d; background: #21262d; color: #c9d1d9; transition: all 0.15s; }
.btn:hover { background: #30363d; }
.btn-primary { background: #1f6feb; border-color: #388bfd; color: #fff; }
.btn-primary:hover { background: #388bfd; }
</style>
