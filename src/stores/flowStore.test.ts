// flowStore.test.ts — 核心状态管理测试
import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useFlowStore } from '../stores/flowStore'
import type { FlowNode, FlowEdge } from '../components/flow/pinTypes'

function makeNode(id: string, type = 'http', label = 'Test'): FlowNode {
  return { id, type, label, position: { x: 0, y: 0 }, config: {} }
}

function makeEdge(id: string, source = 'n1', target = 'n2'): FlowEdge {
  return { id, source, target, sourceHandle: 'output', targetHandle: 'input' }
}

describe('flowStore', () => {
  let store: ReturnType<typeof useFlowStore>

  beforeEach(() => {
    setActivePinia(createPinia())
    store = useFlowStore()
  })

  describe('节点操作', () => {
    it('addNode 添加节点并设置状态为 idle', () => {
      const node = makeNode('n1')
      store.addNode(node)
      expect(store.nodes).toHaveLength(1)
      expect(store.nodeStatuses['n1']).toBe('idle')
      expect(store.dirty).toBe(true)
    })

    it('removeNode 删除节点及其连线', () => {
      store.addNode(makeNode('n1'))
      store.addNode(makeNode('n2'))
      store.addEdge(makeEdge('e1', 'n1', 'n2'))
      store.removeNode('n1')
      expect(store.nodes).toHaveLength(1)
      expect(store.edges).toHaveLength(0)
      expect(store.nodeStatuses['n1']).toBeUndefined()
    })

    it('updateNodePosition 更新坐标', () => {
      store.addNode(makeNode('n1'))
      store.updateNodePosition('n1', { x: 100, y: 200 })
      expect(store.getNode('n1')!.position).toEqual({ x: 100, y: 200 })
    })

    it('updateNodeLabel 更新标签', () => {
      store.addNode(makeNode('n1'))
      store.updateNodeLabel('n1', 'New Label')
      expect(store.getNode('n1')!.label).toBe('New Label')
    })

    it('updateNodeConfig 合并配置', () => {
      store.addNode(makeNode('n1', 'http', 'Test'))
      store.updateNodeConfig('n1', { url: 'https://example.com' })
      expect(store.getNode('n1')!.config).toEqual({ url: 'https://example.com' })
    })

    it('getNode 返回 undefined 当节点不存在', () => {
      expect(store.getNode('ghost')).toBeUndefined()
    })
  })

  describe('连线操作', () => {
    it('addEdge 添加唯一连线', () => {
      store.addEdge(makeEdge('e1'))
      expect(store.edges).toHaveLength(1)
      expect(store.dirty).toBe(true)
    })

    it('addEdge 不重复添加相同连线', () => {
      const edge = makeEdge('e1')
      store.addEdge(edge)
      store.addEdge(edge)
      expect(store.edges).toHaveLength(1)
    })

    it('removeEdge 删除连线', () => {
      store.addEdge(makeEdge('e1'))
      store.removeEdge('e1')
      expect(store.edges).toHaveLength(0)
    })
  })

  describe('状态操作', () => {
    it('setNodeStatus 设置节点状态', () => {
      store.addNode(makeNode('n1'))
      store.setNodeStatus('n1', 'running')
      expect(store.nodeStatuses['n1']).toBe('running')
    })

    it('resetAllStatuses 重置所有状态为 idle', () => {
      store.addNode(makeNode('n1'))
      store.addNode(makeNode('n2'))
      store.setNodeStatus('n1', 'success')
      store.setNodeStatus('n2', 'error')
      store.resetAllStatuses()
      expect(store.nodeStatuses['n1']).toBe('idle')
      expect(store.nodeStatuses['n2']).toBe('idle')
    })
  })

  describe('加载/保存', () => {
    it('load 加载完整工作流', () => {
      store.load({
        name: 'Test WF',
        id: 'wf1',
        nodes: [makeNode('a'), makeNode('b')],
        edges: [makeEdge('e1', 'a', 'b')],
      })
      expect(store.workflowName).toBe('Test WF')
      expect(store.workflowId).toBe('wf1')
      expect(store.nodes).toHaveLength(2)
      expect(store.edges).toHaveLength(1)
      expect(store.dirty).toBe(false)
    })

    it('toJSON 导出工作流', () => {
      store.load({
        name: 'Export WF',
        nodes: [makeNode('x')],
        edges: [],
      })
      const json = store.toJSON()
      expect(json.name).toBe('Export WF')
      expect(json.nodes).toHaveLength(1)
      expect(json.edges).toHaveLength(0)
    })
  })

  describe('计算属性', () => {
    it('nodeCount 反映节点数量', () => {
      expect(store.nodeCount).toBe(0)
      store.addNode(makeNode('n1'))
      store.addNode(makeNode('n2'))
      expect(store.nodeCount).toBe(2)
    })

    it('edgeCount 反映连线数量', () => {
      expect(store.edgeCount).toBe(0)
      store.addEdge(makeEdge('e1'))
      store.addEdge(makeEdge('e2', 'n1', 'n3'))
      expect(store.edgeCount).toBe(2)
    })
  })

  describe('清空', () => {
    it('clear 重置所有状态', () => {
      store.addNode(makeNode('n1'))
      store.addEdge(makeEdge('e1'))
      store.setWorkflowName('Something')
      store.clear()
      expect(store.nodes).toHaveLength(0)
      expect(store.edges).toHaveLength(0)
      expect(store.workflowName).toBe('未命名工作流')
      expect(store.workflowId).toBeNull()
      expect(store.dirty).toBe(false)
    })
  })
})
