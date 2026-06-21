<script setup lang="ts">
import { watch, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useEditorActions } from '../composables/useEditorActions'
import type { LogEntry } from '../composables/useEditorEnhancements'
import { safeListen } from '../utils/tauri'
import StepCard from '../components/StepCard.vue'
import ActionIcon from '../components/ActionIcon.vue'
import ContainerConfigPanel from '../components/ContainerConfigPanel.vue'
import DebugPanel from '../components/DebugPanel.vue'
import CodeView from '../components/CodeView.vue'
import CanvasEditor from '../components/CanvasEditor.vue'
import Button from '../components/ui/button/Button.vue'
import Tabs from '../components/ui/tabs/Tabs.vue'
import TabsList from '../components/ui/tabs/TabsList.vue'
import TabsTrigger from '../components/ui/tabs/TabsTrigger.vue'
import TabsContent from '../components/ui/tabs/TabsContent.vue'
import Dialog from '../components/ui/dialog/Dialog.vue'
import { GripVertical } from 'lucide-vue-next'
import DialogContent from '../components/ui/dialog/DialogContent.vue'
import DialogTitle from '../components/ui/dialog/DialogTitle.vue'
import DialogDescription from '../components/ui/dialog/DialogDescription.vue'
import DialogFooter from '../components/ui/dialog/DialogFooter.vue'
import WorkflowHeader from '../components/WorkflowHeader.vue'
import AddStepDialog from '../components/AddStepDialog.vue'
import ActionTypePicker from '../components/ActionTypePicker.vue'


const { t } = useI18n()

const props = defineProps<{
  workflowId?: string | null
}>()

const emit = defineEmits<{
  'schedule': [id: string]
  'workflow-updated': []
  'workflow-deleted': []
}>()

const a = useEditorActions()

// ─── Lifecycle: load workflow on id change ───
watch(() => props.workflowId, async (newId) => {
  if (newId) {
    await a.store.loadWorkflow(newId)
  } else if (newId === null || newId === undefined) {
    a.store.current = {
      name: t('editor.untitled'),
      description: '',
      steps: [],
    }
    a.store.dirty = false
  }
}, { immediate: true })

// ─── Schedule (with emit bridge) ───
async function handleSchedule() {
  const id = await a.onScheduleClick()
  if (id) emit('schedule', id)
}

// ─── Save (with emit) ───
async function handleSave() {
  await a.onSave()
  emit('workflow-updated')
}

async function handleSaveAs() {
  await a.onSaveAs()
  emit('workflow-updated')
}

function handleDelete() {
  a.onDelete()
}

async function handleDoDelete() {
  await a.doDelete()
  emit('workflow-deleted')
}

// ─── Active view tab ───
import { ref } from 'vue'
const activeView = ref<'visual' | 'code' | 'canvas'>('visual')

// ─── Keyboard shortcuts ───
function onKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    e.preventDefault(); handleSave(); return
  }
  if ((e.ctrlKey || e.metaKey) && e.key === 'z' && !e.shiftKey) {
    e.preventDefault(); a.enh.undo(); return
  }
  if ((e.ctrlKey || e.metaKey) && (e.key === 'y' || (e.key === 'z' && e.shiftKey))) {
    e.preventDefault(); a.enh.redo(); return
  }
  if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
    e.preventDefault(); a.enh.toggleSearch(); return
  }
  if (e.key === 'Escape') {
    if (a.enh.searchVisible.value) { a.enh.searchVisible.value = false; a.enh.searchQuery.value = '' }
    else if (a.showAddStep.value) a.showAddStep.value = false
    else if (a.addActionStepId.value) a.addActionStepId.value = null
  }
}

// ─── Event listeners ───
let unlistenLogStep: (() => void) | null = null
let unlistenLogRun: (() => void) | null = null

