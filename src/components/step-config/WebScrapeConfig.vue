<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { WorkflowStep } from '../../types/workflow'
import { getNodeFields } from '../../config/node-fields'
import ConfigFieldRenderer from './ConfigFieldRenderer.vue'

const props = defineProps<{ allSteps?: WorkflowStep[] }>()
const config = defineModel<Record<string, unknown>>('config', { required: true })

// ─── Existing config fields ───
const fields = computed(() => {
  const allFields = getNodeFields(props.allSteps || [])
  return (allFields['web_scrape'] || []).filter(f => !f.show || f.show(config.value))
})

// ─── Preview state ───
const previewLoading = ref(false)
const previewError = ref('')
const screenshot = ref('')
const previewElements = ref<any[]>([])
const previewUrl = ref('')
const previewTitle = ref('')
const viewport = ref({ width: 0, height: 0 })

// Selected elements in preview
const selectedElement = ref<any>(null)
const hoveredElement = ref<any>(null)

// Image container ref for coordinate calculations
const imgContainer = ref<HTMLElement | null>(null)
const imgNaturalSize = ref({ w: 0, h: 0 })
const scale = ref(1)

// ─── Extract rules management ───
interface ExtractField {
  name: string
  selector: string
}

interface ExtractRule {
  selector: string
  fields: ExtractField[]
  previewTexts?: string[]
}

const extractRules = ref<ExtractRule[]>([])

// Parse existing extract config
watch(() => config.value.extract, (val) => {
  if (Array.isArray(val)) {
    extractRules.value = (val as any[]).map((rule: any) => ({
      selector: rule.selector || '',
      fields: (rule.fields && typeof rule.fields === 'object'
        ? Object.entries(rule.fields as Record<string, unknown>).map(([name, selector]) => ({
            name,
            selector: String(selector || ''),
          }))
        : []),
    }))
  }
}, { immediate: true, deep: true })

// Sync extract rules back to config
function syncExtractToConfig() {
  const rules = extractRules.value
    .filter(r => r.selector)
    .map(r => {
      const fieldsObj: Record<string, string> = {}
      r.fields.forEach(f => { if (f.name && f.selector) fieldsObj[f.name] = f.selector })
      return {
        selector: r.selector,
        ...(Object.keys(fieldsObj).length > 0 ? { fields: fieldsObj } : {}),
      }
    })
  config.value.extract = rules
}

// ─── Preview actions ───
async function loadPreview() {
  const url = String(config.value.url || '').trim()
  if (!url) {
    previewError.value = '请先输入目标 URL'
    return
  }

  previewLoading.value = true
  previewError.value = ''
  screenshot.value = ''
  previewElements.value = []
  selectedElement.value = null

  try {
    const result = await invoke<any>('web_scrape_preview', {
      url,
      headless: config.value.headless !== false,
    })

    if (result && result.screenshot) {
      screenshot.value = result.screenshot
      previewElements.value = result.elements || []
      previewUrl.value = result.url || url
      previewTitle.value = result.title || ''
      viewport.value = {
        width: result.viewport?.width || 1280,
        height: result.viewport?.height || 720,
      }
    } else {
      previewError.value = '预览返回数据为空'
    }
  } catch (err: any) {
    const msg = String(err).includes('预览需要浏览器环境')
      ? '预览需要浏览器环境（Playwright/Python 未安装）'
      : String(err)
    previewError.value = msg
  } finally {
    previewLoading.value = false
  }
}

// ─── Image scale calculation ───
function updateScale() {
  if (!imgContainer.value) return
  const img = imgContainer.value.querySelector('img')
  if (!img) return
  const displayedW = img.clientWidth
  const naturalW = imgNaturalSize.value.w || viewport.value.width
  if (naturalW > 0 && displayedW > 0) {
    scale.value = displayedW / naturalW
  }
}

function onImgLoad(e: Event) {
  const img = e.target as HTMLImageElement
  imgNaturalSize.value = { w: img.naturalWidth, h: img.naturalHeight }
  nextTick(updateScale)
}

// ─── Element click/selection in preview ───
function onElementClick(el: any) {
  selectedElement.value = el
}

function onElementHover(el: any | null) {
  hoveredElement.value = el
}

// Get scaled style for overlay
function overlayStyle(el: any) {
  const s = scale.value
  return {
    left: `${el.bbox.x * s}px`,
    top: `${el.bbox.y * s}px`,
    width: `${el.bbox.w * s}px`,
    height: `${el.bbox.h * s}px`,
  }
}

