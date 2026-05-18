<script setup lang="ts">
import { ref, computed, onMounted, watch, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useWorkflowStore } from '../stores/workflowStore'
import { useStepRunner } from '../composables/useStepRunner'
import { useToast } from '../composables/useToast'
import { useEditorEnhancements, type LogEntry } from '../composables/useEditorEnhancements'
import { useGlobalStatus } from '../composables/useGlobalStatus'
import { useOpsConsole } from '../composables/useOpsConsole'
import { safeInvoke, safeListen } from '../utils/tauri'
import StepCard from '../components/StepCard.vue'
import ActionIcon from '../components/ActionIcon.vue'
import ContainerConfigPanel from '../components/ContainerConfigPanel.vue'
import CodeView from '../components/CodeView.vue'
import Button from '../components/ui/button/Button.vue'
import Input from '../components/ui/input/Input.vue'
import Card from '../components/ui/card/Card.vue'
import Tabs from '../components/ui/tabs/Tabs.vue'
import TabsList from '../components/ui/tabs/TabsList.vue'
import TabsTrigger from '../components/ui/tabs/TabsTrigger.vue'
import TabsContent from '../components/ui/tabs/TabsContent.vue'
import Dialog from '../components/ui/dialog/Dialog.vue'
import DialogContent from '../components/ui/dialog/DialogContent.vue'
import DialogTitle from '../components/ui/dialog/DialogTitle.vue'
import DialogDescription from '../components/ui/dialog/DialogDescription.vue'
import DialogFooter from '../components/ui/dialog/DialogFooter.vue'
import type { ContainerType, Step } from '../types/types'
import { CONTAINER_DEFS, getActionDefs } from '../types/node-registry'

