<script setup lang="ts">
// ParamField — 统一参数字段渲染组件
// 支持两种 schema：
//   1. 旧 ActionParam（key/label/type）—— 向后兼容
//   2. 新 ParamDef（name/field_type/desc）—— schema-driven 自动渲染
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Target, Link, FolderOpen } from 'lucide-vue-next'
import type { StepGroup } from '../composables/useVariableRefs'
import type { ParamDef } from '../types/types'
import Input from './ui/input/Input.vue'
import Label from './ui/label/Label.vue'
import Textarea from './ui/textarea/Textarea.vue'
import Checkbox from './ui/checkbox/Checkbox.vue'
import Switch from './ui/switch/Switch.vue'
import Select from './ui/select/Select.vue'
import Button from './ui/button/Button.vue'

const { t } = useI18n()

// ─── 旧 ActionParam 接口（向后兼容） ───

interface LegacyParamDef {
  key: string
  label: string
  type: 'text' | 'number' | 'select' | 'checkbox' | 'textarea'
  placeholder?: string
  default?: unknown
  options?: { label: string; value: string }[]
  hint?: string
}

const props = defineProps<{
  /** 旧格式参数定义 */
  param?: LegacyParamDef
  /** 新 schema-driven 参数定义（优先级高于 param） */
  paramDef?: ParamDef
  modelValue: unknown
  groupedRefs?: StepGroup[]
  /** 显示元素选择器按钮（browser selector 字段） */
  showElementPicker?: boolean
  /** 元素选择器是否正在选择中 */
  pickingElement?: boolean
  /** 数据来源提示 */
  dataHint?: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: unknown]
  'pick-element': []
}>()

// ─── 统一内部字段表示 ───

interface NormalizedField {
  key: string
  label: string
  type: 'text' | 'number' | 'select' | 'checkbox' | 'textarea' | 'json' | 'code' | 'file_path' | 'boolean'
  placeholder: string
  default: unknown
  options: { label: string; value: string }[]
  hint: string
  required: boolean
  lang: string
  group: string
  isSchemaDriven: boolean
}

/** 将 ParamDef.field_type 映射到内部渲染类型 */
function mapFieldType(ft: ParamDef['field_type']): NormalizedField['type'] {
  const map: Record<ParamDef['field_type'], NormalizedField['type']> = {
    string: 'text',
    number: 'number',
    boolean: 'boolean',
    select: 'select',
    json: 'json',
    code: 'code',
    file_path: 'file_path',
    text: 'textarea',
  }
  return map[ft] || 'text'
}

const field = computed<NormalizedField>(() => {
  // 新 schema-driven 格式优先
  if (props.paramDef) {
    const pd = props.paramDef
    return {
      key: pd.name,
      label: pd.desc || pd.name,
      type: mapFieldType(pd.field_type),
      placeholder: pd.desc || '',
      default: pd.default,
      options: (pd.options || []).map(o => ({ label: o, value: o })),
      hint: '',
      required: pd.required,
      lang: pd.lang || '',
      group: pd.group || 'basic',
      isSchemaDriven: true,
    }
  }
  // 旧格式 fallback
  const p = props.param!
  return {
    key: p.key,
    label: p.label,
    type: p.type,
    placeholder: p.placeholder || '',
    default: p.default,
    options: p.options || [],
    hint: p.hint || '',
    required: false,
    lang: '',
    group: 'basic',
    isSchemaDriven: false,
  }
})

// ─── Input handlers ───
function onTextInput(e: Event) {
  emit('update:modelValue', (e.target as HTMLInputElement).value)
}
function onNumberInput(e: Event) {
  const v = (e.target as HTMLInputElement).value
  emit('update:modelValue', v === '' ? '' : Number(v))
}
function onCheckboxChange(val: boolean) {
  emit('update:modelValue', val)
}
function onTextareaInput(e: Event) {
  emit('update:modelValue', (e.target as HTMLTextAreaElement).value)
}

// ─── JSON 验证 ───
const jsonError = ref('')
function onJsonInput(e: Event) {
  const val = (e.target as HTMLTextAreaElement).value
  jsonError.value = ''
  if (val.trim()) {
    try { JSON.parse(val) } catch { jsonError.value = 'JSON 格式无效' }
  }
  emit('update:modelValue', val)
}

// ─── File path picker (Tauri) ───
async function pickFilePath() {
  try {
    // Tauri v2 对话框插件（运行时动态加载，类型安全）
    const mod = await Function('return import("@tauri-apps/plugin-dialog")')()
    const selected = await mod.open({ multiple: false, title: '选择文件' })
    if (selected && typeof selected === 'string') emit('update:modelValue', selected)
  } catch {
    // 非 Tauri 环境或插件未安装，忽略
  }
}