function overlayClass(el: any) {
  const cls = ['preview-overlay']
  if (selectedElement.value?.selector === el.selector) cls.push('selected')
  else if (hoveredElement.value?.selector === el.selector) cls.push('hovered')
  if (el.type === 'container') cls.push('type-container')
  else if (el.type === 'interactive') cls.push('type-interactive')
  else cls.push('type-text')
  return cls
}

// ─── Rule management ───
function setContainerSelector() {
  if (!selectedElement.value) return
  const sel = selectedElement.value.selector
  // Find or create the first rule
  if (extractRules.value.length === 0) {
    extractRules.value.push({ selector: sel, fields: [] })
  } else {
    extractRules.value[0].selector = sel
  }
  syncExtractToConfig()
}

function addFieldFromSelected() {
  if (!selectedElement.value) return
  if (extractRules.value.length === 0) return

  const sel = selectedElement.value.selector
  const rule = extractRules.value[0]

  // Auto-generate field name from tag + text
  const text = (selectedElement.value.text || '').substring(0, 20).replace(/\s+/g, '_').replace(/[^\w\u4e00-\u9fff]/g, '') || 'field'
  const name = text || `${selectedElement.value.tag}_${rule.fields.length + 1}`

  // Don't add duplicates
  if (rule.fields.some(f => f.selector === sel)) return

  rule.fields.push({ name, selector: sel })
  syncExtractToConfig()
}

function removeField(idx: number) {
  if (extractRules.value.length === 0) return
  extractRules.value[0].fields.splice(idx, 1)
  syncExtractToConfig()
}

function removeContainerRule() {
  if (extractRules.value.length === 0) return
  extractRules.value[0].selector = ''
  extractRules.value[0].fields = []
  syncExtractToConfig()
}

function updateFieldName(idx: number, name: string) {
  if (extractRules.value.length === 0) return
  extractRules.value[0].fields[idx].name = name
  syncExtractToConfig()
}

// ─── Preview extraction results ───
const extractionPreview = ref<any[]>([])
const extractionLoading = ref(false)

async function previewExtraction() {
  if (extractRules.value.length === 0 || !extractRules.value[0].selector) return
  extractionLoading.value = true
  try {
    // Use eval on existing elements data to simulate extraction
    const rule = extractRules.value[0]
    const containerSel = rule.selector
    const results: any[] = []

    // Find all elements matching the container selector
    const containerEls = previewElements.value.filter(el => {
      try {
        return el.selector === containerSel || el.selector.includes(containerSel)
      } catch { return false }
    })

    // For each container, extract fields from child elements
    if (containerEls.length > 0 && rule.fields.length > 0) {
      // Group by container's bbox: find elements inside each container
      for (const container of containerEls) {
        const item: Record<string, string> = {}
        for (const field of rule.fields) {
          // Find child elements within this container
          const childEls = previewElements.value.filter(el => {
            return el.bbox.x >= container.bbox.x &&
              el.bbox.y >= container.bbox.y &&
              el.bbox.x + el.bbox.w <= container.bbox.x + container.bbox.w &&
              el.bbox.y + el.bbox.h <= container.bbox.y + container.bbox.h &&
              el.selector === field.selector
          })
          item[field.name] = childEls.length > 0 ? (childEls[0].text || '') : ''
        }
        results.push(item)
      }
    } else if (containerEls.length > 0) {
      // Simple mode: just show text from matching elements
      for (const el of containerEls) {
        results.push({ text: el.text || '' })
      }
    }

    extractionPreview.value = results.length > 0 ? results : [{ _hint: '未找到匹配元素，请检查选择器是否正确' }]
  } catch (e) {
    extractionPreview.value = [{ _error: String(e) }]
  } finally {
    extractionLoading.value = false
  }
}

// ─── Filter visible elements by zoom level ───
// Too many tiny elements make overlay unusable; filter by min size
const visibleElements = computed(() => {
  const minPx = 3 // minimum displayed pixels
  return previewElements.value.filter(el => {
    const w = el.bbox.w * scale.value
    const h = el.bbox.h * scale.value
    return w >= minPx && h >= minPx
  })
})

// ─── Resize observer ───
let resizeObserver: ResizeObserver | null = null

onMounted(() => {
  if (imgContainer.value) {
    resizeObserver = new ResizeObserver(() => updateScale())
    resizeObserver.observe(imgContainer.value)
  }
})

