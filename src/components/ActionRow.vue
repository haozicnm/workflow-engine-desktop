<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import type { Action, ContainerType, ActionStatus, Step } from '../types/workflow'
import { getActionDef, getActionLabel } from '../types/workflow'
import { safeInvoke } from '../utils/tauri'
import { cn } from '@/lib/utils'
import Button from './ui/button/Button.vue'
import Card from './ui/card/Card.vue'
import CardContent from './ui/card/CardContent.vue'
import Input from './ui/input/Input.vue'
import Label from './ui/label/Label.vue'
import Textarea from './ui/textarea/Textarea.vue'
import Checkbox from './ui/checkbox/Checkbox.vue'
import Select from './ui/select/Select.vue'

const props = defineProps<{
  action: Action
  containerType: ContainerType
  status?: ActionStatus
  stepId?: string
  stepLabel?: string
  steps?: Step[]
  expanded?: boolean
}>()

const emit = defineEmits<{
  remove: []
  click: []
  rename: [label: string]
  'update-params': [params: Record<string, unknown>]
  'toggle-expand': []
}>()

const actionDef = computed(() => getActionDef(props.containerType, props.action.type))
const displayLabel = () => getActionLabel(props.action, props.containerType)

// 变量引用名称：stepId.actionLabel
const varName = computed(() => {
  const stepId = props.stepId
  const actionLabel = displayLabel()
  return stepId ? `${stepId}.${actionLabel}` : actionLabel
})

// ─── Local params (for editing) ───
const localParams = ref<Record<string, unknown>>({ ...props.action.params })

watch(
  () => props.action.id,
  () => { localParams.value = { ...props.action.params } },
)

watch(
  () => props.action.params,
  (newParams) => { localParams.value = { ...newParams } },
  { deep: true },
)

function onParamChange(key: string, value: unknown) {
  localParams.value[key] = value
  emit('update-params', { ...localParams.value })
}

function onTextInput(key: string, e: Event) {
  onParamChange(key, (e.target as HTMLInputElement).value)
}

function onNumberInput(key: string, e: Event) {
  const v = (e.target as HTMLInputElement).value
  onParamChange(key, v === '' ? '' : Number(v))
}

function onCheckboxChange(key: string, val: boolean) {
  onParamChange(key, val)
}

function onTextareaInput(key: string, e: Event) {
  onParamChange(key, (e.target as HTMLTextAreaElement).value)
}

// ─── Element picker ───
const pickingElement = ref(false)

function getStepUrl(): string | undefined {
  if (!props.steps?.length) return undefined
  for (let i = props.steps.length - 1; i >= 0; i--) {
    const step = props.steps[i]
    if (step.config?.url) return step.config.url as string
    const navAction = step.actions?.find(a => a.type === 'navigate')
    if (navAction?.params?.url) return navAction.params.url as string
  }
  return undefined
}

async function onPickElement(fieldKey: string) {
  pickingElement.value = true
  try {
    const url = getStepUrl()
    const result = await safeInvoke<{ selector: string }>('browser_pick_element', { url: url || null })
    if (result?.selector) {
      localParams.value[fieldKey] = result.selector
      emit('update-params', { ...localParams.value })
    }
  } catch (e) {
    console.error('元素选择失败:', e)
  } finally {
    pickingElement.value = false
  }
}

// ─── Variable reference ───
interface VarRef {
  id: string
  label: string
  icon: string
  type: 'step' | 'action'
}

const availableRefs = computed<VarRef[]>(() => {
  if (!props.steps?.length) return []
  const refs: VarRef[] = []
  for (const step of props.steps) {
    const stepIcon = step.type === 'browser' ? '🌐' : step.type === 'excel' ? '📊' : step.type === 'word' ? '📝' : step.type === 'logic' ? '🔀' : '⚡'
    // 步骤级引用：{{stepId}} → 整个步骤输出
    refs.push({ id: step.id, label: step.label, icon: stepIcon, type: 'step' })
    // 动作级引用：{{stepId.actionLabel}} → 单个动作输出
    for (const action of (step.actions || [])) {
      const actionLabel = action.label || action.type
      refs.push({ id: `${step.id}.${actionLabel}`, label: `${step.label} › ${actionLabel}`, icon: '⚡', type: 'action' })
    }
  }
  return refs
})

