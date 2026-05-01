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

      <!-- 浏览器截图 + 元素选择 -->
      <div v-else-if="previewType === 'browser'" class="preview-content">
        <div v-if="screenshotB64" class="screenshot-container" ref="screenshotContainer">
          <img
            :src="'data:image/png;base64,' + screenshotB64"
            class="preview-screenshot"
            @load="onScreenshotLoad"
            @click="onScreenshotClick"
            ref="screenshotImg"
          />
          <!-- 元素遮罩层 -->
          <div class="element-overlays" v-if="showOverlays">
            <div
              v-for="(el, i) in visibleElements"
              :key="i"
              class="element-overlay"
              :class="{ hovered: hoveredIdx === i, selected: selectedIdx === i }"
              :style="el.style"
              @mouseenter="hoveredIdx = i"
              @mouseleave="hoveredIdx = i === hoveredIdx ? -1 : hoveredIdx"
              @click.stop="selectElement(i)"
            ></div>
          </div>
        </div>
        <!-- 元素详情弹层 -->
        <div v-if="selectedIdx >= 0 && visibleElements[selectedIdx]" class="element-detail">
          <div class="detail-row"><b>{{ visibleElements[selectedIdx].tag }}</b> {{ visibleElements[selectedIdx].type }}</div>
          <div class="detail-row detail-text">{{ visibleElements[selectedIdx].text || '(无文本)' }}</div>
          <div class="detail-row detail-selector">{{ visibleElements[selectedIdx].selector }}</div>
          <button class="btn-use" @click="useElement">📌 使用此元素</button>
        </div>
        <div v-if="pageInfo" class="page-info">
          <span>{{ pageInfo.title }}</span>
          <span class="page-url">{{ pageInfo.url }}</span>
        </div>
      </div>

      <!-- Excel 表格交互预览 -->
      <div v-else-if="previewType === 'excel'" class="preview-content">
        <div v-if="sheetName" class="sheet-tabs">
          <span class="sheet-tab active">{{ sheetName }}</span>
        </div>
        <div class="table-wrap">
          <table class="excel-table" @mouseup="onExcelMouseUp">
            <tbody>
              <tr v-for="(row, ri) in excelRows" :key="ri">
                <td
                  v-for="(cell, ci) in row"
                  :key="ci"
                  :class="{
                    header: ri === 0,
                    selected: isCellSelected(ri, ci),
                    'range-start': ri === selStart[0] && ci === selStart[1],
                    'range-end': ri === selEnd[0] && ci === selEnd[1],
                  }"
                  @mousedown="onCellMouseDown(ri, ci, $event)"
                >{{ cell ?? '' }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div class="table-info">
          {{ excelRows.length }} 行 × {{ excelCols }} 列
          <span v-if="selectionText" class="selection-info"> | 已选: {{ selectionText }}</span>
        </div>
        <button v-if="selectionText" class="btn-use" @click="useExcelRange">📌 填入范围</button>
      </div>

      <!-- Word 文档预览 -->
      <div v-else-if="previewType === 'word'" class="preview-content">
        <div class="word-doc">
          <p
            v-for="(para, i) in wordParagraphs"
            :key="i"
            :class="['word-para', { selected: selectedWordPara === i }]"
            @click="selectedWordPara = i"
          >
            {{ para }}
          </p>
        </div>
        <div class="table-info">{{ wordParagraphs.length }} 段</div>
        <button v-if="selectedWordPara >= 0" class="btn-use" @click="useWordText">📌 使用选中文本</button>
      </div>

      <!-- 不支持的节点 -->
      <div v-else class="preview-empty">
        <span>此节点类型暂不支持预览</span>
      </div>
    </template>
  </aside>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { toRaw } from 'vue'
import { safeInvoke, safeListen } from '../../utils/tauri'
import type { LGraphNode, IBaseWidget } from '@comfyorg/litegraph'

// ═══ Props ═══
const props = defineProps<{
  lgNode: LGraphNode | null
}>()

const emit = defineEmits<{
  (e: 'update-widget', node: LGraphNode, widgetName: string, value: string): void
}>()

// ═══ State ═══
const loading = ref(false)
const error = ref('')
const previewType = ref<'browser' | 'excel' | 'word' | null>(null)

// Browser state
const screenshotB64 = ref('')
const pageInfo = ref<{ title: string; url: string } | null>(null)
const elements = ref<any[]>([])
const screenshotImg = ref<HTMLImageElement | null>(null)
const screenshotContainer = ref<HTMLDivElement | null>(null)
const imgNaturalSize = ref({ w: 1280, h: 720 })
const imgDisplaySize = ref({ w: 0, h: 0 })
const hoveredIdx = ref(-1)
const selectedIdx = ref(-1)

// Excel state
const excelRows = ref<(string | number | null)[][]>([])
const excelCols = ref(0)
const sheetName = ref('')
const selStart = ref<[number, number]>([-1, -1])
const selEnd = ref<[number, number]>([-1, -1])

// Word state
const wordParagraphs = ref<string[]>([])
const selectedWordPara = ref(-1)

// ═══ Helpers ═══
function getWidgetValue(node: LGraphNode, name: string): string | undefined {
  const raw = toRaw(node)
  const widgets: IBaseWidget[] = raw.widgets || []
  const w = widgets.find((w: IBaseWidget) => w.name === name)
  return w?.value as string | undefined
}

function setWidgetValue(name: string, value: string) {
  if (!props.lgNode) return
  emit('update-widget', props.lgNode, name, value)
}

function detectType(node: LGraphNode): 'browser' | 'excel' | 'word' | null {
  const t = node.type
  if (!t) return null
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

const showOverlays = computed(() => elements.value.length > 0 && imgDisplaySize.value.w > 0)

// Filter to interactive + container elements, limit to top 80
const visibleElements = computed(() => {
  const scaleX = imgDisplaySize.value.w / imgNaturalSize.value.w
  const scaleY = imgDisplaySize.value.h / imgNaturalSize.value.h
  return elements.value
    .filter((el: any) => el.type !== 'text' || el.child_count > 0)
    .slice(0, 80)
    .map((el: any) => ({
      ...el,
      style: {
        left: `${el.bbox.x * scaleX}px`,
        top: `${el.bbox.y * scaleY}px`,
        width: `${el.bbox.w * scaleX}px`,
        height: `${el.bbox.h * scaleY}px`,
      },
    }))
})

// ═══ Browser interactive ═══
function onScreenshotLoad() {
  if (!screenshotImg.value) return
  imgNaturalSize.value = {
    w: screenshotImg.value.naturalWidth,
    h: screenshotImg.value.naturalHeight,
  }
  imgDisplaySize.value = {
    w: screenshotImg.value.clientWidth,
    h: screenshotImg.value.clientHeight,
  }
}

function onScreenshotClick(e: MouseEvent) {
  // Deselect if clicking background
  if (e.target === screenshotImg.value) {
    selectedIdx.value = -1
  }
}

function selectElement(i: number) {
  selectedIdx.value = i
}

// ═══ Excel interactive ═══
function isCellSelected(r: number, c: number): boolean {
  if (selStart.value[0] < 0 || selEnd.value[0] < 0) return false
  const rMin = Math.min(selStart.value[0], selEnd.value[0])
  const rMax = Math.max(selStart.value[0], selEnd.value[0])
  const cMin = Math.min(selStart.value[1], selEnd.value[1])
  const cMax = Math.max(selStart.value[1], selEnd.value[1])
  return r >= rMin && r <= rMax && c >= cMin && c <= cMax
}

function onCellMouseDown(r: number, c: number, e: MouseEvent) {
  selStart.value = [r, c]
  selEnd.value = [r, c]
}

function onExcelMouseUp(e: MouseEvent) {
  // range already set by mousedown, keep the end at the last clicked cell
  const target = e.target as HTMLElement
  if (target.tagName !== 'TD') return
  const tr = target.parentElement
  if (!tr) return
  const ri = Array.from(tr.parentElement!.children).indexOf(tr)
  const ci = Array.from(tr.children).indexOf(target)
  selEnd.value = [ri, ci]
}

const selectionText = computed(() => {
  if (selStart.value[0] < 0 || selEnd.value[0] < 0) return ''
  const rMin = Math.min(selStart.value[0], selEnd.value[0])
  const rMax = Math.max(selStart.value[0], selEnd.value[0])
  const cMin = Math.min(selStart.value[1], selEnd.value[1])
  const cMax = Math.max(selStart.value[1], selEnd.value[1])
  const colToLetter = (c: number) => {
    let s = ''
    c++
    while (c > 0) { c--; s = String.fromCharCode(65 + (c % 26)) + s; c = Math.floor(c / 26) }
    return s
  }
  return `${colToLetter(cMin)}${rMin + 1}:${colToLetter(cMax)}${rMax + 1}`
})

// ═══ Actions: fill back to node ═══
function useElement() {
  const el = visibleElements.value[selectedIdx.value]
  if (!el) return
  // Try to fill the most relevant widget
  setWidgetValue('selector', el.selector)
}

function useExcelRange() {
  setWidgetValue('range', selectionText.value)
}

function useWordText() {
  const text = wordParagraphs.value[selectedWordPara.value]
  if (text) {
    setWidgetValue('text', text)
  }
}

// ═══ Preview loaders ═══
async function loadBrowserPreview(node: LGraphNode) {
  const url = getWidgetValue(node, 'url')
  if (!url) {
    error.value = '节点未设置 URL'
    return
  }

  const result: any = await safeInvoke('web_scrape_preview', {
    url,
    headless: true,
    viewportWidth: 1280,
    viewportHeight: 720,
  })

  // Sidecar returns { success, data: { screenshot, elements, ... } }
  const d = result?.data || result
  if (!d || d.error) {
    error.value = d?.error || '预览失败'
    return
  }

  screenshotB64.value = d.screenshot || ''
  pageInfo.value = d.title ? { title: d.title, url: d.url || url } : null
  elements.value = d.elements || []
  imgNaturalSize.value = d.viewport ? { w: d.viewport.width, h: d.viewport.height } : { w: 1280, h: 720 }
}

async function loadExcelPreview(node: LGraphNode) {
  const path = getWidgetValue(node, 'path')
  if (!path) {
    error.value = '节点未设置文件路径'
    return
  }

  const sheet = getWidgetValue(node, 'sheet') || undefined
  const result: any = await safeInvoke('preview_excel', { path, sheet })

  if (result.error) {
    error.value = result.error
    return
  }

  sheetName.value = result.sheet || ''
  excelRows.value = result.data || []
  excelCols.value = result.cols || 0
  selStart.value = [-1, -1]
  selEnd.value = [-1, -1]
}

async function loadWordPreview(node: LGraphNode) {
  const path = getWidgetValue(node, 'path')
  if (!path) {
    error.value = '节点未设置文件路径'
    return
  }

  const result: any = await safeInvoke('preview_word', { path })

  if (result.error) {
    error.value = result.error
    return
  }

  wordParagraphs.value = result.paragraphs || []
  selectedWordPara.value = -1
}

async function loadPreview() {
  const node = props.lgNode
  if (!node) return

  const type = detectType(node)
  previewType.value = type
  if (!type) return

  loading.value = true
  error.value = ''
  selectedIdx.value = -1
  hoveredIdx.value = -1

  try {
    if (type === 'browser') await loadBrowserPreview(node)
    else if (type === 'excel') await loadExcelPreview(node)
    else if (type === 'word') await loadWordPreview(node)
  } catch (e: any) {
    error.value = typeof e === 'string' ? e : (e?.message || '预览加载失败（需要 Tauri 运行时）')
  } finally {
    loading.value = false
  }
}

function refresh() { loadPreview() }

// ═══ Watch node changes ═══
watch(() => props.lgNode, () => {
  screenshotB64.value = ''
  pageInfo.value = null
  elements.value = []
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
.btn-refresh:hover { background: #21262d; color: #c9d1d9; }

.preview-loading,
.preview-error,
.preview-empty {
  padding: 24px 16px;
  text-align: center;
  color: #8b949e;
  font-size: 13px;
}
.preview-error { color: #f85149; }

.preview-content {
  flex: 1;
  overflow: auto;
  padding: 8px;
}

/* ── Browser ── */
.screenshot-container {
  position: relative;
  display: inline-block;
  cursor: crosshair;
}
.preview-screenshot {
  display: block;
  width: 100%;
  border-radius: 4px;
  border: 1px solid #30363d;
}

.element-overlays {
  position: absolute;
  top: 0; left: 0;
  width: 100%; height: 100%;
  pointer-events: none;
}
.element-overlay {
  position: absolute;
  border: 1px solid transparent;
  pointer-events: auto;
  cursor: pointer;
  transition: border-color 0.1s, background 0.1s;
}
.element-overlay:hover,
.element-overlay.hovered {
  border-color: #58a6ff;
  background: rgba(88, 166, 255, 0.08);
}
.element-overlay.selected {
  border-color: #3fb950;
  background: rgba(63, 185, 80, 0.15);
}

.element-detail {
  margin-top: 8px;
  padding: 8px 10px;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 6px;
  font-size: 11px;
}
.detail-row {
  margin-bottom: 4px;
  color: #c9d1d9;
}
.detail-row b { color: #f0883e; }
.detail-text { color: #8b949e; max-height: 40px; overflow: hidden; }
.detail-selector {
  font-family: monospace;
  font-size: 10px;
  color: #58a6ff;
  word-break: break-all;
}

.btn-use {
  margin-top: 8px;
  padding: 4px 12px;
  background: #1f6feb;
  color: #fff;
  border: none;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
  width: 100%;
}
.btn-use:hover { background: #388bfd; }

.page-info {
  display: flex;
  flex-direction: column;
  margin-top: 8px;
  font-size: 12px;
  color: #8b949e;
}
.page-url { color: #58a6ff; word-break: break-all; }

/* ── Excel ── */
.sheet-tabs { display: flex; margin-bottom: 6px; }
.sheet-tab {
  padding: 3px 10px;
  background: #1f6feb;
  color: #fff;
  border-radius: 4px;
  font-size: 12px;
}

.table-wrap {
  overflow: auto;
  max-height: 320px;
  border: 1px solid #30363d;
  border-radius: 6px;
}

.excel-table {
  border-collapse: collapse;
  font-size: 11px;
  width: max-content;
  min-width: 100%;
  user-select: none;
}
.excel-table td {
  padding: 2px 8px;
  border: 1px solid #21262d;
  color: #c9d1d9;
  white-space: nowrap;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  cursor: cell;
}
.excel-table td.header {
  background: #21262d;
  font-weight: 600;
  color: #e6edf3;
  cursor: pointer;
}
.excel-table td.selected {
  background: rgba(31, 111, 235, 0.2);
}
.excel-table td.range-start {
  outline: 2px solid #58a6ff;
  outline-offset: -2px;
  z-index: 1;
  position: relative;
}
.excel-table td.range-end {
  outline: 2px solid #58a6ff;
  outline-offset: -2px;
}

.table-info {
  margin-top: 6px;
  font-size: 11px;
  color: #484f58;
}
.selection-info {
  color: #58a6ff;
  font-weight: 600;
}

/* ── Word ── */
.word-doc {
  padding: 10px;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 6px;
  max-height: 320px;
  overflow: auto;
}
.word-para {
  margin: 0 0 6px 0;
  padding: 4px 6px;
  font-size: 12px;
  line-height: 1.5;
  color: #c9d1d9;
  cursor: pointer;
  border-radius: 3px;
  transition: background 0.1s;
}
.word-para:hover { background: rgba(88, 166, 255, 0.08); }
.word-para.selected {
  background: rgba(31, 111, 235, 0.2);
  border-left: 3px solid #58a6ff;
  padding-left: 9px;
}
.word-para:last-child { margin-bottom: 0; }
</style>
