<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { safeInvoke } from '../utils/tauri'
import { useToast } from '../composables/useToast'
import Button from '../components/ui/button/Button.vue'
import Badge from '../components/ui/badge/Badge.vue'
import Card from '../components/ui/card/Card.vue'
import ActionIcon from '../components/ActionIcon.vue'
import Select from '../components/ui/select/Select.vue'
import Tabs from '../components/ui/tabs/Tabs.vue'
import TabsList from '../components/ui/tabs/TabsList.vue'
import TabsTrigger from '../components/ui/tabs/TabsTrigger.vue'
import TabsContent from '../components/ui/tabs/TabsContent.vue'
import { cn } from '@/lib/utils'

const { t } = useI18n()

const toast = useToast()
const emit = defineEmits<{ 'back': [] }>()

const filterWorkflowId = ref<string | null>(null)

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

interface StepLogEntry {
  id: number
  step_run_id: string
  run_id: string
  step_id: string
  level: string
  message: string
  timestamp: string
}

const runs = ref<RunHistoryItem[]>([])
const loading = ref(false)
const expandedId = ref<string | null>(null)
const detailCache = ref<Record<string, RunDetail>>({})
const logCache = ref<Record<string, StepLogEntry[]>>({})
const loadingDetail = ref<string | null>(null)
const detailTab = ref<'steps' | 'logs'>('steps')
const workflowList = ref<{ id: string; name: string }[]>([])

onMounted(async () => {
  await Promise.all([loadRuns(), loadWorkflowList()])
})

async function loadWorkflowList() {
  try {
    const list: any[] = await safeInvoke('workflow_list') || []
    workflowList.value = list.map(w => ({ id: w.id, name: w.name }))
  } catch (e) {
    console.warn('加载工作流列表失败:', e)
  }
}

async function loadRuns() {
  loading.value = true
  try {
    const result = await safeInvoke<RunHistoryItem[]>('run_list', {
      workflowId: filterWorkflowId.value,
      limit: 100,
    })
    runs.value = Array.isArray(result) ? result : []
  } catch (e: any) {
    toast.error(t('history.loadFailed') + ': ' + (e.message || e))
  } finally {
    loading.value = false
  }
}

async function toggleExpand(runId: string) {
  if (expandedId.value === runId) {
    expandedId.value = null
    detailTab.value = 'steps'
    return
  }
  expandedId.value = runId
  detailTab.value = 'steps'
  if (!detailCache.value[runId]) {
    loadingDetail.value = runId
    try {
      const detail = await safeInvoke<RunDetail>('run_detail', { runId })
      if (detail) detailCache.value[runId] = detail
    } catch (e: any) {
      toast.error(t('history.loadDetailFailed') + ': ' + (e.message || e))
    } finally {
      loadingDetail.value = null
    }
  }
}

async function loadLogs(runId: string) {
  if (logCache.value[runId]) return
  try {
    logCache.value[runId] = await safeInvoke<StepLogEntry[]>('run_step_logs', { runId }) || []
  } catch (e: any) {
    toast.error(t('history.loadLogsFailed') + ': ' + (e.message || e))
    logCache.value[runId] = []
  }
}

function onTabChange(tab: string) {
  detailTab.value = tab as 'steps' | 'logs'
  if (tab === 'logs' && expandedId.value) {
    loadLogs(expandedId.value)
  }
}

function onFilterChange() {
  loadRuns()
}

function clearFilter() {
  filterWorkflowId.value = null
  onFilterChange()
}
const filterOptions = computed(() => [
  { value: '__all__', label: t('history.allWorkflows') },
  ...workflowList.value.map(wf => ({ value: wf.id, label: wf.name })),
])

function formatTime(iso: string): string {
  try {
    return new Date(iso).toLocaleString()
  } catch {
    return iso
  }
}

