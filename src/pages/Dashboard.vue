<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { safeInvoke, safeListen } from '../utils/tauri'
import { useToast } from '../composables/useToast'
import { useGlobalStatus } from '../composables/useGlobalStatus'
import Button from '../components/ui/button/Button.vue'
import Input from '../components/ui/input/Input.vue'
import SidebarHeader from '../components/ui/sidebar/SidebarHeader.vue'
import SidebarContent from '../components/ui/sidebar/SidebarContent.vue'
import SidebarFooter from '../components/ui/sidebar/SidebarFooter.vue'
import SidebarGroup from '../components/ui/sidebar/SidebarGroup.vue'
import SidebarGroupLabel from '../components/ui/sidebar/SidebarGroupLabel.vue'
import SidebarMenuItem from '../components/ui/sidebar/SidebarMenuItem.vue'
import SidebarMenuButton from '../components/ui/sidebar/SidebarMenuButton.vue'
import SidebarTrigger from '../components/ui/sidebar/SidebarTrigger.vue'
import { inject, type Ref } from 'vue'

interface WorkflowItem {
  id: string
  name: string
  description: string
  enabled: boolean
  created_at: string
  updated_at: string
}

defineProps<{
  selectedId: string | null
}>()

const emit = defineEmits<{
  'open-workflow': [id?: string]
  'open-settings': []
  'open-history': []
  'workflow-created': [id: string]
}>()

const toast = useToast()
const globalStatus = useGlobalStatus()
const workflows = ref<WorkflowItem[]>([])
const loading = ref(false)

const sidebar = inject<{ open: Ref<boolean>; toggle: () => void }>('sidebar')

const searchQuery = ref('')

const filteredWorkflows = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return workflows.value
  return workflows.value.filter(w =>
    w.name.toLowerCase().includes(q) || w.description.toLowerCase().includes(q)
  )
})

let unlistenRunUpdate: (() => void) | null = null
let unlistenWorkflowChanged: (() => void) | null = null
let unlistenScheduleChanged: (() => void) | null = null

onMounted(async () => {
  await loadList()
  try {
    unlistenRunUpdate = await safeListen('run-update', (event: { payload: { status: string; error?: string } }) => {
      const { status, error } = event.payload
      if (status === 'completed') toast.success('工作流执行完成')
      else if (status === 'failed') toast.error('工作流执行失败: ' + (error || '未知错误'))
    })
    unlistenWorkflowChanged = await safeListen('workflow-changed', () => {
      loadList()
    })
    unlistenScheduleChanged = await safeListen('schedule-changed', () => {
      globalStatus.refreshSchedules()
    })
  } catch (e) { console.warn('无法监听事件:', e) }
})

onUnmounted(() => {
  unlistenRunUpdate?.()
  unlistenWorkflowChanged?.()
  unlistenScheduleChanged?.()
})

async function loadList() {
  loading.value = true
  try {
    workflows.value = await safeInvoke<WorkflowItem[]>('workflow_list') || []
  } catch (e: unknown) {
    toast.error('获取工作流列表失败: ' + ((e as Error).message || e))
  } finally { loading.value = false }
}

function selectWorkflow(item: WorkflowItem) {
  emit('open-workflow', item.id)
}

function onNewWorkflow() {
  emit('open-workflow', undefined)
}

async function onImportFile() {
  try {
    const input = document.createElement('input')
    input.type = 'file'
    input.accept = '.json,.yaml,.yml'
    input.onchange = async () => {
      const file = input.files?.[0]
      if (!file) return
      const content = await file.text()
      await importFromContent(content)
    }
    input.click()
  } catch (e: unknown) {
    toast.error('导入失败: ' + ((e as Error).message || e))
  }
}

