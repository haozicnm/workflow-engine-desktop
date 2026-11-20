import { afterEach, describe, expect, it, vi } from 'vitest'
import { useGlobalStatus } from '../useGlobalStatus'

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('API health status', () => {
  it('does not report an HTML SPA fallback as an online backend', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(
      '<!doctype html><html></html>',
      { status: 200, headers: { 'Content-Type': 'text/html' } },
    )))
    const status = useGlobalStatus()

    await status.refreshApiStatus()

    expect(status.state.apiOnline).toBe(false)
  })

  it('requires the health payload to contain status=ok', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(
      JSON.stringify({ status: 'ok', version: '9.1.1' }),
      { status: 200, headers: { 'Content-Type': 'application/json' } },
    )))
    const status = useGlobalStatus()

    await status.refreshApiStatus()

    expect(status.state.apiOnline).toBe(true)
  })
})