function insertRef(fieldKey: string, refId: string) {
  const input = document.querySelector(`[data-field="${fieldKey}"]`) as HTMLInputElement | HTMLTextAreaElement
  if (!input) return
  const refText = `{{${refId}}}`
  const start = input.selectionStart ?? 0
  const end = input.selectionEnd ?? 0
  const current = String(localParams.value[fieldKey] ?? '')
  const newVal = current.slice(0, start) + refText + current.slice(end)
  localParams.value[fieldKey] = newVal
  emit('update-params', { ...localParams.value })
}

// ─── Double-click rename ───
const editing = ref(false)
const editValue = ref('')
const editInput = ref<HTMLInputElement | null>(null)

function startRename() {
  editValue.value = displayLabel()
  editing.value = true
  setTimeout(() => editInput.value?.focus(), 0)
}

function confirmRename() {
  const v = editValue.value.trim()
  if (v && v !== displayLabel()) {
    emit('rename', v)
  }
  editing.value = false
}

function cancelRename() {
  editing.value = false
}

const statusColor: Record<string, string> = {
  success: 'bg-success',
  running: 'bg-warning',
  error: 'bg-danger',
  idle: 'bg-muted',
}

function getColorClass(): string {
  return statusColor[props.status || 'idle'] || 'bg-muted'
}

const hasParams = computed(() => actionDef.value && actionDef.value.params.length > 0)
</script>

