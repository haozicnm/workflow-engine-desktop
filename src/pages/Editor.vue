<script setup lang="ts">
import { ref, computed, onMounted, watch, onUnmounted } from 'vue'
import { useWorkflowStore } from '../stores/workflowStore'
import { useStepRunner } from '../composables/useStepRunner'
import { useToast } from '../composables/useToast'
import { useEditorEnhancements, type LogEntry } from '../composables/useEditorEnhancements'
import { useGlobalStatus } from '../composables/useGlobalStatus'
import { safeInvoke, safeListen } from '../utils/tauri'
import StepCard from '../components/StepCard.vue'
import ActionIcon from '../components/ActionIcon.vue'
import ContainerConfigPanel from '../components/ContainerConfigPanel.vue'
import CodeView from '../components/CodeView.vue'
import Button from '../components/ui/button/Button.vue'
import Input from '../components/ui/input/Input.vue'
import Badge from '../components/ui/badge/Badge.vue'
import Card from '../components/ui/card/Card.vue'
import CardContent from '../components/ui/card/CardContent.vue'
import CardHeader from '../components/ui/card/CardHeader.vue'
import CardTitle from '../components/ui/card/CardTitle.vue'
import ScrollArea from '../components/ui/scroll-area/ScrollArea.vue'
import Separator from '../components/ui/separator/Separator.vue'
import Textarea from '../components/ui/textarea/Textarea.vue'
import Tabs from '../components/ui/tabs/Tabs.vue'
import TabsList from '../components/ui/tabs/TabsList.vue'
import TabsTrigger from '../components/ui/tabs/TabsTrigger.vue'
import TabsContent from '../components/ui/tabs/TabsContent.vue'
import type { ContainerType, Step } from '../types/types'
import { CONTAINER_DEFS, getContainerDef, getActionDefs } from '../types/node-registry'
import { cn } from '@/lib/utils'

const props = defineProps<{
  workflowId?: string | null
}>()

const emit = defineEmits<{
  'schedule': [id: string]
  'workflow-updated': []
  'workflow-deleted': []
}>()

const store = useWorkflowStore()
const { runWorkflow, stopWorkflow, isRunning } = useStepRunner()
const toast = useToast()
const enh = useEditorEnhancements()
const globalStatus = useGlobalStatus()

let currentRunId: string | null = null

const editingName = ref(false)
const showCardMenu = ref(false)
const cardMenuBtnRef = ref<InstanceType<typeof Button> | null>(null)
const cardMenuPosStyle = ref<Record<string, string>>({})

function toggleCardMenu() {
  showCardMenu.value = !showCardMenu.value
  if (showCardMenu.value && cardMenuBtnRef.value?.$el) {
    const rect = (cardMenuBtnRef.value.$el as HTMLElement).getBoundingClientRect()
    cardMenuPosStyle.value = {
      top: `${rect.bottom + 4}px`,
      left: `${rect.right - 176}px`, // 176px = w-44
    }
  }
}
const nameInput = ref('')

const selectedStepId = ref<string | null>(null)
const configStepId = ref<string | null>(null)
const showAddStep = ref(false)
const activeView = ref<'visual' | 'code'>('visual')
const isRecording = ref(false)
const recordingError = ref('')

function getContainerUrl(): string | undefined {
  if (!workflow.value || !selectedStepId.value) return undefined
  const step = store.findStep(selectedStepId.value)
  if (!step) return undefined
  const navAction = step.actions?.find(a => a.type === 'navigate')
  if (navAction?.params?.url) return navAction.params.url as string
  if (step.config?.url) return step.config.url as string
  return undefined
}

async function onStartRecording(_stepId?: string) {
  try {
    const url = getContainerUrl()
    await safeInvoke('browser_recording_start', { url: url || null })
    isRecording.value = true
    recordingError.value = ''
    toast.info(url ? `录制已开始，已打开 ${url}` : '录制已开始，请在浏览器中操作')
  } catch (e: unknown) {
    recordingError.value = (e as Error).message || '启动录制失败'
    toast.error('启动录制失败: ' + recordingError.value)
  }
}

