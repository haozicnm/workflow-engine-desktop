<script setup lang="ts">
import { STEP_TYPES } from '../types/workflow'

const emit = defineEmits<{
  'add-step': [type: string]
}>()

function handleClick(type: string) {
  emit('add-step', type)
}

function handleDragStart(e: DragEvent, type: string) {
  e.dataTransfer?.setData('step-type', type)
}

// ─── 快速模板 ───

const presets = [
  {
    id: 'http-get',
    icon: '🌐',
    label: 'HTTP GET',
    steps: [{ type: 'http', name: 'HTTP GET', config: { action: 'GET', url: '' } }],
  },
  {
    id: 'excel-read-write',
    icon: '📗',
    label: 'Excel 读写',
    steps: [
      { type: 'excel', name: '读取Excel', config: { action: 'read', path: '', sheet: 'Sheet1' } },
      { type: 'excel', name: '写入Excel', config: { action: 'write', path: '', sheet: 'Sheet1', data: [] } },
    ],
  },
  {
    id: 'browser-scrape',
    icon: '🌍',
    label: '浏览器操作',
    steps: [
      { type: 'browser', name: '打开页面', config: { action: 'navigate', url: '' } },
      { type: 'browser', name: '等待元素', config: { action: 'wait', selector: '', timeout: 5000 } },
      { type: 'browser', name: '获取文本', config: { action: 'text', selector: '' } },
    ],
  },
  {
    id: 'web-scrape',
    icon: '🕷️',
    label: '网页抓取',
    steps: [
      { type: 'web_scrape', name: '抓取数据', config: {
        url: '', wait_for: 'body',
        extract: [{ selector: '.item', fields: { title: '.title', link: 'a[href]' } }],
        pagination: { next: '.next-page', max_pages: 3 },
        scroll: false, delay_ms: 1000,
      }},
    ],
  },
  {
    id: 'scrape-excel',
    icon: '📊',
    label: '抓取→Excel',
    steps: [
      { type: 'web_scrape', name: '抓取网页', config: {
        url: '', wait_for: 'body',
        extract: [{ selector: '.item', fields: { name: '.title', value: '.price' } }],
        delay_ms: 1000,
      }},
      { type: 'excel', name: '写入Excel', config: { action: 'write', path: './output.xlsx', sheet: 'Sheet1', data: '{{step_scrape.items}}' } },
    ],
  },
  {
    id: 'loop-map',
    icon: '🔄',
    label: '循环+映射',
    steps: [
      { type: 'loop', name: '循环处理', config: { items: [], body: [] } },
      { type: 'map', name: '数据映射', config: { source: '', template: {} } },
    ],
  },
  {
    id: 'loop-table',
    icon: '📋',
    label: '循环→表格',
    steps: [
      { type: 'loop', name: '循环采集', config: {
        items: 'step_data',
        body: [{ id: 'step_fetch', name: '获取数据', type: 'http', config: { action: 'GET', url: '' } }],
        table: [
          { header: '名称', field: 'step_fetch.body.name' },
          { header: '值', field: 'step_fetch.body.value' },
        ],
      }},
      { type: 'excel', name: '写入Excel', config: { action: 'write', path: './output.xlsx', sheet: 'Sheet1', data: '{{step_loop.table.data}}' } },
    ],
  },
  {
    id: 'while-excel',
    icon: '🔁',
    label: 'While循环',
    steps: [
      { type: 'excel', name: '读取数据', config: { action: 'extract_column', path: './data.xlsx', sheet: 'Sheet1', column: 'A' } },
      { type: 'while', name: '逐行处理', config: {
        items: 'step_read',
        condition: { op: 'not_empty' },
        body: [{ id: 'step_process', name: '处理当前行', type: 'http', config: { action: 'GET', url: '' } }],
        max_iterations: 1000,
      }},
    ],
  },
  {
    id: 'approval-notify',
    icon: '✅',
    label: '审批+通知',
    steps: [
      { type: 'approval', name: '人工审批', config: { message: '请审批', timeout: 300 } },
      { type: 'notify', name: '完成通知', config: { notify_type: 'system', title: '完成', body: '' } },
    ],
  },
  {
    id: 'desktop-automate',
    icon: '🖱️',
    label: '桌面自动化',
    steps: [
      { type: 'window', name: '激活窗口', config: { action: 'activate', title: '' } },
      { type: 'mouse_keyboard', name: '点击', config: { action: 'click', x: 100, y: 200 } },
      { type: 'mouse_keyboard', name: '输入文字', config: { action: 'type', text: '' } },
    ],
  },
  {
    id: 'ocr-extract',
    icon: '🔍',
    label: 'OCR识别',
    steps: [
      { type: 'ocr', name: '识别屏幕', config: { action: 'read', lang: 'chi_sim+eng' } },
      { type: 'data', name: '处理结果', config: { action: 'set', key: 'text', value: '' } },
    ],
  },
  {
    id: 'sub-workflow',
    icon: '📦',
    label: '子流程调用',
    steps: [
      { type: 'sub_workflow', name: '执行子流程', config: { workflow_yaml: '', vars_mapping: {}, output_key: 'result' } },
    ],
  },
  {
    id: 'record-playback',
    icon: '🎬',
    label: '录制回放',
    steps: [
      { type: 'recording', name: '开始录制', config: { action: 'start', headless: false } },
      { type: 'recording', name: '停止录制', config: { action: 'stop' } },
    ],
  },
]

function addPreset(preset: typeof presets[0]) {
  for (const s of preset.steps) {
    emit('add-step', s.type)
  }
}
</script>

<template>
  <div class="palette">
    <div class="palette-title">步骤类型</div>
    <div
      v-for="st in STEP_TYPES"
      :key="st.type"
      class="palette-item"
      draggable="true"
      :style="{ '--accent': st.color }"
      @click="handleClick(st.type)"
      @dragstart="handleDragStart($event, st.type)"
    >
      <span class="item-icon">{{ st.icon }}</span>
      <span class="item-label">{{ st.label }}</span>
    </div>

    <!-- 快速模板 -->
    <div class="preset-section">
      <div class="preset-title">⚡ 快速模板</div>
      <div class="preset-list">
        <button v-for="p in presets" :key="p.id" class="preset-item" @click="addPreset(p)">
          <span>{{ p.icon }}</span>
          <span>{{ p.label }}</span>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.palette {
  padding: 12px;
}
.palette-title {
  font-size: 11px;
  text-transform: uppercase;
  color: #6e7681;
  margin-bottom: 8px;
  letter-spacing: 0.5px;
}
.palette-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;
  border: 1px solid transparent;
  margin-bottom: 2px;
  font-size: 13px;
  color: #c9d1d9;
}
.palette-item:hover {
  background: #21262d;
  border-color: var(--accent);
}
.item-icon { font-size: 15px; }

/* ─── 快速模板 ─── */
.preset-section { padding: 8px 12px; border-top: 1px solid #30363d; }
.preset-title { font-size: 10px; color: #6e7681; text-transform: uppercase; margin-bottom: 6px; letter-spacing: 0.5px; }
.preset-list { display: flex; flex-direction: column; gap: 2px; }
.preset-item {
  display: flex; align-items: center; gap: 6px;
  padding: 5px 8px; border: none; background: none;
  color: #c9d1d9; font-size: 11px; cursor: pointer;
  border-radius: 4px; text-align: left;
}
.preset-item:hover { background: #21262d; }
</style>