onUnmounted(() => {
  resizeObserver?.disconnect()
})

// Listen for URL changes to auto-clear preview
watch(() => config.value.url, () => {
  // Don't auto-clear, user may want to keep preview while editing URL
})
</script>

<template>
  <div class="webscrape-config">
    <!-- Existing config fields -->
    <ConfigFieldRenderer v-model:config="config" :fields="fields" />

    <!-- ─── Preview Section ─── -->
    <div class="preview-section">
      <div class="preview-header">
        <span class="preview-title-text">🔍 页面预览</span>
        <button
          class="preview-btn"
          :disabled="previewLoading || !config.url"
          @click="loadPreview"
        >
          {{ previewLoading ? '⏳ 加载中...' : '预览页面' }}
        </button>
      </div>

      <!-- Error -->
      <div v-if="previewError" class="preview-error">
        ⚠️ {{ previewError }}
      </div>

      <!-- Loading -->
      <div v-if="previewLoading" class="preview-loading">
        <div class="spinner"></div>
        <span>正在加载页面...</span>
      </div>

      <!-- Screenshot + Overlays -->
      <div
        v-if="screenshot && !previewLoading"
        ref="imgContainer"
        class="screenshot-container"
      >
        <div class="screenshot-info">
          <span class="screenshot-url" :title="previewUrl">{{ previewTitle || previewUrl }}</span>
        </div>
        <div class="screenshot-wrapper" style="position: relative; display: inline-block;">
          <img
            :src="'data:image/png;base64,' + screenshot"
            class="screenshot-img"
            @load="onImgLoad"
            alt="页面截图"
          />
          <!-- Overlay hotzones -->
          <div
            v-for="el in visibleElements"
            :key="el.selector"
            :class="overlayClass(el)"
            :style="overlayStyle(el)"
            @click.stop="onElementClick(el)"
            @mouseenter="onElementHover(el)"
            @mouseleave="onElementHover(null)"
            :title="`${el.tag} | ${el.selector}\n${(el.text || '').substring(0, 80)}`"
          ></div>
        </div>
      </div>

      <!-- Selected element info -->
      <div v-if="selectedElement && screenshot" class="selected-info">
        <div class="selected-header">
          <span class="selected-tag">{{ selectedElement.tag }}</span>
          <span class="selected-type-badge" :class="'badge-' + selectedElement.type">
            {{ selectedElement.type === 'container' ? '📦 容器' : selectedElement.type === 'interactive' ? '🖱 交互' : '📝 文本' }}
          </span>
        </div>
        <div class="selector-display">
          <code class="selector-code">{{ selectedElement.selector }}</code>
          <button class="copy-btn" @click="navigator.clipboard.writeText(selectedElement.selector)" title="复制选择器">
            📋
          </button>
        </div>
        <div class="selected-text" v-if="selectedElement.text">
          {{ selectedElement.text.substring(0, 150) }}{{ selectedElement.text.length > 150 ? '...' : '' }}
        </div>
        <div class="selected-actions">
          <button class="action-btn primary" @click="setContainerSelector">📦 设为容器选择器</button>
          <button
            class="action-btn"
            @click="addFieldFromSelected"
            :disabled="extractRules.length === 0 || !extractRules[0].selector"
          >
            ➕ 添加为字段
          </button>
        </div>
      </div>

      <!-- Extract Rules Summary -->
      <div v-if="extractRules.length > 0 && extractRules[0].selector" class="rules-summary">
        <div class="rules-header">
          <span>📋 提取规则</span>
          <button class="remove-btn" @click="removeContainerRule">🗑 清空</button>
        </div>
        <div class="rule-container">
          <div class="rule-label">容器:</div>
          <code class="rule-selector">{{ extractRules[0].selector }}</code>
        </div>
        <div v-if="extractRules[0].fields.length > 0" class="fields-list">
          <div class="rule-label">字段:</div>
          <div v-for="(field, idx) in extractRules[0].fields" :key="idx" class="field-item">
            <input
              class="field-name-input"
              :value="field.name"
              @input="(e: Event) => updateFieldName(idx, (e.target as HTMLInputElement).value)"
              placeholder="字段名"
            />
            <code class="field-selector">{{ field.selector }}</code>
            <button class="field-remove" @click="removeField(idx)">✕</button>
          </div>
        </div>
        <button
          class="preview-extract-btn"
          @click="previewExtraction"
          :disabled="extractionLoading || !extractRules[0].selector"
        >
          {{ extractionLoading ? '⏳' : '👁' }} 预览抓取结果
        </button>

        <!-- Extraction Preview Results -->
        <div v-if="extractionPreview.length > 0" class="extraction-results">
          <div class="extraction-header">抓取结果预览 ({{ extractionPreview.length }} 条)</div>
          <div class="extraction-table-wrapper">
            <table class="extraction-table">
              <thead>
                <tr>
                  <th v-for="col in Object.keys(extractionPreview[0])" :key="col">{{ col }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="(row, ri) in extractionPreview.slice(0, 20)" :key="ri">
                  <td v-for="col in Object.keys(extractionPreview[0])" :key="col">
                    {{ String(row[col] || '').substring(0, 80) }}{{ String(row[col] || '').length > 80 ? '...' : '' }}
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <div v-if="extractionPreview.length > 20" class="extraction-more">
            ... 还有 {{ extractionPreview.length - 20 }} 条结果未显示
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.webscrape-config {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* ─── Preview Section ─── */
.preview-section {
  border-top: 1px solid #21262d;
  padding-top: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.preview-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.preview-title-text {
  font-size: 13px;
  font-weight: 600;
  color: #e1e4e8;
}

.preview-btn {
  background: #238636;
  color: #fff;
  border: none;
  padding: 6px 16px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
  font-weight: 500;
  transition: background 0.15s;
}
.preview-btn:hover:not(:disabled) {
  background: #2ea043;
}
.preview-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.preview-error {
  background: rgba(248, 81, 73, 0.1);
  border: 1px solid rgba(248, 81, 73, 0.3);
  color: #f85149;
  padding: 10px 14px;
  border-radius: 6px;
  font-size: 12px;
}

.preview-loading {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 32px;
  justify-content: center;
  color: #8b949e;
  font-size: 13px;
}

.spinner {
  width: 20px;
  height: 20px;
  border: 2px solid #30363d;
  border-top-color: #58a6ff;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* ─── Screenshot Container ─── */
.screenshot-container {
  border: 1px solid #30363d;
  border-radius: 8px;
  overflow: hidden;
  background: #0d1117;
}

.screenshot-info {
  padding: 6px 10px;
  background: #161b22;
  border-bottom: 1px solid #30363d;
  font-size: 11px;
  color: #8b949e;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.screenshot-wrapper {
  position: relative;
  display: inline-block;
  line-height: 0;
  max-width: 100%;
}

.screenshot-img {
  display: block;
  max-width: 100%;
  height: auto;
  user-select: none;
  -webkit-user-drag: none;
}

/* ─── Overlays ─── */
.preview-overlay {
  position: absolute;
  border: 1px solid transparent;
  cursor: pointer;
  transition: all 0.12s ease;
  z-index: 10;
  pointer-events: auto;
}

.preview-overlay.type-container {
  border-color: rgba(88, 166, 255, 0.15);
}
.preview-overlay.type-interactive {
  border-color: rgba(210, 153, 34, 0.15);
}
.preview-overlay.type-text {
  border-color: rgba(139, 148, 158, 0.10);
}

.preview-overlay:hover,
.preview-overlay.hovered {
  border-color: rgba(88, 166, 255, 0.7) !important;
  background: rgba(88, 166, 255, 0.08);
  z-index: 20;
}

.preview-overlay.selected {
  border: 2px solid #3fb950 !important;
  background: rgba(74, 222, 128, 0.15) !important;
  z-index: 30;
}

/* ─── Selected Element Info ─── */
.selected-info {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 8px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.selected-header {
  display: flex;
  align-items: center;
  gap: 8px;
}

.selected-tag {
  font-family: 'Cascadia Code', 'Fira Code', monospace;
  font-size: 13px;
  color: #58a6ff;
  background: rgba(88, 166, 255, 0.1);
  padding: 2px 8px;
  border-radius: 4px;
}

.selected-type-badge {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 4px;
}
.badge-container {
  background: rgba(88, 166, 255, 0.1);
  color: #58a6ff;
}
.badge-interactive {
  background: rgba(210, 153, 34, 0.1);
  color: #d29922;
}
.badge-text {
  background: rgba(139, 148, 158, 0.1);
  color: #8b949e;
}

.selector-display {
  display: flex;
  align-items: center;
  gap: 8px;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 6px;
  padding: 6px 10px;
}

.selector-code {
  flex: 1;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
  font-size: 12px;
  color: #7ee787;
  word-break: break-all;
}

.copy-btn {
  background: none;
  border: 1px solid #30363d;
  color: #8b949e;
  padding: 2px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
  transition: all 0.15s;
  flex-shrink: 0;
}
.copy-btn:hover {
  border-color: #58a6ff;
  color: #58a6ff;
}

.selected-text {
  font-size: 12px;
  color: #8b949e;
  background: rgba(139, 148, 158, 0.05);
  padding: 6px 10px;
  border-radius: 4px;
  max-height: 60px;
  overflow-y: auto;
  white-space: pre-wrap;
  word-break: break-all;
}

.selected-actions {
  display: flex;
  gap: 8px;
}

.action-btn {
  padding: 5px 14px;
  border: 1px solid #30363d;
  background: #21262d;
  color: #c9d1d9;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.15s;
}
.action-btn:hover:not(:disabled) {
  border-color: #58a6ff;
  background: #30363d;
}
.action-btn.primary {
  background: #238636;
  border-color: #238636;
  color: #fff;
}
.action-btn.primary:hover:not(:disabled) {
  background: #2ea043;
  border-color: #2ea043;
}
.action-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

/* ─── Extract Rules Summary ─── */
.rules-summary {
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 8px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.rules-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 13px;
  font-weight: 600;
  color: #e1e4e8;
}

.remove-btn {
  background: none;
  border: 1px solid rgba(248, 81, 73, 0.3);
  color: #f85149;
  padding: 2px 10px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 11px;
}
.remove-btn:hover {
  background: rgba(248, 81, 73, 0.1);
}

.rule-container {
  display: flex;
  align-items: center;
  gap: 8px;
}

.rule-label {
  font-size: 11px;
  color: #8b949e;
  white-space: nowrap;
  min-width: 36px;
}

.rule-selector {
  font-family: 'Cascadia Code', 'Fira Code', monospace;
  font-size: 12px;
  color: #7ee787;
  background: rgba(126, 231, 135, 0.1);
  padding: 3px 8px;
  border-radius: 4px;
  word-break: break-all;
}

.fields-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding-left: 8px;
}

.field-item {
  display: flex;
  align-items: center;
  gap: 6px;
}

.field-name-input {
  width: 80px;
  background: #0d1117;
  border: 1px solid #30363d;
  color: #c9d1d9;
  padding: 3px 6px;
  border-radius: 4px;
  font-size: 12px;
  flex-shrink: 0;
}
.field-name-input:focus {
  outline: none;
  border-color: #58a6ff;
}

.field-selector {
  font-family: 'Cascadia Code', 'Fira Code', monospace;
  font-size: 11px;
  color: #d2a8ff;
  background: rgba(210, 168, 255, 0.08);
  padding: 2px 6px;
  border-radius: 4px;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.field-remove {
  background: none;
  border: none;
  color: #6e7681;
  cursor: pointer;
  font-size: 12px;
  padding: 2px 4px;
  flex-shrink: 0;
}
.field-remove:hover {
  color: #f85149;
}

.preview-extract-btn {
  align-self: flex-start;
  padding: 5px 14px;
  border: 1px solid #30363d;
  background: #21262d;
  color: #c9d1d9;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.15s;
  margin-top: 4px;
}
.preview-extract-btn:hover:not(:disabled) {
  border-color: #58a6ff;
  background: #30363d;
}
.preview-extract-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

/* ─── Extraction Results Table ─── */
.extraction-results {
  margin-top: 8px;
  border: 1px solid #21262d;
  border-radius: 6px;
  overflow: hidden;
}

.extraction-header {
  padding: 6px 10px;
  background: #0d1117;
  font-size: 11px;
  color: #8b949e;
  border-bottom: 1px solid #21262d;
}

.extraction-table-wrapper {
  max-height: 300px;
  overflow: auto;
}

.extraction-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 11px;
}

.extraction-table th {
  background: #0d1117;
  padding: 6px 8px;
  text-align: left;
  font-weight: 600;
  color: #58a6ff;
  border-bottom: 1px solid #21262d;
  position: sticky;
  top: 0;
  z-index: 2;
}

.extraction-table td {
  padding: 4px 8px;
  border-bottom: 1px solid #161b22;
  color: #c9d1d9;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.extraction-table tr:hover td {
  background: #161b22;
}

.extraction-more {
  padding: 6px 10px;
  font-size: 11px;
  color: #6e7681;
  text-align: center;
  background: #0d1117;
}
</style>