function calcDuration(started: string, finished: string | null): string {
  if (!finished) {
    try {
      const ms = new Date().getTime() - new Date(started).getTime()
      if (ms < 1000) return t('history.runningDuration', { text: `${ms}ms` })
      if (ms < 60000) return t('history.runningDuration', { text: `${(ms / 1000).toFixed(1)}s` })
      const m = Math.floor(ms / 60000)
      const s = Math.round((ms % 60000) / 1000)
      return t('history.runningDuration', { text: `${m}m ${s}s` })
    } catch {
      return '—'
    }
  }
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

function statusBadge(status: string): { icon: string; variant: 'success' | 'destructive' | 'default' | 'secondary'; label: string } {
  const statusKey = status as keyof typeof statusLabelMap
  const label = statusLabelMap[statusKey] || status
  switch (status) {
    case 'completed': return { icon: 'CheckCircle', variant: 'success', label }
    case 'failed': return { icon: 'XCircle', variant: 'destructive', label }
    case 'running': return { icon: 'Loader', variant: 'default', label }
    default: return { icon: '⏸', variant: 'secondary', label }
  }
}

const statusLabelMap: Record<string, string> = {
  completed: '', failed: '', running: '', cancelled: '', pending: '',
}
// populate from i18n
function initStatusLabels() {
  statusLabelMap.completed = t('statusLabel.completed')
  statusLabelMap.failed = t('statusLabel.failed')
  statusLabelMap.running = t('statusLabel.running')
  statusLabelMap.cancelled = t('statusLabel.cancelled')
  statusLabelMap.pending = t('statusLabel.pending')
}
initStatusLabels()

function logLevelColor(level: string): string {
  switch (level) {
    case 'error': return 'text-danger'
    case 'warn': return 'text-warning'
    case 'success': return 'text-success'
    default: return 'text-muted-foreground'
  }
}

function formatOutput(output: any): string {
  if (!output) return ''
  const s = typeof output === 'string' ? output : JSON.stringify(output, null, 2)
  return s.length > 200 ? s.substring(0, 200) + '...' : s
}

const stats = computed(() => {
  const total = runs.value.length
  const completed = runs.value.filter(r => r.status === 'completed').length
  const failed = runs.value.filter(r => r.status === 'failed').length
  const running = runs.value.filter(r => r.status === 'running').length
  return { total, completed, failed, running }
})
</script>

<template>
  <div class="h-full overflow-y-auto p-6 space-y-4">
    <!-- Header -->
    <div class="flex items-center justify-between flex-wrap gap-3">
      <div class="flex items-center gap-3">
        <Button variant="outline" size="sm" class="text-xs" @click="emit('back')">{{ t('history.back') }}</Button>
        <h2 class="text-3xl font-bold tracking-tight">{{ t('history.title') }}</h2>
        <Badge v-if="!loading" variant="secondary" class="text-[10px]">{{ t('history.items', { n: runs.length }) }}</Badge>
      </div>
      <div class="flex items-center gap-2">
        <Select
          :model-value="filterWorkflowId ?? '__all__'"
          @update:model-value="v => { filterWorkflowId = (v === '__all__' ? null : v); onFilterChange() }"
          :options="filterOptions"
          :placeholder="t('history.allWorkflows')"
        />
        <Button v-if="filterWorkflowId" variant="outline" size="sm" class="text-[11px]" @click="clearFilter">{{ t('history.clearFilter') }}</Button>
      </div>
    </div>

    <!-- Stats -->
    <div v-if="runs.length > 0" class="flex gap-4" role="status" aria-label="运行统计">
      <span class="text-sm text-muted-foreground">{{ t('history.total', { n: stats.total }) }}</span>
      <span class="text-sm text-success" aria-label="成功 {{ stats.completed }}">✓ {{ stats.completed }}</span>
      <span class="text-sm text-danger" aria-label="失败 {{ stats.failed }}">✗ {{ stats.failed }}</span>
      <span v-if="stats.running > 0" class="text-sm text-primary" aria-label="运行中 {{ stats.running }}">◷ {{ stats.running }}</span>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="flex items-center justify-center gap-2.5 h-[200px] text-muted-foreground">
      <div class="w-5 h-5 border-2 border-border border-t-primary rounded-full animate-spin" />
      <span>{{ t('common.loading') }}</span>
    </div>

    <!-- Empty -->
    <div v-else-if="runs.length === 0" class="flex flex-col items-center justify-center h-[300px] gap-3 text-muted-foreground">
      <div class="text-5xl">📭</div>
      <div class="text-base text-foreground">{{ t('history.noHistory') }}</div>
      <div class="text-sm">{{ t('empty.runWorkflow') }}</div>
    </div>

    <!-- Run list -->
    <div v-else class="flex flex-col gap-2">
      <Card
        v-for="run in runs"
        :key="run.id"
        :class="cn(
          'overflow-hidden transition-colors',
          expandedId === run.id ? 'border-primary/25' : 'hover:border-foreground/20',
        )"
      >
        <button
          class="flex items-center gap-3 px-4 py-3.5 cursor-pointer transition-colors hover:bg-secondary w-full text-left"
          :aria-expanded="expandedId === run.id"
          :aria-label="expandedId === run.id ? t('common.close') : t('common.detail')"
          @click="toggleExpand(run.id)"
        >
          <div class="shrink-0">
            <Badge :variant="statusBadge(run.status).variant" class="text-xs whitespace-nowrap">
              <ActionIcon :name="statusBadge(run.status).icon" cls="w-4 h-4 inline" /> {{ statusBadge(run.status).label }}
            </Badge>
          </div>
          <div class="flex-1 min-w-0">
            <div class="text-sm font-semibold text-foreground">{{ run.workflow_name }}</div>
            <div class="flex gap-4 mt-1 text-xs text-muted-foreground">
              <span>🕐 {{ formatTime(run.started_at) }}</span>
              <span>⏱ {{ calcDuration(run.started_at, run.finished_at) }}</span>
            </div>
          </div>
          <div class="shrink-0">
            <span :class="cn('text-sm text-muted-foreground/50 transition-transform inline-block', expandedId === run.id ? 'rotate-90' : '')">▸</span>
          </div>
        </button>
        <div v-if="run.error" class="px-4 py-2 bg-destructive/5 text-destructive text-xs font-mono border-t border-destructive/20">
          ✗ {{ run.error }}
        </div>

        <!-- Expanded detail -->
        <div v-if="expandedId === run.id" class="border-t border-border">
          <div v-if="loadingDetail === run.id" class="flex items-center gap-2 px-4 py-4 text-muted-foreground text-sm">
            <div class="w-3.5 h-3.5 border-[1.5px] border-border border-t-primary rounded-full animate-spin" />
            {{ t('history.loadingDetail') }}
          </div>
          <div v-else-if="detailCache[run.id]" class="px-4 pb-4">
            <!-- Tabs -->
            <Tabs :model-value="detailTab" @update:model-value="onTabChange" class="mt-3">
              <TabsList>
                <TabsTrigger value="steps">{{ t('history.stepsTab', { n: detailCache[run.id].steps.length }) }}</TabsTrigger>
                <TabsTrigger value="logs">{{ t('history.logsTab', { n: logCache[run.id]?.length ?? 0 }) }}</TabsTrigger>
              </TabsList>

              <TabsContent value="steps">
                <div
                  v-for="(step, idx) in detailCache[run.id].steps"
                  :key="step.id"
                  :class="cn(
                    'grid items-center gap-2 px-2.5 py-2 rounded-md mb-1 hover:bg-secondary',
                    step.status === 'running' && 'border-l-2 border-primary',
                    step.status === 'completed' && 'border-l-2 border-success',
                    step.status === 'failed' && 'border-l-2 border-danger',
                  )"
                  style="grid-template-columns: 28px 22px 1fr auto;"
                >
                  <div class="text-[11px] text-muted-foreground/50 text-center">{{ idx + 1 }}</div>
                  <div class="text-sm"><ActionIcon :name="statusBadge(step.status).icon" cls="w-4 h-4" /></div>
                  <div class="text-sm text-foreground font-mono">{{ step.step_id }}</div>
                  <div class="text-[11px] text-muted-foreground">{{ calcDuration(step.started_at, step.finished_at) }}</div>
                  <div v-if="step.error" class="col-start-2 col-end-[-1] text-[11px] text-destructive font-mono">{{ step.error }}</div>
                  <div v-if="step.output" class="col-start-2 col-end-[-1] mt-1">
                    <pre class="text-[11px] text-muted-foreground bg-background p-2 rounded-md m-0 overflow-x-auto font-mono max-h-[120px] overflow-y-auto">{{ formatOutput(step.output) }}</pre>
                  </div>
                </div>
                <div v-if="detailCache[run.id].steps.length === 0" class="text-center text-muted-foreground/50 text-sm py-3">{{ t('history.noStepsRecord') }}</div>
              </TabsContent>

              <!-- Logs list -->
              <TabsContent value="logs" class="max-h-[400px] overflow-y-auto">
                <div v-if="!logCache[run.id]" class="flex items-center gap-2 px-2 py-4 text-muted-foreground text-sm">
                  <div class="w-3.5 h-3.5 border-[1.5px] border-border border-t-primary rounded-full animate-spin" />
                  {{ t('history.loadingLogs') }}
                </div>
                <div v-else-if="logCache[run.id].length === 0" class="text-center text-muted-foreground/50 text-sm py-3">{{ t('history.noLogsRecord') }}</div>
                <div
                  v-else
                  v-for="log in logCache[run.id]"
                  :key="log.id"
                  :class="cn('flex gap-2.5 px-2 py-0.5 text-xs font-mono rounded-sm hover:bg-secondary transition-colors', logLevelColor(log.level))"
                >
                  <span class="text-muted-foreground/50 whitespace-nowrap shrink-0">{{ formatTime(log.timestamp) }}</span>
                  <span class="break-all">{{ log.message }}</span>
                </div>
              </TabsContent>
            </Tabs>
          </div>
        </div>
      </Card>
    </div>
  </div>
</template>
