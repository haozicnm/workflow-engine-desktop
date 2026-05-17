// utils/tauri.ts — 共享 safeInvoke / safeListen，供所有 Vue 组件使用
// 在浏览器 dev 模式下优雅降级（不抛异常，不白屏）
import { addOp } from '../composables/useOpsConsole'

const isTauri =
  typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__

/**
 * 安全调用 Tauri 命令。dev 模式下返回 undefined + console.warn。
 * 所有调用自动记录到全局操作控制台。
 */
export async function safeInvoke<T = unknown>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T | undefined> {
  if (!isTauri) {
    console.warn(`[dev] safeInvoke("${command}") 跳过 — 不在 Tauri 环境`)
    return undefined
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
 * 安全监听 Tauri 事件。dev 模式下返回空 unlisten 函数。
 */
export async function safeListen<T = unknown>(
  event: string,
  handler: (event: { payload: T }) => void,
): Promise<() => void> {
  if (!isTauri) {
    console.warn(`[dev] safeListen("${event}") 跳过 — 不在 Tauri 环境`)
    return () => {}
  }
  // 动态 import，避免静态依赖导致浏览器报错
  const { listen } = await import('@tauri-apps/api/event')
  const unlisten = await listen<T>(event, handler)
  return () => unlisten()
}
