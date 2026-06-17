<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Action, ContainerType, ActionStatus, Step } from '../types/types'

const { t } = useI18n()
import { X } from 'lucide-vue-next'
import { getActionDef, getActionLabel, getContainerDef } from '../types/node-registry'
import ActionIcon from './ActionIcon.vue'
import { useVariableRefs } from '../composables/useVariableRefs'
import { cn } from '@/lib/utils'
import Button from './ui/button/Button.vue'

import ParamField from './ParamField.vue'
import ElementPicker from './ElementPicker.vue'



const props = withDefaults(defineProps<{
  action: Action
  containerType: ContainerType
  status?: ActionStatus
  stepId?: string
  stepLabel?: string
  steps?: Step[]
  expanded?: boolean
  siblingActions?: Action[]
}>(), {
  status: undefined,
  stepId: undefined,
  stepLabel: undefined,
  steps: () => [],
  expanded: true,
  siblingActions: () => [],
})

// Find the action immediately before the current one
const prevAction = computed(() => {
  if (!props.siblingActions?.length) return undefined
  const idx = props.siblingActions.findIndex(a => a.id === props.action.id)
  if (idx <= 0) return undefined
  return props.siblingActions[idx - 1]
})

const emit = defineEmits<{
  remove: []
  click: []
  rename: [label: string]
  'update-params': [params: Record<string, unknown>]
  'toggle-expand': []
}>()

const actionDef = computed(() => getActionDef(props.containerType, props.action.type, t))
const displayLabel = () => getActionLabel(props.action, props.containerType, t)

// 变量引用名称
const varName = computed(() => {
  const stepId = props.stepId
  return stepId ? `${stepId}.${props.action.id}` : props.action.id
})

// ─── Variable refs (from composable) ───
const { groupedRefs } = useVariableRefs(
  () => props.steps || [],
  () => props.stepId,
)

// ─── Local params ───
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

// ─── Element picker (snapshot mode) ───
const pickingElement = ref(false)
const canPick = true
const showElementPicker = ref(false)
const pickTargetField = ref('')

function onPickElement(fieldKey: string) {
  pickTargetField.value = fieldKey
  showElementPicker.value = true
}

function onElementPickerSelect(ref: string) {
  if (pickTargetField.value) {
    localParams.value[pickTargetField.value] = ref
    emit('update-params', { ...localParams.value })
  }
  showElementPicker.value = false
  pickTargetField.value = ''
}

function onElementPickerClose() {
  showElementPicker.value = false
  pickTargetField.value = ''
}

// ─── Double-click rename ───
const editing = ref(false)
const editValue = ref('')
const editInput = ref<HTMLInputElement | null>(null)

function startRename() {
  editValue.value = props.action.label || displayLabel()
  editing.value = true
  setTimeout(() => editInput.value?.focus(), 0)
}

function confirmRename() {
  const v = editValue.value.trim()
  if (v && v !== props.action.label) {
    emit('rename', v)
  }
  editing.value = false
}

function cancelRename() {
  editing.value = false
}

// Status color
const statusColor: Record<string, string> = {
  success: 'bg-success',
  running: 'bg-warning',
  error: 'bg-danger',
  idle: 'bg-muted',
}

const statusDotClass = computed(() => {
  return statusColor[props.status || 'idle'] || 'bg-muted'
})

const hasParams = computed(() => (actionDef.value?.params?.length ?? 0) > 0)
const isBrowser = computed(() => props.containerType === 'browser')

// Selector fields (for element picker)
function isSelectorField(key: string): boolean {
  return isBrowser.value && (key === 'selector' || key === 'ref' || key.includes('selector'))
}

</script>

