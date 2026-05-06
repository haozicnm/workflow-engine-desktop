<script setup lang="ts">
import { ref, computed, watch, defineAsyncComponent } from 'vue'
import type { Step, StepRunState, LogicCondition, LogicConditionGroup } from '../types/workflow'
import { LOGIC_OPERATORS, uid } from '../types/workflow'
import { safeInvoke } from '../utils/tauri'
import Button from './ui/button/Button.vue'
import Input from './ui/input/Input.vue'

const StepCard = defineAsyncComponent(() => import('./StepCard.vue'))

const props = defineProps<{
  step: Step
  runState?: StepRunState
  steps?: Step[]  // 工作流所有步骤（用于变量引用）
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
  'update-condition-group': [stepId: string, group: LogicConditionGroup]
  'add-sub-step': [stepId: string, branch: 'then' | 'else']
  'remove-sub-step': [stepId: string, branch: 'then' | 'else', subStepId: string]
  'open-config': [stepId: string]
  'update-error-strategy': [stepId: string, strategy: any]
  'start-recording': [stepId: string]
  'stop-recording': [stepId: string]
}>()

// ─── Condition Group State ───
const conditionGroup = computed<LogicConditionGroup>(() => {
  return props.step.conditionGroup || { combinator: 'and', conditions: [] }
})

const conditions = computed(() => conditionGroup.value.conditions)
const combinator = computed(() => conditionGroup.value.combinator)

function updateGroup(newGroup: LogicConditionGroup) {
  emit('update-condition-group', props.step.id, newGroup)
}

function addCondition() {
  const newConditions = [...conditions.value, {
    id: uid('cond'),
    left: '',
    op: 'contains',
    right: '',
  }]
  updateGroup({ ...conditionGroup.value, conditions: newConditions })
}

function removeCondition(condId: string) {
  const newConditions = conditions.value.filter(c => c.id !== condId)
  updateGroup({ ...conditionGroup.value, conditions: newConditions })
}

// ─── Output Template ───
const outputTemplate = computed({
  get: () => (props.step.config?.output_template as string) || '',
  set: (val: string) => {
    if (!props.step.config) props.step.config = {}
    props.step.config.output_template = val
  },
})

function updateCondition(condId: string, field: keyof LogicCondition, value: string) {
  const newConditions = conditions.value.map(c =>
    c.id === condId ? { ...c, [field]: value } : c
  )
  updateGroup({ ...conditionGroup.value, conditions: newConditions })
}

function toggleCombinator() {
  updateGroup({
    ...conditionGroup.value,
    combinator: combinator.value === 'and' ? 'or' : 'and',
  })
}

function getOpDef(opType: string) {
  return LOGIC_OPERATORS.find(o => o.type === opType)
}

// ─── Variable reference ───
const activeVarField = ref<string | null>(null)  // format: "condId:left" or "condId:right"
const varSearch = ref('')

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
    if (step.id === props.step.id) continue // 跳过自身
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

const filteredRefs = computed(() => {
  if (!varSearch.value.trim()) return availableRefs.value
  const q = varSearch.value.toLowerCase()
  return availableRefs.value.filter(r =>
    r.label.toLowerCase().includes(q) || r.id.toLowerCase().includes(q),
  )
})

function insertVarRef(refId: string) {
  if (!activeVarField.value) return
  const [condId, field] = activeVarField.value.split(':')
  const refText = `{{${refId}}}`
  updateCondition(condId, field as 'left' | 'right', refText)
  activeVarField.value = null
  varSearch.value = ''
}

function openVarPicker(condId: string, field: 'left' | 'right') {
  const key = `${condId}:${field}`
  activeVarField.value = activeVarField.value === key ? null : key
  varSearch.value = ''
}

// ─── Then/Else branches (same as before) ───
</script>

