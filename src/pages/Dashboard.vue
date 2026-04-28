<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { useWorkflowStore } from '../stores/workflow'
import { useToast } from '../composables/useToast'
import WorkflowGrid from '../components/WorkflowGrid.vue'
import TemplateSection from '../components/TemplateSection.vue'
import ScheduleSection from '../components/ScheduleSection.vue'
import ProgressSection from '../components/ProgressSection.vue'
import ScheduleDialog from '../components/ScheduleDialog.vue'

interface WorkflowItem {
  id: string
  name: string
  description: string
  enabled: boolean
  created_at: string
  updated_at: string
}

interface ScheduleInfo {
  id: string
  workflow_name: string
  cron_expr: string
  enabled: boolean
  last_run_at: string | null
}

import pkg from '../package.json'
const APP_VERSION = pkg.version

// ─── 开发进度 ───
const milestones = [
  { id: 'P0', label: '项目骨架', desc: 'Tauri 窗口 + 数据层 + 空白前端 + 4 个页面', done: true },
  { id: 'P1', label: '引擎核心', desc: '解析器 / 调度器 / 状态机 / HTTP·数据·脚本·条件·循环节点 / 重试·超时 / DB 持久化 / 事件推送', done: true },
  { id: 'P2', label: '文件节点 + 桌面录制', desc: 'Excel / Word / 操作录制→工作流转换', done: true },
  { id: 'P3', label: '前端画布 v1', desc: '拖拽编辑器 + YAML 双向编辑 + 流程连线 + 运行历史', done: true },
  { id: 'P4', label: '桌面集成', desc: '系统托盘 + 定时调度 + 内置 Python + Playwright', done: true },
  { id: 'v2.0', label: 'ComfyUI DAG 编辑器', desc: 'Vue Flow 画布 + 节点拖拽 + 属性面板 + 撤销/重做 + 自动保存', done: true },
  { id: '1.0β', label: '正式 Beta', desc: '全面代码审查 + Bug 修复 + 可正式使用', done: true },
  { id: '1.1β', label: '网页抓取增强', desc: 'web_scrape 声明式节点 / 浏览器 +16 动作 / Cookie·代理·多标签页 / 步骤耗时显示', done: true },
]

const router = useRouter()
const store = useWorkflowStore()
const toast = useToast()
const workflows = ref<WorkflowItem[]>([])
const loading = ref(false)
const deleting = ref<string | null>(null)
const cloning = ref<string | null>(null)
const fileInput = ref<HTMLInputElement | null>(null)
const creatingFromTemplate = ref<string | null>(null)
const scheduleDialog = ref<InstanceType<typeof ScheduleDialog> | null>(null)
const schedules = ref<ScheduleInfo[]>([])
const scheduleLoading = ref(false)

// ─── 搜索 ───
const searchQuery = ref('')
const filteredWorkflows = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return workflows.value
  return workflows.value.filter(w =>
    w.name.toLowerCase().includes(q) || w.description.toLowerCase().includes(q)
  )
})

interface TemplateItem {
  id: string
  name: string
  description: string
}
const templates = ref<TemplateItem[]>([])

let unlistenRunUpdate: (() => void) | null = null

onMounted(async () => {
  await Promise.all([loadList(), loadTemplates(), loadSchedules()])
  // 监听工作流执行结果
  try {
    const { listen } = await import('@tauri-apps/api/event')
    unlistenRunUpdate = await listen('run-update', (event: { payload: { status: string; error?: string } }) => {
      const { status, error } = event.payload
      if (status === 'completed') {
        toast.success('工作流执行完成 ✅')
      } else if (status === 'failed') {
        toast.error('工作流执行失败: ' + (error || '未知错误'))
      }
    })
  } catch (e) {
    console.warn('无法监听执行事件:', e)
  }
})

onUnmounted(() => {
  unlistenRunUpdate?.()
})

async function loadList() {
  loading.value = true
  try {
    workflows.value = await invoke<WorkflowItem[]>('workflow_list')
  } catch (e: unknown) {
    toast.error('获取工作流列表失败: ' + ((e as Error).message || e))
  } finally {
    loading.value = false
  }
}

async function loadTemplates() {
  try {
    templates.value = await invoke<TemplateItem[]>('template_list')
  } catch (e) {
    console.warn('加载内置模板失败:', e)
  }
}

