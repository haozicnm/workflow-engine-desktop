<script setup lang="ts">
import { ref, computed } from 'vue'
import type { Step, StepRunState, ContainerType, ErrorStrategy } from '../types/workflow'
import { getContainerDef, isContainerType } from '../types/workflow'
import ActionRow from './ActionRow.vue'
import LogicBranch from './LogicBranch.vue'
import Card from './ui/card/Card.vue'
import CardContent from './ui/card/CardContent.vue'
import Button from './ui/button/Button.vue'
import Input from './ui/input/Input.vue'
import Label from './ui/label/Label.vue'
import Textarea from './ui/textarea/Textarea.vue'
import Checkbox from './ui/checkbox/Checkbox.vue'
import Select from './ui/select/Select.vue'
import Badge from './ui/badge/Badge.vue'
import { cn } from '@/lib/utils'

const props = defineProps<{
  step: Step
  runState?: StepRunState
  totalSteps?: number
  currentStepIndex?: number
  isRecording?: boolean
  steps?: Step[]  // 工作流所有步骤（传给 LogicBranch 用于变量引用）
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
  'update-condition-group': [stepId: string, group: import('../types/workflow').LogicConditionGroup]
  'add-sub-step': [stepId: string, branch: 'then' | 'else']
  'remove-sub-step': [stepId: string, branch: 'then' | 'else', subStepId: string]
  'open-config': [stepId: string]
  'update-error-strategy': [stepId: string, strategy: ErrorStrategy]
  'start-recording': [stepId: string]
  'stop-recording': [stepId: string]
}>()

// ─── Action expand/collapse ───
const expandedActionId = ref<string | null>(null)

function toggleActionExpand(actionId: string) {
  expandedActionId.value = expandedActionId.value === actionId ? null : actionId
}

const containerDef = computed(() => getContainerDef(props.step.type))
const isContainer = computed(() => isContainerType(props.step.type))

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

function onConfigParamChange(key: string, value: unknown) {
  props.step.config[key] = value
}
function onConfigInput(key: string, e: Event) {
  onConfigParamChange(key, (e.target as HTMLInputElement).value)
}
function onConfigNumber(key: string, e: Event) {
  const v = (e.target as HTMLInputElement).value
  onConfigParamChange(key, v === '' ? '' : Number(v))
}
function onConfigSelect(key: string, e: Event) {
  onConfigParamChange(key, (e.target as HTMLSelectElement).value)
}
function onConfigCheckbox(key: string, val: boolean) {
  onConfigParamChange(key, val)
}
function onConfigTextarea(key: string, e: Event) {
  onConfigParamChange(key, (e.target as HTMLTextAreaElement).value)
}

const statusColor: Record<string, string> = {
  success: 'bg-success',
  running: 'bg-warning',
  error: 'bg-danger',
  idle: 'bg-muted',
}

const statusBadgeColor = computed(() => {
  const status = props.runState?.status || 'idle'
  return statusColor[status] || 'bg-muted'
})

const showOutput = ref(false)
const formattedOutput = computed(() => {
  const out = props.runState?.output
  if (!out) return ''
  if (typeof out === 'string') return out
  return JSON.stringify(out, null, 2)
})

const showErrMenu = ref(false)
const errStrategyLabel = computed(() => {
  const s = props.step.onError
  if (!s || s === 'fail') return '终止'
  if (s === 'ignore') return '忽略'
  if (typeof s === 'object' && 'branch' in s) return '跳转'
  return '终止'
})

function setErrStrategy(s: ErrorStrategy) {
  emit('update-error-strategy', props.step.id, s)
  showErrMenu.value = false
}

