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

// API health polling
let apiTimer: ReturnType<typeof setInterval> | null = null
const { refreshApiStatus } = useGlobalStatus()
onMounted(() => {
  refreshApiStatus()
  apiTimer = setInterval(refreshApiStatus, 15_000)
})
onUnmounted(() => { if (apiTimer) clearInterval(apiTimer) })

// WebBridge health polling
let webBridgeTimer: ReturnType<typeof setInterval> | null = null
const { refreshWebBridgeStatus } = useGlobalStatus()
onMounted(() => {
  refreshWebBridgeStatus()
  webBridgeTimer = setInterval(refreshWebBridgeStatus, 30_000)
})
onUnmounted(() => { if (webBridgeTimer) clearInterval(webBridgeTimer) })

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
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  }
  return d.toLocaleString([], { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' })
}
</script>

<template>
  <div class="status-bar h-7 border-t border-[var(--border-neutral-l1)] bg-[var(--bg-base-secondary)] flex items-center px-3 gap-3 text-xs text-[var(--text-tertiary)] select-none shrink-0">
    <!-- IPC daemon status -->
    <span class="flex items-center gap-1.5" :title="state.ipcOnline ? t('statusBar.daemonConnected') : t('statusBar.daemonDisconnected')">
      <span class="w-1.5 h-1.5 rounded-full" :class="state.ipcOnline ? 'bg-[var(--status-success-default)]/80' : 'bg-[var(--status-error-default)]/60'"></span>
      <span v-if="state.ipcOnline" class="text-[var(--text-tertiary)]/70">{{ t('dashboard.daemonStatus') }}</span>
      <span v-else class="text-[var(--status-error-default)]/70">{{ t('dashboard.daemonOffline') }}</span>
    </span>

    <span class="text-[var(--text-disabled)]">│</span>

    <!-- API server status -->
    <span class="flex items-center gap-1.5" :title="state.apiOnline ? t('statusBar.apiOnline', '后端服务在线') : t('statusBar.apiOffline', '后端服务离线')">
      <span class="w-1.5 h-1.5 rounded-full" :class="state.apiOnline ? 'bg-[var(--status-success-default)]/80' : 'bg-[var(--status-error-default)]/60'"></span>
      <span class="text-[var(--text-tertiary)]/70">{{ state.apiOnline ? t('statusBar.apiOnlineShort', '服务在线') : t('statusBar.apiOfflineShort', '服务离线') }}</span>
    </span>

    <span class="text-[var(--text-disabled)]">│</span>

    <!-- WebBridge status -->
    <span class="flex items-center gap-1.5" :title="state.webbridgeConnected ? `WebBridge 已连接 (v${state.webbridgeVersion})` : 'WebBridge 未连接'">
      <span class="w-1.5 h-1.5 rounded-full" :class="state.webbridgeConnected ? 'bg-[var(--status-success-default)]/80' : 'bg-[var(--bg-overlay-l3)]'"></span>
      <span class="text-[var(--text-tertiary)]/70">{{ state.webbridgeConnected ? t('statusBar.webbridgeConnected', 'WebBridge') : t('statusBar.webbridgeDisconnected', 'WebBridge') }}</span>
      <span v-if="state.webbridgeConnected" class="text-[var(--text-tertiary)]/40">✓</span>
    </span>

    <span class="text-[var(--text-disabled)]">│</span>

    <!-- Idle state -->
    <template v-if="isIdle">
      <span class="flex items-center gap-1.5">
        <span class="w-1.5 h-1.5 rounded-full bg-[var(--status-success-default)]/70"></span>
        {{ t('statusBar.ready') }}
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
          <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-[var(--bg-brand)]/70 opacity-75"></span>
          <span class="relative inline-flex rounded-full h-2 w-2 bg-[var(--bg-brand)]"></span>
        </span>
        <span class="text-[var(--text-default)] font-medium">{{ run.name }}</span>
        <span v-if="run.currentStep" class="text-[var(--text-tertiary)]/70">
          {{ run.currentStep }}
        </span>
        <span v-if="run.progress" class="text-[var(--text-tertiary)]/50 tabular-nums">
          {{ run.progress.done }}/{{ run.progress.total }}
        </span>
        <span class="text-[var(--text-tertiary)]/40 tabular-nums">
          {{ formatElapsed(Date.now() - run.startedAt) }}
        </span>
      </div>
    </template>

    <!-- Separator -->
    <span v-if="hasRunning && hasScheduled" class="text-[var(--text-disabled)]">│</span>

    <!-- Scheduled workflows -->
    <template v-if="hasScheduled">
      <div class="flex items-center gap-1.5">
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-[var(--status-warning-default)]/80"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
        <span>{{ t('statusBar.scheduledCount', { n: state.scheduledWorkflows.length }) }}</span>
        <span v-for="s in state.scheduledWorkflows.slice(0, 3)" :key="s.id" class="flex items-center gap-1 text-[var(--text-tertiary)]/60">
          <span class="max-w-[100px] truncate">{{ s.workflowName }}</span>
          <span class="text-[var(--text-tertiary)]/40">{{ formatNextRun(s.nextRun) }}</span>
        </span>
        <span v-if="state.scheduledWorkflows.length > 3" class="text-[var(--text-tertiary)]/40">
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
