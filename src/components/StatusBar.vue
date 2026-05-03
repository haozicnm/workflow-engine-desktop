<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'

interface ActiveRun {
  runId: string
  workflowName: string
  currentStep: string | null
  currentStepName: string | null
  totalSteps: number
  stepCount: number
  startedAt: number
}

const activeRuns = ref<ActiveRun[]>([])
const recentCompleted = ref(0)
const recentFailed = ref(0)
const elapsed = ref('')

let unlisteners: (() => void)[] = []
let elapsedTimer: ReturnType<typeof setInterval> | null = null
let clearTimer: ReturnType<typeof setTimeout> | null = null

onMounted(async () => {
  const { listen } = await import('@tauri-apps/api/event')

  unlisteners.push(
    await listen<{ run_id: string; workflow_name: string; status: string }>('run-update', (event) => {
      const { run_id, workflow_name, status } = event.payload
      if (status === 'running') {
        if (!activeRuns.value.find(r => r.runId === run_id)) {
          activeRuns.value.push({
            runId: run_id,
            workflowName: workflow_name || '工作流',
            currentStep: null,
            currentStepName: null,
            totalSteps: 0,
            stepCount: 0,
            startedAt: Date.now(),
          })
        }
      } else {
        activeRuns.value = activeRuns.value.filter(r => r.runId !== run_id)
        if (status === 'completed') recentCompleted.value++
        else if (status === 'failed') recentFailed.value++
        scheduleClearRecent()
      }
    })
  )

  unlisteners.push(
    await listen<{ run_id: string; step_name: string; total_steps: number; status: string }>('step-update', (event) => {
      const { run_id, step_name, total_steps, status } = event.payload
      const run = activeRuns.value.find(r => r.runId === run_id)
      if (run) {
        if (total_steps) run.totalSteps = total_steps
        if (status === 'running') {
          run.currentStepName = step_name || null
          run.stepCount++
        }
      }
    })
  )

  elapsedTimer = setInterval(updateElapsed, 1000)
})

onUnmounted(() => {
  unlisteners.forEach(fn => fn())
  if (elapsedTimer) clearInterval(elapsedTimer)
  if (clearTimer) clearTimeout(clearTimer)
})

function updateElapsed() {
  if (activeRuns.value.length > 0) {
    const ms = Date.now() - activeRuns.value[0].startedAt
    const sec = Math.floor(ms / 1000)
    const min = Math.floor(sec / 60)
    const hr = Math.floor(min / 60)
    elapsed.value = hr > 0
      ? `${hr}:${String(min % 60).padStart(2, '0')}:${String(sec % 60).padStart(2, '0')}`
      : `${min}:${String(sec % 60).padStart(2, '0')}`
  } else {
    elapsed.value = ''
  }
}

function scheduleClearRecent() {
  if (clearTimer) clearTimeout(clearTimer)
  clearTimer = setTimeout(() => {
    recentCompleted.value = 0
    recentFailed.value = 0
  }, 5000)
}

const hasActivity = computed(() =>
  activeRuns.value.length > 0 || recentCompleted.value > 0 || recentFailed.value > 0
)
</script>

<template>
  <div class="status-bar">
    <div class="status-left" v-if="hasActivity">
      <template v-if="activeRuns.length > 0">
        <span class="status-dot running"></span>
        <template v-if="activeRuns.length === 1">
          <span class="status-name">{{ activeRuns[0].workflowName }}</span>
          <span class="status-step" v-if="activeRuns[0].currentStepName">
            — {{ activeRuns[0].currentStepName }}
          </span>
          <span class="status-count" v-if="activeRuns[0].totalSteps > 0">
            {{ activeRuns[0].stepCount }}/{{ activeRuns[0].totalSteps }}
          </span>
        </template>
        <template v-else>
          <span class="status-name">{{ activeRuns.length }} 个工作流运行中</span>
        </template>
      </template>
    </div>
    <div class="status-right">
      <span class="status-version">v3.2.0</span>
      <span class="status-elapsed" v-if="activeRuns.length > 0 && elapsed">⏱ {{ elapsed }}</span>
      <span class="status-recent-ok" v-if="recentCompleted > 0">✅ {{ recentCompleted }}</span>
      <span class="status-recent-fail" v-if="recentFailed > 0">❌ {{ recentFailed }}</span>
    </div>
  </div>
</template>

<style scoped>
.status-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 28px;
  padding: 0 16px;
  background: #161b22;
  border-top: 1px solid #30363d;
  font-size: 12px;
  color: #8b949e;
  flex-shrink: 0;
}
.status-left {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  overflow: hidden;
}
.status-right {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}
.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.status-dot.running {
  background: #3fb950;
  animation: pulse 1.5s ease-in-out infinite;
}
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
.status-name {
  color: #e1e4e8;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.status-step {
  color: #8b949e;
  white-space: nowrap;
}
.status-count {
  color: #58a6ff;
  white-space: nowrap;
}
.status-version {
  color: #484f58;
  font-size: 11px;
  margin-right: 12px;
}

.status-elapsed {
  color: #58a6ff;
  font-variant-numeric: tabular-nums;
}
.status-recent-ok {
  color: #3fb950;
}
.status-recent-fail {
  color: #f85149;
}
</style>
