<template>
  <aside class="preview-panel">
    <div v-if="!lgNode" class="panel-empty">
      <span>选中文件节点预览内容</span>
    </div>

    <template v-else>
      <!-- 面板标题 -->
      <div class="panel-header">
        <span class="panel-title">{{ previewTitle }}</span>
        <button class="btn-refresh" title="刷新预览" @click="refresh">🔄</button>
      </div>

      <!-- 加载中 -->
      <div v-if="loading" class="preview-loading">⏳ 加载预览...</div>

      <!-- 错误 -->
      <div v-else-if="error" class="preview-error">{{ error }}</div>

      <!-- 浏览器截图预览 -->
      <div v-else-if="previewType === 'browser'" class="preview-content">
        <img
          v-if="screenshotB64"
          :src="'data:image/png;base64,' + screenshotB64"
          class="preview-screenshot"
          @click="toggleZoom"
          :class="{ zoomed }"
        />
        <div v-if="pageInfo" class="page-info">
          <span>{{ pageInfo.title }}</span>
          <span class="page-url">{{ pageInfo.url }}</span>
        </div>
      </div>

      <!-- Excel 表格预览 -->
      <div v-else-if="previewType === 'excel'" class="preview-content">
        <div v-if="sheetName" class="sheet-tabs">
          <span class="sheet-tab active">{{ sheetName }}</span>
        </div>
        <div class="table-wrap">
          <table class="excel-table">
            <tbody>
              <tr v-for="(row, ri) in excelRows" :key="ri">
                <td
                  v-for="(cell, ci) in row"
                  :key="ci"
                  :class="{ header: ri === 0 }"
                >{{ cell ?? '' }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div class="table-info">{{ excelRows.length }} 行 × {{ excelCols }} 列</div>
      </div>

      <!-- Word 文档预览 -->
      <div v-else-if="previewType === 'word'" class="preview-content">
        <div class="word-doc">
          <p v-for="(para, i) in wordParagraphs" :key="i" class="word-para">
            {{ para }}
          </p>
        </div>
        <div class="table-info">{{ wordParagraphs.length }} 段</div>
      </div>

      <!-- 不支持的节点类型 -->
      <div v-else class="preview-empty">
        <span>此节点类型暂不支持预览</span>
        <span class="preview-hint">支持：浏览器页面 / Excel 表格 / Word 文档</span>
      </div>
    </template>
  </aside>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { toRaw } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { LGraphNode, IBaseWidget } from '@comfyorg/litegraph'

// ═══ Props ═══
const props = defineProps<{
  lgNode: LGraphNode | null
}>()

// ═══ State ═══
const loading = ref(false)
const error = ref('')
const previewType = ref<'browser' | 'excel' | 'word' | null>(null)

// Browser state
const screenshotB64 = ref('')
const pageInfo = ref<{ title: string; url: string } | null>(null)
const zoomed = ref(false)

// Excel state
const excelRows = ref<(string | number | null)[][]>([])
const excelCols = ref(0)
const sheetName = ref('')

// Word state
const wordParagraphs = ref<string[]>([])

// ═══ Helpers ═══
function getWidgetValue(node: LGraphNode, name: string): string | undefined {
  const raw = toRaw(node)
  const widgets: IBaseWidget[] = raw.widgets || []
  const w = widgets.find((w: IBaseWidget) => w.name === name)
  return w?.value as string | undefined
}

function detectType(node: LGraphNode): 'browser' | 'excel' | 'word' | null {
  const t = node.type
  if (!t) return null

  // Browser automation nodes
  const browserTypes = ['navigate', 'screenshot', 'extract', 'click', 'fill', 'evaluate', 'scroll', 'wait', 'pdf', 'browser']
  if (browserTypes.includes(t)) return 'browser'

  if (t === 'excel') return 'excel'
  if (t === 'word') return 'word'

  return null
}

const previewTitle = computed(() => {
  if (!props.lgNode) return '预览'
  const t = detectType(props.lgNode)
  if (t === 'browser') return '🌐 页面预览'
  if (t === 'excel') return '📊 表格预览'
  if (t === 'word') return '📄 文档预览'
  return '预览'
})

// ═══ Preview loaders ═══
async function loadBrowserPreview(node: LGraphNode) {
  const url = getWidgetValue(node, 'url')
  if (!url) {
    error.value = '节点未设置 URL'
    return
  }

  const result: any = await invoke('web_scrape_preview', {
    url,
    headless: true,
    viewportWidth: 1280,
    viewportHeight: 720,
  })

  if (result.error) {
    error.value = result.error
    return
  }

  screenshotB64.value = result.screenshot || ''
  pageInfo.value = result.title ? { title: result.title, url: result.url || url } : null
}

async function loadExcelPreview(node: LGraphNode) {
  const path = getWidgetValue(node, 'path')
  if (!path) {
    error.value = '节点未设置文件路径'
    return
  }

  const sheet = getWidgetValue(node, 'sheet') || undefined
  const result: any = await invoke('preview_excel', { path, sheet })

  if (result.error) {
    error.value = result.error
    return
  }

  sheetName.value = result.sheet || ''
  excelRows.value = result.data || []
  excelCols.value = result.cols || 0
}

async function loadWordPreview(node: LGraphNode) {
  const path = getWidgetValue(node, 'path')
  if (!path) {
    error.value = '节点未设置文件路径'
    return
  }

  const result: any = await invoke('preview_word', { path })

  if (result.error) {
    error.value = result.error
    return
  }

  wordParagraphs.value = result.paragraphs || []
}

async function loadPreview() {
  const node = props.lgNode
  if (!node) return

  const type = detectType(node)
  previewType.value = type

  if (!type) return

  loading.value = true
  error.value = ''

  try {
    if (type === 'browser') await loadBrowserPreview(node)
    else if (type === 'excel') await loadExcelPreview(node)
    else if (type === 'word') await loadWordPreview(node)
  } catch (e: any) {
    // In dev mode (no Tauri), show friendly message
    error.value = typeof e === 'string' ? e : (e?.message || '预览加载失败（需要 Tauri 运行时）')
  } finally {
    loading.value = false
  }
}

function refresh() {
  loadPreview()
}

function toggleZoom() {
  zoomed.value = !zoomed.value
}

// ═══ Watch node changes ═══
watch(() => props.lgNode, () => {
  screenshotB64.value = ''
  pageInfo.value = null
  excelRows.value = []
  excelCols.value = 0
  sheetName.value = ''
  wordParagraphs.value = []
  error.value = ''

  loadPreview()
}, { immediate: true })
</script>

<style scoped>
.preview-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #161b22;
  overflow: hidden;
}

