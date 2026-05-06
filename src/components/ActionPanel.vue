<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import type { Action, ContainerType, Step } from '../types/workflow'
import { getActionDef, getActionLabel } from '../types/workflow'
import { safeInvoke } from '../utils/tauri'
import Button from './ui/button/Button.vue'
import Input from './ui/input/Input.vue'
import Label from './ui/label/Label.vue'
import Textarea from './ui/textarea/Textarea.vue'
import Checkbox from './ui/checkbox/Checkbox.vue'
import Select from './ui/select/Select.vue'
import { cn } from '@/lib/utils'

const props = defineProps<{
  action: Action
  containerType: ContainerType
  steps?: Step[]
}>()

const emit = defineEmits<{
  'update-params': [params: Record<string, unknown>]
  close: []
}>()

const actionDef = computed(() => getActionDef(props.containerType, props.action.type))
const localParams = ref<Record<string, unknown>>({ ...props.action.params })

// ─── Element picker (T15) ───
const pickingElement = ref(false)

// ─── Continuous pick session ───
const pickSessionActive = ref(false)
const pickSessionCurrentField = ref<string | null>(null)

function getStepUrl(): string | undefined {
  if (!props.steps?.length) return undefined
  for (let i = props.steps.length - 1; i >= 0; i--) {
    const step = props.steps[i]
    if (step.config?.url) return step.config.url as string
    const navAction = step.actions.find(a => a.type === 'navigate')
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

// 获取所有 selector 类型的字段
const selectorFields = computed(() => {
  if (!actionDef.value || props.containerType !== 'browser') return []
  return actionDef.value.params.filter(
    p => p.type === 'text' && (p.key === 'selector' || p.key.includes('selector'))
  )
})

async function startPickSession() {
  if (pickSessionActive.value) {
    // 停止
    await stopPickSession()
    return
  }
  if (!selectorFields.value.length) return

  pickSessionActive.value = true
  const url = getStepUrl()

  try {
    // 1. 启动拾取会话
    await safeInvoke('browser_pick_session_start', { url: url || null })

    // 2. 从第一个 selector 字段开始
    pickNext(selectorFields.value[0].key)
  } catch (e) {
    console.error('启动连续拾取失败:', e)
    pickSessionActive.value = false
  }
}

async function pickNext(fieldKey: string) {
  if (!pickSessionActive.value) return
  pickSessionCurrentField.value = fieldKey

  try {
    const result = await safeInvoke<{ selector: string }>('browser_pick_next')
    if (!pickSessionActive.value) return

    if (result?.selector) {
      localParams.value[fieldKey] = result.selector
      emit('update-params', { ...localParams.value })

      // 找下一个 selector 字段
      const fields = selectorFields.value
      const currentIdx = fields.findIndex(f => f.key === fieldKey)
      const nextField = currentIdx >= 0 && currentIdx + 1 < fields.length ? fields[currentIdx + 1] : null

      if (nextField) {
        // 继续拾取下一个
        pickNext(nextField.key)
      } else {
        // 所有 selector 字段都拾取完了
        await stopPickSession()
      }
    }
  } catch (e) {
    console.error('连续拾取失败:', e)
    await stopPickSession()
  }
}

async function stopPickSession() {
  pickSessionActive.value = false
  pickSessionCurrentField.value = null
  try {
    await safeInvoke('browser_pick_session_stop')
  } catch (e) {
    // ignore
  }
}

// ─── Variable reference ───
interface VarRef {
  id: string        // 插入标识，如 "容器名_动作名"
  label: string     // 显示名称
  icon: string
  type: 'step' | 'action'
}

const availableRefs = computed<VarRef[]>(() => {
  if (!props.steps?.length) return []
  const refs: VarRef[] = []
  for (const step of props.steps) {
    // 步骤级别变量
    const stepIcon = step.type === 'browser' ? '🌐' : step.type === 'excel' ? '📊' : step.type === 'word' ? '📝' : step.type === 'logic' ? '🔀' : '⚡'
    refs.push({ id: step.label, label: step.label, icon: stepIcon, type: 'step' })
    // 动作级别变量
    for (const action of (step.actions || [])) {
      const actionLabel = action.label || action.type
      refs.push({ id: `${step.label}_${actionLabel}`, label: `${step.label} › ${actionLabel}`, icon: '⚡', type: 'action' })
    }
  }
  return refs
})

function insertRef(fieldKey: string, refId: string) {
  const input = document.querySelector(`[data-field="${fieldKey}"]`) as HTMLInputElement | HTMLTextAreaElement
  if (!input) return
  const refText = `{{${refId}.output}}`
  const start = input.selectionStart ?? 0
  const end = input.selectionEnd ?? 0
  const current = String(localParams.value[fieldKey] ?? '')
  const newVal = current.slice(0, start) + refText + current.slice(end)
  localParams.value[fieldKey] = newVal
  emit('update-params', { ...localParams.value })
}

watch(
  () => props.action.id,
  () => {
    // 切换动作时停止拾取会话
    if (pickSessionActive.value) {
      stopPickSession()
    }
    localParams.value = { ...props.action.params }
  },
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

function onSelectChange(key: string, e: Event) {
  onParamChange(key, (e.target as HTMLSelectElement).value)
}

function onCheckboxChange(key: string, val: boolean) {
  onParamChange(key, val)
}

function onTextareaInput(key: string, e: Event) {
  onParamChange(key, (e.target as HTMLTextAreaElement).value)
}

function stepTypeIcon(type: string): string {
  if (type === 'browser') return '🌐'
  if (type === 'excel') return '📊'
  if (type === 'word') return '📝'
  return '🔀'
}
</script>

<template>
  <div class="bg-card border border-border rounded-lg p-4 max-w-[400px] shadow-lg">
    <!-- Header -->
    <div class="flex items-center gap-2 mb-4 pb-3 border-b border-border">
      <span class="text-lg">{{ actionDef?.icon || '⚡' }}</span>
      <span class="flex-1 text-sm font-medium text-foreground">
        {{ getActionLabel(action, containerType) }}
      </span>
      <!-- 连续拾取按钮 -->
      <Button
        v-if="selectorFields.length > 0"
        variant="outline"
        size="sm"
        class="h-6 text-[11px] px-2"
        :class="pickSessionActive ? 'bg-primary/10 border-primary text-primary' : ''"
        :title="pickSessionActive ? '点击结束连续拾取' : '连续拾取所有 selector 字段'"
        @click="startPickSession"
      >
        {{ pickSessionActive ? '⏹ 结束' : '🎯 连续拾取' }}
      </Button>
      <Button variant="ghost" size="icon" class="h-6 w-6" @click="emit('close')">
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
      </Button>
    </div>

    <!-- 拾取状态提示 -->
    <div v-if="pickSessionActive && pickSessionCurrentField" class="mb-3 px-2 py-1.5 bg-primary/10 border border-primary/30 rounded text-xs text-primary flex items-center gap-1.5">
      <span class="relative flex h-2 w-2">
        <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-primary opacity-75"></span>
        <span class="relative inline-flex rounded-full h-2 w-2 bg-primary"></span>
      </span>
      拾取中 — 在浏览器中点击元素填充「{{ actionDef?.params.find(p => p.key === pickSessionCurrentField)?.label || pickSessionCurrentField }}」
    </div>

    <!-- Param fields -->
    <div v-if="actionDef && actionDef.params.length > 0" class="space-y-3">
      <div
        v-for="param in actionDef.params"
        :key="param.key"
      >
        <!-- Label -->
        <div class="flex items-center gap-1 mb-1.5">
          <Label class="text-xs text-muted-foreground">{{ param.label }}</Label>
        </div>

        <!-- Text input -->
        <div v-if="param.type === 'text'" class="flex gap-1">
          <Input
            :data-field="param.key"
            type="text"
            :model-value="(localParams[param.key] as string) ?? (param.default as string) ?? ''"
            :placeholder="param.placeholder"
            class="flex-1 h-8 text-xs"
            :class="pickSessionActive && pickSessionCurrentField === param.key ? 'ring-2 ring-primary animate-pulse' : ''"
            @input="onTextInput(param.key, $event)"
          />
          <Button
            v-if="containerType === 'browser' && (param.key === 'selector' || param.key.includes('selector'))"
            variant="outline"
            size="sm"
            class="h-8 w-8 p-0 shrink-0"
            :class="(pickingElement || (pickSessionActive && pickSessionCurrentField === param.key)) ? 'text-warning' : ''"
            :title="pickSessionActive ? '连续拾取中' : pickingElement ? '选择中...' : '🎯 从页面选择元素'"
            @click="pickSessionActive ? null : onPickElement(param.key)"
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
          :rows="4"
          class="text-xs"
          @input="onTextareaInput(param.key, $event)"
        />

        <!-- Variable reference selector (text/textarea 字段下方) -->
        <div v-if="(param.type === 'text' || param.type === 'textarea') && availableRefs.length > 0" class="mt-1.5">
          <div class="flex items-center gap-1.5">
            <span class="text-[11px] text-muted-foreground shrink-0">🔗 引用</span>
            <select
              class="flex-1 h-7 text-xs bg-background border border-border rounded px-2 text-foreground cursor-pointer hover:border-primary/50 transition-colors"
              @change="(e: Event) => { const v = (e.target as HTMLSelectElement).value; if (v) insertRef(param.key, v); (e.target as HTMLSelectElement).value = ''; }"
            >
              <option value="">— 选择变量 —</option>
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
          <!-- 显示当前字段中已有的引用 -->
          <div
            v-if="typeof localParams[param.key] === 'string' && (localParams[param.key] as string).includes('{{')"
            class="mt-1 flex flex-wrap gap-1"
          >
            <span
              v-for="m in (localParams[param.key] as string).match(/\{\{[^}]+\}\}/g) || []"
              :key="m"
              class="inline-flex items-center gap-0.5 px-1.5 py-0.5 text-[10px] font-mono bg-primary/10 text-primary rounded"
            >
              {{ m }}
            </span>
          </div>
        </div>
      </div>
    </div>

    <!-- No params -->
    <div v-else class="text-center text-muted-foreground text-sm py-3">
      此动作没有可配置的参数
    </div>
  </div>
</template>
