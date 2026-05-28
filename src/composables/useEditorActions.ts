import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useWorkflowStore } from '../stores/workflowStore'
import { useStepRunner } from './useStepRunner'
import { useToast } from './useToast'
import { useEditorEnhancements } from './useEditorEnhancements'
import { useGlobalStatus } from './useGlobalStatus'
import { useOpsConsole } from './useOpsConsole'
import { useRegistry } from './useRegistry'
import { safeInvoke } from '../utils/tauri'
import type { ContainerType, Step, ErrorStrategy } from '../types/types'
import { getActionDefs } from '../types/node-registry'

export function useEditorActions() {
  const { t } = useI18n()
  const store = useWorkflowStore()
  const { runWorkflow, stopWorkflow, isRunning } = useStepRunner()
  const toast = useToast()
  const enh = useEditorEnhancements()
  const globalStatus = useGlobalStatus()
  const ops = useOpsConsole()
  const registry = useRegistry()

  let currentRunId: string | null = null

  // ─── State ───
  const editingName = ref(false)
  const nameInput = ref('')
  const selectedStepId = ref<string | null>(null)
  const configStepId = ref<string | null>(null)
  const showAddStep = ref(false)
  const showDeleteConfirm = ref(false)
  const pendingDelete = ref<{ name: string; id: string }>({ name: '', id: '' })
  const addActionStepId = ref<string | null>(null)
  const addActionOptions = ref<{ type: string; label: string; icon: string }[]>([])
  const dragIndex = ref<number | null>(null)
  const dropIndex = ref<number | null>(null)

  // ─── Computed ───
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

  const isLocked = computed(() => workflow.value?.locked === true)

  // ─── Name editing ───
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

  // ─── Save / Export / Delete ───
  async function onSave() {
    const ok = await store.saveWorkflow()
    if (ok) {
      toast.show(t('editor.saveSuccess'), 'success')
      if (store.lastWarnings.length > 0) {
        const msg = store.lastWarnings.slice(0, 5).join('\n')
        const extra = store.lastWarnings.length > 5 ? `\n...还有 ${store.lastWarnings.length - 5} 条` : ''
        toast.show(`变量引用警告:\n${msg}${extra}`, 'info')
      }
    } else {
      toast.show(t('editor.saveFailed'), 'error')
    }
  }

  async function onSaveAs() {
    if (!workflow.value) return
    const originalName = workflow.value.name
    workflow.value.name = originalName + ' (' + t('common.copy') + ')'
    workflow.value.id = undefined as unknown as string
    store.dirty = true
    const ok = await store.saveWorkflow()
    if (ok) {
      toast.show(t('editor.savedAs', { name: workflow.value.name }), 'success')
    } else {
      toast.show(t('error.saveFailed'), 'error')
    }
  }

  function onExport() {
    if (!workflow.value) return
    store.exportJson(workflow.value)
    toast.show(t('editor.exported'), 'success')
  }

  function onDelete() {
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

  // ─── Run / Stop ───
  async function onRun() {
    if (!workflow.value) return
    enh.clearLogs()
    if (!ops.visible.value) ops.toggle()
    if (workflow.value.id) {
      globalStatus.registerRun(workflow.value.id, workflow.value.name)
      currentRunId = workflow.value.id
    }
    await runWorkflow(workflow.value)
  }

  function onStop() {
    stopWorkflow()
  }

  // ─── Schedule ───
  async function onScheduleClick(): Promise<string | null> {
    if (!workflow.value) return null
    if (!workflow.value.id) {
      const ok = await store.saveWorkflow()
      if (!ok) { toast.show(t('editor.saveFirst'), 'error'); return null }
    }
    return workflow.value.id || null
  }

  // ─── Step CRUD ───
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

  // ─── Action CRUD ───
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

  // ─── Config panel ───
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

  // ─── Condition / Error ───
  function onUpdateCondition(stepId: string, condition: string) {
    if (!workflow.value) return
    const step = store.findStep(stepId)
    if (step) {
      step.condition = condition
      store.dirty = true
    }
  }

  function onErrorStrategyChange(stepId: string, strategy: ErrorStrategy) {
    if (!workflow.value) return
    const step = store.findStep(stepId)
    if (step) {
      step.onError = strategy
      store.dirty = true
    }
  }

  // ─── Drag & Drop ───
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

  return {
    // State
    store, workflow, isRunning, isLocked,
    editingName, nameInput,
    selectedStepId, configStepId, showAddStep,
    showDeleteConfirm, pendingDelete,
    addActionStepId, addActionOptions,
    dragIndex, dropIndex,
    filteredSteps, configStep,
    // Enhancements / services
    enh, globalStatus, ops, registry, toast,
    currentRunId: () => currentRunId,
    clearRunId: () => { currentRunId = null },
    // Actions
    onStartEditName, onFinishEditName,
    onSave, onSaveAs, onExport, onDelete, doDelete, onToggleLock,
    onRun, onStop, onScheduleClick,
    onAddStep, onRemoveStep,
    onAddAction, onSelectActionType, onRemoveAction,
    onOpenConfig, onUpdateContainerConfig, onCloseConfig,
    onUpdateCondition, onErrorStrategyChange,
    onDragStart, onDragOver, onDragLeave, onDrop, onDragEnd,
  }
}
