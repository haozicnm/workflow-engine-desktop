<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { safeInvoke } from '../utils/tauri'
import StepDetail from './StepDetail.vue'

interface StepPreview {
  step_id: string
  step_name: string
  step_type: string
  status: string
  duration_ms: number
  summary: string
  detail: any
  bundle_path: string | null
}

const props = defineProps<{
  runId: string
}>()

const trajectory = ref<StepPreview[]>([])
const loading = ref(false)
const selectedStepId = ref<string | null>(null)

const selectedStep = computed(() =>
  trajectory.value.find(s => s.step_id === selectedStepId.value) ?? null
)

async function loadTrajectory() {
  if (!props.runId) return
  loading.value = true
  try {
    trajectory.value = await safeInvoke<StepPreview[]>('get_trajectory', { runId: props.runId }) || []
  } catch (e: any) {
    console.warn('加载 trajectory 失败:', e)
  } finally {
    loading.value = false
  }
}

function selectStep(stepId: string) {
  selectedStepId.value = selectedStepId.value === stepId ? null : stepId
}

function statusIcon(status: string): string {
  switch (status) {
    case 'completed': return '✓'
    case 'failed': return '✗'
    case 'skipped': return '⏭'
    default: return '?'
  }
}

function statusColor(status: string): string {
  switch (status) {
    case 'completed': return 'text-[var(--status-success-default)]'
    case 'failed': return 'text-[var(--status-error-default)]'
    case 'skipped': return 'text-[var(--text-tertiary)]'
    default: return 'text-[var(--text-tertiary)]'
  }
}

function formatMs(ms: number): string {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

// Load on mount and when runId changes
watch(() => props.runId, loadTrajectory, { immediate: true })
</script>

<template>
  <div class="flex flex-col h-full max-h-[600px]">
    <div v-if="loading" class="flex items-center gap-2 px-2 py-4 text-[var(--text-tertiary)] text-sm">
      <div class="w-3.5 h-3.5 border-[1.5px] border-[var(--border-neutral-l1)] border-t-primary rounded-full animate-spin" />
      <span>加载执行轨迹...</span>
    </div>

    <div v-else-if="trajectory.length === 0" class="text-center text-[var(--text-tertiary)]/50 text-sm py-3">
      暂无预览数据
    </div>

    <div v-else class="flex gap-4 flex-1 min-h-0">
      <!-- Timeline (left) -->
      <div class="flex-1 overflow-y-auto min-w-0">
        <div
          v-for="(step, idx) in trajectory"
          :key="step.step_id"
          class="cursor-pointer transition-colors hover:bg-[var(--bg-overlay-l1)]/50 rounded-md"
          :class="selectedStepId === step.step_id ? 'bg-[var(--bg-overlay-l1)]/80' : ''"
          @click="selectStep(step.step_id)"
        >
          <div class="flex items-start gap-2.5 px-3 py-2">
            <!-- Timeline dot + line -->
            <div class="flex flex-col items-center shrink-0 pt-0.5">
              <div
                class="w-2.5 h-2.5 rounded-full border-2"
                :class="{
                  'bg-[var(--status-success-default)] border-[var(--status-success-default)]': step.status === 'completed',
                  'bg-[var(--status-error-default)] border-[var(--status-error-default)]': step.status === 'failed',
                  'border-muted-foreground/30': step.status === 'skipped',
                  'border-[var(--border-neutral-l1)]': step.status !== 'completed' && step.status !== 'failed' && step.status !== 'skipped',
                }"
              />
              <div
                v-if="idx < trajectory.length - 1"
                class="w-px flex-1 min-h-[12px]"
                :class="trajectory[idx + 1].status === 'completed' ? 'bg-[var(--status-success-default)]/30' : 'bg-border'"
              />
            </div>

            <!-- Content -->
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2">
                <span :class="statusColor(step.status)" class="text-xs">{{ statusIcon(step.status) }}</span>
                <span class="text-xs font-medium text-[var(--text-default)] truncate">{{ step.step_name }}</span>
                <span class="text-[10px] text-[var(--text-tertiary)]/50 shrink-0">{{ step.step_type }}</span>
                <span v-if="step.duration_ms > 0" class="text-[10px] text-[var(--text-tertiary)]/50 shrink-0">{{ formatMs(step.duration_ms) }}</span>
              </div>
              <div class="text-[11px] text-[var(--text-tertiary)] mt-0.5 truncate">{{ step.summary }}</div>
            </div>
          </div>
        </div>
      </div>

      <!-- Step detail (right) -->
      <div v-if="selectedStep" class="w-[320px] shrink-0 border-l border-[var(--border-neutral-l1)] pl-3 overflow-y-auto">
        <StepDetail :step="selectedStep" :run-id="runId" />
      </div>
    </div>
  </div>
</template>
