/**
 * registry-state — 共享状态层
 *
 * 解决 useNodeSchema ↔ node-registry 循环依赖：
 *   - useNodeSchema.ts 加载 schema 后写入 setRegistryDefs()
 *   - node-registry.ts 的查询函数读取 getRegistryDefs()
 *   - 两个模块都从此模块导入，无环
 *
 * 落后兼容：如果 syncNodeSchema() 尚未调用，fallback 到 CONTAINER_DEFS。
 */
import type { ContainerDef } from './types'

let _defs: ContainerDef[] | null = null

/** 写入：syncNodeSchema() 合并完成后调用 */
export function setRegistryDefs(defs: ContainerDef[]) {
  _defs = defs
}

/** 读取：null 表示尚未从 schema 加载，调用方应回退到 CONTAINER_DEFS */
export function getSchemaDefs(): ContainerDef[] | null {
  return _defs
}
