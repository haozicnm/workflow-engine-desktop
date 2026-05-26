// utils/tauri.ts — 共享 safeInvoke / safeListen，供所有 Vue 组件使用
// 在浏览器 dev 模式下优雅降级为 HTTP REST API / SSE
import { addOp } from '../composables/useOpsConsole'

const isTauri =
  typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__

/** Base URL for REST API calls (empty string in Tauri mode) */
export const API_BASE = isTauri ? '' : ''

// ═══════════════════════════════════════════════════════════════
// Command → HTTP route mapping
// ═══════════════════════════════════════════════════════════════

interface RouteEntry {
  method: 'GET' | 'POST' | 'PUT' | 'DELETE'
  path: string // URL template, e.g. /api/workflows/{id}
}

/** Routes with no dynamic path parameters */
const FIXED_ROUTES: Record<string, RouteEntry> = {
  workflow_list:         { method: 'GET',    path: '/api/workflows' },
  workflow_create:       { method: 'POST',   path: '/api/workflows' },
  workflow_validate:     { method: 'POST',   path: '/api/workflows/validate' },
  run_list:              { method: 'GET',    path: '/api/runs' },
  run_start:             { method: 'POST',   path: '/api/runs' },
  schedule_list:         { method: 'GET',    path: '/api/schedules' },
  schedule_create:       { method: 'POST',   path: '/api/schedules' },
  approval_list_pending: { method: 'GET',    path: '/api/approvals/pending' },
  approval_response:     { method: 'POST',   path: '/api/approvals/respond' },
  node_list_types:       { method: 'GET',    path: '/api/nodes/types' },
  settings_get:          { method: 'GET',    path: '/api/settings' },
  settings_update:       { method: 'PUT',    path: '/api/settings' },
  check_ipc:             { method: 'GET',    path: '/api/system/check-ipc' },
  system_check_browser:  { method: 'GET',    path: '/api/system/check-browser' },
  clear_logs:            { method: 'POST',   path: '/api/system/clear-logs' },
  open_log_dir:          { method: 'POST',   path: '/api/system/open-log-dir' },
}

/** Routes with {param} placeholders that are substituted from args */
const DYNAMIC_ROUTES: Record<string, RouteEntry> = {
  workflow_get:       { method: 'GET',    path: '/api/workflows/{id}' },
  workflow_update:    { method: 'PUT',    path: '/api/workflows/{id}' },
  workflow_delete:    { method: 'DELETE', path: '/api/workflows/{id}' },
  workflow_lock:      { method: 'POST',   path: '/api/workflows/{id}/lock' },
  workflow_save_yaml: { method: 'POST',   path: '/api/workflows/{id}/yaml' },
  run_cancel:         { method: 'POST',   path: '/api/runs/{run_id}/cancel' },
  run_detail:         { method: 'GET',    path: '/api/runs/{run_id}/detail' },
  run_step_logs:      { method: 'GET',    path: '/api/runs/{run_id}/step-logs' },
  schedule_update:    { method: 'PUT',    path: '/api/schedules/{id}' },
  schedule_delete:    { method: 'DELETE', path: '/api/schedules/{id}' },
  get_trajectory:     { method: 'GET',    path: '/api/preview/trajectory/{run_id}' },
  get_bundle_files:   { method: 'GET',    path: '/api/preview/bundle-files/{run_id}/{step_id}' },
  read_bundle_file:   { method: 'GET',    path: '/api/preview/bundle-file/{run_id}/{step_id}/{filename}' },
}

/** Browser-only recording commands — not available in HTTP mode */
const BROWSER_COMMANDS = new Set([
  'browser_pick_next',
  'browser_pick_session_start',
  'browser_pick_session_stop',
  'browser_recording_start',
  'browser_recording_stop',
])

// ═══════════════════════════════════════════════════════════════
// HTTP helper utilities
// ═══════════════════════════════════════════════════════════════

/** Convert snake_case to camelCase: run_id → runId */
function snakeToCamel(s: string): string {
  return s.replace(/_([a-z])/g, (_, c) => c.toUpperCase())
}

/** Convert camelCase to snake_case: runId → run_id */
function camelToSnake(s: string): string {
  return s.replace(/[A-Z]/g, (c) => '_' + c.toLowerCase())
}

/**
 * Look up a URL parameter in args, trying both the template key
 * (e.g. "run_id") and its camelCase variant (e.g. "runId").
 * This handles Tauri 2's automatic camelCase↔snake_case conversion.
 */
