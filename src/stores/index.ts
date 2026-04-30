import { createPinia } from 'pinia'
export const pinia = createPinia()

export { useWorkflowStore } from './workflowStore'
export { useEditorStore } from './editorStore'
export { useFlowStore } from './flowStore'