async function importFromContent(content: string) {
  try {
    let name = '导入的工作流'
    try {
      const parsed = JSON.parse(content)
      if (parsed.name) name = parsed.name
    } catch {
      try {
        const result = await safeInvoke<{ name?: string; step_count?: number }>('workflow_validate', { yaml: content })
        if (result) {
          const id = await safeInvoke<string>('workflow_create', { name: result.name || name, description: '' })
          if (id) {
            await safeInvoke('workflow_save_yaml', { id, yaml: content })
            toast.success(`已导入「${result.name}」（${result.step_count} 步）`)
            await loadList()
            emit('workflow-created', id)
            return
          }
        }
      } catch { /* fall through */ }
      toast.error('无法识别文件格式，请使用 JSON 或 YAML')
      return
    }
    const id = await safeInvoke<string>('workflow_create', { name, description: '' })
    if (id) {
      await safeInvoke('workflow_save_yaml', { id, yaml: content })
      toast.success(`已导入「${name}」`)
      await loadList()
      emit('workflow-created', id)
    }
  } catch (e: unknown) {
    toast.error('导入失败: ' + ((e as Error).message || e))
  }
}

function onSettings() {
  emit('open-settings')
}

function onHistory() {
  emit('open-history')
}

defineExpose({ loadList })
</script>

<template>
  <SidebarHeader>
    <div class="flex items-center" :class="sidebar?.open.value ? 'justify-between' : 'justify-center'">
      <span v-if="sidebar?.open.value" class="text-xl font-bold tracking-tight text-primary">WorkFlow</span>
      <SidebarTrigger />
    </div>
    <!-- Search -->
    <Input
      v-if="sidebar?.open.value"
      v-model="searchQuery"
      placeholder="搜索工作流..."
      class="h-8 text-xs"
    />
    <Button v-if="sidebar?.open.value" size="sm" class="bg-primary text-primary-foreground w-full" @click="onNewWorkflow">
      ＋ 新建
    </Button>
  </SidebarHeader>

  <SidebarContent>
    <SidebarGroup>
      <SidebarGroupLabel label="工作流" />

      <!-- Loading skeleton -->
      <div v-if="loading" class="space-y-2 px-2">
        <div class="h-8 bg-secondary/50 rounded animate-pulse" />
        <div class="h-8 bg-secondary/50 rounded animate-pulse w-3/4" />
        <div class="h-8 bg-secondary/50 rounded animate-pulse w-1/2" />
      </div>

      <!-- Empty state -->
      <div v-else-if="!filteredWorkflows.length" class="px-2 py-4 text-center text-xs text-muted-foreground">
        暂无工作流
      </div>

      <template v-else>
        <SidebarMenuItem v-for="wf in filteredWorkflows" :key="wf.id">
        <SidebarMenuButton
          :active="selectedId === wf.id"
          :tooltip="wf.name"
          @click.stop="selectWorkflow(wf)"
        >
          <template #icon>
            <span class="flex items-center justify-center w-5 h-5 rounded bg-primary/10 text-primary text-[10px] font-bold shrink-0">
              {{ wf.name.charAt(0) }}
            </span>
          </template>
          <span class="truncate text-sm text-foreground">
            {{ wf.name }}
          </span>
        </SidebarMenuButton>
      </SidebarMenuItem>
      </template>
    </SidebarGroup>
  </SidebarContent>

  <SidebarFooter>
    <SidebarMenuItem>
      <SidebarMenuButton tooltip="设置" @click="onSettings">
        <template #icon>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
            <circle cx="12" cy="12" r="3" />
          </svg>
        </template>
        设置
      </SidebarMenuButton>
    </SidebarMenuItem>
    <SidebarMenuItem>
      <SidebarMenuButton tooltip="历史" @click="onHistory">
        <template #icon>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
            <path d="M3 3v5h5" />
            <path d="M12 7v5l4 2" />
          </svg>
        </template>
        历史
      </SidebarMenuButton>
    </SidebarMenuItem>
    <SidebarMenuItem>
      <SidebarMenuButton tooltip="导入" @click="onImportFile">
        <template #icon>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="7 10 12 15 17 10" />
            <line x1="12" y1="15" x2="12" y2="3" />
          </svg>
        </template>
        导入
      </SidebarMenuButton>
    </SidebarMenuItem>
  </SidebarFooter>
</template>