onMounted(async () => {
  a.registry.refreshDynamicTypes()

  unlistenLogStep = await safeListen<{
    run_id: string; step_id: string; step_name: string; status: string; output?: unknown; error?: string | null
  }>('step-update', (event) => {
    const { step_id, step_name, status, error } = event.payload
    const level: LogEntry['level'] = status === 'error' ? 'error' : 'info'
    const msg = status === 'error' ? (error || t('error.runFailed')) : status === 'running' ? t('toast.running') : status === 'success' ? t('editor.stepSuccess') : status
    a.enh.addLog({ time: new Date().toLocaleTimeString(), stepId: step_id, stepName: step_name || step_id, status, message: msg, level })
    const detail = status === 'error' ? (error || '') : `step: ${step_name || step_id}`
    a.ops.addOp({ source: 'agent', category: 'event', name: `step: ${status}`, status: status === 'error' ? 'fail' : 'ok', detail })
    const runId = a.currentRunId()
    if (runId && a.workflow.value) {
      const steps = a.workflow.value.steps || []
      const done = steps.filter((_, i) => {
        const s = a.store.runStates[steps[i].id]
        return s && (s.status === 'success' || s.status === 'error')
      }).length
      a.globalStatus.updateRunProgress(runId, step_name || step_id, done, steps.length)
    }
  })

  unlistenLogRun = await safeListen<{ run_id: string; status: string; error?: string }>('run-update', (event) => {
    const { status, error } = event.payload
    a.enh.addLog({ time: new Date().toLocaleTimeString(), stepId: '*', stepName: t('nav.workflows'), status, message: status === 'completed' ? t('editor.runComplete') : status === 'error' ? t('editor.runFailedMsg', { error }) : status, level: status === 'error' ? 'error' : 'info' })
    a.ops.addOp({
      source: 'agent', category: 'event', name: `run: ${status}`,
      status: status === 'failed' ? 'fail' : 'ok',
      detail: status === 'completed' ? t('toast.completed') : status === 'failed' ? (error || t('common.unknown')) : status,
    })
    const runId = a.currentRunId()
    if (runId && (status === 'completed' || status === 'failed' || status === 'cancelled')) {
      a.globalStatus.unregisterRun(runId)
      a.clearRunId()
    }
  })
})

onUnmounted(() => {
  unlistenLogStep?.()
  unlistenLogRun?.()
  const runId = a.currentRunId()
  if (runId) {
    a.globalStatus.unregisterRun(runId)
    a.clearRunId()
  }
})
</script>