// ─── Variable reference dropdown ───
const openDropdown = ref(false)
const dropdownPos = ref({ top: 0, left: 0 })
const triggerRef = ref<HTMLElement | null>(null)

function toggleDropdown(e: Event) {
  if (openDropdown.value) {
    openDropdown.value = false
    return
  }
  const btn = (e.target as HTMLElement).closest('button') || triggerRef.value
  if (btn) {
    const rect = btn.getBoundingClientRect()
    dropdownPos.value = { top: rect.bottom + 4, left: rect.left }
  }
  openDropdown.value = true
}
function closeDropdown() {
  openDropdown.value = false
}
function selectRef(refId: string) {
  insertRef(refId)
  openDropdown.value = false
}

function insertRef(refId: string) {
  const input = document.querySelector(`[data-field="${field.value.key}"]`) as HTMLInputElement | HTMLTextAreaElement
  if (!input) return
  const refText = `{{${refId}}}`
  const start = input.selectionStart ?? 0
  const end = input.selectionEnd ?? 0
  const current = String(props.modelValue ?? '')
  const newVal = current.slice(0, start) + refText + current.slice(end)
  emit('update:modelValue', newVal)
}

// 已有引用标签
const refTags = computed(() => {
  if (typeof props.modelValue !== 'string') return []
  return props.modelValue.match(/\{\{[^}]+\}\}/g) || []
})

const hasRefs = computed(() => (props.groupedRefs?.length ?? 0) > 0)
const canRef = computed(() => ['text', 'textarea', 'json', 'code', 'file_path'].includes(field.value.type))
</script>