async function onStopRecording(_stepId?: string) {
  try {
    const result = await safeInvoke<{ actions?: unknown[]; workflow_json?: unknown }>('browser_recording_stop')
    isRecording.value = false
    if (result?.workflow_json) {
      toast.success(`录制完成，已捕获 ${Array.isArray(result.actions) ? result.actions.length : 0} 个操作`)
    } else {
      toast.info('录制已停止')
    }
  } catch (e: unknown) {
    isRecording.value = false
    toast.error('停止录制失败: ' + ((e as Error).message || e))
  }
}

const addActionStepId = ref<string | null>(null)
const addActionOptions = ref<{ type: string; label: string; icon: string }[]>([])

const dragIndex = ref<number | null>(null)
const dropIndex = ref<number | null>(null)

const workflow = computed(() => store.current)

const filteredSteps = computed(() => {
  if (!workflow.value) return []
  if (!enh.searchQuery.value.trim()) return workflow.value.steps
  const q = enh.searchQuery.value.toLowerCase()
  return workflow.value.steps.filter(step =>
    step.label.toLowerCase().includes(q) ||
    step.type.toLowerCase().includes(q) ||
    step.id.toLowerCase().includes(q),
  )
})

const configStep = computed<Step | null>(() => {
  if (!workflow.value || !configStepId.value) return null
  return store.findStep(configStepId.value)
})

watch(() => props.workflowId, async (newId) => {
  if (newId) {
    await store.loadWorkflow(newId)
  } else if (newId === null || newId === undefined) {
    store.current = {
      name: '未命名工作流',
      description: '',
      steps: [],
    }
    store.dirty = false
  }
}, { immediate: true })


function onStartEditName() {
  if (!workflow.value) return
  editingName.value = true
  nameInput.value = workflow.value.name
}

function onFinishEditName() {
  if (workflow.value && nameInput.value.trim()) {
    store.setWorkflowName(nameInput.value.trim())
  }
  editingName.value = false
}

async function onSave() {
  const ok = await store.saveWorkflow()
  if (ok) {
    toast.show('保存成功', 'success')
    // 显示变量引用警告
    if (store.lastWarnings.value.length > 0) {
      const msg = store.lastWarnings.value.slice(0, 5).join('\n')
      const extra = store.lastWarnings.value.length > 5 ? `\n...还有 ${store.lastWarnings.value.length - 5} 条` : ''
      toast.show(`变量引用警告:\n${msg}${extra}`, 'info')
    }
    emit('workflow-updated')
  } else {
    toast.show('保存失败', 'error')
  }
}

async function onScheduleClick() {
  if (!workflow.value) return
  // 未保存时先自动保存
  if (!workflow.value.id) {
    const ok = await store.saveWorkflow()
    if (!ok) { toast.show('请先保存工作流', 'error'); return }
    emit('workflow-updated')
  }
  if (workflow.value.id) {
    emit('schedule', workflow.value.id)
  }
}

async function onSaveAs() {
  if (!workflow.value) return
  const originalName = workflow.value.name
  workflow.value.name = originalName + ' (副本)'
  workflow.value.id = undefined as unknown as string
  store.dirty = true
  const ok = await store.saveWorkflow()
  if (ok) {
    toast.show(`已另存为「${workflow.value.name}」`, 'success')
    emit('workflow-updated')
  } else {
    toast.show('另存失败', 'error')
  }
}

async function onExport() {
  if (!workflow.value) return
  store.exportJson(workflow.value)
  toast.show('已导出工作流', 'success')
}

async function onDelete() {
  if (!workflow.value) return
  const name = workflow.value.name
  if (!confirm(`确定删除「${name}」？此操作不可撤销。`)) return
  const id = workflow.value.id
  if (id) {
    await store.deleteWorkflow(id)
  }
  store.current = null
  store.dirty = false
  toast.show(`已删除「${name}」`, 'success')
  emit('workflow-deleted')
}

async function onRun() {
  if (!workflow.value) return
  enh.clearLogs()
  // Register in global status
  if (workflow.value.id) {
    globalStatus.registerRun(workflow.value.id, workflow.value.name)
    currentRunId = workflow.value.id
  }
  await runWorkflow(workflow.value)
}

async function onStop() {
  stopWorkflow()
}


function onAddStep(type: ContainerType) {
  store.addStep(type)
  showAddStep.value = false
}

function onRemoveStep(stepId: string) {
  store.removeStep(stepId)
  if (selectedStepId.value === stepId) {
    selectedStepId.value = null
  }
}