function getArg(
  args: Record<string, unknown> | undefined,
  key: string,
): unknown | undefined {
  if (!args) return undefined
  if (key in args) return args[key]
  const camel = snakeToCamel(key)
  if (camel !== key && camel in args) return args[camel]
  return undefined
}

/** Build URL from route entry, substituting {param} placeholders */
function buildUrl(entry: RouteEntry, args?: Record<string, unknown>): string {
  return entry.path.replace(/\{(\w+)\}/g, (_match, key: string) => {
    const val = getArg(args, key)
    if (val === undefined || val === null) {
      console.warn(`[tauri] Missing URL parameter "${key}" in args`, args)
      return ''
    }
    return encodeURIComponent(String(val))
  })
}

/** Recursively convert all object keys from camelCase to snake_case */
function toSnakeKeys(obj: unknown): unknown {
  if (obj === null || obj === undefined) return obj
  if (Array.isArray(obj)) return obj.map(toSnakeKeys)
  if (typeof obj === 'object') {
    const result: Record<string, unknown> = {}
    for (const [k, v] of Object.entries(obj as Record<string, unknown>)) {
      result[camelToSnake(k)] = toSnakeKeys(v)
    }
    return result
  }
  return obj
}

/**
 * Build JSON request body for POST/PUT.
 *  1. Removes path-parameter keys (already in URL)
 *  2. Unwraps `{ settings }` for settings_update (Tauri wrapper vs flat REST API)
 *  3. Converts remaining keys from camelCase to snake_case
 */
function buildBody(
  entry: RouteEntry,
  args?: Record<string, unknown>,
): string | undefined {
  if (!args || Object.keys(args).length === 0) return undefined

  const bodyObj: Record<string, unknown> = { ...args }

  // Remove path parameters (both snake_case keys and camelCase variants)
  const placeholders = entry.path.match(/\{(\w+)\}/g) || []
  for (const ph of placeholders) {
    const key = ph.slice(1, -1)
    delete bodyObj[key]
    const camel = snakeToCamel(key)
    if (camel !== key) delete bodyObj[camel]
  }

  // If nothing left after removing path params, send no body
  if (Object.keys(bodyObj).length === 0) return undefined

  // --- Special cases ---

  // settings_update: caller passes { settings: { theme, language, ... } }
  // but REST API expects flat { theme, language, ... }
  if (
    Object.keys(bodyObj).length === 1 &&
    'settings' in bodyObj &&
    bodyObj.settings !== null &&
    typeof bodyObj.settings === 'object'
  ) {
    const unwrapped = toSnakeKeys(bodyObj.settings) as Record<string, unknown>
    return Object.keys(unwrapped).length > 0
      ? JSON.stringify(unwrapped)
      : undefined
  }

  // Default: convert camelCase keys to snake_case for REST API
  const converted = toSnakeKeys(bodyObj) as Record<string, unknown>
  return JSON.stringify(converted)
}

/**
 * Execute an HTTP request mapped from a Tauri command.
 * Handles URL construction, body serialization, error handling,
 * timeout, and operation-console logging.
 */
async function httpInvoke<T = unknown>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  // Browser recording commands — not available outside Tauri
  if (BROWSER_COMMANDS.has(command)) {
    console.warn(`[tauri] "${command}" is not available in browser mode`)
    return null as T
  }

  const entry = FIXED_ROUTES[command] ?? DYNAMIC_ROUTES[command]
  if (!entry) {
    throw new Error(`[tauri] Unknown command "${command}" — no HTTP mapping`)
  }

  const url = buildUrl(entry, args)
  const controller = new AbortController()
  const timeoutId = setTimeout(() => controller.abort(), 30_000)

  const start = Date.now()

  try {
    const init: RequestInit = {
      method: entry.method,
      signal: controller.signal,
      headers: { 'Content-Type': 'application/json' },
    }

    if (entry.method === 'POST' || entry.method === 'PUT') {
      const body = buildBody(entry, args)
      if (body) init.body = body
    }

    const resp = await fetch(url, init)
    const elapsed = Date.now() - start

    if (!resp.ok) {
      let errMsg: string
      try {
        const errBody = await resp.json()
        errMsg = errBody.error || errBody.message || resp.statusText
      } catch {
        errMsg = resp.statusText || `HTTP ${resp.status}`
      }
      addOp({
        source: 'gui',
        category: 'http',
        name: command,
        status: 'fail',
        elapsed,
        detail: errMsg,
      })
      throw new Error(errMsg)
    }

    // Parse response
    const contentType = resp.headers.get('content-type') || ''
    let result: T

    if (contentType.includes('application/json')) {
      result = (await resp.json()) as T
    } else {
      const text = await resp.text()
      result = (text || undefined) as T
    }

    addOp({
      source: 'gui',
      category: 'http',
      name: command,
      status: 'ok',
      elapsed,
      detail: args ? JSON.stringify(args).slice(0, 120) : undefined,
    })

    return result
  } catch (e: unknown) {
    const elapsed = Date.now() - start

    if (e instanceof DOMException && e.name === 'AbortError') {
      addOp({
        source: 'gui',
        category: 'http',
        name: command,
        status: 'fail',
        elapsed,
        detail: 'Request timed out (30s)',
      })
      throw new Error(`[tauri] Request "${command}" timed out after 30s`)
    }

    addOp({
      source: 'gui',
      category: 'http',
      name: command,
      status: 'fail',
      elapsed,
      detail: (e as Error).message || String(e),
    })
    throw e
  } finally {
    clearTimeout(timeoutId)
  }
}

