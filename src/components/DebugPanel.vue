<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useWorkflowStore } from '../stores/workflowStore'
import { safeInvoke, safeListen } from '../utils/tauri'
import { Bug, Play, SkipForward, Square, ChevronRight, ChevronDown, X } from 'lucide-vue-next'
import Button from './ui/button/Button.vue'
import Badge from './ui/badge/Badge.vue'
import ScrollArea from './ui/scroll-area/ScrollArea.vue'

const { t } = useI18n()
const store = useWorkflowStore()

const props = defineProps<{
  workflowId?: string
  debugRunId?: string | null
  isRunning: boolean
}>()

const emit = defineEmits<{
  'set-breakpoint': [stepId: string]
  'remove-breakpoint': [stepId: string]
}>()

// ─── 调试状态 ───
const debugState = ref<'idle' | 'paused' | 'running'>('idle')
const currentStepId = ref<string | null>(null)
const variables = ref<Record<string, unknown>>({})
const callStack = ref<string[]>([])
const expandedGroups = ref<Set<string>>(new Set(['step_outputs']))

// ─── 变量分组 ───
const variableGroups = computed(() => {
  const groups: { key: string; label: string; vars: { key: string; value: unknown }[] }[] = []
  const stepOutputs: typeof groups[0]['vars'] = []
  const workflowVars: typeof groups[0]['vars'] = []
  const internalVars: typeof groups[0]['vars'] = []

  for (const [k, v] of Object.entries(variables.value)) {
    if (k.startsWith('_')) {
      internalVars.push({ key: k, value: v })
    } else if (k.includes('.') || store.current?.steps.some(s => k === s.id || k.startsWith(s.id + '.'))) {
      stepOutputs.push({ key: k, value: v })
    } else {
      workflowVars.push({ key: k, value: v })
    }
  }

  if (stepOutputs.length) groups.push({ key: 'step_outputs', label: t('debug.stepOutputs'), vars: stepOutputs })
  if (workflowVars.length) groups.push({ key: 'workflow_vars', label: t('debug.workflowVars'), vars: workflowVars })
  if (internalVars.length) groups.push({ key: 'internal', label: t('debug.internalVars'), vars: internalVars })

  return groups
})

// ─── 断点列表 ───
const breakpoints = computed(() =>
  (store.current?.steps || []).filter(s => s.breakpoint)
)

// ─── 调试控制 ───
const activeRunId = computed(() => props.debugRunId || props.workflowId || null)

async function onStepOver() {
  if (!activeRunId.value) return
  try {
    await safeInvoke('debug_step', { runId: activeRunId.value })
    debugState.value = 'paused'
  } catch (e) {
    console.error('debug_step failed:', e)
  }
}

async function onContinue() {
  if (!activeRunId.value) return
  try {
    await safeInvoke('debug_continue', { runId: activeRunId.value })
    debugState.value = 'running'
  } catch (e) {
    console.error('debug_continue failed:', e)
  }
}

async function onStop() {
  if (!activeRunId.value) return
  try {
    await safeInvoke('run_cancel', { runId: activeRunId.value })
    debugState.value = 'idle'
  } catch (e) {
    console.error('run_cancel failed:', e)
  }
}

function toggleGroup(key: string) {
  if (expandedGroups.value.has(key)) {
    expandedGroups.value.delete(key)
  } else {
    expandedGroups.value.add(key)
  }
}

// ─── 变量轮询（暂停时刷新）──
let refreshTimer: ReturnType<typeof setInterval> | null = null

async function refreshVariables() {
  if (!activeRunId.value || debugState.value !== 'paused') return
  try {
    const vars = await safeInvoke<Record<string, unknown>>('debug_vars', { runId: activeRunId.value })
    if (vars) {
      // debug_vars 返回 {variables: {...}, step_outputs: {...}}，需要展平
      const v = vars as Record<string, unknown>
      variables.value = {
        ...(typeof v.variables === 'object' && v.variables ? v.variables as Record<string, unknown> : {}),
        ...(typeof v.step_outputs === 'object' && v.step_outputs ? v.step_outputs as Record<string, unknown> : {}),
      }
    }
  } catch { /* 运行已结束或不可用 */ }
}