function onAddAction(stepId: string) {
  const step = store.findStep(stepId)
  if (!step) return
  addActionOptions.value = getActionDefs(step.type).map(a => ({
    type: a.type, label: a.label, icon: a.icon,
  }))
  addActionStepId.value = stepId
}

function onSelectActionType(type: string) {
  if (!addActionStepId.value) return
  store.addAction(addActionStepId.value, type)
  addActionStepId.value = null
}

function onRemoveAction(stepId: string, actionId: string) {
  store.removeAction(stepId, actionId)
}

function onOpenConfig(stepId: string) {
  configStepId.value = stepId
}

function onUpdateContainerConfig(config: Record<string, unknown>) {
  if (!workflow.value || !configStepId.value) return
  const step = store.findStep(configStepId.value)
  if (step) {
    step.config = config
    store.dirty = true
  }
}

function onCloseConfig() {
  configStepId.value = null
}

function onUpdateCondition(stepId: string, condition: string) {
  if (!workflow.value) return
  const step = store.findStep(stepId)
  if (step) {
    step.condition = condition
    store.dirty = true
  }
}

function onErrorStrategyChange(stepId: string, strategy: import('../types/types').ErrorStrategy) {
  if (!workflow.value) return
  const step = store.findStep(stepId)
  if (step) {
    step.onError = strategy
    store.dirty = true
  }
}

function onDragStart(index: number, e: DragEvent) {
  dragIndex.value = index
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = 'move'
    e.dataTransfer.setData('text/plain', String(index))
  }
}

function onDragOver(index: number, e: DragEvent) {
  e.preventDefault()
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
  dropIndex.value = index
}

function onDragLeave() {
  dropIndex.value = null
}

function onDrop(index: number) {
  if (dragIndex.value !== null && dragIndex.value !== index) {
    store.moveStep(dragIndex.value, index)
  }
  dragIndex.value = null
  dropIndex.value = null
}

function onDragEnd() {
  dragIndex.value = null
  dropIndex.value = null
}

function onKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    e.preventDefault()
    onSave()
    return
  }
  if ((e.ctrlKey || e.metaKey) && e.key === 'z' && !e.shiftKey) {
    e.preventDefault()
    enh.undo()
    return
  }
  if ((e.ctrlKey || e.metaKey) && (e.key === 'y' || (e.key === 'z' && e.shiftKey))) {
    e.preventDefault()
    enh.redo()
    return
  }
  if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
    e.preventDefault()
    enh.toggleSearch()
    return
  }
  if (e.key === 'Escape') {
    if (enh.searchVisible.value) { enh.searchVisible.value = false; enh.searchQuery.value = '' }
    else if (showAddStep.value) showAddStep.value = false
    else if (addActionStepId.value) addActionStepId.value = null
  }
}

let unlistenLogStep: (() => void) | null = null
let unlistenLogRun: (() => void) | null = null

onMounted(async () => {
  unlistenLogStep = await safeListen<{
    run_id: string; step_id: string; step_name: string; status: string; output?: unknown; error?: string | null
  }>('step-update', (event) => {
    const { step_id, step_name, status, error } = event.payload
    const level: LogEntry['level'] = status === 'error' ? 'error' : status === 'running' ? 'info' : 'info'
    const msg = status === 'error' ? (error || '执行失败') : status === 'running' ? '开始执行...' : status === 'success' ? '执行成功' : status
    enh.addLog({ time: new Date().toLocaleTimeString(), stepId: step_id, stepName: step_name || step_id, status, message: msg, level })
    // Update global status progress
    if (currentRunId && workflow.value) {
      const steps = workflow.value.steps || []
      const done = steps.filter((_, i) => {
        const states = store.runStates
        const s = states[steps[i].id]
        return s && (s.status === 'success' || s.status === 'error')
      }).length
      globalStatus.updateRunProgress(currentRunId, step_name || step_id, done, steps.length)
    }
  })

  unlistenLogRun = await safeListen<{ run_id: string; status: string; error?: string }>('run-update', (event) => {
    const { status, error } = event.payload
    enh.addLog({ time: new Date().toLocaleTimeString(), stepId: '*', stepName: '工作流', status, message: status === 'completed' ? '运行完成' : status === 'error' ? `运行失败: ${error}` : status, level: status === 'error' ? 'error' : 'info' })
    // Unregister from global status when run finishes
    if (currentRunId && (status === 'completed' || status === 'failed' || status === 'cancelled')) {
      globalStatus.unregisterRun(currentRunId)
      currentRunId = null
    }
  })
})

