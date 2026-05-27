/**
 * useNodeSchema — 应用启动时从后端 /api/nodes/schema 拉取完整节点目录，
 * 将前端未覆盖的节点注入 _dynamicDefs，使编辑器能显示后端全部节点类型。
 *
 * 调用时机：App.vue onMounted，仅执行一次。
 */
import {
  CONTAINER_DEFS,
  registerDynamicNode,
  clearDynamicNodes,
} from '@/types/node-registry'
import type { ContainerType } from '@/types/types'

// 后端 API 基础地址（与 status polling 保持一致）
const API_BASE = 'http://localhost:19528'

interface BackendNodeMeta {
  type: string
  label: string
  icon: string
  category: string
  desc: string
}

let _loaded = false

/**
 * 从后端拉取节点 schema，将前端未定义的节点注册为 dynamic defs。
 * 前端已有的节点（CONTAINER_DEFS）保持不动——它们带有完整的 params/UI 元数据。
 * 后端独有的节点注册为 minimal def，至少能在节点选择器中显示。
 *
 * @returns 新注册的节点类型数量
 */
export async function syncNodeSchema(): Promise<number> {
  if (_loaded) return 0

  try {
    const resp = await fetch(`${API_BASE}/api/nodes/schema`)
    if (!resp.ok) return 0

    const nodes: BackendNodeMeta[] = await resp.json()
    if (!Array.isArray(nodes) || nodes.length === 0) return 0

    // 清除旧的动态节点（插件卸载场景）
    clearDynamicNodes()

    // 前端已有类型集合
    const existingTypes = new Set<string>(CONTAINER_DEFS.map(d => d.type))

    let count = 0
    for (const node of nodes) {
      if (existingTypes.has(node.type)) continue

      // 后端类型可能是前端 ContainerType 未覆盖的新类型，用 as 强转
      registerDynamicNode({
        type: node.type as unknown as ContainerType,
        label: node.label || node.type,
        icon: node.icon || 'Package',
        color: '#a5d6ff',
        description: node.desc || node.label || node.type,
        isContainer: false,
        params: [{ key: 'config', label: '参数 (JSON)', type: 'textarea' as const }],
        category: node.category,
      })
      count++
    }

    _loaded = true
    console.log(`[NodeSchema] 同步完成：后端 ${nodes.length} 种，新增 ${count} 种`)
    return count
  } catch (e) {
    // 后端可能还未就绪，静默处理
    console.warn('[NodeSchema] 同步失败（后端未就绪？）:', e)
    return 0
  }
}
