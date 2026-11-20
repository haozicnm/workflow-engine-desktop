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
})
