<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Settings, Plus, Check, X, Loader2 } from 'lucide-vue-next'
import type { Step, StepRunState, ErrorStrategy } from '../types/types'
import { getContainerDef, isContainerType, getContainerColorVar } from '../types/node-registry'
import ActionIcon from './ActionIcon.vue'
import { useVariableRefs } from '../composables/useVariableRefs'
import ActionRow from './ActionRow.vue'
import LogicBranch from './LogicBranch.vue'
import ParamField from './ParamField.vue'
import Card from './ui/card/Card.vue'
import Button from './ui/button/Button.vue'
import Badge from './ui/badge/Badge.vue'
import { cn } from '@/lib/utils'

const { t } = useI18n()

const props = defineProps<{
  step: Step
  runState?: StepRunState
  totalSteps?: number
  currentStepIndex?: number
  steps?: Step[]  // 工作流所有步骤（传给 LogicBranch / ParamField 用于变量引用）
}>()

const emit = defineEmits<{
  'add-action': [stepId: string]
  'remove-action': [stepId: string, actionId: string]
  'action-click': [stepId: string, actionId: string]
  'rename-action': [stepId: string, actionId: string, label: string]
  'update-action-params': [stepId: string, actionId: string, params: Record<string, unknown>]
  'remove-step': [stepId: string]
  'rename-step': [stepId: string, label: string]
  'update-condition': [stepId: string, condition: string]
  'update-condition-group': [stepId: string, group: import('../types/types').LogicConditionGroup]
  'update-run-condition': [stepId: string, condition: import('../types/types').StepCondition | null]
  'update-step-config': [stepId: string, key: string, value: unknown]
  'open-config': [stepId: string]
  'update-error-strategy': [stepId: string, strategy: ErrorStrategy]
}>()

// ─── Action expand/collapse ───
const expandedActionId = ref<string | null>(null)

function toggleActionExpand(actionId: string) {
  expandedActionId.value = expandedActionId.value === actionId ? null : actionId
}

const containerDef = computed(() => getContainerDef(props.step.type, t))
const isContainer = computed(() => isContainerType(props.step.type))

// ─── Variable refs (for simple steps) ───
const { groupedRefs } = useVariableRefs(
  () => props.steps || [],
  () => props.step.id,
)

// ─── Double-click rename step ───
const editingLabel = ref(false)
const labelEditValue = ref('')
const labelEditInput = ref<HTMLInputElement | null>(null)

function startRenameStep() {
  labelEditValue.value = props.step.label
  editingLabel.value = true
  setTimeout(() => labelEditInput.value?.focus(), 0)
}

function confirmRenameStep() {
  const v = labelEditValue.value.trim()
  if (v && v !== props.step.label) {
    emit('rename-step', props.step.id, v)
  }
  editingLabel.value = false
}

function cancelRenameStep() {
  editingLabel.value = false
}

// ─── Config param change (simple steps) ───
function onConfigParamChange(key: string, value: unknown) {
  emit('update-step-config', props.step.id, key, value)
}

// ─── Status ───

// ─── Output ───
const showOutput = ref(false)
const formattedOutput = computed(() => {
  const out = props.runState?.output
  if (!out) return ''
  if (typeof out === 'string') return out
  return JSON.stringify(out, null, 2)
})

// ─── ⋯ Menu ───
const showMenu = ref(false)
const menuBtnRef = ref<InstanceType<typeof Button> | null>(null)
const menuPosStyle = ref<Record<string, string>>({})

function toggleMenu() {
  showMenu.value = !showMenu.value
  if (showMenu.value && menuBtnRef.value?.$el) {
    const rect = (menuBtnRef.value.$el as HTMLElement).getBoundingClientRect()
    menuPosStyle.value = {
      top: `${rect.bottom + 4}px`,
      left: `${rect.right - 208}px`, // 208px = w-52
    }
  }
}

// ─── Error strategy ───
const showErrMenu = ref(false)
const errStrategyLabel = computed(() => {
  const s = props.step.onError
  if (!s || s === 'fail') return t('editor.errorFail')
  if (s === 'ignore') return t('editor.errorIgnore')
  if (typeof s === 'object' && 'branch' in s) return t('editor.errorBranch')
  return t('editor.errorFail')
})

function setErrStrategy(s: ErrorStrategy) {
  emit('update-error-strategy', props.step.id, s)
  showErrMenu.value = false
  showMenu.value = false
}