<template>
  <div class="bg-background border border-border rounded-md p-3">
    <!-- ─── 条件构建器 ─── -->
    <div class="mb-3">
      <div class="flex items-center justify-between mb-2">
        <span class="text-xs text-muted-foreground font-medium">判断条件</span>
        <Button
          variant="ghost"
          size="sm"
          class="h-6 text-[11px] px-2"
          @click="addCondition"
        >＋ 添加条件</Button>
      </div>

      <!-- 空状态 -->
      <div v-if="conditions.length === 0" class="text-xs text-muted-foreground/60 py-2 text-center border border-dashed border-border rounded">
        点击「添加条件」开始构建判断逻辑
      </div>

      <!-- 条件列表 -->
      <div v-for="(cond, idx) in conditions" :key="cond.id" class="space-y-1.5">
        <!-- 组合符 AND/OR -->
        <div v-if="idx > 0" class="flex items-center justify-center py-0.5">
          <button
            class="text-[10px] font-bold px-2 py-0.5 rounded transition-colors"
            :class="combinator === 'and' ? 'bg-primary/10 text-primary' : 'bg-warning/10 text-warning'"
            @click="toggleCombinator"
          >
            {{ combinator === 'and' ? 'AND' : 'OR' }}
          </button>
        </div>

        <!-- 单个条件行 -->
        <div class="flex items-center gap-1.5 bg-card border border-border rounded px-2 py-1.5">
          <!-- 左侧值 -->
          <div class="flex-1 min-w-0 relative">
            <Input
              :model-value="cond.left"
              placeholder="变量或值"
              class="h-7 text-xs font-mono"
              @input="updateCondition(cond.id, 'left', ($event.target as HTMLInputElement).value)"
            />
            <!-- 变量引用按钮 -->
            <button
              class="absolute right-1 top-1/2 -translate-y-1/2 text-[10px] opacity-40 hover:opacity-100"
              :class="activeVarField === `${cond.id}:left` ? 'opacity-100 text-primary' : ''"
              title="引用变量"
              @click="openVarPicker(cond.id, 'left')"
            >🔗</button>
            <!-- 变量下拉 -->
            <div v-if="activeVarField === `${cond.id}:left`" class="absolute left-0 top-full mt-1 z-50 w-64 bg-card border border-border rounded-md shadow-lg overflow-hidden">
              <Input v-model="varSearch" placeholder="搜索变量..." class="h-6 text-xs rounded-none border-0 border-b border-border" />
              <div class="max-h-[150px] overflow-y-auto">
                <div
                  v-for="ref in filteredRefs"
                  :key="ref.id"
                  class="flex items-center gap-1.5 px-2 py-1 text-xs cursor-pointer hover:bg-secondary"
                  @click="insertVarRef(ref.id)"
                >
                  <span class="shrink-0">{{ ref.icon }}</span>
                  <span class="flex-1 truncate" :class="ref.type === 'action' ? 'text-muted-foreground' : 'font-medium'">{{ ref.label }}</span>
                  <span class="text-[10px] text-muted-foreground font-mono">{{ ref.id }}</span>
                </div>
              </div>
            </div>
          </div>

          <!-- 操作符 -->
          <select
            :value="cond.op"
            class="h-7 text-xs bg-secondary border border-border rounded px-1.5 min-w-[80px] cursor-pointer"
            @change="updateCondition(cond.id, 'op', ($event.target as HTMLSelectElement).value)"
          >
            <option v-for="op in LOGIC_OPERATORS" :key="op.type" :value="op.type">
              {{ op.icon }} {{ op.label }}
            </option>
          </select>

          <!-- 右侧值（仅 hasRight 时显示） -->
          <div v-if="getOpDef(cond.op)?.hasRight" class="flex-1 min-w-0 relative">
            <Input
              :model-value="cond.right"
              placeholder="比较值"
              class="h-7 text-xs font-mono"
              @input="updateCondition(cond.id, 'right', ($event.target as HTMLInputElement).value)"
            />
            <button
              class="absolute right-1 top-1/2 -translate-y-1/2 text-[10px] opacity-40 hover:opacity-100"
              :class="activeVarField === `${cond.id}:right` ? 'opacity-100 text-primary' : ''"
              title="引用变量"
              @click="openVarPicker(cond.id, 'right')"
            >🔗</button>
            <div v-if="activeVarField === `${cond.id}:right`" class="absolute left-0 top-full mt-1 z-50 w-64 bg-card border border-border rounded-md shadow-lg overflow-hidden">
              <Input v-model="varSearch" placeholder="搜索变量..." class="h-6 text-xs rounded-none border-0 border-b border-border" />
              <div class="max-h-[150px] overflow-y-auto">
                <div
                  v-for="ref in filteredRefs"
                  :key="ref.id"
                  class="flex items-center gap-1.5 px-2 py-1 text-xs cursor-pointer hover:bg-secondary"
                  @click="insertVarRef(ref.id)"
                >
                  <span class="shrink-0">{{ ref.icon }}</span>
                  <span class="flex-1 truncate" :class="ref.type === 'action' ? 'text-muted-foreground' : 'font-medium'">{{ ref.label }}</span>
                  <span class="text-[10px] text-muted-foreground font-mono">{{ ref.id }}</span>
                </div>
              </div>
            </div>
          </div>

          <!-- 删除按钮 -->
          <Button
            variant="ghost"
            size="icon"
            class="h-6 w-6 shrink-0 text-muted-foreground hover:text-destructive"
            @click="removeCondition(cond.id)"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
          </Button>
        </div>
      </div>
    </div>

    <!-- ─── 输出模板（可选） ─── -->
    <div class="mb-3">
      <div class="flex items-center justify-between mb-1">
        <span class="text-xs text-muted-foreground font-medium">输出模板</span>
        <span class="text-[10px] text-muted-foreground/60">可选，自定义 value 输出</span>
      </div>
      <Input
        v-model="outputTemplate"
        placeholder="{{left}} 或 {{变量引用}}，留空则透传原始输入值"
        class="h-7 text-xs font-mono"
      />
      <div class="text-[10px] text-muted-foreground/50 mt-1">
        输出格式：{ branch: "true/false", value: 模板渲染值, result: true/false }
      </div>
    </div>

    <!-- ─── Then / Else 分支 ─── -->
    <div class="grid grid-cols-2 gap-3">
      <!-- ✅ 满足时 -->
      <div>
        <div class="text-sm font-medium text-success mb-2 flex items-center gap-1">✅ 满足时</div>
        <div class="min-h-[40px]">
          <StepCard
            v-for="subStep in (step.thenSteps || [])"
            :key="subStep.id"
            :step="subStep"
            :steps="steps"
            @add-action="(id) => emit('add-action', id)"
            @remove-action="(id, aId) => emit('remove-action', id, aId)"
            @action-click="(id, aId) => emit('action-click', id, aId)"
            @rename-action="(id, aId, label) => emit('rename-action', id, aId, label)"
            @update-action-params="(id, aId, params) => emit('update-action-params', id, aId, params)"
            @remove-step="(id) => emit('remove-sub-step', step.id, 'then', id)" 
            @rename-step="(id, label) => emit('rename-step', id, label)"
            @update-condition="(id, c) => emit('update-condition', id, c)"
            @update-condition-group="(id, g) => emit('update-condition-group', id, g)"
            @add-sub-step="(id, b) => emit('add-sub-step', id, b)"
            @remove-sub-step="(id, b, sId) => emit('remove-sub-step', id, b, sId)"
          />
        </div>
        <Button
          variant="ghost"
          size="sm"
          class="w-full mt-1 text-xs text-success hover:text-success hover:bg-success/10"
          @click="emit('add-sub-step', step.id, 'then')"
        >
          ＋ 增加步骤
        </Button>
      </div>

      <!-- ❌ 不满足时 -->
      <div>
        <div class="text-sm font-medium text-danger mb-2 flex items-center gap-1">❌ 不满足时</div>
        <div class="min-h-[40px]">
          <StepCard
            v-for="subStep in (step.elseSteps || [])"
            :key="subStep.id"
            :step="subStep"
            :steps="steps"
            @add-action="(id) => emit('add-action', id)"
            @remove-action="(id, aId) => emit('remove-action', id, aId)"
            @action-click="(id, aId) => emit('action-click', id, aId)"
            @rename-action="(id, aId, label) => emit('rename-action', id, aId, label)"
            @update-action-params="(id, aId, params) => emit('update-action-params', id, aId, params)"
            @remove-step="(id) => emit('remove-sub-step', step.id, 'else', id)" 
            @rename-step="(id, label) => emit('rename-step', id, label)"
            @update-condition="(id, c) => emit('update-condition', id, c)"
            @update-condition-group="(id, g) => emit('update-condition-group', id, g)"
            @add-sub-step="(id, b) => emit('add-sub-step', id, b)"
            @remove-sub-step="(id, b, sId) => emit('remove-sub-step', id, b, sId)"
          />
        </div>
        <Button
          variant="ghost"
          size="sm"
          class="w-full mt-1 text-xs text-danger hover:text-danger hover:bg-danger/10"
          @click="emit('add-sub-step', step.id, 'else')"
        >
          ＋ 增加步骤
        </Button>
      </div>
    </div>
  </div>
</template>
