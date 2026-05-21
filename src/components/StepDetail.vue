<script setup lang="ts">
import { ref, watch } from 'vue'
import { safeInvoke } from '../utils/tauri'
import BundleViewer from './BundleViewer.vue'

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
  step: StepPreview
  runId: string
}>()

const bundleFiles = ref<string[]>([])
const selectedFile = ref<string | null>(null)
const fileContent = ref<string | null>(null)
const loadingBundle = ref(false)
const loadingFile = ref(false)

async function loadBundleFiles() {
  if (!props.step.bundle_path) return
  loadingBundle.value = true
  try {
    bundleFiles.value = await safeInvoke<string[]>('get_bundle_files', {
      runId: props.runId,
      stepId: props.step.step_id,
    }) || []
  } catch (e: any) {
    console.warn('加载 bundle 文件列表失败:', e)
  } finally {
    loadingBundle.value = false
  }
}

async function loadFile(filename: string) {
  selectedFile.value = filename
  loadingFile.value = true
  try {
    fileContent.value = await safeInvoke<string>('read_bundle_file', {
      runId: props.runId,
      stepId: props.step.step_id,
      filename,
    }) || ''
  } catch (e: any) {
    fileContent.value = `读取失败: ${e.message || e}`
  } finally {
    loadingFile.value = false
  }
}

function formatMs(ms: number): string {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

watch(() => props.step.bundle_path, loadBundleFiles, { immediate: true })
</script>

<template>
  <div class="space-y-3 text-xs">
    <!-- Step info header -->
    <div class="space-y-1">
      <div class="flex items-center gap-2">
        <span class="font-semibold text-foreground">{{ step.step_name }}</span>
        <span class="text-muted-foreground/50">({{ step.step_id }})</span>
      </div>
      <div class="flex gap-3 text-muted-foreground">
        <span>{{ step.step_type }}</span>
        <span>{{ step.status }}</span>
        <span v-if="step.duration_ms > 0">{{ formatMs(step.duration_ms) }}</span>
      </div>
      <div class="text-muted-foreground">{{ step.summary }}</div>
    </div>

    <!-- Detail JSON -->
    <details class="cursor-pointer">
      <summary class="text-muted-foreground/70 hover:text-muted-foreground">详情 JSON</summary>
      <pre class="mt-1 p-2 bg-secondary/30 rounded text-[10px] font-mono max-h-[200px] overflow-auto">{{ JSON.stringify(step.detail, null, 2) }}</pre>
    </details>

    <!-- Bundle files -->
    <div v-if="step.bundle_path" class="space-y-2">
      <div class="text-muted-foreground/70 font-medium">Bundle 快照</div>

      <div v-if="loadingBundle" class="flex items-center gap-1.5 text-muted-foreground/50">
        <div class="w-3 h-3 border-[1.5px] border-border border-t-primary rounded-full animate-spin" />
      </div>

      <div v-else-if="bundleFiles.length === 0" class="text-muted-foreground/50">
        无文件
      </div>

      <div v-else class="space-y-1">
        <button
          v-for="file in bundleFiles"
          :key="file"
          class="block w-full text-left px-2 py-1 rounded text-[10px] font-mono transition-colors"
          :class="selectedFile === file ? 'bg-primary/10 text-primary' : 'hover:bg-secondary/50 text-muted-foreground'"
          @click="loadFile(file)"
        >
          {{ file }}
        </button>
      </div>

      <!-- File content viewer -->
      <div v-if="selectedFile && !loadingFile" class="border border-border rounded-md overflow-hidden">
        <div class="px-2 py-1 bg-secondary/30 text-[10px] text-muted-foreground border-b border-border">
          {{ selectedFile }}
        </div>
        <BundleViewer :content="fileContent || ''" :filename="selectedFile" />
      </div>
      <div v-else-if="loadingFile" class="flex items-center gap-1.5 text-muted-foreground/50 text-[10px] px-2">
        <div class="w-3 h-3 border-[1.5px] border-border border-t-primary rounded-full animate-spin" />
        加载中...
      </div>
    </div>

    <div v-else class="text-muted-foreground/50">
      无 Bundle 快照
    </div>
  </div>
</template>
