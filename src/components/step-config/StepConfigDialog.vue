<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import type { WorkflowStep } from '../../types/workflow'
import { STEP_LABELS, STEP_ICONS } from '../../types/workflow'
import { getNodeFields } from '../../config/node-fields'
import { useToast } from '../../composables/useToast'

import HttpConfig from './HttpConfig.vue'
import DataConfig from './DataConfig.vue'
import ScriptConfig from './ScriptConfig.vue'
import ConditionConfig from './ConditionConfig.vue'
import LoopConfig from './LoopConfig.vue'
import WhileConfig from './WhileConfig.vue'
import ExcelConfig from './ExcelConfig.vue'
import WordConfig from './WordConfig.vue'
import BrowserConfig from './BrowserConfig.vue'
import WebScrapeConfig from './WebScrapeConfig.vue'
import NotifyConfig from './NotifyConfig.vue'
import ApprovalConfig from './ApprovalConfig.vue'
import ParallelConfig from './ParallelConfig.vue'
import MouseKeyboardConfig from './MouseKeyboardConfig.vue'
import WindowConfig from './WindowConfig.vue'
import OcrConfig from './OcrConfig.vue'
import RecordingConfig from './RecordingConfig.vue'
import SubWorkflowConfig from './SubWorkflowConfig.vue'
import MapConfig from './MapConfig.vue'

const toast = useToast()

const props = defineProps<{
  step: WorkflowStep | null
  allSteps?: WorkflowStep[]
}>()

const emit = defineEmits<{
  save: [data: { name: string; type: string; config: Record<string, unknown> }]
  close: []
}>()

const name = ref('')
const type = ref('http')
const config = ref<Record<string, unknown>>({})
const showJson = ref(false)
const jsonText = ref('{}')
const jsonError = ref('')

const configComponent = computed(() => {
  const map: Record<string, any> = {
    http: HttpConfig,
    data: DataConfig,
    script: ScriptConfig,
    condition: ConditionConfig,
    loop: LoopConfig,
    while: WhileConfig,
    excel: ExcelConfig,
    word: WordConfig,
    browser: BrowserConfig,
    web_scrape: WebScrapeConfig,
    notify: NotifyConfig,
    approval: ApprovalConfig,
    parallel: ParallelConfig,
    mouse_keyboard: MouseKeyboardConfig,
    window: WindowConfig,
    ocr: OcrConfig,
    recording: RecordingConfig,
    sub_workflow: SubWorkflowConfig,
    map: MapConfig,
  }
  return map[type.value] || null
})

// ─── 初始化 ───

watch(() => props.step, (s) => {
  if (s) {
    name.value = s.name
    type.value = s.type
    // Deep clone config，将 browser params 展平到 config 顶层
    const raw = JSON.parse(JSON.stringify(s.config || {}))
    if (s.type === 'browser' && raw.params) {
      // 迁移旧格式：config.params → config 顶层
      Object.assign(raw, raw.params)
      delete raw.params
    }
    config.value = raw as Record<string, unknown>
    jsonText.value = JSON.stringify(raw, null, 2)
    jsonError.value = ''
    showJson.value = false
  }
}, { immediate: true })

// ─── JSON 模式切换 ───

function toggleJsonMode() {
  if (!showJson.value) {
    jsonText.value = JSON.stringify(config.value, null, 2)
    jsonError.value = ''
  } else {
    try {
      config.value = JSON.parse(jsonText.value)
      jsonError.value = ''
    } catch (e: unknown) {
      jsonError.value = 'JSON 格式错误: ' + (e as Error).message
      return
    }
  }
  showJson.value = !showJson.value
}

// ─── 校验 ───

function validateConfig(): string | null {
  const allFields = getNodeFields(props.allSteps || [])
  const fields = allFields[type.value] || []
  for (const field of fields) {
    if (field.show && !field.show(config.value)) continue
    if (['url', 'path', 'action', 'source', 'left'].includes(field.key)) {
      const val = config.value[field.key]
      if (val === undefined || val === null || val === '') {
        return `${field.label} 不能为空`
      }
    }
  }
  if (type.value === 'http' && config.value.url) {
    try { new URL(config.value.url as string) } catch { return 'URL 格式无效' }
  }
  return null
}

// ─── 保存 ───

