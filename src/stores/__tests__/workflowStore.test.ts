import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useWorkflowStore } from '../workflowStore'

describe('workflowStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('starts with null current and empty list', () => {
    const store = useWorkflowStore()
    expect(store.current).toBeNull()
    expect(store.workflowList).toEqual([])
    expect(store.loading).toBe(false)
  })

  it('addStep appends a step with correct container type', () => {
    const store = useWorkflowStore()
    store.current = {
      id: 'wf-1',
      name: 'test',
      description: '',
      locked: false,
      steps: [],
    }
    store.addStep('browser')
    expect(store.current!.steps).toHaveLength(1)
    expect(store.current!.steps[0].type).toBe('browser')
    expect(store.dirty).toBe(true)
  })

  it('removeStep deletes the correct step', () => {
    const store = useWorkflowStore()
    store.current = {
      id: 'wf-1',
      name: 'test',
      description: '',
      locked: false,
      steps: [
        { id: 's1', type: 'http', actions: [], label: 'Step 1', config: {}, expanded: false },
        { id: 's2', type: 'shell', actions: [], label: 'Step 2', config: {}, expanded: false },
      ],
    }
    store.removeStep('s1')
    expect(store.current!.steps).toHaveLength(1)
    expect(store.current!.steps[0].id).toBe('s2')
  })

  it('moveStep reorders correctly', () => {
    const store = useWorkflowStore()
    store.current = {
      id: 'wf-1',
      name: 'test',
      description: '',
      locked: false,
      steps: [
        { id: 's1', type: 'http', actions: [], label: 'S1', config: {}, expanded: false },
        { id: 's2', type: 'shell', actions: [], label: 'S2', config: {}, expanded: false },
        { id: 's3', type: 'notify', actions: [], label: 'S3', config: {}, expanded: false },
      ],
    }
    store.moveStep(0, 2)
    expect(store.current!.steps[0].id).toBe('s2')
    expect(store.current!.steps[1].id).toBe('s3')
    expect(store.current!.steps[2].id).toBe('s1')
  })

  it('renaming a step sets dirty', () => {
    const store = useWorkflowStore()
    store.current = {
      id: 'wf-1',
      name: 'test',
      description: '',
      locked: false,
      steps: [
        { id: 's1', type: 'http', actions: [], label: 'Old', config: {}, expanded: false },
      ],
    }
    store.renameStep('s1', 'New Name')
    expect(store.current!.steps[0].label).toBe('New Name')
    expect(store.dirty).toBe(true)
  })
})
