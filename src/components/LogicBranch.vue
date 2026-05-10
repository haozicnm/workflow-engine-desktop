<script setup lang="ts">
import { ref, computed } from 'vue'
import { Link } from 'lucide-vue-next'
import type { Step, StepRunState, LogicCondition, LogicConditionGroup } from '../types/types'
import { uid } from '../types/types'
import { LOGIC_OPERATORS } from '../types/node-registry'
import ActionIcon from './ActionIcon.vue'
import { useVariableRefs } from '../composables/useVariableRefs'
import Button from './ui/button/Button.vue'
import Input from './ui/input/Input.vue'
import Select from './ui/select/Select.vue'

const props = defineProps<{
  step: Step
  runState?: StepRunState
  steps?: Step[]
}>()

const emit = defineEmits<{
  'update-condition': [stepId: string, condition: string]
  'update-condition-group': [stepId: string, group: LogicConditionGroup]
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

// ─── Variable reference (from composable, skip self) ───
const { availableRefs } = useVariableRefs(
  () => (props.steps || []).filter(s => s.id !== props.step.id),
  () => props.step.id,
)

const activeVarField = ref<string | null>(null)
const varSearch = ref('')

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
</script>

<template>
  <!-- 条件构建器（无外层壳，直接平铺在 Card body 内） -->
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
      <!-- AND/OR -->
      <div v-if="idx > 0" class="flex items-center justify-center py-0.5">
        <Button
          variant="ghost"
          size="sm"
          class="text-[10px] font-bold px-2 py-0.5 rounded transition-colors"
          :class="combinator === 'and' ? 'bg-primary/10 text-primary' : 'bg-warning/10 text-warning'"
          @click="toggleCombinator"
        >
          {{ combinator === 'and' ? 'AND' : 'OR' }}
        </Button>
      </div>

      <!-- 条件行 -->
      <div class="flex items-center gap-2 bg-muted/30 border border-border rounded px-2 py-2">
        <!-- 左值 -->
        <div class="flex-1 min-w-0 relative">
          <Input
            :model-value="cond.left"
            placeholder="变量或值"
            class="h-8 text-xs font-mono pr-7"
            @input="updateCondition(cond.id, 'left', ($event.target as HTMLInputElement).value)"
          />
          <button
            class="absolute right-1.5 top-1/2 -translate-y-1/2 text-[11px] opacity-40 hover:opacity-100 hover:text-foreground transition-colors"
            :class="activeVarField === `${cond.id}:left` ? 'opacity-100 text-primary' : ''"
            title="引用变量"
            aria-label="引用变量"
            @click="openVarPicker(cond.id, 'left')"
          ><Link class="w-3 h-3" /></button>
          <div v-if="activeVarField === `${cond.id}:left`" class="absolute left-0 top-full mt-1 z-50 w-72 bg-card border border-border rounded-md shadow-lg overflow-hidden">
            <Input v-model="varSearch" placeholder="搜索变量..." class="h-7 text-xs rounded-none border-0 border-b border-border" />
            <div class="max-h-[200px] overflow-y-auto">
              <Button
                v-for="ref in filteredRefs"
                :key="ref.id"
                variant="ghost"
                class="w-full justify-start h-auto flex items-center gap-1.5 px-2 py-1.5 text-xs"
                @click="insertVarRef(ref.id)"
              >
                <ActionIcon :name="ref.icon" cls="w-4 h-4 shrink-0" />
                <span class="flex-1 truncate text-left" :class="ref.type === 'action' ? 'text-muted-foreground' : 'font-medium'">{{ ref.label }}</span>
                <span class="text-[10px] text-muted-foreground font-mono">{{ ref.id }}</span>
              </Button>
            </div>
          </div>
        </div>

        <!-- 操作符 -->
        <div class="shrink-0">
        <Select
          :model-value="cond.op"
          :options="LOGIC_OPERATORS.map(o => ({ value: o.type, label: o.label }))"
          @update:model-value="updateCondition(cond.id, 'op', $event)"
        />
        </div>

        <!-- 右值 -->
        <div v-if="getOpDef(cond.op)?.hasRight" class="flex-1 min-w-0 relative">
          <Input
            :model-value="cond.right"
            placeholder="比较值"
            class="h-8 text-xs font-mono pr-7"
            @input="updateCondition(cond.id, 'right', ($event.target as HTMLInputElement).value)"
          />
          <button
            class="absolute right-1.5 top-1/2 -translate-y-1/2 text-[11px] opacity-40 hover:opacity-100 hover:text-foreground transition-colors"
            :class="activeVarField === `${cond.id}:right` ? 'opacity-100 text-primary' : ''"
            title="引用变量"
            aria-label="引用变量"
            @click="openVarPicker(cond.id, 'right')"
          ><Link class="w-3 h-3" /></button>
          <div v-if="activeVarField === `${cond.id}:right`" class="absolute left-0 top-full mt-1 z-50 w-72 bg-card border border-border rounded-md shadow-lg overflow-hidden">
            <Input v-model="varSearch" placeholder="搜索变量..." class="h-7 text-xs rounded-none border-0 border-b border-border" />
            <div class="max-h-[200px] overflow-y-auto">
              <Button
                v-for="ref in filteredRefs"
                :key="ref.id"
                variant="ghost"
                class="w-full justify-start h-auto flex items-center gap-1.5 px-2 py-1.5 text-xs"
                @click="insertVarRef(ref.id)"
              >
                <ActionIcon :name="ref.icon" cls="w-4 h-4 shrink-0" />
                <span class="flex-1 truncate text-left" :class="ref.type === 'action' ? 'text-muted-foreground' : 'font-medium'">{{ ref.label }}</span>
                <span class="text-[10px] text-muted-foreground font-mono">{{ ref.id }}</span>
              </Button>
            </div>
          </div>
        </div>

        <!-- 删除 -->
        <Button
          variant="ghost"
          size="icon"
          class="h-7 w-7 shrink-0 text-muted-foreground hover:text-destructive"
          aria-label="删除条件"
          @click="removeCondition(cond.id)"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
        </Button>
      </div>
    </div>
  </div>

  <!-- 输出模板 -->
  <div class="mb-3">
    <div class="flex items-center justify-between mb-1">
      <span class="text-xs text-muted-foreground font-medium">输出模板</span>
      <span class="text-[10px] text-muted-foreground/60">可选，自定义 value 输出</span>
    </div>
    <Input
      v-model="outputTemplate"
      placeholder="{{left}} 或 {{变量引用}}，留空则透传原始输入值"
      class="h-8 text-xs font-mono"
    />
    <div class="text-[10px] text-muted-foreground/50 mt-1">
      输出格式：{ branch: "true/false", value: 模板渲染值, result: true/false }
    </div>
  </div>

  <!-- 执行结果 -->
  <div v-if="runState?.output" class="mb-3 bg-muted/30 border border-border rounded px-3 py-2 text-xs font-mono">
    <div class="text-muted-foreground mb-1 text-[10px] uppercase tracking-wide">执行结果</div>
    <div class="flex items-center gap-2">
      <span :class="(runState.output as Record<string, unknown>)?.result ? 'text-success' : 'text-destructive'">
        {{ (runState.output as Record<string, unknown>)?.result ? '✓ 真' : '✗ 假' }}
      </span>
      <span class="text-muted-foreground">→</span>
      <span class="text-foreground">branch: {{ (runState.output as Record<string, unknown>)?.branch }}</span>
      <span v-if="(runState.output as Record<string, unknown>)?.value !== undefined" class="text-muted-foreground">
        | value: {{ (runState.output as Record<string, unknown>)?.value }}
      </span>
    </div>
  </div>

  <!-- 耗时 -->
  <div v-if="runState?.duration" class="mt-2 text-[10px] text-muted-foreground text-right">
    耗时 {{ runState.duration }}ms
  </div>
</template>
