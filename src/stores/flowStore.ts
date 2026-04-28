// ─── Flow Store (Pinia) — 管理画布节点、连线、执行状态 ───
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { FlowNode, FlowEdge, NodeStatus } from '../components/flow/pinTypes'

export const useFlowStore = defineStore('flow', () => {
  // ─── 画布数据 ───
  const nodes = ref<FlowNode[]>([])
  const edges = ref<FlowEdge[]>([])
  const nodeStatuses = ref<Record<string, NodeStatus>>({})
  const stepOutputs = ref<Record<string, unknown>>({})

  // ─── 工作流元数据 ───
  const workflowName = ref('未命名工作流')
  const workflowId = ref<string | null>(null)
  const dirty = ref(false)

  // ─── 计数器用于生成唯一 ID ───
  let counter = 0

  // ─── 节点操作 ───
  function addNode(node: FlowNode) {
    nodes.value.push(node)
    nodeStatuses.value[node.id] = 'idle'
    dirty.value = true
  }

  function removeNode(id: string) {
    nodes.value = nodes.value.filter(n => n.id !== id)
    // 移除关联连线
    edges.value = edges.value.filter(e => e.source !== id && e.target !== id)
    delete nodeStatuses.value[id]
    dirty.value = true
  }

  function updateNodePosition(id: string, position: { x: number; y: number }) {
    const node = nodes.value.find(n => n.id === id)
    if (node) {
      node.position = position
      dirty.value = true
    }
  }

  function updateNodeLabel(id: string, label: string) {
    const node = nodes.value.find(n => n.id === id)
    if (node) {
      node.label = label
      dirty.value = true
    }
  }

  function updateNodeConfig(id: string, config: Record<string, unknown>) {
    const node = nodes.value.find(n => n.id === id)
    if (node) {
      node.config = { ...node.config, ...config }
      dirty.value = true
    }
  }

  function getNode(id: string): FlowNode | undefined {
    return nodes.value.find(n => n.id === id)
  }

  // ─── 连线操作 ───
  function addEdge(edge: FlowEdge) {
    const exists = edges.value.some(
      e =>
        e.source === edge.source &&
        e.target === edge.target &&
        e.sourceHandle === edge.sourceHandle &&
        e.targetHandle === edge.targetHandle
    )
    if (!exists) {
      edges.value.push(edge)
      dirty.value = true
    }
  }

  function removeEdge(id: string) {
    edges.value = edges.value.filter(e => e.id !== id)
    dirty.value = true
  }

  // ─── 状态操作 ───
  function setNodeStatus(id: string, status: NodeStatus) {
    nodeStatuses.value[id] = status
  }

  function setNodeOutput(id: string, output: unknown) {
    stepOutputs.value[id] = output
    // 同时更新 nodes 数组中的 output
    const node = nodes.value.find(n => n.id === id)
    if (node) node.output = output
  }

  function resetAllStatuses() {
    for (const id of Object.keys(nodeStatuses.value)) {
      nodeStatuses.value[id] = 'idle'
    }
  }

  // ─── 节点计数 ───
  const nodeCount = computed(() => nodes.value.length)
  const edgeCount = computed(() => edges.value.length)

  // ─── 标题 ───
  function setWorkflowName(name: string) {
    workflowName.value = name
  }

  function setWorkflowId(id: string | null) {
    workflowId.value = id
  }

  // ─── 清空 ───
  function clear() {
    nodes.value = []
    edges.value = []
    nodeStatuses.value = {}
    workflowName.value = '未命名工作流'
    workflowId.value = null
    dirty.value = false
    counter = 0
  }

  // ─── 加载/保存 ───
  function load({ name, id, nodes: ns, edges: es }: {
    name: string
    id?: string | null
    nodes: FlowNode[]
    edges: FlowEdge[]
  }) {
    workflowName.value = name
    workflowId.value = id ?? null
    nodes.value = ns
    edges.value = es
    dirty.value = false
    for (const n of ns) {
      nodeStatuses.value[n.id] = 'idle'
    }
    // 从现有 ID 恢复计数器
    counter = ns.length
  }

  function toJSON() {
    return {
      name: workflowName.value,
      nodes: nodes.value,
      edges: edges.value,
    }
  }

  // ─── 生成唯一节点 ID ───
  function generateId(): string {
    counter++
    return `node_${Date.now().toString(36)}_${counter.toString(36)}`
  }

  return {
    // 状态
    nodes,
    edges,
    nodeStatuses,
    workflowName,
    workflowId,
    dirty,
    // 计算属性
    nodeCount,
    edgeCount,
    // 方法
    addNode,
    removeNode,
    updateNodePosition,
    updateNodeLabel,
    updateNodeConfig,
    getNode,
    addEdge,
    removeEdge,
    setNodeStatus,
    resetAllStatuses,
    setWorkflowName,
    setWorkflowId,
    stepOutputs,
    setNodeOutput,
    clear,
    load,
    toJSON,
    generateId,
  }
})