// ─── 业务操作 ───

function openEditor(id?: string) {
  router.push(id ? `/editor/${id}` : '/editor/new')
}

async function createFromTemplate(tpl: TemplateItem) {
  creatingFromTemplate.value = tpl.id
  try {
    const yaml = await invoke<string | null>('template_get_yaml', { id: tpl.id })
    if (!yaml) { toast.error('模板内容为空'); return }
    const id = await invoke<string>('workflow_create', { name: tpl.name, description: tpl.description })
    await invoke('workflow_save_yaml', { id, yaml })
    toast.success(`已从模板创建「${tpl.name}」`)
    await loadList()
    openEditor(id)
  } catch (e: unknown) {
    toast.error('创建失败: ' + ((e as Error).message || e))
  } finally {
    creatingFromTemplate.value = null
  }
}

async function toggleEnabled(item: WorkflowItem) {
  try {
    await invoke('workflow_update', { id: item.id, enabled: !item.enabled })
    item.enabled = !item.enabled
    toast.success(`已${item.enabled ? '启用' : '禁用'}「${item.name}」`)
  } catch (e: unknown) {
    toast.error('更新状态失败: ' + ((e as Error).message || e))
  }
}

async function deleteWorkflow(item: WorkflowItem) {
  if (!confirm(`确定删除「${item.name}」？此操作不可撤销。`)) return
  deleting.value = item.id
  try {
    await invoke('workflow_delete', { id: item.id })
    workflows.value = workflows.value.filter(w => w.id !== item.id)
    toast.success(`已删除「${item.name}」`)
  } catch (e: unknown) {
    toast.error('删除失败: ' + ((e as Error).message || e))
  } finally { deleting.value = null }
}

async function runWorkflow(item: WorkflowItem) {
  try {
    await invoke<string>('run_start', { workflowId: item.id })
    toast.info(`「${item.name}」已启动`)
  } catch (e: unknown) {
    toast.error('执行失败: ' + ((e as Error).message || e))
  }
}

async function cloneWorkflow(item: WorkflowItem) {
  cloning.value = item.id
  try {
    const newId = await store.cloneWorkflow(item.id)
    if (newId) {
      toast.success(`已克隆「${item.name}」`)
      await loadList()
    } else { toast.error('克隆失败') }
  } catch (e: unknown) {
    toast.error('克隆失败: ' + ((e as Error).message || e))
  } finally { cloning.value = null }
}

