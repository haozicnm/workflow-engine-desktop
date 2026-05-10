<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { Clock } from 'lucide-vue-next'
import { safeInvoke } from '../utils/tauri'
import { useToast } from '../composables/useToast'
import Button from './ui/button/Button.vue'
import Input from './ui/input/Input.vue'
import Badge from './ui/badge/Badge.vue'
import Card from './ui/card/Card.vue'
import CardHeader from './ui/card/CardHeader.vue'
import CardTitle from './ui/card/CardTitle.vue'
import CardContent from './ui/card/CardContent.vue'
import Separator from './ui/separator/Separator.vue'
import { cn } from '@/lib/utils'

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
const cronPresets = [
  { label: '每小时', value: '0 * * * *' },
  { label: '每天 9:00', value: '0 9 * * *' },
  { label: '每天 18:00', value: '0 18 * * *' },
  { label: '工作日 9:00', value: '0 9 * * 1-5' },
  { label: '每周一 9:00', value: '0 9 * * 1' },
  { label: '每月1号 9:00', value: '0 9 1 * *' },
]

const cronDescription = computed(() => describeCron(newCron.value))

onMounted(loadSchedules)

async function loadSchedules() {
  loading.value = true
  try {
    const all = await safeInvoke<ScheduleItem[]>('schedule_list')
    schedules.value = props.workflowId
      ? all.filter(s => s.workflow_id === props.workflowId)
      : all
  } catch (e) {
    console.error('加载调度失败:', e)
  } finally { loading.value = false }
}

async function onCreate() {
  if (!props.workflowId) return
  if (!newCron.value.trim()) { toast.error('请输入 Cron 表达式'); return }
  try {
    await safeInvoke('schedule_create', { workflowId: props.workflowId, cronExpr: newCron.value.trim() })
    toast.success('定时调度已创建')
    showCreate.value = false
    await loadSchedules()
  } catch (e: unknown) {
    toast.error('创建失败: ' + ((e as Error).message || e))
  }
}

async function onToggle(item: ScheduleItem) {
  try {
    await safeInvoke('schedule_update', { id: item.id, enabled: !item.enabled })
    item.enabled = !item.enabled
    toast.success(item.enabled ? '已启用' : '已禁用')
  } catch (e: unknown) {
    toast.error('更新失败: ' + ((e as Error).message || e))
  }
}

async function onDelete(item: ScheduleItem) {
  try {
    await safeInvoke('schedule_delete', { id: item.id })
    schedules.value = schedules.value.filter(s => s.id !== item.id)
    toast.success('已删除')
  } catch (e: unknown) {
    toast.error('删除失败: ' + ((e as Error).message || e))
  }
}

function describeCron(expr: string): string {
  const parts = expr.trim().split(/\s+/)
  if (parts.length !== 5) return '无效的 Cron 表达式'
  const [min, hour, dom, mon, dow] = parts
  if (min === '0' && hour === '*' && dom === '*' && mon === '*' && dow === '*') return '每小时整点'
  if (hour !== '*' && min !== '*' && dom === '*' && mon === '*' && dow === '*') return `每天 ${hour.padStart(2, '0')}:${min.padStart(2, '0')}`
  if (hour !== '*' && min !== '*' && dow === '1-5') return `工作日 ${hour.padStart(2, '0')}:${min.padStart(2, '0')}`
  if (dow !== '*' && dow !== '?') return `每${dow} ${hour}:${min}`
  if (dom !== '*') return `每月${dom}号 ${hour}:${min}`
  return expr
}

function formatDate(d: string | null): string {
  if (!d) return '-'
  return new Date(d).toLocaleString('zh-CN')
}
</script>

<template>
  <div class="h-full flex flex-col">
    <!-- Header -->
    <div class="flex items-center justify-between px-6 py-4 border-b border-border shrink-0">
      <div>
        <h2 class="text-base font-semibold text-foreground">定时调度</h2>
        <p v-if="workflowName" class="text-xs text-muted-foreground mt-0.5">{{ workflowName }}</p>
      </div>
      <div class="flex gap-1.5">
        <Button v-if="workflowId" variant="outline" size="sm" class="h-7 text-xs" @click="showCreate = !showCreate">
          {{ showCreate ? '取消' : '＋ 新建' }}
        </Button>
        <Button variant="ghost" size="icon" class="h-7 w-7" aria-label="关闭" @click="emit('close')">
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
        </Button>
      </div>
    </div>

    <!-- Create form -->
    <Transition name="collapse">
      <div v-if="showCreate" class="border-b border-border bg-muted/30">
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
                  ? 'border-primary text-primary bg-primary/10'
                  : 'text-muted-foreground hover:bg-secondary hover:text-foreground',
              )"
              @click="newCron = p.value"
            >{{ p.label }}</Button>
          </div>
          <div class="flex gap-2 items-center">
            <Input v-model="newCron" placeholder="Cron 表达式 (分 时 日 月 周)" class="flex-1 h-8 text-xs font-mono" />
            <Button variant="default" size="sm" class="h-8 text-xs bg-success hover:bg-success/90" @click="onCreate">创建</Button>
          </div>
          <p class="text-[11px] text-muted-foreground">{{ cronDescription }}</p>
        </div>
      </div>
    </Transition>

    <!-- List -->
    <div class="flex-1 overflow-y-auto">
      <div v-if="loading" class="flex items-center justify-center py-8">
        <span class="text-muted-foreground text-sm">加载中...</span>
      </div>
      <div v-else-if="!schedules.length" class="flex flex-col items-center justify-center py-8 text-center">
        <Clock class="w-8 h-8 mb-2 text-muted-foreground" />
        <span class="text-muted-foreground text-sm">{{ workflowId ? '此工作流暂无定时调度' : '暂无定时调度' }}</span>
      </div>
      <div v-else class="p-4 space-y-3">
        <Card v-for="item in schedules" :key="item.id" class="shadow-none">
          <CardContent class="p-4">
            <div class="flex items-start justify-between gap-3">
              <div class="flex-1 min-w-0 space-y-1">
                <div class="text-sm font-medium text-foreground truncate">{{ item.workflow_name }}</div>
                <div class="text-xs font-mono text-primary">{{ item.cron_expr }}</div>
                <div class="text-xs text-muted-foreground">{{ describeCron(item.cron_expr) }}</div>
                <div class="text-[10px] text-muted-foreground/60">
                  上次: {{ formatDate(item.last_run) }} · 下次: {{ formatDate(item.next_run) }}
                </div>
              </div>
              <div class="flex flex-col gap-1.5 items-end shrink-0">
                <Badge
                  :variant="item.enabled ? 'success' : 'secondary'"
                  class="cursor-pointer text-[10px]"
                  @click="onToggle(item)"
                >{{ item.enabled ? '启用' : '禁用' }}</Badge>
                <Button
                  variant="ghost"
                  size="icon"
                  class="text-muted-foreground hover:text-destructive hover:bg-destructive/10 opacity-40 hover:opacity-100 transition-opacity"
                  aria-label="删除调度"
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
