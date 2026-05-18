<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useGlobalStatus } from '../composables/useGlobalStatus'

const { t } = useI18n()

const { state } = useGlobalStatus()

// Force reactivity tick every second for elapsed time
const now = ref(Date.now())
let tickTimer: ReturnType<typeof setInterval> | null = null
onMounted(() => { tickTimer = setInterval(() => { now.value = Date.now() }, 1000) })
onUnmounted(() => { if (tickTimer) clearInterval(tickTimer) })

// IPC health polling
let ipcTimer: ReturnType<typeof setInterval> | null = null
const { refreshIpcStatus } = useGlobalStatus()
onMounted(() => {
  refreshIpcStatus()
  ipcTimer = setInterval(refreshIpcStatus, 30_000)
})
onUnmounted(() => { if (ipcTimer) clearInterval(ipcTimer) })

const runningList = computed(() => {
  void now.value
  return Array.from(state.runningWorkflows.values())
})
const hasRunning = computed(() => runningList.value.length > 0)
const hasScheduled = computed(() => state.scheduledWorkflows.length > 0)
const isIdle = computed(() => !hasRunning.value && !hasScheduled.value)

function formatElapsed(ms: number): string {
  const s = Math.floor(ms / 1000)
  if (s < 60) return `${s}s`
  const m = Math.floor(s / 60)
  return `${m}m${s % 60}s`
}

function formatNextRun(iso: string | null): string {
  if (!iso) return '-'
  const d = new Date(iso)
  const now = new Date()
  const diff = d.getTime() - now.getTime()
  if (diff < 0) return '即将'
  if (diff < 60_000) return `${Math.ceil(diff / 1000)}s`
  if (diff < 3_600_000) return `${Math.ceil(diff / 60_000)}min`
  const today = new Date()
  if (d.toDateString() === today.toDateString()) {
    return d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
  }
  return d.toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' })
}
</script>

<template>
  <div class="status-bar h-7 border-t border-border bg-card/50 backdrop-blur-sm flex items-center px-3 gap-3 text-xs text-muted-foreground select-none shrink-0">
    <!-- IPC daemon status -->
    <span class="flex items-center gap-1.5" :title="state.ipcOnline ? '守护进程已连接' : '守护进程未连接'">
      <span class="w-1.5 h-1.5 rounded-full" :class="state.ipcOnline ? 'bg-success/80' : 'bg-destructive/60'"></span>
      <span v-if="state.ipcOnline" class="text-muted-foreground/70">{{ t('dashboard.daemonStatus') }}</span>
      <span v-else class="text-destructive/70">{{ t('dashboard.daemonOffline') }}</span>
    </span>

    <span class="text-border">│</span>

    <!-- Idle state -->
    <template v-if="isIdle">
      <span class="flex items-center gap-1.5">
        <span class="w-1.5 h-1.5 rounded-full bg-success/70"></span>
        就绪
      </span>
    </template>

    <!-- Running workflows -->
    <template v-if="hasRunning">
      <div
        v-for="run in runningList"
        :key="run.id"
        class="flex items-center gap-1.5"
      >
        <span class="relative flex h-2 w-2">
          <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-primary/70 opacity-75"></span>
          <span class="relative inline-flex rounded-full h-2 w-2 bg-primary"></span>
        </span>
        <span class="text-foreground font-medium">{{ run.name }}</span>
        <span v-if="run.currentStep" class="text-muted-foreground/70">
          {{ run.currentStep }}
        </span>
        <span v-if="run.progress" class="text-muted-foreground/50 tabular-nums">
          {{ run.progress.done }}/{{ run.progress.total }}
        </span>
        <span class="text-muted-foreground/40 tabular-nums">
          {{ formatElapsed(Date.now() - run.startedAt) }}
        </span>
      </div>
    </template>

    <!-- Separator -->
    <span v-if="hasRunning && hasScheduled" class="text-border">│</span>

    <!-- Scheduled workflows -->
    <template v-if="hasScheduled">
      <div class="flex items-center gap-1.5">
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-warning/80"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
        <span>{{ state.scheduledWorkflows.length }}个定时任务</span>
        <span v-for="s in state.scheduledWorkflows.slice(0, 3)" :key="s.id" class="flex items-center gap-1 text-muted-foreground/60">
          <span class="max-w-[100px] truncate">{{ s.workflowName }}</span>
          <span class="text-muted-foreground/40">{{ formatNextRun(s.nextRun) }}</span>
        </span>
        <span v-if="state.scheduledWorkflows.length > 3" class="text-muted-foreground/40">
          +{{ state.scheduledWorkflows.length - 3 }}
        </span>
      </div>
    </template>
  </div>
</template>

<style scoped>
.status-bar {
  font-variant-numeric: tabular-nums;
}
</style>
