<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Clock } from 'lucide-vue-next'
import { safeInvoke } from '../utils/tauri'
import { useToast } from '../composables/useToast'
import Button from './ui/button/Button.vue'
import Input from './ui/input/Input.vue'
import Badge from './ui/badge/Badge.vue'
import Card from './ui/card/Card.vue'
import CardContent from './ui/card/CardContent.vue'
import { cn } from '@/lib/utils'

const { t } = useI18n()

interface ScheduleItem {
  id: string
  workflow_id: string
  workflow_name: string
  cron_expr: string
  enabled: boolean
  last_run: string | null
  next_run: string | null
}

const props = defineProps<{
  workflowId?: string
  workflowName?: string
}>()

const emit = defineEmits<{
  close: []
}>()

const toast = useToast()
const schedules = ref<ScheduleItem[]>([])
const loading = ref(false)

const showCreate = ref(false)
const newCron = ref('0 9 * * *')
const cronPresets = computed(() => [
  { label: t('cronPreset.everyHour'), value: '0 * * * *' },
  { label: t('cronPreset.everyDay9'), value: '0 9 * * *' },
  { label: t('cronPreset.everyDay18'), value: '0 18 * * *' },
  { label: t('cronPreset.weekday9'), value: '0 9 * * 1-5' },
  { label: t('cronPreset.monday9'), value: '0 9 * * 1' },
  { label: t('cronPreset.monthly1st9'), value: '0 9 1 * *' },
])

const cronDescription = computed(() => describeCron(newCron.value))

onMounted(loadSchedules)

async function loadSchedules() {
  loading.value = true
  try {
    const all = await safeInvoke<ScheduleItem[]>('schedule_list') ?? []
    schedules.value = props.workflowId
      ? all.filter(s => s.workflow_id === props.workflowId)
      : all
  } catch (e) {
    console.error('加载调度失败:', e)
  } finally { loading.value = false }
}

async function onCreate() {
  if (!props.workflowId) return
  if (!newCron.value.trim()) { toast.error(t('schedule.enterCron')); return }
  try {
    await safeInvoke('schedule_create', { workflowId: props.workflowId, cronExpr: newCron.value.trim() })
    toast.success(t('toast.created'))
    showCreate.value = false
    await loadSchedules()
  } catch (e: unknown) {
    toast.error(t('schedule.createFailed') + ': ' + ((e as Error).message || e))
  }
}

async function onToggle(item: ScheduleItem) {
  try {
    await safeInvoke('schedule_update', { id: item.id, enabled: !item.enabled })
    item.enabled = !item.enabled
    toast.success(item.enabled ? t('toast.updated') : t('toast.updated'))
  } catch (e: unknown) {
    toast.error(t('schedule.updateFailed') + ': ' + ((e as Error).message || e))
  }
}

async function onDelete(item: ScheduleItem) {
  try {
    await safeInvoke('schedule_delete', { id: item.id })
    schedules.value = schedules.value.filter(s => s.id !== item.id)
    toast.success(t('toast.deleted'))
  } catch (e: unknown) {
    toast.error(t('schedule.deleteFailed') + ': ' + ((e as Error).message || e))
  }
}

function describeCron(expr: string): string {
  const parts = expr.trim().split(/\s+/)
  if (parts.length !== 5) return t('error.invalidInput')
  const [min, hour, dom, mon, dow] = parts
  if (min === '0' && hour === '*' && dom === '*' && mon === '*' && dow === '*') return `${hour.padStart(2, '0')}:${min.padStart(2, '0')}`
  if (hour !== '*' && min !== '*' && dom === '*' && mon === '*' && dow === '*') return t('cronDesc.daily', { hour: hour.padStart(2, '0'), min: min.padStart(2, '0') })
  if (hour !== '*' && min !== '*' && dow === '1-5') return t('cronDesc.weekdays', { hour: hour.padStart(2, '0'), min: min.padStart(2, '0') })
  if (dow !== '*' && dow !== '?') return t('cronDesc.everyDow', { dow, hour, min })
  if (dom !== '*') return t('cronDesc.monthly', { dom, hour, min })
  return expr
}

function formatDate(d: string | null): string {
  if (!d) return '-'
  return new Date(d).toLocaleString()
}
</script>