onUnmounted(() => {
  unlistenLogStep?.()
  unlistenLogRun?.()
  // Clean up global status if run is still active
  if (currentRunId) {
    globalStatus.unregisterRun(currentRunId)
    currentRunId = null
  }
})
</script>

<template>
  <div
    class="flex-1 flex flex-col bg-background text-foreground overflow-hidden min-h-0"
    tabindex="0"
    @keydown="onKeydown"
  >
    <!-- Workflow Detail Card -->
    <Card color="#6e7681" class="mx-[var(--spacing-section-padding-x)] mt-6 shrink-0">
      <div class="px-4 py-3">
        <!-- Row 1: Title + Actions -->
        <div class="flex items-center gap-3">
          <!-- Name -->
          <div v-if="!editingName" class="flex-1 min-w-0">
            <span
              class="text-base font-semibold text-foreground cursor-text hover:text-primary transition-colors"
              title="点击编辑名称"
              @click="onStartEditName"
            >
              {{ workflow?.name || '未命名工作流' }}
            </span>
            <span v-if="store.dirty" class="text-warning text-xs ml-1">●</span>
            <span v-if="enh.lastSavedAt.value" class="text-xs text-muted-foreground ml-2">{{ enh.lastSavedAt.value }}</span>
          </div>
          <div v-else class="flex-1 min-w-0">
            <Input
              ref="nameInputRef"
              v-model="nameInput"
              class="h-7 max-w-[300px] text-sm font-semibold"
              @blur="onFinishEditName"
              @keydown.enter="onFinishEditName"
              @keydown.escape="editingName = false"
            />
          </div>

          <!-- Primary action -->
          <Button v-if="!isRunning" variant="default" size="sm" class="h-8 bg-success hover:bg-success/90 text-success-foreground shrink-0" @click="onRun">▶ 运行</Button>
          <Button v-else variant="destructive" size="sm" class="h-8 shrink-0" @click="onStop">■ 停止</Button>

          <!-- ⋯ Menu -->
          <div class="relative shrink-0" @click.stop>
            <Button ref="cardMenuBtnRef" variant="ghost" size="icon" class="text-muted-foreground hover:text-foreground opacity-50 hover:opacity-100 transition-opacity" @click="toggleCardMenu">⋯</Button>
          </div>
        </div>

        <!-- Row 2: Description (inline, click to expand) -->
        <div class="mt-1.5">
          <input
            v-if="workflow"
            :value="workflow.description"
            placeholder="输入工作流描述..."
            class="w-full text-xs text-muted-foreground bg-transparent border-0 outline-none placeholder:text-muted-foreground/40 hover:text-foreground transition-colors"
            @input="workflow.description = ($event.target as HTMLInputElement).value; store.dirty = true"
          />
        </div>
      </div>
    </Card>

    <!-- Card ⋯ Dropdown (teleported) -->
    <Teleport to="body">
      <div v-if="showCardMenu" class="fixed inset-0 z-40" @click="showCardMenu = false" />
      <div v-if="showCardMenu" class="fixed z-50 w-44 bg-background border border-border rounded-md shadow-lg py-1" :style="cardMenuPosStyle">
        <button class="w-full text-left px-3 py-2 text-sm hover:bg-accent flex items-center gap-2 transition-colors" @click="onSave(); showCardMenu = false">保存</button>
        <button class="w-full text-left px-3 py-2 text-sm hover:bg-accent flex items-center gap-2 transition-colors" @click="onSaveAs(); showCardMenu = false">另存为</button>
        <button class="w-full text-left px-3 py-2 text-sm hover:bg-accent flex items-center gap-2 transition-colors" @click="onExport(); showCardMenu = false">导出</button>
        <button class="w-full text-left px-3 py-2 text-sm hover:bg-accent flex items-center gap-2 transition-colors" @click="onScheduleClick(); showCardMenu = false">定时</button>
        <div class="border-t border-border my-1" />
        <button class="w-full text-left px-3 py-2 text-sm text-destructive hover:bg-destructive/10 flex items-center gap-2 transition-colors" @click="onDelete(); showCardMenu = false">删除</button>
      </div>
    </Teleport>

    <!-- Main Content -->
    <Tabs v-model="activeView" default-value="visual" class="flex-1 flex flex-col overflow-hidden min-h-0">
      <div class="px-[var(--spacing-section-padding-x)] pt-4 pb-0 shrink-0">
        <TabsList>
          <TabsTrigger value="visual">可视化</TabsTrigger>
          <TabsTrigger value="code">代码</TabsTrigger>
        </TabsList>
      </div>

      <TabsContent value="visual" class="flex-1 overflow-visible mt-0 p-0 min-h-0">
        <div class="flex flex-1 min-h-0">
          <!-- Step list area -->
          <div class="flex-1 overflow-y-auto px-[var(--spacing-section-padding-x)] pt-6 pb-12 space-y-[var(--spacing-step-gap)] min-h-0">
            <div v-if="!workflow?.steps?.length" class="text-center py-16 text-muted-foreground">
              <div class="text-lg text-foreground mb-2">还没有步骤</div>
              <div class="text-sm">点击下方「增加步骤」开始构建工作流</div>
            </div>

            <!-- Step cards (with drag) -->
            <div
              v-for="(step, index) in filteredSteps"
              :key="step.id"
              draggable="true"
              class="flex items-stretch transition-all"
              :class="{
                'opacity-40 scale-[0.98]': dragIndex === index,
                'border-t-2 border-primary -mt-0.5': dropIndex === index && dragIndex !== index,
              }"
              @dragstart="onDragStart(index, $event)"
              @dragover="onDragOver(index, $event)"
              @dragleave="onDragLeave"
              @drop="onDrop(index)"
              @dragend="onDragEnd"
            >
              <!-- Drag handle -->
              <div class="w-6 flex items-center justify-center text-border cursor-grab text-sm select-none shrink-0 rounded-l-md transition-colors hover:text-muted-foreground hover:bg-card active:cursor-grabbing">
                ⠿
              </div>

              <StepCard
                class="flex-1"
                :step="step"
                :run-state="store.runStates[step.id]"
                :total-steps="filteredSteps.length"
                :current-step-index="index"
                :is-recording="isRecording"
                :steps="workflow?.steps"
                @add-action="onAddAction"
                @remove-action="onRemoveAction"
                @rename-action="(sId, aId, label) => store.renameAction(sId, aId, label)"
                @update-action-params="(sId, aId, params) => store.updateActionParams(sId, aId, params)"
                @remove-step="onRemoveStep"
                @rename-step="(sId, label) => store.renameStep(sId, label)"
                @update-condition="onUpdateCondition"
                @update-condition-group="(sId, g) => store.updateConditionGroup(sId, g)"
                @update-run-condition="(sId, cond) => store.updateRunCondition(sId, cond)"
                @update-step-config="(sId, key, val) => store.updateStepConfig(sId, key, val)"
                @open-config="onOpenConfig"
                @update-error-strategy="onErrorStrategyChange"
                @start-recording="onStartRecording"
                @stop-recording="onStopRecording"
              />
            </div>

            <!-- Add step button -->
            <div class="relative mt-2">
              <Button
                variant="outline"
                size="lg"
                class="w-full py-2.5 text-sm text-muted-foreground border-dashed hover:border-primary hover:text-primary"
                @click="showAddStep = !showAddStep"
              >
                <ActionIcon name="Plus" cls="w-4 h-4" /> 增加步骤
              </Button>
              <Teleport to="body">
                <Transition name="fade">
                  <div v-if="showAddStep" class="fixed inset-0 z-[100]" @click="showAddStep = false">
                    <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-card border border-border rounded-lg p-2 min-w-[260px] shadow-xl" @click.stop>
                      <div
                        v-for="def in CONTAINER_DEFS"
                        :key="def.type"
                        class="flex items-center gap-2.5 px-3 py-2.5 rounded-md cursor-pointer transition-colors hover:bg-secondary"
                        @click="onAddStep(def.type)"
                      >
                        <ActionIcon :name="def.icon" cls="w-5 h-5 shrink-0" />
                        <div class="flex-1 min-w-0">
                          <div class="text-sm font-medium text-foreground">{{ def.label }}</div>
                          <div class="text-[11px] text-muted-foreground truncate">{{ def.description }}</div>
                        </div>
                      </div>
                    </div>
                  </div>
                </Transition>
              </Teleport>
            </div>
          </div>

          <!-- Container config panel (right side) -->
          <Transition name="panel-slide">
            <ContainerConfigPanel
              v-if="configStep"
              :step="configStep"
              @update-config="onUpdateContainerConfig"
              @close="onCloseConfig"
            />
          </Transition>
        </div>
      </TabsContent>

      <TabsContent value="code" class="flex-1 overflow-hidden mt-0 p-0">
        <CodeView
          v-if="workflow"
          :workflow="workflow"
          @update:workflow="(wf) => { if (store.current) { store.current.steps = wf.steps; store.current.name = wf.name; store.dirty = true } }"
        />
      </TabsContent>
    </Tabs>

    <!-- Log Panel (bottom bar, collapsed by default) -->
    <div
      class="border-t border-border bg-background shrink-0 cursor-pointer select-none"
      @click="enh.logPanelVisible.value = !enh.logPanelVisible.value"
    >
      <div class="flex items-center justify-between px-3 py-1.5">
        <div class="flex items-center gap-2">
          <span class="text-[11px] text-muted-foreground">{{ enh.logPanelVisible.value ? '▼' : '▶' }}</span>
          <span class="text-xs text-foreground">📟 运行日志</span>
          <span v-if="enh.logs.value.length" class="text-[10px] text-muted-foreground bg-secondary rounded px-1.5 py-0.5">{{ enh.logs.value.length }}</span>
        </div>
        <Button
          v-if="enh.logPanelVisible.value"
          variant="ghost"
          size="sm"
          class="h-5 text-[10px] text-muted-foreground"
          @click.stop="enh.clearLogs()"
        >清空</Button>
      </div>
      <Transition name="collapse">
        <div v-if="enh.logPanelVisible.value" class="max-h-[180px] overflow-y-auto border-t border-border">
          <div v-if="!enh.logs.value.length" class="text-muted-foreground text-xs p-3">
            暂无日志，运行工作流后显示
          </div>
          <div
            v-for="(log, i) in enh.logs.value"
            :key="i"
            class="flex items-baseline gap-2 px-3 py-0.5 text-[11px] font-mono text-muted-foreground hover:bg-card transition-colors"
            :class="{ 'text-danger': log.level === 'error', 'text-warning': log.level === 'warn' }"
          >
            <span class="text-muted-foreground/50 shrink-0 w-[70px]">{{ log.time }}</span>
            <span class="text-primary shrink-0 w-[100px] truncate">{{ log.stepName }}</span>
            <span class="flex-1">{{ log.message }}</span>
          </div>
        </div>
      </Transition>
    </div>

    <!-- Action type picker popup -->
    <Teleport to="body">
      <Transition name="fade">
        <div
          v-if="addActionStepId"
          class="fixed inset-0 bg-black/50 flex items-center justify-center z-[100]"
          @click="addActionStepId = null"
        >
          <div class="bg-card border border-border rounded-xl p-4 min-w-[280px] max-h-[400px] overflow-y-auto shadow-2xl" @click.stop>
            <div class="text-sm font-semibold text-foreground mb-3 px-1">选择动作类型</div>
            <div
              v-for="opt in addActionOptions"
              :key="opt.type"
              class="flex items-center gap-2.5 px-3 py-2.5 rounded-md cursor-pointer transition-colors hover:bg-secondary"
              @click="onSelectActionType(opt.type)"
            >
              <ActionIcon :name="opt.icon" cls="w-4 h-4" />
              <span class="text-sm font-medium text-foreground">{{ opt.label }}</span>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.panel-slide-enter-active,
.panel-slide-leave-active {
  transition: transform 0.2s ease, opacity 0.2s ease;
}
.panel-slide-enter-from,
.panel-slide-leave-to {
  transform: translateX(20px);
  opacity: 0;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.collapse-enter-active,
.collapse-leave-active {
  transition: all 0.15s ease;
  overflow: hidden;
}
.collapse-enter-from,
.collapse-leave-to {
  max-height: 0;
  opacity: 0;
}
</style>