watch(() => debugState.value, (state) => {
  if (refreshTimer) { clearInterval(refreshTimer); refreshTimer = null }
  if (state === 'paused') {
    refreshVariables() // 立即刷新一次
    refreshTimer = setInterval(refreshVariables, 2000) // 每 2 秒轮询
  }
})

// ─── SSE 监听 ───
let unlistenBreakpoint: (() => void) | null = null
let unlistenRun: (() => void) | null = null
let unlistenVariable: (() => void) | null = null

async function startListening() {
  unlistenBreakpoint = await safeListen<{ run_id: string; step_id: string; reason: string; variables: Record<string, unknown> }>(
    'breakpoint-hit',
    (event) => {
      debugState.value = 'paused'
      currentStepId.value = event.payload.step_id
      variables.value = event.payload.variables || {}
      callStack.value.push(event.payload.step_id)
    }
  )

  unlistenVariable = await safeListen<{ run_id: string; variables: Record<string, unknown>; step_outputs: Record<string, unknown> }>(
    'variable-update',
    (event) => {
      const v = event.payload
      variables.value = {
        ...(v.variables || {}),
        ...(v.step_outputs || {}),
      }
    }
  )

  unlistenRun = await safeListen<{ run_id: string; status: string }>(
    'run-update',
    (event) => {
      const { status } = event.payload
      if (status === 'completed' || status === 'failed' || status === 'cancelled') {
        debugState.value = 'idle'
        currentStepId.value = null
      }
    }
  )
}

watch(() => props.isRunning, (running) => {
  if (running) {
    startListening()
    debugState.value = 'running'
  } else {
    debugState.value = 'idle'
    currentStepId.value = null
    callStack.value = []
  }
}, { immediate: true })

onUnmounted(() => {
  unlistenBreakpoint?.()
  unlistenRun?.()
  unlistenVariable?.()
  if (refreshTimer) { clearInterval(refreshTimer); refreshTimer = null }
})

// ─── 格式化变量值 ───
function formatValue(v: unknown): string {
  if (v === null || v === undefined) return 'null'
  if (typeof v === 'string') return v.length > 100 ? v.slice(0, 100) + '…' : v
  if (typeof v === 'object') {
    const s = JSON.stringify(v, null, 2)
    return s.length > 200 ? s.slice(0, 200) + '…' : s
  }
  return String(v)
}
</script>

