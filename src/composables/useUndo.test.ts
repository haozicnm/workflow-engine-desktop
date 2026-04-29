// useUndo.test.ts — 撤销/重做测试
import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useFlowStore } from '../stores/flowStore'
import { useUndo } from '../composables/useUndo'
import type { FlowNode } from '../components/flow/pinTypes'

function makeNode(id: string, type = 'http'): FlowNode {
  return { id, type, label: `Node ${id}`, position: { x: 0, y: 0 }, config: {} }
}

describe('useUndo', () => {
  let store: ReturnType<typeof useFlowStore>

  beforeEach(() => {
    setActivePinia(createPinia())
    store = useFlowStore()
  })

  it('init 创建初始快照', () => {
    const undo = useUndo()
    undo.init()
    expect(undo.history.value).toHaveLength(1)
    expect(undo.historyIndex.value).toBe(0)
  })

  it('pushState 保存快照后 undo 恢复', () => {
    const undo = useUndo()
    undo.init()
    store.addNode(makeNode('n1'))
    undo.pushState()
    store.addNode(makeNode('n2'))
    undo.pushState()
    expect(undo.history.value).toHaveLength(3)

    const ok = undo.undo()
    expect(ok).toBe(true)
    expect(store.nodes).toHaveLength(1)
    expect(store.getNode('n1')).toBeDefined()
    expect(store.getNode('n2')).toBeUndefined()
  })

  it('redo 恢复已撤销的状态', () => {
    const undo = useUndo()
    undo.init()
    store.addNode(makeNode('n1'))
    undo.pushState()
    store.addNode(makeNode('n2'))
    undo.pushState()

    undo.undo()
    expect(store.nodes).toHaveLength(1)

    const ok = undo.redo()
    expect(ok).toBe(true)
    expect(store.nodes).toHaveLength(2)
  })

  it('undo 在边界返回 false', () => {
    const undo = useUndo()
    undo.init()
    expect(undo.undo()).toBe(false)
  })

  it('redo 在边界返回 false', () => {
    const undo = useUndo()
    undo.init()
    expect(undo.redo()).toBe(false)
  })

  it('pushState 后分支截断（新操作丢弃旧 redo）', () => {
    const undo = useUndo()
    undo.init()
    store.addNode(makeNode('n1'))
    undo.pushState()
    store.addNode(makeNode('n2'))
    undo.pushState()
    undo.undo() // 回到 n1
    store.addNode(makeNode('n3')) // 新分支
    undo.pushState()

    // redo 应该不可用（分支截断）
    expect(undo.redo()).toBe(false)
    expect(store.nodes).toHaveLength(2) // n1 + n3
  })

  it('undo/redo 恢复连线', () => {
    const undo = useUndo()
    undo.init()
    store.load({
      name: 'Test',
      nodes: [makeNode('a'), makeNode('b')],
      edges: [],
    })
    undo.init()

    store.addEdge({ id: 'e1', source: 'a', target: 'b', sourceHandle: 'o', targetHandle: 'i' })
    undo.pushState()
    expect(store.edgeCount).toBe(1)

    undo.undo()
    expect(store.edgeCount).toBe(0)

    undo.redo()
    expect(store.edgeCount).toBe(1)
  })
})