// ─── Condition (条件执行) ───
const showConditionMenu = ref(false)
const logicSteps = computed(() => (props.steps || []).filter(s => s.type === 'logic'))
const conditionLabel = computed(() => {
  const rc = props.step.runCondition
  if (!rc) return ''
  const refStep = (props.steps || []).find(s => s.id === rc.ref)
  const name = refStep?.label || rc.ref
  if (rc.when === 'both') return `${name}`
  if (rc.when === 'merge') return `↔ ${name}`
  return `${name}=${rc.when === 'true' ? 'T' : 'F'}`
})
function setCondition(ref: string, when: 'true' | 'false' | 'both' | 'merge') {
  if (!ref) {
    emit('update-run-condition', props.step.id, null)
  } else {
    emit('update-run-condition', props.step.id, { ref, when })
  }
  showConditionMenu.value = false
}
function removeCondition() {
  emit('update-run-condition', props.step.id, null)
  showConditionMenu.value = false
  showMenu.value = false
}

function formatDuration(ms?: number): string {
  if (!ms) return ''
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

function colorAt15(colorVar: string): string {
  return `color-mix(in srgb, ${colorVar} 15%, transparent)`
}

const containerColorVar = computed(() => getContainerColorVar(props.step.type))

// Close all sub-menus when ⋯ closes
function closeAllMenus() {
  showMenu.value = false
  showErrMenu.value = false
  showConditionMenu.value = false
}
</script>

<template>
  <Card
    :color="containerColorVar"
    :class="cn(
      'relative',
      runState?.status === 'running' && 'animate-pulse-step',
    )"
  >
    <!-- ═══ Header bar (slim) ═══ -->
    <div
      class="h-[var(--height-step-header)] px-[var(--spacing-card-padding-x)] flex items-center cursor-pointer gap-2 select-none transition-colors group"
      :style="{ background: colorAt15(containerColorVar) }"
      @click="step.expanded = !step.expanded"
    >
      <!-- Icon -->
      <ActionIcon :name="containerDef.icon" cls="w-4 h-4 shrink-0" />

      <!-- Label -->
      <input
        v-if="editingLabel"
        ref="labelEditInput"
        v-model="labelEditValue"
        class="flex-1 min-w-0 bg-transparent border-b border-primary text-sm font-medium text-foreground outline-none px-0.5"
        @blur="confirmRenameStep"
        @keydown.enter="confirmRenameStep"
        @keydown.escape="cancelRenameStep"
        @click.stop
      />
      <span
        v-else
        class="flex-1 text-sm font-medium text-foreground whitespace-nowrap overflow-hidden text-ellipsis"
        :title="t('stepCard.dblclickRename')"
        @dblclick.stop="startRenameStep"
      >
        {{ step.label }}
      </span>

      <!-- Badges (always visible) -->
      <Badge
        v-if="step.onError && step.onError !== 'fail'"
        variant="warning"
        class="text-[10px] px-1.5 py-0.5 shrink-0"
        :title="errStrategyLabel"
      >{{ errStrategyLabel }}</Badge>

      <Badge
        v-if="step.runCondition"
        variant="outline"
        class="text-[10px] px-1.5 py-0.5 border-warning/40 text-warning bg-warning/10 shrink-0"
        :title="conditionLabel"
      >{{ conditionLabel }}</Badge>

      <Badge
        v-if="containerDef?.dangerous"
        variant="warning"
        class="text-[10px] px-1.5 py-0.5 shrink-0"
        :title="t('stepCard.dangerousWarning')"
      >{{ t('stepCard.dangerousBadge') }}</Badge>

      <!-- Duration + Status -->
      <span v-if="runState?.duration" class="text-[10px] text-muted-foreground font-mono shrink-0">
        {{ formatDuration(runState.duration) }}
      </span>
      <!-- Status icon: success=check, error=X, running=spinner, idle=dot -->
      <Check v-if="runState?.status === 'success'" class="w-3.5 h-3.5 text-success shrink-0" />
      <X v-else-if="runState?.status === 'error'" class="w-3.5 h-3.5 text-danger shrink-0" />
      <Loader2 v-else-if="runState?.status === 'running'" class="w-3.5 h-3.5 text-warning animate-spin shrink-0" />
      <span v-else class="w-2 h-2 rounded-full bg-muted shrink-0" />

      <!-- ⋯ Menu button -->
      <div class="relative" @click.stop>
        <Button
          ref="menuBtnRef"
          variant="ghost"
          size="icon"
          class="text-muted-foreground hover:text-foreground opacity-40 group-hover:opacity-100 transition-opacity"
          :title="t('stepCard.moreActions')"
          :aria-label="t('stepCard.moreActions')"
          @click="toggleMenu"
        >⋯</Button>
      </div>

      <!-- Collapse toggle -->
      <span class="text-[11px] text-muted-foreground shrink-0 w-4 text-center">
        {{ step.expanded ? '▼' : '▶' }}
      </span>

      <!-- Remove button -->
      <Button
        variant="ghost"
        size="icon"
        class="text-muted-foreground hover:text-destructive hover:bg-destructive/10 opacity-40 group-hover:opacity-100 transition-opacity"
        :aria-label="t('stepCard.deleteStepAria')"
        @click.stop="emit('remove-step', step.id)"
      >×</Button>
    </div>

    <!-- ⋯ Dropdown (teleported to body) -->
    <Teleport to="body">
      <div
        v-if="showMenu"
        class="fixed inset-0 z-40"
        @click="closeAllMenus"
      />
      <div
        v-if="showMenu"
        class="fixed z-[60] w-52 bg-background border border-border rounded-md shadow-lg py-1"
        :style="menuPosStyle"
      >
          <!-- 容器设置 -->
          <Button
            variant="ghost"
            class="w-full justify-start px-3 py-2 text-sm"
            @click="emit('open-config', step.id); closeAllMenus()"
          >
            <Settings class="w-4 h-4 inline" /> {{ t('stepCard.containerSettings') }}
          </Button>

          <!-- 条件执行 -->
          <div v-if="step.type !== 'logic' && logicSteps.length > 0" class="border-t border-border mt-1 pt-1">
            <div class="px-3 py-1 text-[10px] text-muted-foreground uppercase tracking-wide">{{ t('editor.runCondition') }}</div>
            <div
              v-for="ls in logicSteps"
              :key="ls.id"
              class="border-b border-border/50 last:border-b-0"
            >
              <div class="px-3 py-1 text-[11px] text-muted-foreground bg-muted/30">
                <ActionIcon :name="getContainerDef(ls.type, t).icon" cls="w-3.5 h-3.5 inline" /> {{ ls.label }}
              </div>
              <Button
                v-for="opt in [
                  { value: 'true', icon: 'CheckCircle', label: 'True' },
                  { value: 'false', icon: 'XCircle', label: 'False' },
                  { value: 'both', icon: 'RefreshCw', label: t('editor.runConditionNone') },
                  { value: 'merge', icon: 'Merge', label: t('editor.errorBranch') },
                ]"
                :key="opt.value"
                variant="ghost"
                :class="cn(
                  'w-full justify-start px-3 py-1.5 text-sm',
                  step.runCondition?.ref === ls.id && step.runCondition?.when === opt.value ? 'bg-accent' : '',
                )"
                @click="setCondition(ls.id, opt.value as 'true' | 'false' | 'both' | 'merge')"
              >
                <ActionIcon :name="opt.icon" cls="w-3.5 h-3.5 inline" /> {{ opt.label }}
              </Button>
            </div>
            <Button
              v-if="step.runCondition"
              variant="ghost"
              class="w-full justify-start px-3 py-1.5 text-sm text-destructive hover:bg-destructive/10 hover:text-destructive"
              @click="removeCondition"
            >
              {{ t('stepCard.removeCondition') }}
            </Button>
          </div>

          <!-- 错误策略 -->
          <div class="border-t border-border mt-1 pt-1">
            <div class="px-3 py-1 text-[10px] text-muted-foreground uppercase tracking-wide">{{ t('editor.errorStrategy') }}</div>
            <Button
              v-for="(opt, key) in {
                fail: { icon: 'CircleStop', label: t('editor.errorFail'), desc: t('stepCard.errDescFail') },
                ignore: { icon: 'CircleAlert', label: t('editor.errorIgnore'), desc: t('stepCard.errDescIgnore') },
                branch: { icon: 'ArrowRightLeft', label: t('editor.errorBranch'), desc: t('stepCard.errDescBranch') },
              }"
              :key="key"
              variant="ghost"
              :class="cn(
                'w-full justify-start px-3 py-1.5 text-sm',
                ((!step.onError && key === 'fail') || step.onError === key || (key === 'branch' && typeof step.onError === 'object' && 'branch' in step.onError)) ? 'bg-accent' : '',
              )"
              @click="key === 'fail' ? setErrStrategy('fail') : key === 'ignore' ? setErrStrategy('ignore') : setErrStrategy({ branch: '' })"
            >
              <ActionIcon :name="opt.icon" cls="w-4 h-4" />
              <div class="flex flex-col">
                <span>{{ opt.label }}</span>
                <span class="text-[10px] text-muted-foreground">{{ opt.desc }}</span>
              </div>
            </Button>
          </div>
        </div>
      </Teleport>

    <!-- Progress bar (running) -->
    <div v-if="runState?.status === 'running'" class="h-0.5 bg-secondary overflow-hidden">
      <div class="h-full w-2/5 bg-warning rounded-full animate-progress-slide" />
    </div>

    <!-- ═══ Body (expandable) ═══ -->
    <div v-show="step.expanded" class="px-[var(--spacing-card-padding-x)] py-[var(--spacing-card-padding-y)] bg-card border-t border-border">
      <!-- Simple step → ParamField (with variable refs!) -->
      <template v-if="!isContainer && step.type !== 'logic'">
        <ParamField
          v-for="param in containerDef.params"
          :key="param.key"
          :param="param"
          :model-value="step.config[param.key] ?? param.default"
          :grouped-refs="groupedRefs"
          class="mb-2"
          @update:model-value="v => onConfigParamChange(param.key, v)"
        />
      </template>

      <!-- Logic type -->
      <LogicBranch
        v-else-if="step.type === 'logic'"
        :step="step"
        :run-state="runState"
        :steps="steps"
        @update-condition="(id, c) => emit('update-condition', id, c)"
        @update-condition-group="(id, g) => emit('update-condition-group', id, g)"
        @open-config="(sId: string) => emit('open-config', sId)"
        @add-action="(id: string) => emit('add-action', id)"
        @remove-action="(id: string, aId: string) => emit('remove-action', id, aId)"
        @action-click="(id: string, aId: string) => emit('action-click', id, aId)"
        @rename-action="(id: string, aId: string, label: string) => emit('rename-action', id, aId, label)"
        @remove-step="(id: string) => emit('remove-step', id)"
        @rename-step="(id: string, label: string) => emit('rename-step', id, label)"
      />

      <!-- Container type -->
      <template v-else>
        <ActionRow
          v-for="action in (step.actions || [])"
          :key="action.id"
          :action="action"
          :container-type="step.type"
          :step-id="step.id"
          :step-label="step.label"
          :steps="steps"
          :sibling-actions="step.actions"
          :expanded="expandedActionId === action.id"
          :status="runState?.actionStates?.[action.id] || 'idle'"
          @remove="emit('remove-action', step.id, action.id)"
          @click="emit('action-click', step.id, action.id)"
          @rename="(label) => emit('rename-action', step.id, action.id, label)"
          @toggle-expand="toggleActionExpand(action.id)"
          @update-params="(params) => emit('update-action-params', step.id, action.id, params)"
        />

        <Button
          variant="ghost"
          size="sm"
          class="mt-1 text-xs text-muted-foreground gap-1 w-full justify-start"
          @click="emit('add-action', step.id)"
        >
          <Plus class="w-3.5 h-3.5" /> {{ t('stepCard.addAction') }}
        </Button>
      </template>
    </div>

    <!-- Output display -->
    <div
      v-if="formattedOutput && (runState?.status === 'success' || runState?.status === 'error')"
      class="border-t border-border bg-background"
    >
      <div
        class="flex items-center gap-1.5 px-3 py-1.5 cursor-pointer select-none transition-colors hover:bg-card"
        @click="showOutput = !showOutput"
      >
        <span class="text-[10px] text-muted-foreground w-3.5">{{ showOutput ? '▼' : '▶' }}</span>
        <span class="text-xs text-muted-foreground">
          {{ runState?.status === 'error' ? t('stepCard.outputError') : t('stepCard.outputSuccess') }}
        </span>
      </div>
      <pre v-if="showOutput" class="m-0 px-3 py-2 text-[11px] text-muted-foreground bg-background border-t border-border font-mono max-h-[200px] overflow-auto whitespace-pre-wrap break-all">{{ formattedOutput }}</pre>
    </div>
  </Card>
</template>

<style scoped>
@keyframes animate-progress-slide {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(350%); }
}
@keyframes animate-pulse-step {
  0%, 100% { outline: 2px solid transparent; outline-offset: -1px; }
  50% { outline: 2px solid var(--color-warning); outline-offset: -1px; }
}
.animate-progress-slide { animation: animate-progress-slide 1.2s ease-in-out infinite; }
.animate-pulse-step { animation: animate-pulse-step 1.5s ease-in-out infinite; }
</style>
