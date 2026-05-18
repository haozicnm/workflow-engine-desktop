import { describe, it, expect } from 'vitest'
import { newWorkflow, serializeWorkflow, deserializeWorkflow } from '../types'

describe('workflow serialization', () => {
  it('creates a valid empty workflow', () => {
    const wf = newWorkflow()
    expect(wf.name).toBeDefined()
    expect(Array.isArray(wf.steps)).toBe(true)
    expect(wf.steps).toHaveLength(0)
  })

  it('serialize produces parseable JSON', () => {
    const wf = newWorkflow()
    const json = serializeWorkflow(wf)
    expect(() => JSON.parse(json)).not.toThrow()
  })

  it('round-trip preserves name and steps', () => {
    const wf = newWorkflow()
    wf.name = 'test-workflow'
    wf.description = 'a test'
    const json = serializeWorkflow(wf)
    const restored = deserializeWorkflow(json)
    expect(restored.name).toBe('test-workflow')
    expect(restored.description).toBe('a test')
    expect(restored.steps).toEqual([])
  })

  it('round-trip preserves locked flag', () => {
    const wf = newWorkflow()
    wf.locked = true
    const json = serializeWorkflow(wf)
    const restored = deserializeWorkflow(json)
    expect(restored.locked).toBe(true)
  })
})