// ─── Condition (条件执行) ───
const showConditionMenu = ref(false)
const logicSteps = computed(() => (props.steps || []).filter(s => s.type === 'logic'))
const conditionLabel = computed(() => {
  const rc = props.step.runCondition
  if (!rc) return ''
  const refStep = (props.steps || []).find(s => s.id === rc.ref)
  const name = refStep?.label || rc.ref
  if (rc.when === 'both') return `📌 ${name}`
  if (rc.when === 'merge') return `🔀 合并:${name}`
  return `📌 ${name}=${rc.when === 'true' ? '真' : '假'}`
})
function setCondition(ref: string, when: 'true' | 'false' | 'both' | 'merge') {
  if (!ref) {
    delete props.step.runCondition
  } else {
    props.step.runCondition = { ref, when }
  }
  showConditionMenu.value = false
}
function removeCondition() {
  delete props.step.runCondition
  showConditionMenu.value = false
}

function formatDuration(ms?: number): string {
  if (!ms) return ''
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

function colorAt15(color: string): string {
  const hex = color.replace('#', '')
  const r = parseInt(hex.substring(0, 2), 16)
  const g = parseInt(hex.substring(2, 4), 16)
  const b = parseInt(hex.substring(4, 6), 16)
  return `rgba(${r},${g},${b},0.15)`
}
</script>

<template>
  <Card
    :class="cn(
      'overflow-hidden',
      runState?.status === 'running' && 'animate-pulse-step',
    )"
    :style="{
      borderColor: containerDef.color,
      borderLeftWidth: '3px',
    }"
  >
    <!-- Header bar -->
    <div
      class="h-10 px-4 flex items-center cursor-pointer gap-2 select-none transition-colors group"
      :style="{ background: colorAt15(containerDef.color) }"
      @click="step.expanded = !step.expanded"
    >
      <!-- Container icon -->
      <span class="shrink-0 text-base">{{ containerDef.icon }}</span>

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
        title="双击重命名"
        @dblclick.stop="startRenameStep"
      >
        {{ step.label }}
      </span>

      <!-- Error strategy badge -->
      <Badge
        v-if="step.onError && step.onError !== 'fail'"
        variant="warning"
        class="text-[10px] px-1.5 py-0"
        :title="`错误策略: ${errStrategyLabel}`"
      >⚠{{ errStrategyLabel }}</Badge>

      <!-- Condition badge -->
      <Badge
        v-if="step.runCondition"
        variant="outline"
        class="text-[10px] px-1.5 py-0 border-warning/40 text-warning bg-warning/10 cursor-pointer hover:bg-warning/20"
        :title="`条件执行: ${conditionLabel}`"
        @click.stop="showConditionMenu = !showConditionMenu"
      >{{ conditionLabel }}</Badge>

      <!-- Duration -->
      <span v-if="runState?.duration" class="text-[10px] text-muted-foreground font-mono mr-1">
        {{ formatDuration(runState.duration) }}
      </span>

      <!-- Status dot -->
      <span :class="cn('w-2 h-2 rounded-full shrink-0', statusBadgeColor)" />

      <!-- Recording button (Browser only) -->
      <Button
        v-if="step.type === 'browser' && !props.isRecording"
        variant="ghost"
        size="icon"
        class="w-5 h-5 text-muted-foreground hover:text-foreground hover:bg-secondary opacity-0 group-hover:opacity-100"
        title="录制浏览器操作"
        @click.stop="emit('start-recording', step.id)"
      >🔴</Button>
      <Button
        v-if="step.type === 'browser' && props.isRecording"
        variant="ghost"
        size="icon"
        class="w-5 h-5 text-destructive hover:text-destructive hover:bg-destructive/10 animate-pulse"
        title="停止录制"
        @click.stop="emit('stop-recording', step.id)"
      >⏹</Button>

      <!-- Settings button -->
      <Button
        variant="ghost"
        size="icon"
        class="w-5 h-5 text-muted-foreground hover:text-foreground hover:bg-secondary opacity-0 group-hover:opacity-100"
        @click.stop="emit('open-config', step.id)"
      >⚙</Button>

      <!-- Condition button (not for logic containers) -->
      <Button
        v-if="step.type !== 'logic' && logicSteps.length > 0"
        variant="ghost"
        size="icon"
        class="w-5 h-5 text-muted-foreground hover:text-warning hover:bg-warning/10 opacity-0 group-hover:opacity-100"
        title="条件执行"
        @click.stop="showConditionMenu = !showConditionMenu"
      >🔀</Button>

      <!-- Error strategy button -->
      <Button
        variant="ghost"
        size="icon"
        class="w-5 h-5 text-muted-foreground hover:text-foreground hover:bg-secondary opacity-0 group-hover:opacity-100"
        title="错误处理策略"
        @click.stop="showErrMenu = !showErrMenu"
      >🛡</Button>

      <!-- Collapse toggle -->
      <span class="text-[11px] text-muted-foreground shrink-0 w-4 text-center">
        {{ step.expanded ? '▼' : '▶' }}
      </span>

      <!-- Remove button -->
      <Button
        variant="ghost"
        size="icon"
        class="w-5 h-5 text-muted-foreground hover:text-destructive hover:bg-destructive/10 opacity-0 group-hover:opacity-100"
        @click.stop="emit('remove-step', step.id)"
      >×</Button>
    </div>

    <!-- Error strategy dropdown -->
    <div v-if="showErrMenu" class="bg-card border-t border-border">
      <div class="text-[11px] font-semibold text-muted-foreground uppercase tracking-wide px-2.5 pt-1.5 pb-1">
        错误处理策略
      </div>
      <div
        v-for="(opt, key) in { fail: { icon: '🔴', label: '终止', desc: '步骤失败时终止整个工作流' }, ignore: { icon: '🟡', label: '忽略', desc: '跳过错误，继续下一步' }, branch: { icon: '🔵', label: '跳转', desc: '失败时跳转到指定步骤' } }"
        :key="key"
        :class="cn(
          'flex flex-col gap-0.5 px-2.5 py-1.5 cursor-pointer text-sm transition-colors hover:bg-secondary',
          ((!step.onError && key === 'fail') || step.onError === key || (key === 'branch' && typeof step.onError === 'object' && 'branch' in step.onError))
            ? 'bg-secondary border-l-2 border-primary' : '',
        )"
        @click="key === 'fail' ? setErrStrategy('fail') : key === 'ignore' ? setErrStrategy('ignore') : setErrStrategy({ branch: '' })"
      >
        <span class="text-sm text-foreground">{{ opt.icon }} {{ opt.label }}</span>
        <span class="text-[11px] text-muted-foreground">{{ opt.desc }}</span>
      </div>
    </div>

    <!-- Condition dropdown -->
    <div v-if="showConditionMenu" class="bg-card border-t border-border">
      <div class="text-[11px] font-semibold text-muted-foreground uppercase tracking-wide px-2.5 pt-1.5 pb-1">
        条件执行
      </div>
      <div v-if="logicSteps.length === 0" class="px-2.5 py-2 text-xs text-muted-foreground">
        暂无逻辑判断步骤，请先添加逻辑容器
      </div>
      <template v-else>
        <div
          v-for="ls in logicSteps"
          :key="ls.id"
          class="border-b border-border last:border-b-0"
        >
          <div class="px-2.5 py-1 text-[11px] text-muted-foreground bg-secondary/50">
            {{ ls.icon || '🔀' }} {{ ls.label }}
          </div>
          <div
            v-for="opt in [
              { value: 'true', icon: '✅', label: '为真时执行', desc: '逻辑判断通过则执行本步骤' },
              { value: 'false', icon: '❌', label: '为假时执行', desc: '逻辑判断不通过则执行本步骤' },
              { value: 'both', icon: '🔄', label: '始终执行', desc: '无论真假都执行，保留引用关系' },
              { value: 'merge', icon: '🔀', label: '合并执行', desc: '等待所有分支完成后执行一次' },
            ]"
            :key="opt.value"
            :class="cn(
              'flex items-center gap-2 px-2.5 py-1.5 cursor-pointer text-sm transition-colors hover:bg-secondary',
              step.runCondition?.ref === ls.id && step.runCondition?.when === opt.value
                ? 'bg-secondary border-l-2 border-warning' : '',
            )"
            @click="setCondition(ls.id, opt.value as 'true' | 'false' | 'both' | 'merge')"
          >
            <span class="text-sm">{{ opt.icon }}</span>
            <div class="flex flex-col">
              <span class="text-sm text-foreground">{{ opt.label }}</span>
              <span class="text-[10px] text-muted-foreground">{{ opt.desc }}</span>
            </div>
          </div>
        </div>
        <div
          v-if="step.runCondition"
          class="px-2.5 py-1.5 cursor-pointer text-sm text-destructive transition-colors hover:bg-destructive/10"
          @click="removeCondition"
        >
          ✕ 移除条件
        </div>
      </template>
    </div>

    <!-- Progress bar (running) -->
    <div v-if="runState?.status === 'running'" class="h-0.5 bg-secondary overflow-hidden">
      <div class="h-full w-2/5 bg-warning rounded-full animate-progress-slide" />
    </div>

    <!-- Body (expandable) -->
    <div v-show="step.expanded" class="px-4 py-3 bg-card border-t border-border">
      <!-- Simple step type -->
      <template v-if="!isContainer && step.type !== 'logic'">
        <div
          v-for="param in containerDef.params"
          :key="param.key"
          class="mb-2"
        >
          <Label class="text-[11px] text-muted-foreground block mb-1">{{ param.label }}</Label>
          <Input v-if="param.type === 'text'" type="text" :model-value="(step.config[param.key] as string) ?? (param.default as string) ?? ''" :placeholder="param.placeholder" class="h-8 text-xs" @input="onConfigInput(param.key, $event)" />
          <Input v-else-if="param.type === 'number'" type="number" :model-value="(step.config[param.key] as string) ?? (param.default as string) ?? ''" :placeholder="param.placeholder" class="h-8 text-xs" @input="onConfigNumber(param.key, $event)" />
          <Select v-else-if="param.type === 'select'" :model-value="(step.config[param.key] as string) ?? (param.default as string) ?? ''" :options="param.options" @update:model-value="v => onConfigParamChange(param.key, v)" />
          <div v-else-if="param.type === 'checkbox'" class="flex items-center gap-2">
            <Checkbox :model-value="!!(step.config[param.key] ?? param.default)" @update:model-value="(v) => onConfigCheckbox(param.key, v)" />
          </div>
          <Textarea v-else-if="param.type === 'textarea'" :model-value="String(step.config[param.key] ?? param.default ?? '')" :placeholder="param.placeholder" :rows="3" class="text-xs" @input="onConfigTextarea(param.key, $event)" />
        </div>
      </template>

      <!-- Logic type -->
      <LogicBranch
        v-else-if="step.type === 'logic'"
        :step="step"
        :run-state="runState"
        :steps="steps"
        @update-condition="(id, c) => emit('update-condition', id, c)"
        @update-condition-group="(id, g) => emit('update-condition-group', id, g)"
        @add-sub-step="(id, b) => emit('add-sub-step', id, b)"
        @remove-sub-step="(id, b, sId) => emit('remove-sub-step', id, b, sId)"
        @add-action="(id) => emit('add-action', id)"
        @remove-action="(id, aId) => emit('remove-action', id, aId)"
        @action-click="(id, aId) => emit('action-click', id, aId)"
        @rename-action="(id, aId, label) => emit('rename-action', id, aId, label)"
        @remove-step="(id) => emit('remove-step', id)"
        @rename-step="(id, label) => emit('rename-step', id, label)"
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
          <span class="text-sm">＋</span> 增加动作
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
          {{ runState?.status === 'error' ? '❌ 错误' : '✅ 输出' }}
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
  0%, 100% { box-shadow: 0 0 0 0 rgba(210, 153, 34, 0); }
  50% { box-shadow: 0 0 8px 2px rgba(210, 153, 34, 0.25); }
}
.animate-progress-slide { animation: animate-progress-slide 1.2s ease-in-out infinite; }
.animate-pulse-step { animation: animate-pulse-step 1.5s ease-in-out infinite; }
</style>
