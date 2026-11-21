import { afterEach, describe, expect, it, vi } from 'vitest'
import { safeInvoke } from '../tauri'

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('browser HTTP fallback', () => {
  it('rejects an HTML SPA fallback instead of treating it as workflow data', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(
      '<!doctype html><html><body>SPA fallback</body></html>',
      { status: 200, headers: { 'Content-Type': 'text/html' } },
    )))

    await expect(safeInvoke('workflow_list')).rejects.toThrow('received HTML')
  })

  it('still parses valid JSON API responses', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(
      JSON.stringify([{ id: 'wf-1', name: 'Production', description: '' }]),
      { status: 200, headers: { 'Content-Type': 'application/json' } },
    )))

    const result = await safeInvoke<Array<{ id: string; name: string }>>('workflow_list')
    expect(result).toHaveLength(1)
    expect(result?.[0].name).toBe('Production')
  })

  it('normalizes REST workflow creation to the Tauri string id contract', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(
      JSON.stringify({ id: 'wf-new' }),
      { status: 200, headers: { 'Content-Type': 'application/json' } },
    ))
    vi.stubGlobal('fetch', fetchMock)

    await expect(safeInvoke<string>('workflow_create', {
      name: 'Production',
      description: '',
    })).resolves.toBe('wf-new')
    expect(fetchMock).toHaveBeenCalledWith('/api/workflows', expect.objectContaining({
      method: 'POST',
    }))
  })

  it('instantiates templates through the local HTTP contract', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(
      JSON.stringify({ workflow_id: 'wf-from-template' }),
      { status: 200, headers: { 'Content-Type': 'application/json' } },
    ))
    vi.stubGlobal('fetch', fetchMock)

    await expect(safeInvoke<{ workflow_id: string }>('template_instantiate', {
      name: 'http_fetch',
      params: {},
    })).resolves.toEqual({ workflow_id: 'wf-from-template' })
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/templates/http_fetch/instantiate',
      expect.objectContaining({ method: 'POST', body: '{"params":{}}' }),
    )
  })
})