<template>
  <div>
    <!-- Label -->
    <Label class="text-[11px] text-muted-foreground block mb-1">
      {{ field.label }}
      <span v-if="field.required" class="text-danger ml-0.5">*</span>
    </Label>
    <!-- Hint -->
    <div v-if="field.hint" class="text-[10px] text-muted-foreground/70 mb-1.5">{{ field.hint }}</div>

    <!-- Text input (string / text) -->
    <div v-if="field.type === 'text'" class="flex gap-1">
      <Input
        :data-field="field.key"
        type="text"
        :model-value="(modelValue as string) ?? (field.default as string) ?? ''"
        :placeholder="field.placeholder"
        class="flex-1 h-8 text-xs"
        @input="onTextInput"
      />
      <Button
        v-if="showElementPicker"
        variant="outline"
        size="sm"
        class="h-8 w-8 p-0 shrink-0"
        :class="pickingElement ? 'text-warning' : ''"
        :title="pickingElement ? t('actionRow.picking') : t('actionRow.pickFromPage')"
        @click="emit('pick-element')"
      ><Target class="w-4 h-4" /></Button>
    </div>

    <!-- Number input -->
    <Input
      v-else-if="field.type === 'number'"
      type="number"
      :model-value="(modelValue as string) ?? (field.default as string) ?? ''"
      :placeholder="field.placeholder"
      class="h-8 text-xs"
      @input="onNumberInput"
    />

    <!-- Select -->
    <Select
      v-else-if="field.type === 'select'"
      :model-value="(modelValue as string) ?? (field.default as string) ?? ''"
      :options="field.options"
      @update:model-value="v => emit('update:modelValue', v)"
    />

    <!-- Checkbox (旧格式) -->
    <div v-else-if="field.type === 'checkbox'" class="flex items-center gap-2">
      <Checkbox
        :model-value="!!(modelValue ?? field.default)"
        @update:model-value="onCheckboxChange"
      />
    </div>

    <!-- Boolean / Switch (新 schema-driven) -->
    <div v-else-if="field.type === 'boolean'" class="flex items-center gap-2">
      <Switch
        :model-value="!!(modelValue ?? field.default)"
        @update:model-value="onCheckboxChange"
      />
      <span class="text-[11px] text-muted-foreground">
        {{ (modelValue ?? field.default) ? 'ON' : 'OFF' }}
      </span>
    </div>

    <!-- Textarea (旧格式) -->
    <Textarea
      v-else-if="field.type === 'textarea'"
      :data-field="field.key"
      :model-value="String(modelValue ?? field.default ?? '')"
      :placeholder="field.placeholder"
      :rows="3"
      class="text-xs"
      @input="onTextareaInput"
    />

    <!-- JSON textarea（带验证） -->
    <div v-else-if="field.type === 'json'">
      <Textarea
        :data-field="field.key"
        :model-value="String(modelValue ?? field.default ?? '')"
        :placeholder="field.placeholder || '{ &quot;key&quot;: &quot;value&quot; }'"
        :rows="4"
        class="text-xs font-mono"
        @input="onJsonInput"
      />
      <div v-if="jsonError" class="text-[10px] text-danger mt-0.5">{{ jsonError }}</div>
    </div>

    <!-- Code editor (textarea with monospace + lang badge) -->
    <div v-else-if="field.type === 'code'">
      <div class="flex items-center justify-between mb-1">
        <span v-if="field.lang" class="text-[10px] text-muted-foreground bg-muted px-1.5 py-0.5 rounded font-mono">{{ field.lang }}</span>
      </div>
      <Textarea
        :data-field="field.key"
        :model-value="String(modelValue ?? field.default ?? '')"
        :placeholder="field.placeholder || '// code here'"
        :rows="8"
        class="text-xs font-mono bg-muted/30"
        @input="onTextareaInput"
      />
    </div>

    <!-- File path (input + picker button) -->
    <div v-else-if="field.type === 'file_path'" class="flex gap-1">
      <Input
        :data-field="field.key"
        type="text"
        :model-value="(modelValue as string) ?? (field.default as string) ?? ''"
        :placeholder="field.placeholder || 'C:\\path\\to\\file'"
        class="flex-1 h-8 text-xs font-mono"
        @input="onTextInput"
      />
      <Button
        variant="outline"
        size="sm"
        class="h-8 w-8 p-0 shrink-0"
        title="选择文件"
        @click="pickFilePath"
      ><FolderOpen class="w-4 h-4" /></Button>
    </div>

    <!-- Variable reference (text/textarea/json/code/file_path only) -->
    <div v-if="canRef && hasRefs" class="mt-1">
      <div class="flex items-center gap-1.5">
        <Link class="w-3 h-3 shrink-0 text-muted-foreground" />
        <div class="relative flex-1">
          <Button
            ref="triggerRef"
            variant="outline"
            size="sm"
            type="button"
            class="flex-1 h-6 w-full text-[11px] px-1.5 text-left"
            @click="toggleDropdown"
          >
            {{ t('actionRow.insertVariable') }}
          </Button>
          <!-- Backdrop -->
          <div
            v-if="openDropdown"
            class="fixed inset-0 z-40"
            @click="closeDropdown"
          />
          <!-- Dropdown (fixed to avoid overflow clipping) -->
          <Teleport to="body">
            <div
              v-if="openDropdown"
              class="fixed z-[60] w-64 max-h-[200px] overflow-y-auto bg-background border border-border rounded-md shadow-lg"
              :style="{ top: dropdownPos.top + 'px', left: dropdownPos.left + 'px' }"
            >
            <div
              v-for="group in groupedRefs"
              :key="group.stepId"
            >
              <!-- Step header -->
              <div class="px-2 py-1.5 text-[11px] font-semibold text-foreground bg-muted/50 border-b border-border/50 flex items-center gap-1.5 sticky top-0">
                <span>{{ group.stepIcon }}</span>
                <span>{{ t('actionRow.stepLabel') }}{{ group.stepId.replace('step_', '') }} · {{ group.stepLabel }}</span>
              </div>
              <!-- Output hint -->
              <div v-if="group.outputHint" class="px-2 py-0.5 text-[10px] text-muted-foreground/60 bg-muted/30 font-mono">
                {{ t('actionRow.outputLabel') }} {{ group.outputHint }}
              </div>
              <!-- Step-level -->
              <Button
                variant="ghost"
                type="button"
                class="w-full justify-start h-auto px-2 py-1 pl-5 text-[11px]"
                @click="selectRef(group.stepRef)"
              >
                {{ t('actionRow.entireOutput') }}
              </Button>
              <!-- Actions -->
              <Button
                v-for="act in group.actions"
                :key="act.id"
                variant="ghost"
                type="button"
                class="w-full justify-start h-auto px-2 py-1 pl-5 text-[11px]"
                @click="selectRef(act.ref)"
              >
                {{ act.label }}
                <span v-if="act.isSameContainer" class="text-[10px] text-primary/70 ml-0.5">{{ t('actionRow.sameContainerBadge') }}</span>
              </Button>
            </div>
            </div>
          </Teleport>
        </div>
      </div>
      <!-- Existing ref tags -->
      <div v-if="refTags.length" class="mt-0.5 flex flex-wrap gap-1">
        <span
          v-for="m in refTags"
          :key="m"
          class="inline-flex items-center px-1 py-0.5 text-[10px] font-mono bg-primary/10 text-primary rounded"
        >
          {{ m }}
        </span>
      </div>
    </div>

    <!-- Data hint -->
    <div v-if="dataHint" class="text-[11px] px-2 py-1 mt-1 rounded bg-primary/5 text-muted-foreground flex items-center gap-1">
      {{ t('actionRow.dataFrom') }} <span class="font-medium text-foreground">{{ dataHint }}</span>
    </div>
  </div>
</template>