.panel-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #8b949e;
  font-size: 13px;
  padding: 16px;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border-bottom: 1px solid #30363d;
  flex-shrink: 0;
}

.panel-title {
  font-size: 13px;
  font-weight: 600;
  color: #c9d1d9;
}

.btn-refresh {
  background: none;
  border: 1px solid #30363d;
  color: #8b949e;
  border-radius: 4px;
  padding: 2px 6px;
  cursor: pointer;
  font-size: 12px;
}
.btn-refresh:hover {
  background: #21262d;
  color: #c9d1d9;
}

.preview-loading,
.preview-error,
.preview-empty {
  padding: 24px 16px;
  text-align: center;
  color: #8b949e;
  font-size: 13px;
}

.preview-error {
  color: #f85149;
}

.preview-hint {
  display: block;
  margin-top: 8px;
  font-size: 11px;
  color: #484f58;
}

.preview-content {
  flex: 1;
  overflow: auto;
  padding: 8px;
}

/* ── Browser ── */
.preview-screenshot {
  width: 100%;
  border-radius: 6px;
  border: 1px solid #30363d;
  cursor: zoom-in;
  transition: transform 0.2s;
}
.preview-screenshot.zoomed {
  cursor: zoom-out;
  transform: scale(1.6);
  transform-origin: top left;
}

.page-info {
  display: flex;
  flex-direction: column;
  margin-top: 8px;
  font-size: 12px;
  color: #8b949e;
}
.page-url {
  color: #58a6ff;
  word-break: break-all;
}

/* ── Excel ── */
.sheet-tabs {
  display: flex;
  margin-bottom: 6px;
}
.sheet-tab {
  padding: 3px 10px;
  background: #1f6feb;
  color: #fff;
  border-radius: 4px;
  font-size: 12px;
}

.table-wrap {
  overflow: auto;
  max-height: 400px;
  border: 1px solid #30363d;
  border-radius: 6px;
}

.excel-table {
  border-collapse: collapse;
  font-size: 12px;
  width: max-content;
  min-width: 100%;
}
.excel-table td {
  padding: 3px 10px;
  border: 1px solid #21262d;
  color: #c9d1d9;
  white-space: nowrap;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
}
.excel-table td.header {
  background: #21262d;
  font-weight: 600;
  color: #e6edf3;
}

.table-info {
  margin-top: 6px;
  font-size: 11px;
  color: #484f58;
}

/* ── Word ── */
.word-doc {
  padding: 12px;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 6px;
}
.word-para {
  margin: 0 0 8px 0;
  font-size: 13px;
  line-height: 1.6;
  color: #c9d1d9;
}
.word-para:last-child {
  margin-bottom: 0;
}
</style>
