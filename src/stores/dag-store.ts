// ─── DAG Workflow Store (Pinia) — 节点/连线/状态管理 ───
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { DAGNode, DAGEdge, DAGNodeStatus } from '../types/dag-node'

export const useDAGStore = defineStore('dag', () => {
  const nodes = ref<DAGNode[]>([])
  const edges = ref<DAGEdge[]>([])
  const nodeStatuses = ref<Record<string, DAGNodeStatus>>({})

  // ─── 节点操作 ───
  function addNode(node: DAGNode) {
    nodes.value.push(node)
    nodeStatuses.value[node.id] = 'idle'
  }

  function removeNode(id: string) {
    nodes.value = nodes.value.filter(n => n.id !== id)
    edges.value = edges.value.filter(e => e.source !== id && e.target !== id)
    delete nodeStatuses.value[id]
  }

  function updateNodePosition(id: string, position: { x: number; y: number }) {
    const node = nodes.value.find(n => n.id === id)
    if (node) node.position = position
  }

  function updateNodeConfig(id: string, config: Record<string, unknown>) {
    const node = nodes.value.find(n => n.id === id)
    if (node) node.config = { ...node.config, ...config }
  }

  // ─── 连线操作 ───
  function addEdge(edge: DAGEdge) {
    // 防重复
    const exists = edges.value.some(
      e => e.source === edge.source
        && e.target === edge.target
        && e.sourceHandle === edge.sourceHandle
        && e.targetHandle === edge.targetHandle
    )
    if (!exists) edges.value.push(edge)
  }

  function removeEdge(id: string) {
    edges.value = edges.value.filter(e => e.id !== id)
  }

  // ─── 状态操作 ───
  function setNodeStatus(id: string, status: DAGNodeStatus) {
    nodeStatuses.value[id] = status
  }

  function resetAllStatuses() {
    Object.keys(nodeStatuses.value).forEach(id => {
      nodeStatuses.value[id] = 'idle'
    })
  }

  // ─── 序列化 ───
  const workflowJson = computed(() => ({
    nodes: nodes.value,
    edges: edges.value,
  }))

  function loadWorkflow(data: { nodes: DAGNode[]; edges: DAGEdge[] }) {
    nodes.value = data.nodes
    edges.value = data.edges
    data.nodes.forEach(n => {
      nodeStatuses.value[n.id] = 'idle'
    })
  }

  function clear() {
    nodes.value = []
    edges.value = []
    nodeStatuses.value = {}
  }

  return {
    nodes, edges, nodeStatuses,
    addNode, removeNode, updateNodePosition, updateNodeConfig,
    addEdge, removeEdge,
    setNodeStatus, resetAllStatuses,
    workflowJson, loadWorkflow, clear,
  }
})
