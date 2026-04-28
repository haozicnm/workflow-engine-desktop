import { createPinia } from 'pinia'
export const pinia = createPinia()

export { useWorkflowStore } from './workflowStore'
export { useEditorStore } from './editorStore'
export { useExecutionStore } from './executionStore'

// Re-export legacy store for backward compatibility
export { useWorkflowStore as useWorkflowStoreLegacy } from './workflowStore'