const { t } = useI18n()

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
const ops = useOpsConsole()

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
const showDeleteConfirm = ref(false)
const pendingDelete = ref<{ name: string; id: string }>({ name: '', id: '' })

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
    toast.info(url ? `录制已开始，已打开 ${url}` : t('toast.running'))
  } catch (e: unknown) {
    recordingError.value = (e as Error).message || t('error.generic')
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
      toast.info(t('common.stop'))
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

const translatedContainerDefs = computed(() =>
  CONTAINER_DEFS.map(d => ({
    ...d,
    label: t(`nodeLabel.${d.type}`, d.label),
    description: t(`nodeDesc.${d.type}`, d.description),
  }))
)

watch(() => props.workflowId, async (newId) => {
  if (newId) {
    await store.loadWorkflow(newId)
  } else if (newId === null || newId === undefined) {
    store.current = {
      name: t('editor.untitled'),
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
    toast.show(t('editor.saveSuccess'), 'success')
    // 显示变量引用警告
    if (store.lastWarnings.length > 0) {
      const msg = store.lastWarnings.slice(0, 5).join('\n')
      const extra = store.lastWarnings.length > 5 ? `\n...还有 ${store.lastWarnings.length - 5} 条` : ''
      toast.show(`变量引用警告:\n${msg}${extra}`, 'info')
    }
    emit('workflow-updated')
  } else {
    toast.show(t('editor.saveFailed'), 'error')
  }
}

async function onScheduleClick() {
  if (!workflow.value) return
  // 未保存时先自动保存
  if (!workflow.value.id) {
    const ok = await store.saveWorkflow()
    if (!ok) { toast.show(t('editor.saveFirst'), 'error'); return }
    emit('workflow-updated')
  }
  if (workflow.value.id) {
    emit('schedule', workflow.value.id)
  }
}

async function onSaveAs() {
  if (!workflow.value) return
  const originalName = workflow.value.name
  workflow.value.name = originalName +  + ' (' + t('common.copy') + ')'
  workflow.value.id = undefined as unknown as string
  store.dirty = true
  const ok = await store.saveWorkflow()
  if (ok) {
    toast.show(t('editor.savedAs', { name: workflow.value.name }), 'success')
    emit('workflow-updated')
  } else {
    toast.show(t('error.saveFailed'), 'error')
  }
}

async function onExport() {
  if (!workflow.value) return
  store.exportJson(workflow.value)
  toast.show(t('editor.exported'), 'success')
}

async function onDelete() {
  if (!workflow.value) return
  const name = workflow.value.name
  const id = workflow.value.id
  if (!id) return
  pendingDelete.value = { name, id }
  showDeleteConfirm.value = true
}

async function doDelete() {
  const { id } = pendingDelete.value
  await store.deleteWorkflow(id)
  store.current = null
  store.dirty = false
  toast.show(t('toast.deleted'), 'success')
  emit('workflow-deleted')
  showDeleteConfirm.value = false
}

async function onToggleLock() {
  if (!workflow.value?.id) return
  const newLocked = !workflow.value.locked
  try {
    await safeInvoke('workflow_lock', { id: workflow.value.id, locked: newLocked })
    workflow.value.locked = newLocked
    toast.show(newLocked ? t('editor.locked') : t('editor.unlocked'), 'success')
  } catch (e: unknown) {
    toast.error('操作失败: ' + ((e as Error).message || e))
  }
}

const isLocked = computed(() => workflow.value?.locked === true)

async function onRun() {
  if (!workflow.value) return
  enh.clearLogs()
  // Auto-expand console
  if (!ops.visible.value) ops.toggle()
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
  addActionOptions.value = getActionDefs(step.type, t).map(a => ({
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
    const msg = status === 'error' ? (error || t('error.runFailed')) : status === 'running' ? t('toast.running') : status === 'success' ? t('editor.stepSuccess') : status
    enh.addLog({ time: new Date().toLocaleTimeString(), stepId: step_id, stepName: step_name || step_id, status, message: msg, level })
    // Log to unified console
    const detail = status === 'error' ? (error || '') : `step: ${step_name || step_id}`
    ops.addOp({ source: 'agent', category: 'event', name: `step: ${status}`, status: status === 'error' ? 'fail' : 'ok', detail })
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
    enh.addLog({ time: new Date().toLocaleTimeString(), stepId: '*', stepName: t('nav.workflows'), status, message: status === 'completed' ? t('editor.runComplete') : status === 'error' ? t('editor.runFailedMsg', { error }) : status, level: status === 'error' ? 'error' : 'info' })
    // Log to unified console
    ops.addOp({
      source: 'agent',
      category: 'event',
      name: `run: ${status}`,
      status: status === 'failed' ? 'fail' : 'ok',
      detail: status === 'completed' ? t('toast.completed') : status === 'failed' ? (error || t('common.unknown')) : status,
    })
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
      <!-- Loading skeleton -->
      <div v-if="store.loading" class="px-4 py-3 space-y-3 animate-pulse">
        <div class="h-5 bg-secondary/50 rounded w-1/3" />
        <div class="h-3 bg-secondary/30 rounded w-2/3" />
      </div>
      <div v-else class="px-4 py-3">
        <!-- Row 1: Title + Actions -->
        <div class="flex items-center gap-3">
          <!-- Name -->
          <div v-if="!editingName" class="flex-1 min-w-0">
            <span
              class="text-base font-semibold text-foreground cursor-text hover:text-primary transition-colors"
              :class="{ 'cursor-not-allowed opacity-60': isLocked }"
              :title="isLocked ? t('editor.lockedHint2') : t('editor.clickToEditName')"
              @click="isLocked ? undefined : onStartEditName()"
            >
              {{ workflow?.name || t('editor.untitled') }}
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

          <!-- Lock toggle -->
          <Button
            variant="ghost"
            size="icon"
            class="h-7 w-7 shrink-0"
            :class="isLocked ? 'text-warning' : 'text-muted-foreground/30 hover:text-muted-foreground'"
            :title="isLocked ? t('editor.unlockToEdit') : t('editor.lockToPrevent')"
            @click="onToggleLock"
          >{{ isLocked ? '🔒' : '🔓' }}</Button>

          <!-- Primary action -->
          <Button v-if="!isRunning" variant="default" size="sm" class="h-8 bg-success hover:bg-success/90 text-success-foreground shrink-0" @click="onRun">{{ t('editor.run') }}</Button>
          <Button v-else variant="destructive" size="sm" class="h-8 shrink-0" @click="onStop">{{ t('editor.stop') }}</Button>

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
            :placeholder="t('editor.descPlaceholder')"
            class="w-full text-xs text-muted-foreground bg-transparent border-0 outline-none placeholder:text-muted-foreground/40 hover:text-foreground transition-colors"
            @input="workflow.description = ($event.target as HTMLInputElement).value; store.dirty = true"
          />
        </div>
      </div>
    </Card>

    <!-- Card ⋯ Dropdown (teleported) -->
    <Teleport to="body">
      <div v-if="showCardMenu" class="fixed inset-0 z-40" @click="showCardMenu = false" @keydown.escape="showCardMenu = false" />
      <div v-if="showCardMenu" class="fixed z-50 w-44 bg-background border border-border rounded-md shadow-lg py-1" :style="cardMenuPosStyle">
        <button class="w-full text-left px-3 py-2 text-sm hover:bg-accent flex items-center gap-2 transition-colors" @click="onSave(); showCardMenu = false">{{ t('common.save') }}</button>
        <button class="w-full text-left px-3 py-2 text-sm hover:bg-accent flex items-center gap-2 transition-colors" @click="onSaveAs(); showCardMenu = false">{{ t('editor.saveAs') }}</button>
        <button class="w-full text-left px-3 py-2 text-sm hover:bg-accent flex items-center gap-2 transition-colors" @click="onExport(); showCardMenu = false">{{ t('common.export') }}</button>
        <button class="w-full text-left px-3 py-2 text-sm hover:bg-accent flex items-center gap-2 transition-colors" @click="onScheduleClick(); showCardMenu = false">{{ t('editor.schedule') }}</button>
        <div class="border-t border-border my-1" />
        <button class="w-full text-left px-3 py-2 text-sm text-destructive hover:bg-destructive/10 flex items-center gap-2 transition-colors" @click="onDelete(); showCardMenu = false">{{ t('common.delete') }}</button>
      </div>
    </Teleport>

    <!-- Main Content -->
    <Tabs v-model="activeView" default-value="visual" class="flex-1 flex flex-col overflow-hidden min-h-0">
      <div class="px-[var(--spacing-section-padding-x)] pt-4 pb-0 shrink-0">
        <TabsList>
          <TabsTrigger value="visual">{{ t('editor.visual') }}</TabsTrigger>
          <TabsTrigger value="code">{{ t('editor.code') }}</TabsTrigger>
        </TabsList>
      </div>

      <TabsContent value="visual" class="flex-1 overflow-visible mt-0 p-0 min-h-0">
        <div class="flex flex-1 min-h-0">
          <!-- Step list area -->
          <div class="flex-1 overflow-y-auto px-[var(--spacing-section-padding-x)] pt-6 pb-12 space-y-[var(--spacing-step-gap)] min-h-0">
            <div v-if="!workflow?.steps?.length" class="text-center py-16 text-muted-foreground">
              <div class="text-lg text-foreground mb-2">{{ t('editor.noStepsTitle') }}</div>
              <div class="text-sm">{{ t('editor.noStepsHint') }}</div>
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
              <div class="w-6 flex items-center justify-center text-border cursor-grab select-none shrink-0 rounded-l-md transition-colors hover:text-muted-foreground hover:bg-card active:cursor-grabbing">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><circle cx="9" cy="5" r="1.5"/><circle cx="15" cy="5" r="1.5"/><circle cx="9" cy="12" r="1.5"/><circle cx="15" cy="12" r="1.5"/><circle cx="9" cy="19" r="1.5"/><circle cx="15" cy="19" r="1.5"/></svg>
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
                <ActionIcon name="Plus" cls="w-4 h-4" /> {{ t('editor.addStep') }}
              </Button>
              <Teleport to="body">
                <Transition name="fade">
                  <div v-if="showAddStep" class="fixed inset-0 z-[100]" role="dialog" aria-modal="true" @click="showAddStep = false" @keydown.escape="showAddStep = false">
                    <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-card border border-border rounded-lg p-2 min-w-[260px] shadow-xl" @click.stop>
                      <div
                        v-for="def in translatedContainerDefs"
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
              :steps="workflow?.steps"
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

    <!-- Action type picker popup -->
    <Teleport to="body">
      <Transition name="fade">
        <div
          v-if="addActionStepId"
          class="fixed inset-0 bg-black/50 flex items-center justify-center z-[100]"
          @click="addActionStepId = null"
        >
          <div class="bg-card border border-border rounded-xl p-4 min-w-[280px] max-h-[400px] overflow-y-auto shadow-2xl" @click.stop>
            <div class="text-sm font-semibold text-foreground mb-3 px-1">{{ t('editor.selectActionType') }}</div>
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

    <!-- Delete confirmation dialog -->
    <Dialog :open="showDeleteConfirm" @update:open="showDeleteConfirm = $event">
      <DialogContent class="sm:max-w-md">
        <DialogTitle>{{ t('editor.deleteWorkflow') }}</DialogTitle>
        <DialogDescription>
          {{ t('editor.deleteWorkflowConfirm', { name: pendingDelete.name }) }}
        </DialogDescription>
        <DialogFooter>
          <Button variant="ghost" @click="showDeleteConfirm = false">{{ t('common.cancel') }}</Button>
          <Button variant="destructive" @click="doDelete">{{ t('common.delete') }}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
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
