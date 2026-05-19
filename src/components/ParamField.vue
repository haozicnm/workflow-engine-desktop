<script setup lang="ts">
// ParamField — 统一参数字段渲染组件
// 支持 text/number/select/checkbox/textarea，text/textarea 自带变量引用下拉
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Target, Link } from 'lucide-vue-next'
import type { StepGroup } from '../composables/useVariableRefs'
import Input from './ui/input/Input.vue'
import Label from './ui/label/Label.vue'
import Textarea from './ui/textarea/Textarea.vue'
import Checkbox from './ui/checkbox/Checkbox.vue'
import Select from './ui/select/Select.vue'
import Button from './ui/button/Button.vue'

const { t } = useI18n()



interface ParamDef {
  key: string
  label: string
  type: 'text' | 'number' | 'select' | 'checkbox' | 'textarea'
  placeholder?: string
  default?: unknown
  options?: { label: string; value: string }[]
  hint?: string
}

const props = defineProps<{
  param: ParamDef
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
  const input = document.querySelector(`[data-field="${props.param.key}"]`) as HTMLInputElement | HTMLTextAreaElement
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
const canRef = computed(() => props.param.type === 'text' || props.param.type === 'textarea')
</script>

<template>
  <div>
    <!-- Label -->
    <Label class="text-[11px] text-muted-foreground block mb-1">{{ param.label }}</Label>
    <!-- Hint -->
    <div v-if="param.hint" class="text-[10px] text-muted-foreground/70 mb-1.5">{{ param.hint }}</div>

    <!-- Text input -->
    <div v-if="param.type === 'text'" class="flex gap-1">
      <Input
        :data-field="param.key"
        type="text"
        :model-value="(modelValue as string) ?? (param.default as string) ?? ''"
        :placeholder="param.placeholder"
        class="flex-1 h-8 text-xs"
        @input="onTextInput"
      />
      <Button
        v-if="showElementPicker"
        variant="outline"
        size="sm"
        class="h-8 w-8 p-0 shrink-0"
        :class="pickingElement ? 'text-warning' : ''"
        :title="pickingElement ? '选择中...' : '从页面选择元素'"
        @click="emit('pick-element')"
      ><Target class="w-4 h-4" /></Button>
    </div>

    <!-- Number input -->
    <Input
      v-else-if="param.type === 'number'"
      type="number"
      :model-value="(modelValue as string) ?? (param.default as string) ?? ''"
      :placeholder="param.placeholder"
      class="h-8 text-xs"
      @input="onNumberInput"
    />

    <!-- Select -->
    <Select
      v-else-if="param.type === 'select'"
      :model-value="(modelValue as string) ?? (param.default as string) ?? ''"
      :options="param.options"
      @update:model-value="v => emit('update:modelValue', v)"
    />

    <!-- Checkbox -->
    <div v-else-if="param.type === 'checkbox'" class="flex items-center gap-2">
      <Checkbox
        :model-value="!!(modelValue ?? param.default)"
        @update:model-value="onCheckboxChange"
      />
    </div>

    <!-- Textarea -->
    <Textarea
      v-else-if="param.type === 'textarea'"
      :data-field="param.key"
      :model-value="String(modelValue ?? param.default ?? '')"
      :placeholder="param.placeholder"
      :rows="3"
      class="text-xs"
      @input="onTextareaInput"
    />

    <!-- Variable reference (text/textarea only) -->
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
                ⚡ {{ t('actionRow.entireOutput') }}
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
                ⚡ {{ act.label }}
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