// ═══════════════════════════════════════════════════════════════
// SSE / EventSource state (shared across safeListen calls)
// ═══════════════════════════════════════════════════════════════

let _eventSource: EventSource | null = null
const _sseHandlers = new Map<string, Set<(payload: unknown) => void>>()

/**
 * Get or create the global singleton EventSource connection.
 * All safeListen calls share one connection to /api/events.
 */
function ensureEventSource(): EventSource {
  if (!_eventSource) {
    _eventSource = new EventSource(`${API_BASE}/api/events`)
    _eventSource.onerror = () => {
      console.warn('[tauri] SSE connection error — browser will auto-reconnect')
    }
  }
  return _eventSource
}

/**
 * Register a handler via EventSource (browser SSE fallback).
 * Returns an unlisten function that removes the handler.
 */
async function httpListen<T = unknown>(
  event: string,
  handler: (event: { payload: T }) => void,
): Promise<() => void> {
  const es = ensureEventSource()

  // First time this event type is registered — attach the SSE listener
  if (!_sseHandlers.has(event)) {
    _sseHandlers.set(event, new Set())

    es.addEventListener(event, (e: MessageEvent) => {
      let payload: T
      try {
        payload = JSON.parse(e.data) as T
      } catch {
        payload = e.data as unknown as T
      }

      const handlers = _sseHandlers.get(event)
      if (handlers) {
        const wrapped = { payload }
        for (const h of handlers) {
          h(wrapped)
        }
      }
    })
  }

  const handlers = _sseHandlers.get(event)!
  handlers.add(handler as unknown as (payload: unknown) => void)

  // Return unlisten function
  return () => {
    handlers.delete(handler as unknown as (payload: unknown) => void)
    // Keep EventSource alive for other registered listeners
  }
}

// ═══════════════════════════════════════════════════════════════
// Public API — unchanged signatures
// ═══════════════════════════════════════════════════════════════

/**
 * 安全调用 Tauri 命令。
 * Tauri 环境 → 走现有 invoke
 * 浏览器环境 → 走 HTTP fetch（映射到 REST API）
 * 所有调用自动记录到全局操作控制台。
 */
export async function safeInvoke<T = unknown>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T | undefined> {
  if (!isTauri) {
    return httpInvoke<T>(command, args)
  }

  const start = Date.now()
  const { invoke } = await import('@tauri-apps/api/core')
  try {
    const result = await invoke<T>(command, args)
    const elapsed = Date.now() - start
    addOp({
      source: 'gui',
      category: 'invoke',
      name: command,
      status: 'ok',
      elapsed,
      detail: args ? JSON.stringify(args).slice(0, 120) : undefined,
    })
    return result
  } catch (e: unknown) {
    const elapsed = Date.now() - start
    addOp({
      source: 'gui',
      category: 'invoke',
      name: command,
      status: 'fail',
      elapsed,
      detail: (e as Error).message || String(e),
    })
    throw e
  }
}

/**
 * 安全监听 Tauri 事件。
 * Tauri 环境 → 走现有 listen
 * 浏览器环境 → 走 EventSource（SSE）
 */
export async function safeListen<T = unknown>(
  event: string,
  handler: (event: { payload: T }) => void,
): Promise<() => void> {
  if (!isTauri) {
    return httpListen<T>(event, handler)
  }

  // 动态 import，避免静态依赖导致浏览器报错
  const { listen } = await import('@tauri-apps/api/event')
  const unlisten = await listen<T>(event, handler)
  return () => unlisten()
}
