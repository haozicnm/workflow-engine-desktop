/**
 * useNodeSchema — Schema-first 节点注册
 *
 * 设计原则：
 *   - node-schema.json（/api/nodes/schema）是唯一真相源
 *   - CONTAINER_DEFS 仅提供 UI 覆写（params 表单、颜色、outputHint）
 *   - 加节点只改 node-schema.json，UI 覆写可选
 *
 * 调用时机：App.vue onMounted，仅执行一次。
 */
import {
  CONTAINER_DEFS,
} from '@/types/node-registry'
import { setRegistryDefs } from '@/types/registry-state'
import { safeInvoke } from '@/utils/tauri'
import type { ContainerDef, ContainerType } from '@/types/types'

// ─── Schema 节点原始类型 ───

interface SchemaNode {
  type: string
  label: string
  category: string
  icon: string
  is_container: boolean
  desc: string
  inputs: Array<{ name: string; data_type: string; required?: boolean; desc: string }>
  outputs: Array<{ name: string; data_type: string; desc: string }>
}

interface SchemaFile {
  nodes: SchemaNode[]
  categories: Record<string, { label: string; order: number }>
  container_types: string[]
}

// ─── 图标名转换 ───

/** schema 用 kebab-case（git-branch），前端用 PascalCase（GitBranch） */
function toPascalCase(kebab: string): string {
  return kebab
    .split('-')
    .map(s => s.charAt(0).toUpperCase() + s.slice(1))
    .join('')
}

// ─── category 默认颜色 ───

const CATEGORY_COLORS: Record<string, string> = {
  core: '#539bf5',
  data: '#7ee787',
  file: '#d2a8ff',
  convert: '#daaa3e',
  system: '#8b949e',
  flow: '#d29922',
  browser: '#79c0ff',
  office: '#3fb950',
  desktop: '#f0883e',
  ai: '#f778ba',
  mcp: '#a5d6ff',
}

// ─── 状态 ───

let _loaded = false
let _mergedDefs: ContainerDef[] = []
let _schemaFile: SchemaFile | null = null

/** 合并后的全部容器定义（schema + UI 覆写） */
export function allContainerDefs(): ContainerDef[] {
  return _mergedDefs
}

export function getSchemaFile(): SchemaFile | null {
  return _schemaFile
}

/** 按 category 分组 */
export function nodesByCategory(): Map<string, ContainerDef[]> {
  const map = new Map<string, ContainerDef[]>()
  for (const def of _mergedDefs) {
    const cat = def.category || 'other'
    if (!map.has(cat)) map.set(cat, [])
    map.get(cat)!.push(def)
  }
  return map
}

/** category 元数据（label + order） */
export function categoryMeta(): Record<string, { label: string; order: number }> {
  return _schemaFile?.categories ?? {}
}

// ─── 主加载函数 ───

/**
 * 从后端拉取 node-schema.json，与 CONTAINER_DEFS 合并。
 * @returns 注册的节点总数
 */
export async function syncNodeSchema(): Promise<number> {
  if (_loaded) return _mergedDefs.length

  try {
    const schema = await safeInvoke<SchemaFile>('node_schema')
    if (!schema?.nodes?.length) return 0

    _schemaFile = schema

    // 构建 UI 覆写索引（type → ContainerDef）
    const overrides = new Map<string, ContainerDef>()
    for (const def of CONTAINER_DEFS) {
      overrides.set(def.type, def)
    }

    // schema 是主源，逐个合并 UI 覆写
    _mergedDefs = schema.nodes.map(node => {
      const ui = overrides.get(node.type)

      return {
        type: node.type as ContainerType,
        label: ui?.label || node.label,
        icon: ui?.icon || toPascalCase(node.icon),
        color: ui?.color || CATEGORY_COLORS[node.category] || '#8b949e',
        description: ui?.description || node.desc,
        isContainer: node.is_container,
        params: ui?.params || [],
        paramDefs: ui?.paramDefs,
        outputHint: ui?.outputHint || node.outputs.map(o => o.name).join(', '),
        category: node.category,
      } as ContainerDef
    })

    // 写入共享状态，让 node-registry 查询函数也能读到 schema 数据
    setRegistryDefs(_mergedDefs)

    _loaded = true
    console.log(
      `[NodeSchema] Schema-first 加载完成：${_mergedDefs.length} 节点，` +
      `${overrides.size} 有 UI 覆写，${_mergedDefs.filter(d => d.params.length > 0).length} 有表单定义`
    )
    return _mergedDefs.length
  } catch (e) {
    console.warn('[NodeSchema] 加载失败（后端未就绪？）:', e)
    return 0
  }
}