<template>
  <Card
    class="group shadow-none py-0 transition-all"
    :class="expanded ? 'border-primary/30' : 'cursor-pointer hover:bg-secondary/50'"
  >
    <!-- Header row -->
    <CardContent
      class="flex items-center h-9 px-4 py-0 gap-2"
      :class="hasParams ? 'cursor-pointer' : ''"
      @click="hasParams ? emit('toggle-expand') : emit('click')"
    >
      <!-- Expand indicator -->
      <span v-if="hasParams" class="shrink-0 text-[10px] text-muted-foreground w-3">
        {{ expanded ? '▼' : '▶' }}
      </span>
      <span v-else class="w-3" />

      <!-- Status dot -->
      <span :class="cn('w-2 h-2 rounded-full shrink-0', getColorClass())" />

      <!-- Icon + Label -->
      <span class="flex flex-1 items-center gap-1.5 min-w-0 overflow-hidden">
        <span class="shrink-0 text-sm">{{ actionDef?.icon || '⚡' }}</span>
        <span class="flex-1 min-w-0 overflow-hidden">
          <input
            v-if="editing"
            ref="editInput"
            v-model="editValue"
            class="w-full bg-transparent border-b border-primary text-sm text-foreground outline-none px-0.5"
            @blur="confirmRename"
            @keydown.enter="confirmRename"
            @keydown.escape="cancelRename"
            @click.stop
          />
          <span
            v-else
            class="block truncate text-sm text-foreground"
            title="双击重命名"
            @dblclick.stop="startRename"
          >
            {{ displayLabel() }}
          </span>
        </span>
      </span>

      <!-- Variable name badge -->
      <span
        class="shrink-0 text-[10px] font-mono text-muted-foreground/70 bg-muted/50 px-1.5 py-0.5 rounded max-w-[140px] truncate"
        :title="`引用: {{${varName}.output}}`"
      >
        {{ varName }}
      </span>

      <!-- Remove button -->
      <Button
        variant="ghost"
        size="icon"
        class="h-5 w-5 shrink-0 text-muted-foreground hover:text-destructive opacity-0 group-hover:opacity-100 transition-opacity"
        @click.stop="emit('remove')"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
      </Button>
    </CardContent>

    <!-- Expanded: parameter editing -->
    <div v-if="expanded && hasParams" class="px-3 pb-3 pt-1 border-t border-border/50 space-y-2.5">
      <div
        v-for="param in actionDef!.params"
        :key="param.key"
      >
        <!-- Label -->
        <Label class="text-[11px] text-muted-foreground block mb-1">{{ param.label }}</Label>

        <!-- Text input -->
        <div v-if="param.type === 'text'" class="flex gap-1">
          <Input
            :data-field="param.key"
            type="text"
            :model-value="(localParams[param.key] as string) ?? (param.default as string) ?? ''"
            :placeholder="param.placeholder"
            class="flex-1 h-8 text-xs"
            @input="onTextInput(param.key, $event)"
          />
          <Button
            v-if="containerType === 'browser' && (param.key === 'selector' || param.key.includes('selector'))"
            variant="outline"
            size="sm"
            class="h-8 w-8 p-0 shrink-0"
            :class="pickingElement ? 'text-warning' : ''"
            :title="pickingElement ? '选择中...' : '🎯 从页面选择元素'"
            @click="onPickElement(param.key)"
          >🎯</Button>
        </div>

        <!-- Number input -->
        <Input
          v-else-if="param.type === 'number'"
          type="number"
          :model-value="(localParams[param.key] as string) ?? (param.default as string) ?? ''"
          :placeholder="param.placeholder"
          class="h-8 text-xs"
          @input="onNumberInput(param.key, $event)"
        />

        <!-- Select -->
        <Select
          v-else-if="param.type === 'select'"
          :model-value="(localParams[param.key] as string) ?? (param.default as string) ?? ''"
          :options="param.options"
          @update:model-value="v => onParamChange(param.key, v)"
        />

        <!-- Checkbox -->
        <div v-else-if="param.type === 'checkbox'" class="flex items-center gap-2">
          <Checkbox
            :model-value="!!(localParams[param.key] ?? param.default)"
            @update:model-value="(v) => onCheckboxChange(param.key, v)"
          />
        </div>

        <!-- Textarea -->
        <Textarea
          v-else-if="param.type === 'textarea'"
          :data-field="param.key"
          :model-value="String(localParams[param.key] ?? param.default ?? '')"
          :placeholder="param.placeholder"
          :rows="3"
          class="text-xs"
          @input="onTextareaInput(param.key, $event)"
        />

        <!-- Variable reference (text/textarea) -->
        <div v-if="(param.type === 'text' || param.type === 'textarea') && availableRefs.length > 0" class="mt-1">
          <div class="flex items-center gap-1.5">
            <span class="text-[10px] text-muted-foreground shrink-0">🔗</span>
            <select
              class="flex-1 h-6 text-[11px] bg-background border border-border rounded px-1.5 text-foreground cursor-pointer hover:border-primary/50 transition-colors"
              @change="(e: Event) => { const v = (e.target as HTMLSelectElement).value; if (v) insertRef(param.key, v); (e.target as HTMLSelectElement).value = ''; }"
            >
              <option value="">— 引用变量 —</option>
              <optgroup v-if="availableRefs.some(r => r.type === 'step')" label="步骤">
                <option v-for="ref in availableRefs.filter(r => r.type === 'step')" :key="ref.id" :value="ref.id">
                  {{ ref.icon }} {{ ref.label }}
                </option>
              </optgroup>
              <optgroup v-if="availableRefs.some(r => r.type === 'action')" label="动作">
                <option v-for="ref in availableRefs.filter(r => r.type === 'action')" :key="ref.id" :value="ref.id">
                  {{ ref.label }}
                </option>
              </optgroup>
            </select>
          </div>
          <!-- 已有引用标签 -->
          <div
            v-if="typeof localParams[param.key] === 'string' && (localParams[param.key] as string).includes('{{')"
            class="mt-0.5 flex flex-wrap gap-1"
          >
            <span
              v-for="m in (localParams[param.key] as string).match(/\{\{[^}]+\}\}/g) || []"
              :key="m"
              class="inline-flex items-center px-1 py-0.5 text-[10px] font-mono bg-primary/10 text-primary rounded"
            >
              {{ m }}
            </span>
          </div>
        </div>
      </div>
    </div>
  </Card>
</template>