function onSave() {
  const err = validateConfig()
  if (err) { toast.error(err); return }

  let finalConfig: Record<string, unknown> = config.value
  if (showJson.value) {
    try {
      finalConfig = JSON.parse(jsonText.value) as Record<string, unknown>
      jsonError.value = ''
    } catch (e: unknown) {
      jsonError.value = 'JSON 格式错误: ' + (e as Error).message
      return
    }
  }

  // 序列化：字符串形式的 JSON 自动解析
  const serialized: Record<string, unknown> = {}
  for (const [k, v] of Object.entries(finalConfig)) {
    if (typeof v === 'string') {
      const trimmed = v.trim()
      if ((trimmed.startsWith('{') && trimmed.endsWith('}')) ||
          (trimmed.startsWith('[') && trimmed.endsWith(']'))) {
        try { serialized[k] = JSON.parse(v) } catch { serialized[k] = v }
      } else if (trimmed === 'true') {
        serialized[k] = true
      } else if (trimmed === 'false') {
        serialized[k] = false
      } else if (trimmed !== '' && !isNaN(Number(trimmed))) {
        serialized[k] = Number(trimmed)
      } else {
        serialized[k] = v
      }
    } else {
      serialized[k] = v
    }
  }

  // browser 节点：将展平的字段打包回 params 对象
  if (type.value === 'browser') {
    const { action, ...rest } = serialized
    const params: Record<string, unknown> = {}
    for (const [k, v] of Object.entries(rest)) {
      if (v !== '' && v !== null && v !== undefined) params[k] = v
    }
    // cookie_action → action 映射（cookies 子操作）
    if (action === 'cookies' && params.cookie_action) {
      params.action = params.cookie_action
      delete params.cookie_action
    }
    serialized.params = params
    // 删除展平的字段
    for (const k of Object.keys(params)) delete serialized[k]
    serialized.action = action
  }

  // web_scrape 节点：next_selector + max_pages → pagination 对象
  if (type.value === 'web_scrape') {
    const pagination: Record<string, unknown> = {}
    if (serialized.next_selector) {
      pagination.next = serialized.next_selector
      delete serialized.next_selector
    }
    if (serialized.max_pages) {
      pagination.max_pages = Number(serialized.max_pages)
      delete serialized.max_pages
    }
    if (Object.keys(pagination).length > 0) {
      serialized.pagination = pagination
    }
    // 确保 extract 是数组
    if (typeof serialized.extract === 'string') {
      try { serialized.extract = JSON.parse(serialized.extract as string) } catch {}
    }
  }

  emit('save', { name: name.value, type: type.value, config: serialized })
}
</script>

<template>
  <div class="dialog-overlay" @click.self="emit('close')" @keydown.escape="emit('close')">
    <div class="dialog">
      <div class="dialog-header">
        <h3>{{ STEP_ICONS[type] }} 编辑步骤</h3>
        <button class="close-btn" @click="emit('close')">✕</button>
      </div>

      <div class="dialog-body">
        <div class="field">
          <label>步骤名称</label>
          <input v-model="name" class="input" placeholder="步骤名称" />
        </div>

        <div class="field">
          <label>步骤类型</label>
          <div class="type-display">
            <span class="type-icon">{{ STEP_ICONS[type] }}</span>
            <span>{{ STEP_LABELS[type] || type }}</span>
          </div>
        </div>

        <div class="field">
          <div class="field-header">
            <label>配置</label>
            <button class="btn btn-xs" @click="toggleJsonMode">
              {{ showJson ? '📋 表单模式' : '{ } JSON 模式' }}
            </button>
          </div>

          <!-- 表单模式 -->
          <div v-if="!showJson" class="config-form">
            <component
              :is="configComponent"
              v-model:config="config"
              :all-steps="allSteps"
            />
          </div>

          <!-- JSON 模式 -->
          <div v-else>
            <textarea
              v-model="jsonText"
              class="json-editor"
              spellcheck="false"
              rows="14"
            ></textarea>
            <div v-if="jsonError" class="field-error">{{ jsonError }}</div>
          </div>
        </div>
      </div>

      <div class="dialog-footer">
        <button class="btn" @click="emit('close')">取消</button>
        <button class="btn btn-primary" @click="onSave">💾 保存</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dialog-overlay {
  position: fixed; inset: 0; background: rgba(0,0,0,0.6);
  display: flex; align-items: center; justify-content: center; z-index: 100;
}
.dialog {
  background: #161b22; border: 1px solid #30363d; border-radius: 12px;
  width: 600px; max-height: 85vh; display: flex; flex-direction: column;
}
.dialog-header {
  display: flex; justify-content: space-between; align-items: center;
  padding: 16px 20px; border-bottom: 1px solid #30363d;
}
.dialog-header h3 { margin: 0; font-size: 16px; }
.close-btn {
  background: none; border: none; color: #8b949e;
  font-size: 18px; cursor: pointer; padding: 2px 6px; border-radius: 4px;
}
.close-btn:hover { background: #21262d; color: #e1e4e8; }
.dialog-body { padding: 16px 20px; overflow-y: auto; flex: 1; }
.field { margin-bottom: 14px; }
.field > label {
  display: block; font-size: 12px; color: #8b949e;
  margin-bottom: 4px; text-transform: uppercase; letter-spacing: 0.3px;
}
.field-header {
  display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;
}
.field-header label {
  font-size: 12px; color: #8b949e;
  text-transform: uppercase; letter-spacing: 0.3px; margin: 0;
}
.input {
  width: 100%; background: #0d1117; border: 1px solid #30363d; color: #e1e4e8;
  padding: 6px 10px; border-radius: 6px; font-size: 13px; box-sizing: border-box;
}
.input:focus { outline: none; border-color: #58a6ff; }
.type-display {
  display: flex; align-items: center; gap: 8px;
  padding: 6px 10px; background: #0d1117; border: 1px solid #30363d;
  border-radius: 6px; font-size: 13px; color: #c9d1d9;
}
.type-icon { font-size: 16px; }
.config-form { display: flex; flex-direction: column; gap: 10px; }
.json-editor {
  width: 100%; background: #0d1117; border: 1px solid #30363d; color: #c9d1d9;
  padding: 8px 10px; border-radius: 6px;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
  font-size: 12px; line-height: 1.5; resize: vertical; box-sizing: border-box;
}
.json-editor:focus { outline: none; border-color: #58a6ff; }
.field-error { color: #f85149; font-size: 11px; margin-top: 4px; }
.dialog-footer {
  display: flex; justify-content: flex-end; gap: 8px;
  padding: 12px 20px; border-top: 1px solid #30363d;
}
</style>
