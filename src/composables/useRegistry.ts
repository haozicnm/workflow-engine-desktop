/**
 * useRegistry — locale-aware wrapper around node-registry.ts
 * 
 * All UI-facing strings (label, description, outputHint, param labels)
 * are resolved through vue-i18n. The static registries remain as the
 * source of truth for type/icon/color/params structure.
 */
import { useI18n } from 'vue-i18n'
import type { ContainerDef, ActionDef, ContainerType, Action, Step } from '@/types/types'
import {
  CONTAINER_DEFS,
  allContainerDefs,
  BROWSER_ACTIONS,
  EXCEL_ACTIONS,
  WORD_ACTIONS,
  LOGIC_ACTIONS,
  LOGIC_OPERATORS,
  BODY_STEP_ACTIONS,
  FILE_ACTIONS,
  getContainerDef as rawGetContainerDef,
  newAction as rawNewAction,
  newStep as rawNewStep,
} from '@/types/node-registry'
import { syncNodeSchema } from '@/composables/useNodeSchema'

/** Build a locale-aware ContainerDef from the raw one */
function localizeContainerDef(raw: ContainerDef, t: (key: string) => string): ContainerDef {
  return {
    ...raw,
    label: t(`nodeLabel.${raw.type}`) || raw.label,
    description: t(`nodeDesc.${raw.type}`) || raw.description,
    outputHint: t(`nodeOutputHint.${raw.type}`) || raw.outputHint,
    params: raw.params.map(p => ({
      ...p,
      label: t(`paramLabel.${p.key}`) || p.label,
      options: p.options?.map(o => ({
        ...o,
        label: t(`paramLabel.${o.value}`) || t(`common.${o.value}`) || o.label,
      })),
    })),
  }
}

/** Build a locale-aware ActionDef from the raw one */
function localizeActionDef(raw: ActionDef, t: (key: string) => string): ActionDef {
  return {
    ...raw,
    label: t(`actionLabel.${raw.type}`) || raw.label,
    params: raw.params.map(p => ({
      ...p,
      label: t(`paramLabel.${p.key}`) || p.label,
      options: p.options?.map(o => ({
        ...o,
        label: t(`paramLabel.${o.value}`) || t(`common.${o.value}`) || o.label,
      })),
    })),
  }
}

/** Memoization cache keyed by locale */
const _cache = new Map<string, {
  containerDefs: ContainerDef[]
  browserActions: ActionDef[]
  excelActions: ActionDef[]
  wordActions: ActionDef[]
  logicActions: ActionDef[]
  logicOperators: typeof LOGIC_OPERATORS
  bodyStepActions: ActionDef[]
  fileActions: ActionDef[]
}>()

function buildCache(locale: string, t: (key: string) => string) {
  const result = {
    containerDefs: allContainerDefs().map(d => localizeContainerDef(d, t)),
    browserActions: BROWSER_ACTIONS.map(a => localizeActionDef(a, t)),
    excelActions: EXCEL_ACTIONS.map(a => localizeActionDef(a, t)),
    wordActions: WORD_ACTIONS.map(a => localizeActionDef(a, t)),
    logicActions: LOGIC_ACTIONS,
    logicOperators: LOGIC_OPERATORS.map(o => ({ ...o, label: t(`actionLabel.${o.type}`) || o.label })),
    bodyStepActions: BODY_STEP_ACTIONS.map(a => localizeActionDef(a, t)),
    fileActions: FILE_ACTIONS.map(a => localizeActionDef(a, t)),
  }
  _cache.set(locale, result)
  return result
}

/**
 * The main composable — returns locale-aware registry accessors.
 * Use this in any component that displays node/action labels.
 */
export function useRegistry() {
  const { t, locale } = useI18n()
  const loc = locale.value as string
  
  // Build cache on first access per locale
  const cache = _cache.get(loc) || buildCache(loc, t)

  function getContainerDef(type: string): ContainerDef {
    return cache!.containerDefs.find(d => d.type === type) || cache!.containerDefs[0]
  }

  function getActionDefs(containerType: ContainerType): ActionDef[] {
    switch (containerType) {
      case 'file': return cache!.fileActions
      case 'browser': return cache!.browserActions
      case 'excel': return cache!.excelActions
      case 'word': return cache!.wordActions
      case 'cursor': return cache!.bodyStepActions
      case 'loop': return cache!.bodyStepActions
      default: return []
    }
  }

  function getActionDef(containerType: ContainerType, actionType: string): ActionDef | undefined {
    return getActionDefs(containerType).find(a => a.type === actionType)
  }

  function getActionLabel(action: Action, containerType?: ContainerType): string {
    if (action.label) return action.label
    if (containerType) {
      const def = getActionDef(containerType, action.type)
      if (def) return def.label
    }
    return action.type
  }

  function isContainerType(type: ContainerType): boolean {
    const def = cache!.containerDefs.find(d => d.type === type)
    return def?.isContainer === true
  }

  function getContainerColorVar(type: string): string {
    const raw = rawGetContainerDef(type)
    return raw?.color || '#8b949e'
  }

  // Factory functions (use raw for structural aspects, locale for label)
  function newAction(type: string, containerType?: ContainerType, existingActions?: Action[], stepId?: string): Action {
    return rawNewAction(type, containerType, existingActions, stepId)
  }

  function newStep(containerType: ContainerType, existingSteps?: Step[]): Step {
    return rawNewStep(containerType, existingSteps)
  }

  // Also expose raw logic operators for the condition builder
  function getLogicOperators() {
    return cache!.logicOperators
  }

  /** 从后端 schema 同步节点类型（插件安装后 / 首次加载时调用） */
  async function refreshDynamicTypes() {
    const count = await syncNodeSchema()
    if (count > 0) {
      // schema 加载成功，清除 locale 缓存以重建
      _cache.delete(loc)
    }
  }

  return {
    containerDefs: cache!.containerDefs,
    getContainerDef,
    getActionDefs,
    getActionDef,
    getActionLabel,
    isContainerType,
    getContainerColorVar,
    getLogicOperators,
    newAction,
    newStep,
    refreshDynamicTypes,
    // Raw exports for cases where we need non-UI data
    raw: {
      CONTAINER_DEFS,
      BROWSER_ACTIONS,
      EXCEL_ACTIONS,
      WORD_ACTIONS,
      LOGIC_ACTIONS,
      LOGIC_OPERATORS,
      BODY_STEP_ACTIONS,
      FILE_ACTIONS,
    },
  }
}