async function exportWorkflow(item: WorkflowItem) {
  try {
    const wf = await invoke<{ yaml: string | null }>('workflow_get', { id: item.id })
    if (wf?.yaml) {
      const blob = new Blob([wf.yaml], { type: 'text/yaml' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url; a.download = item.name + '.yaml'; a.click()
      URL.revokeObjectURL(url)
      toast.success(`已导出「${item.name}」`)
    }
  } catch (e: unknown) {
    toast.error('导出失败: ' + ((e as Error).message || e))
  }
}

function triggerImport() { fileInput.value?.click() }

async function handleImport(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  input.value = ''
  try {
    const result = await store.importYaml(file)
    if (!result) { toast.error('导入失败: 无效的工作流 YAML'); return }
    const id = await invoke<string>('workflow_create', { name: result.name, description: '从文件导入' })
    await invoke('workflow_save_yaml', { id, yaml: result.yaml })
    toast.success(`已导入「${result.name}」`)
    await loadList()
  } catch (e: unknown) {
    toast.error('导入失败: ' + ((e as Error).message || e))
  }
}

// ─── 定时计划 ───

async function loadSchedules() {
  scheduleLoading.value = true
  try {
    schedules.value = await invoke<ScheduleInfo[]>('schedule_list')
  } catch (e) {
    console.warn('加载计划失败:', e)
  } finally { scheduleLoading.value = false }
}

async function toggleSchedule(s: ScheduleInfo) {
  try {
    await invoke('schedule_update', { id: s.id, enabled: !s.enabled })
    s.enabled = !s.enabled
    toast.success(`计划已${s.enabled ? '启用' : '禁用'}`)
  } catch (e: unknown) {
    toast.error((e as Error).message || String(e))
  }
}

async function deleteSchedule(s: ScheduleInfo) {
  if (!confirm(`确定删除此定时计划？`)) return
  try {
    await invoke('schedule_delete', { id: s.id })
    schedules.value = schedules.value.filter(x => x.id !== s.id)
    toast.success('计划已删除')
  } catch (e: unknown) {
    toast.error((e as Error).message || String(e))
  }
}

function openScheduleDialog(schedule?: ScheduleInfo) {
  const wfs = workflows.value.map(w => ({ id: w.id, name: w.name }))
  scheduleDialog.value?.open(wfs, schedule)
}

async function onScheduleSaved() { await loadSchedules() }
</script>

<template>
  <div class="dashboard">
    <!-- 顶部 -->
    <div class="dash-header">
      <div class="dash-title">
        <h2>📋 工作流</h2>
        <span class="wf-count" v-if="!loading">{{ workflows.length }} 个</span>
      </div>
      <div class="dash-actions">
        <button class="btn btn-sm" @click="triggerImport">📥 导入 YAML</button>
        <button class="btn btn-sm" @click="router.push('/history')">📊 运行历史</button>
        <button class="btn btn-sm" @click="router.push('/settings')">⚙️ 设置</button>
        <button class="btn btn-primary" @click="openEditor()">＋ 新建工作流</button>
        <input ref="fileInput" type="file" accept=".yaml,.yml" style="display:none" @change="handleImport" />
      </div>
    </div>

    <!-- 搜索 -->
    <div class="dash-search" v-if="!loading && workflows.length > 0">
      <input type="text" v-model="searchQuery" placeholder="🔍 搜索工作流名称或描述..." class="search-input" />
      <span class="search-count" v-if="searchQuery.trim()">{{ filteredWorkflows.length }}/{{ workflows.length }}</span>
    </div>

    <!-- 工作流网格 -->
    <WorkflowGrid
      :workflows="filteredWorkflows"
      :loading="loading"
      :deleting="deleting"
      :cloning="cloning"
      :search-query="searchQuery"
      @open-editor="openEditor"
      @toggle-enabled="toggleEnabled"
      @delete-workflow="deleteWorkflow"
      @run-workflow="runWorkflow"
      @clone-workflow="cloneWorkflow"
      @export-workflow="exportWorkflow"
    />

    <!-- 内置模板 -->
    <TemplateSection
      :templates="templates"
      :creating-from-template="creatingFromTemplate"
      @create-from-template="createFromTemplate"
    />

    <!-- 定时计划 -->
    <ScheduleSection
      :schedules="schedules"
      :loading="scheduleLoading"
      @toggle-schedule="toggleSchedule"
      @delete-schedule="deleteSchedule"
      @edit-schedule="(s: ScheduleInfo) => openScheduleDialog(s)"
      @new-schedule="openScheduleDialog()"
    />

    <ScheduleDialog ref="scheduleDialog" @saved="onScheduleSaved" />

    <!-- 开发进度 -->
    <ProgressSection :milestones="milestones" :version="APP_VERSION" />
  </div>
</template>

<style scoped>
.dashboard { padding: 24px; height: 100%; overflow-y: auto; }
.dash-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px; }
.dash-title { display: flex; align-items: center; gap: 10px; }
.dash-title h2 { margin: 0; font-size: 20px; color: #e1e4e8; }
.wf-count { font-size: 12px; color: #6e7681; background: #21262d; padding: 2px 8px; border-radius: 10px; }
.dash-actions { display: flex; gap: 8px; }

.btn { padding: 6px 14px; border-radius: 6px; font-size: 13px; font-weight: 500; cursor: pointer; border: 1px solid #30363d; background: #21262d; color: #c9d1d9; transition: all 0.15s; }
.btn:hover { background: #30363d; }
.btn-sm { padding: 5px 12px; font-size: 12px; }
.btn-primary { background: #1f6feb; border-color: #388bfd; color: #fff; }
.btn-primary:hover { background: #388bfd; }

.dash-search { display: flex; align-items: center; gap: 10px; margin-bottom: 16px; }
.search-input { flex: 1; padding: 8px 12px; background: #0d1117; border: 1px solid #30363d; border-radius: 6px; color: #e1e4e8; font-size: 13px; outline: none; transition: border-color 0.2s; }
.search-input:focus { border-color: #58a6ff; }
.search-input::placeholder { color: #6e7681; }
.search-count { font-size: 12px; color: #8b949e; white-space: nowrap; }
</style>
