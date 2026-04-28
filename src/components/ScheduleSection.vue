<script setup lang="ts">
import { ref } from 'vue'
import { describeCron } from '../utils/cron'

interface ScheduleItem {
  id: string
  workflow_name: string
  cron_expr: string
  enabled: boolean
  last_run_at: string | null
}

const props = defineProps<{
  schedules: ScheduleItem[]
  loading: boolean
}>()

const emit = defineEmits<{
  'toggle-schedule': [s: ScheduleItem]
  'delete-schedule': [s: ScheduleItem]
  'edit-schedule': [s: ScheduleItem]
  'new-schedule': []
  'refresh': []
}>()

const showSchedules = ref(false)

function formatDate(iso: string): string {
  try {
    const d = new Date(iso)
    const pad = (n: number) => String(n).padStart(2, '0')
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
  } catch { return iso }
}

function formatLastRun(iso: string | null): string {
  if (!iso) return '从未运行'
  return formatDate(iso)
}
</script>

<template>
  <div class="schedule-section">
    <button class="progress-toggle" @click="showSchedules = !showSchedules">
      <span>⏰ 定时计划 · {{ schedules.length }} 个</span>
      <span class="toggle-arrow" :class="{ open: showSchedules }">▸</span>
    </button>
    <div v-if="showSchedules" class="schedule-body">
      <div v-if="loading" class="schedule-loading">加载中...</div>
      <div v-else-if="schedules.length === 0" class="schedule-empty">
        暂无定时计划。点击下方按钮创建。
      </div>
      <div v-else class="schedule-list">
        <div v-for="s in schedules" :key="s.id" class="schedule-card" :class="{ off: !s.enabled }">
          <div class="schedule-main">
            <div class="schedule-info">
              <div class="schedule-wf">{{ s.workflow_name }}</div>
              <div class="schedule-cron">
                <span class="cron-badge">{{ s.cron_expr }}</span>
                <span v-if="describeCron(s.cron_expr) !== s.cron_expr" class="cron-human">{{ describeCron(s.cron_expr) }}</span>
              </div>
              <div class="schedule-meta">上次运行: {{ formatLastRun(s.last_run_at) }}</div>
            </div>
            <div class="schedule-actions">
              <button class="action-btn" @click="emit('toggle-schedule', s)">
                {{ s.enabled ? '⏸ 禁用' : '▶ 启用' }}
              </button>
              <button class="action-btn" @click="emit('edit-schedule', s)">✏️ 编辑</button>
              <button class="action-btn danger" @click="emit('delete-schedule', s)">🗑</button>
            </div>
          </div>
        </div>
      </div>
      <button class="btn btn-primary btn-sm" style="margin-top: 10px" @click="emit('new-schedule')">
        ＋ 新建定时计划
      </button>
    </div>
  </div>
</template>

<style scoped>
.schedule-section { margin-top: 8px; border: 1px solid #30363d; border-radius: 10px; overflow: hidden; background: #161b22; }
.progress-toggle { display: flex; justify-content: space-between; align-items: center; width: 100%; padding: 12px 16px; background: none; border: none; color: #8b949e; font-size: 13px; cursor: pointer; transition: background 0.15s; }
.progress-toggle:hover { background: #1c2128; }
.toggle-arrow { font-size: 14px; transition: transform 0.2s; }
.toggle-arrow.open { transform: rotate(90deg); }
.schedule-body { padding: 0 16px 16px; border-top: 1px solid #21262d; }
.schedule-loading { font-size: 12px; color: #6e7681; padding: 12px 0; }
.schedule-empty { font-size: 12px; color: #484f58; padding: 16px 0; text-align: center; }
.schedule-list { display: flex; flex-direction: column; gap: 8px; margin-top: 12px; }
.schedule-card { background: #0d1117; border: 1px solid #21262d; border-radius: 8px; padding: 12px 14px; transition: border-color 0.15s; }
.schedule-card:hover { border-color: #484f58; }
.schedule-card.off { opacity: 0.6; }
.schedule-main { display: flex; justify-content: space-between; align-items: center; gap: 12px; }
.schedule-info { flex: 1; min-width: 0; }
.schedule-wf { font-size: 13px; font-weight: 600; color: #e1e4e8; margin-bottom: 4px; }
.schedule-cron { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
.cron-badge { font-size: 12px; font-family: 'Cascadia Code', monospace; color: #58a6ff; background: #1f6feb15; padding: 1px 6px; border-radius: 4px; }
.cron-human { font-size: 11px; color: #8b949e; }
.schedule-meta { font-size: 11px; color: #6e7681; }
.schedule-actions { display: flex; gap: 4px; flex-shrink: 0; }
.action-btn { background: #21262d; border: 1px solid #30363d; color: #c9d1d9; padding: 4px 10px; border-radius: 6px; font-size: 12px; cursor: pointer; transition: all 0.15s; }
.action-btn:hover { background: #30363d; border-color: #484f58; }
.action-btn.danger:hover { background: #da363322; border-color: #da363344; color: #f85149; }

.btn { padding: 6px 14px; border-radius: 6px; font-size: 13px; font-weight: 500; cursor: pointer; border: 1px solid #30363d; background: #21262d; color: #c9d1d9; transition: all 0.15s; }
.btn:hover { background: #30363d; }
.btn-sm { padding: 5px 12px; font-size: 12px; }
.btn-primary { background: #1f6feb; border-color: #388bfd; color: #fff; }
.btn-primary:hover { background: #388bfd; }
</style>