<template>
  <div class="flex flex-col h-full">
    <!-- Header -->
    <div class="flex items-center gap-2 px-3 py-2 border-b border-[var(--border-neutral-l1)] shrink-0">
      <Bug class="w-4 h-4 text-[var(--text-tertiary)]" />
      <span class="text-xs font-medium uppercase tracking-wide text-[var(--text-tertiary)]">{{ t('debug.title') }}</span>
      <Badge v-if="debugState === 'paused'" variant="outline" class="ml-auto text-[10px]" style="color:var(--status-alert-default);background:var(--status-alert-surface-l1);border-color:var(--status-alert-default)">
        {{ t('debug.paused') }}
      </Badge>
      <Badge v-else-if="debugState === 'running'" variant="outline" class="ml-auto text-[10px]" style="color:var(--status-success-default);background:var(--status-success-surface-l1);border-color:var(--status-success-default)">
        {{ t('debug.running') }}
      </Badge>
    </div>

    <!-- Controls -->
    <div class="flex items-center gap-1 px-3 py-2 border-b border-[var(--border-neutral-l1)] shrink-0">
      <Button variant="ghost" size="sm" :disabled="debugState !== 'paused'" @click="onStepOver" :title="t('debug.stepOver')">
        <SkipForward class="w-3.5 h-3.5" />
      </Button>
      <Button variant="ghost" size="sm" :disabled="debugState !== 'paused'" @click="onContinue" :title="t('debug.continue')">
        <Play class="w-3.5 h-3.5" />
      </Button>
      <Button variant="ghost" size="sm" :disabled="!isRunning" @click="onStop" :title="t('debug.stop')">
        <Square class="w-3.5 h-3.5" />
      </Button>
    </div>

    <ScrollArea class="flex-1 overflow-auto">
      <!-- Current Step -->
      <div v-if="currentStepId" class="px-3 py-2 border-b border-[var(--border-neutral-l1)]">
        <div class="text-[10px] uppercase tracking-wide text-[var(--text-tertiary)] mb-1">{{ t('debug.currentStep') }}</div>
        <div class="flex items-center gap-1">
          <span class="inline-block w-2 h-2 rounded-full bg-[var(--status-warning-default)] animate-pulse" />
          <span class="text-sm font-medium">{{ currentStepId }}</span>
        </div>
      </div>

      <!-- Breakpoints -->
      <div v-if="breakpoints.length > 0" class="px-3 py-2 border-b border-[var(--border-neutral-l1)]">
        <div class="text-[10px] uppercase tracking-wide text-[var(--text-tertiary)] mb-1">{{ t('debug.breakpoints') }}</div>
        <div
          v-for="bp in breakpoints"
          :key="bp.id"
          class="flex items-center gap-2 py-0.5 text-sm cursor-pointer hover:bg-[var(--bg-overlay-l2)]/50 rounded px-1 -mx-1"
        >
          <span class="inline-block w-2 h-2 rounded-full bg-[var(--status-error-default)]" />
          <span class="flex-1 truncate">{{ bp.label || bp.id }}</span>
          <Button
            variant="ghost" size="icon" class="h-5 w-5 opacity-40 hover:opacity-100"
            @click="emit('remove-breakpoint', bp.id)"
          ><X class="w-3 h-3" /></Button>
        </div>
      </div>

      <!-- Variables -->
      <div class="px-3 py-2">
        <div class="text-[10px] uppercase tracking-wide text-[var(--text-tertiary)] mb-1">{{ t('debug.variables') }}</div>
        <div v-if="variableGroups.length === 0" class="text-xs text-[var(--text-tertiary)]/50 py-2">
          {{ t('debug.noVariables') }}
        </div>
        <div v-for="group in variableGroups" :key="group.key" class="mb-1">
          <div
            class="flex items-center gap-1 py-0.5 text-xs text-[var(--text-tertiary)] cursor-pointer hover:text-[var(--text-default)]"
            @click="toggleGroup(group.key)"
          >
            <ChevronDown v-if="expandedGroups.has(group.key)" class="w-3 h-3" />
            <ChevronRight v-else class="w-3 h-3" />
            {{ group.label }} ({{ group.vars.length }})
          </div>
          <Transition name="collapse">
            <div v-if="expandedGroups.has(group.key)" class="pl-3">
              <div
                v-for="v in group.vars"
                :key="v.key"
                class="flex gap-1 py-0.5 text-xs font-mono"
              >
                <span class="text-[var(--text-brand)] shrink-0">{{ v.key }}</span>
                <span class="text-[var(--text-tertiary)]">=</span>
                <span class="text-[var(--text-default)] truncate">{{ formatValue(v.value) }}</span>
              </div>
            </div>
          </Transition>
        </div>
      </div>

      <!-- Call Stack -->
      <div v-if="callStack.length > 0" class="px-3 py-2 border-t border-[var(--border-neutral-l1)]">
        <div class="text-[10px] uppercase tracking-wide text-[var(--text-tertiary)] mb-1">{{ t('debug.callStack') }}</div>
        <div
          v-for="(step, i) in callStack"
          :key="i"
          class="flex items-center gap-1 py-0.5 text-xs font-mono text-[var(--text-tertiary)]"
        >
          <span class="w-3 text-right text-[10px]">{{ i }}</span>
          <span>{{ step }}</span>
        </div>
      </div>
    </ScrollArea>
  </div>
</template>
