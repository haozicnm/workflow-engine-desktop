<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { safeInvoke, safeListen } from '../utils/tauri'
import type { UnlistenFn } from '@tauri-apps/api/event'

const vars = ref<Record<string, unknown>>({})
const stepOutputs = ref<Record<string, unknown>>({})
const loading = ref(false)

let unlisten: UnlistenFn | null = null

onMounted(async () => {
  unlisten = await safeListen<{ variables: Record<string, unknown>; step_outputs: Record<string, unknown> }>(
    'variable-update',
    (event) => {
      vars.value = event.payload.variables || {}
      stepOutputs.value = event.payload.step_outputs || {}
    }
  )
  await refresh()
})

onUnmounted(() => {
  unlisten?.()
})

async function refresh() {
  loading.value = true
  try {
    const data = await safeInvoke<{ variables: Record<string, unknown>; step_outputs: Record<string, unknown> }>('debug_vars', { runId: '' })
    if (data) {
      vars.value = data.variables || {}
      stepOutputs.value = data.step_outputs || {}
    }
  } catch {
    // 无活跃运行
  } finally {
    loading.value = false
  }
}

function formatValue(v: unknown): string {
  if (v === null || v === undefined) return '—'
  if (typeof v === 'string') return v.length > 100 ? v.slice(0, 100) + '...' : v
  try {
    const s = JSON.stringify(v, null, 2)
    return s.length > 200 ? s.slice(0, 200) + '...' : s
  } catch {
    return String(v)
  }
}

const hasData = () => Object.keys(vars.value).length > 0 || Object.keys(stepOutputs.value).length > 0
</script>

<template>
  <div class="debug-panel">
    <div class="dp-header">
      <span>🔍 变量查看</span>
      <button class="dp-refresh" @click="refresh" :disabled="loading">🔄</button>
    </div>

    <div v-if="!hasData()" class="dp-empty">
      运行工作流后查看变量和步骤输出
    </div>

    <div v-else class="dp-body">
      <!-- 上下文变量 -->
      <div v-if="Object.keys(vars).length > 0" class="dp-section">
        <div class="dp-section-title">📦 变量</div>
        <div v-for="(val, key) in vars" :key="key" class="dp-item">
          <span class="dp-key">{{ key }}</span>
          <pre class="dp-val">{{ formatValue(val) }}</pre>
        </div>
      </div>

      <!-- 步骤输出 -->
      <div v-if="Object.keys(stepOutputs).length > 0" class="dp-section">
        <div class="dp-section-title">📤 步骤输出</div>
        <div v-for="(val, key) in stepOutputs" :key="key" class="dp-item">
          <span class="dp-key">{{ key }}</span>
          <pre class="dp-val">{{ formatValue(val) }}</pre>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.debug-panel { padding: 16px; height: 100%; overflow-y: auto; }
.dp-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; color: #e1e4e8; font-weight: 600; }
.dp-refresh { background: #21262d; border: 1px solid #30363d; color: #c9d1d9; border-radius: 4px; cursor: pointer; padding: 2px 6px; font-size: 12px; }
.dp-refresh:hover { background: #30363d; }
.dp-empty { color: #484f58; font-size: 13px; text-align: center; padding: 40px 0; }
.dp-body { display: flex; flex-direction: column; gap: 16px; }
.dp-section-title { font-size: 11px; color: #6e7681; text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 6px; }
.dp-item { margin-bottom: 8px; }
.dp-key { font-size: 12px; font-weight: 600; color: #58a6ff; display: block; margin-bottom: 2px; font-family: 'Cascadia Code', monospace; }
.dp-val { font-size: 11px; color: #8b949e; background: #0d1117; padding: 6px 8px; border-radius: 4px; margin: 0; overflow-x: auto; font-family: 'Cascadia Code', monospace; max-height: 100px; overflow-y: auto; white-space: pre-wrap; word-break: break-all; }
</style>