<template>
  <div
    class="flex-1 flex flex-col bg-background text-foreground overflow-hidden min-h-0"
    tabindex="0"
    @keydown="onKeydown"
  >
    <!-- Workflow Header -->
    <WorkflowHeader
      :workflow="a.workflow.value"
      :loading="a.store.loading"
      :dirty="a.store.dirty"
      :is-locked="a.isLocked.value"
      :is-running="a.isRunning.value"
      :editing-name="a.editingName.value"
      :name-input="a.nameInput.value"
      :last-saved-at="a.enh.lastSavedAt.value"
      @update:editing-name="a.editingName.value = $event"
      @update:name-input="a.nameInput.value = $event"
      @finish-edit-name="a.onFinishEditName()"
      @save="handleSave()"
      @save-as="handleSaveAs()"
      @export="a.onExport()"
      @schedule="handleSchedule()"
      @delete="handleDelete()"
      @toggle-lock="a.onToggleLock()"
      @run="a.onRun()"
      @stop="a.onStop()"
    />

    <!-- Main Content -->
    <Tabs v-model="activeView" default-value="visual" class="flex-1 flex flex-col overflow-hidden min-h-0">
      <div class="px-[var(--spacing-section-padding-x)] pt-4 pb-0 shrink-0">
        <TabsList>
          <TabsTrigger value="visual">{{ t('editor.visual') }}</TabsTrigger>
          <TabsTrigger value="canvas">{{ t('editor.canvas') }}</TabsTrigger>
          <TabsTrigger value="code">{{ t('editor.code') }}</TabsTrigger>
        </TabsList>
      </div>

      <TabsContent value="visual" class="flex-1 overflow-visible mt-0 p-0 min-h-0">
        <div class="flex flex-1 min-h-0">
          <!-- Step list area -->
          <div class="flex-1 overflow-y-auto px-[var(--spacing-section-padding-x)] pt-6 pb-12 space-y-[var(--spacing-step-gap)] min-h-0">
            <div v-if="!a.workflow.value?.steps?.length" class="flex flex-col items-center justify-center py-16 text-center">
              <ActionIcon name="Layers" cls="w-10 h-10 text-muted-foreground/30 mb-4" />
              <div class="text-lg font-medium text-foreground mb-1">{{ t('editor.noStepsTitle') }}</div>
              <div class="text-sm text-muted-foreground">{{ t('editor.noStepsHint') }}</div>
            </div>

            <!-- Step cards (with drag) -->
            <div
              v-for="(step, index) in a.filteredSteps.value"
              :key="step.id"
              draggable="true"
              class="flex items-stretch transition-all"
              :class="{
                'opacity-40 scale-[0.98]': a.dragIndex.value === index,
                'border-t-2 border-primary -mt-0.5': a.dropIndex.value === index && a.dragIndex.value !== index,
              }"
              @dragstart="a.onDragStart(index, $event)"
              @dragover="a.onDragOver(index, $event)"
              @dragleave="a.onDragLeave"
              @drop="a.onDrop(index)"
              @dragend="a.onDragEnd"
            >
              <!-- Drag handle -->
              <div class="w-6 flex items-center justify-center text-muted-foreground cursor-grab select-none shrink-0 rounded-l-md transition-colors hover:text-foreground hover:bg-muted active:cursor-grabbing">
                <GripVertical class="w-3 h-3 shrink-0" />
              </div>

              <StepCard
                class="flex-1"
                :step="step"
                :run-state="a.store.runStates[step.id]"
                :total-steps="a.filteredSteps.value.length"
                :current-step-index="index"
                :steps="a.workflow.value?.steps"
                @add-action="a.onAddAction"
                @remove-action="a.onRemoveAction"
                @rename-action="(sId, aId, label) => a.store.renameAction(sId, aId, label)"
                @update-action-params="(sId, aId, params) => a.store.updateActionParams(sId, aId, params)"
                @remove-step="a.onRemoveStep"
                @rename-step="(sId, label) => a.store.renameStep(sId, label)"
                @update-condition="a.onUpdateCondition"
                @update-condition-group="(sId, g) => a.store.updateConditionGroup(sId, g)"
                @update-run-condition="(sId, cond) => a.store.updateRunCondition(sId, cond)"
                @update-step-config="(sId, key, val) => a.store.updateStepConfig(sId, key, val)"
                @open-config="a.onOpenConfig"
                @update-error-strategy="a.onErrorStrategyChange"
              />
            </div>

            <!-- Add step button -->
            <div class="relative mt-2">
              <Button
                variant="outline"
                size="lg"
                class="w-full py-2.5 text-sm text-muted-foreground border-dashed hover:border-primary hover:text-primary"
                @click="a.showAddStep.value = !a.showAddStep.value"
              >
                <ActionIcon name="Plus" cls="w-4 h-4" /> {{ t('editor.addStep') }}
              </Button>
            </div>
          </div>

          <!-- Container config panel (right side) -->
          <Transition name="panel-slide">
            <ContainerConfigPanel
              v-if="a.configStep.value"
              :step="a.configStep.value"
              :steps="a.workflow.value?.steps"
              @update-config="a.onUpdateContainerConfig"
              @close="a.onCloseConfig"
            />
          </Transition>

          <!-- Debug panel (right side, when running) -->
          <Transition name="panel-slide">
            <div
              v-if="a.isRunning.value && !a.configStep.value"
              class="w-[320px] border-l border-border bg-background shrink-0 overflow-hidden"
            >
              <DebugPanel
                :workflow-id="a.workflow.value?.id"
                :is-running="a.isRunning.value"
                @set-breakpoint="(stepId) => { if (a.workflow.value) { const s = a.workflow.value.steps.find(s => s.id === stepId); if (s) s.breakpoint = true } }"
                @remove-breakpoint="(stepId) => { if (a.workflow.value) { const s = a.workflow.value.steps.find(s => s.id === stepId); if (s) s.breakpoint = false } }"
              />
            </div>
          </Transition>
        </div>
      </TabsContent>

      <TabsContent value="code" class="flex-1 overflow-hidden mt-0 p-0">
        <CodeView
          v-if="a.workflow.value"
          :workflow="a.workflow.value"
          @update:workflow="(wf) => { if (a.store.current) { a.store.current.steps = wf.steps; a.store.current.name = wf.name; a.store.dirty = true } }"
        />
      </TabsContent>

      <TabsContent value="canvas" class="flex-1 overflow-hidden mt-0 p-0 min-h-0">
        <CanvasEditor
          :workflow="a.workflow.value"
          :run-states="a.store.runStates"
          @add-edge="(from, to) => a.store.addEdge(from, to)"
          @remove-edge="(from, to) => a.store.removeEdge(from, to)"
        />
      </TabsContent>
    </Tabs>

    <!-- Add step dialog -->
    <AddStepDialog
      :show="a.showAddStep.value"
      @close="a.showAddStep.value = false"
      @select="(type) => a.onAddStep(type)"
    />

    <!-- Action type picker -->
    <ActionTypePicker
      :show="!!a.addActionStepId.value"
      :options="a.addActionOptions.value"
      @close="a.addActionStepId.value = null"
      @select="(type) => a.onSelectActionType(type)"
    />

    <!-- Delete confirmation dialog -->
    <Dialog :open="a.showDeleteConfirm.value" @update:open="a.showDeleteConfirm.value = $event">
      <DialogContent class="sm:max-w-md">
        <DialogTitle>{{ t('editor.deleteWorkflow') }}</DialogTitle>
        <DialogDescription>
          {{ t('editor.deleteWorkflowConfirm', { name: a.pendingDelete.value.name }) }}
        </DialogDescription>
        <DialogFooter>
          <Button variant="ghost" @click="a.showDeleteConfirm.value = false">{{ t('common.cancel') }}</Button>
          <Button variant="destructive" @click="handleDoDelete">{{ t('common.delete') }}</Button>
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