<template>
  <div class="h-full flex flex-col">
    <!-- Header -->
    <div class="flex items-center justify-between px-6 py-4 border-b border-[var(--border-neutral-l1)] shrink-0">
      <div>
        <h2 class="text-base font-semibold text-[var(--text-default)]">{{ t('schedule.title') }}</h2>
        <p v-if="workflowName" class="text-xs text-[var(--text-tertiary)] mt-0.5">{{ workflowName }}</p>
      </div>
      <div class="flex gap-1.5">
        <Button v-if="workflowId" variant="outline" size="sm" class="h-7 text-xs" @click="showCreate = !showCreate">
          {{ showCreate ? t('common.cancel') : t('schedule.newSchedule') }}
        </Button>
        <Button variant="ghost" size="icon" class="h-7 w-7" :aria-label="t('schedule.closeAria')" @click="emit('close')">
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
        </Button>
      </div>
    </div>

    <!-- Create form -->
    <Transition name="collapse">
      <div v-if="showCreate" class="border-b border-[var(--border-neutral-l1)] bg-[var(--bg-overlay-l1)]/30">
        <div class="px-6 py-4 space-y-3">
          <div class="flex gap-1.5 flex-wrap">
            <Button
              v-for="p in cronPresets"
              :key="p.value"
              variant="outline"
              size="sm"
              :class="cn(
                'h-6 text-[11px]',
                newCron === p.value
                  ? 'border-[var(--bg-brand)] text-[var(--text-brand)] bg-[var(--bg-brand)]/10'
                  : 'text-[var(--text-tertiary)] hover:bg-[var(--bg-overlay-l1)] hover:text-[var(--text-default)]',
              )"
              @click="newCron = p.value"
            >{{ p.label }}</Button>
          </div>
          <div class="flex gap-2 items-center">
            <Input v-model="newCron" :placeholder="t('schedule.cronPlaceholder')" class="flex-1 h-8 text-xs font-mono" />
            <Button variant="default" size="sm" class="h-8 text-xs bg-[var(--status-success-default)] hover:bg-[var(--status-success-default)]/90" @click="onCreate">{{ t('common.create') }}</Button>
          </div>
          <p class="text-[11px] text-[var(--text-tertiary)]">{{ cronDescription }}</p>
        </div>
      </div>
    </Transition>

    <!-- List -->
    <div class="flex-1 overflow-y-auto">
      <div v-if="loading" class="flex items-center justify-center py-8">
        <span class="text-[var(--text-tertiary)] text-sm">{{ t('common.loading') }}</span>
      </div>
      <div v-else-if="!schedules.length" class="flex flex-col items-center justify-center py-8 text-center">
        <Clock class="w-8 h-8 mb-2 text-[var(--text-tertiary)]" />
        <span class="text-[var(--text-tertiary)] text-sm">{{ workflowId ? t('schedule.noSchedulesFor') : t('schedule.noSchedules') }}</span>
      </div>
      <div v-else class="p-4 space-y-3">
        <Card v-for="item in schedules" :key="item.id" class="shadow-none">
          <CardContent class="p-4">
            <div class="flex items-start justify-between gap-3">
              <div class="flex-1 min-w-0 space-y-1">
                <div class="text-sm font-medium text-[var(--text-default)] truncate">{{ item.workflow_name }}</div>
                <div class="text-xs font-mono text-[var(--text-brand)]">{{ item.cron_expr }}</div>
                <div class="text-xs text-[var(--text-tertiary)]">{{ describeCron(item.cron_expr) }}</div>
                <div class="text-[10px] text-[var(--text-tertiary)]/60">
                  {{ t('schedule.lastNext', { last: formatDate(item.last_run), next: formatDate(item.next_run) }) }}
                </div>
              </div>
              <div class="flex flex-col gap-1.5 items-end shrink-0">
                <Badge
                  :variant="item.enabled ? 'success' : 'secondary'"
                  class="cursor-pointer text-[10px]"
                  @click="onToggle(item)"
                >{{ item.enabled ? t('schedule.enabled') : t('schedule.disabled') }}</Badge>
                <Button
                  variant="ghost"
                  size="icon"
                  class="text-[var(--text-tertiary)] hover:text-[var(--status-error-default)] hover:bg-[var(--status-error-default)]/10 opacity-40 hover:opacity-100 transition-opacity"
                  :aria-label="t('schedule.deleteAria')"
                  @click="onDelete(item)"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  </div>
</template>

<style scoped>
.collapse-enter-active,
.collapse-leave-active {
  transition: all 0.15s ease;
  overflow: hidden;
}
.collapse-enter-from,
.collapse-leave-to {
  max-height: 0;
  opacity: 0;
}
</style>
