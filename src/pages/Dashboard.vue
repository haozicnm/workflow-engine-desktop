<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
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
import ActionIcon from '../components/ActionIcon.vue'
import { Package, History, Download } from 'lucide-vue-next'
import { inject, type Ref } from 'vue'

const { t } = useI18n()

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
  'open-plugins': []
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
      if (status === 'completed') toast.success(t('dashboard.workflowCompleted'))
      else if (status === 'failed') toast.error(t('dashboard.workflowFailed') + ': ' + (error || t('dashboard.unknownError')))
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
    toast.error(t('dashboard.listFailed') + ': ' + ((e as Error).message || e))
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
    toast.error(t('dashboard.importFailed') + ': ' + ((e as Error).message || e))
  }
}

async function importFromContent(content: string) {
  try {
    let name = t('dashboard.importedWorkflow')
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
            toast.success(t('dashboard.importedWithCount', { name: result.name, count: result.step_count }))
            await loadList()
            emit('workflow-created', id)
            return
          }
        }
      } catch { /* fall through */ }
      toast.error(t('dashboard.unrecognizedFormat'))
      return
    }
    const id = await safeInvoke<string>('workflow_create', { name, description: '' })
    if (id) {
      await safeInvoke('workflow_save_yaml', { id, yaml: content })
      toast.success(t('dashboard.imported', { name }))
      await loadList()
      emit('workflow-created', id)
    }
  } catch (e: unknown) {
    toast.error(t('dashboard.importFailed') + ': ' + ((e as Error).message || e))
  }
}

function onSettings() {
  emit('open-settings')
}

function onHistory() {
  emit('open-history')
}

function onPlugins() {
  emit('open-plugins')
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
      :placeholder="t('nav.searchWorkflow')"
      class="h-8 text-xs"
    />
    <Button v-if="sidebar?.open.value" size="sm" class="bg-primary text-primary-foreground w-full" @click="onNewWorkflow">
      ＋ {{ t('common.create') }}
    </Button>
  </SidebarHeader>

  <SidebarContent>
    <SidebarGroup>
      <SidebarGroupLabel :label="t('nav.workflows')" />

      <!-- Loading skeleton -->
      <div v-if="loading" class="space-y-2 px-2">
        <div class="h-8 bg-secondary/50 rounded animate-pulse" />
        <div class="h-8 bg-secondary/50 rounded animate-pulse w-3/4" />
        <div class="h-8 bg-secondary/50 rounded animate-pulse w-1/2" />
      </div>

      <!-- Empty state (no workflows at all) -->
      <div v-else-if="workflows.length === 0" class="px-2 py-4 text-center text-xs text-muted-foreground">
        {{ t('nav.noWorkflows') }}
      </div>

      <!-- Empty search results -->
      <div v-else-if="!filteredWorkflows.length" class="px-2 py-4 text-center text-xs text-muted-foreground">
        {{ t('nav.noSearchResults') }}
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

  <SidebarFooter class="shrink-0">
    <SidebarMenuItem>
      <SidebarMenuButton :tooltip="t('nav.plugins')" @click="onPlugins">
        <template #icon>
          <Package class="w-4 h-4" />
        </template>
        {{ t('nav.plugins') }}
      </SidebarMenuButton>
    </SidebarMenuItem>
    <SidebarMenuItem>
      <SidebarMenuButton :tooltip="t('nav.settings')" @click="onSettings">
        <template #icon>
          <ActionIcon name="Settings" cls="w-4 h-4" />
        </template>
        {{ t('nav.settings') }}
      </SidebarMenuButton>
    </SidebarMenuItem>
    <SidebarMenuItem>
      <SidebarMenuButton :tooltip="t('nav.history')" @click="onHistory">
        <template #icon>
          <History class="w-4 h-4" />
        </template>
        {{ t('nav.history') }}
      </SidebarMenuButton>
    </SidebarMenuItem>
    <SidebarMenuItem>
      <SidebarMenuButton :tooltip="t('common.import')" @click="onImportFile">
        <template #icon>
          <Download class="w-4 h-4" />
        </template>
        {{ t('common.import') }}
      </SidebarMenuButton>
    </SidebarMenuItem>
  </SidebarFooter>
</template>