<template>
  <div class="rounded-md border border-border/50 overflow-visible hover:border-border transition-colors">
    <!-- Header (collapsed) -->
    <div
      :class="cn(
        'flex items-center h-[var(--height-action-row)] px-[var(--spacing-card-padding-x)] py-0 gap-2 cursor-pointer select-none group transition-colors hover:bg-accent/50',
      )"
      @click="emit('toggle-expand')"
    >
      <!-- Expand indicator -->
      <span class="text-[10px] text-muted-foreground w-3 shrink-0">
        {{ expanded ? '▼' : '▶' }}
      </span>

      <!-- Status dot -->
      <span :class="cn('w-2 h-2 rounded-full shrink-0', statusDotClass)" />

      <!-- Icon + Label -->
      <ActionIcon :name="getContainerDef(containerType, t).icon" cls="w-4 h-4 shrink-0" />

      <input
        v-if="editing"
        ref="editInput"
        v-model="editValue"
        class="flex-1 min-w-0 bg-transparent border-b border-primary text-xs text-foreground outline-none px-0.5"
        @blur="confirmRename"
        @keydown.enter="confirmRename"
        @keydown.escape="cancelRename"
        @click.stop
      />
      <span
        v-else
        class="flex-1 text-xs text-foreground whitespace-nowrap overflow-hidden text-ellipsis"
        :title="t('actionRow.dblclickRename')"
        @dblclick.stop="startRename"
      >
        {{ displayLabel() }}
      </span>

      <!-- Variable name badge -->
      <span
        class="text-[10px] font-mono text-primary/70 bg-primary/5 px-1.5 py-0.5 rounded shrink-0"
        :title="`引用: ${varName}`"
      >
        {{ varName }}
      </span>

      <!-- Delete -->
      <Button
        variant="ghost"
        size="icon"
        class="text-muted-foreground hover:text-destructive hover:bg-destructive/10 opacity-0 group-hover:opacity-100 transition-opacity h-6 w-6 shrink-0"
        :aria-label="t('actionRow.deleteActionAria')"
        @click.stop="emit('remove')"
      >
        <X class="w-3 h-3" />
      </Button>
    </div>

    <!-- Expanded: parameters or "no params" message -->
    <Transition name="action-expand">
      <div v-if="expanded" class="px-3 pb-3 pt-1 border-t border-border/50 space-y-2.5">
        <!-- No params message -->
        <div v-if="!hasParams" class="text-[11px] text-muted-foreground/60 py-1 text-center">
          {{ t('actionRow.noParamsHint') }}
        </div>

        <!-- Data flow hint -->
        <div v-if="hasParams && siblingActions" class="text-[11px] px-2 py-1 rounded bg-primary/5 text-muted-foreground flex items-center gap-1">
          <template v-if="prevAction">
            {{ t('actionRow.dataFrom') }} <span class="font-medium text-foreground">{{ prevAction.label || prevAction.type }}</span>
          </template>
          <template v-else>
            {{ t('actionRow.noUpstream') }}
          </template>
        </div>

        <!-- Param fields -->
        <ParamField
          v-for="param in actionDef!.params"
          :key="param.key"
          :param="param"
          :model-value="localParams[param.key] ?? param.default"
          :grouped-refs="groupedRefs"
          :show-element-picker="canPick && isSelectorField(param.key)"
          :picking-element="pickingElement"
          @update:model-value="v => onParamChange(param.key, v)"
          @pick-element="onPickElement(param.key)"
        />

      </div>
    </Transition>

    <!-- Element Picker Modal -->
    <ElementPicker
      v-if="showElementPicker"
      @select="onElementPickerSelect"
      @close="onElementPickerClose"
    />
  </div>
</template>

<style scoped>
.action-expand-enter-active,
.action-expand-leave-active {
  transition: all 0.2s ease;
  overflow: hidden;
}
.action-expand-enter-from,
.action-expand-leave-to {
  max-height: 0;
  opacity: 0;
  padding-top: 0;
  padding-bottom: 0;
  border-top-width: 0;
}
.action-expand-enter-to,
.action-expand-leave-from {
  max-height: 500px;
  opacity: 1;
}
</style>
