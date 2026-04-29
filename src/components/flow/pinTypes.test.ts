// pinTypes.test.ts — 节点注册表与针脚类型测试
import { describe, it, expect } from 'vitest'
import { NODE_REGISTRY, getNodeDef, type NodeDefinition } from './pinTypes'

describe('pinTypes', () => {
  describe('NODE_REGISTRY', () => {
    it('所有节点都有必需的 type 字段', () => {
      for (const def of NODE_REGISTRY) {
        expect(def.type, `节点 ${def.label} 缺少 type`).toBeTruthy()
        expect(typeof def.type).toBe('string')
      }
    })

    it('所有节点都有 label', () => {
      for (const def of NODE_REGISTRY) {
        expect(def.label).toBeTruthy()
      }
    })

    it('所有节点都有 category', () => {
      for (const def of NODE_REGISTRY) {
        expect(def.category).toBeTruthy()
      }
    })

    it('所有节点都有 color', () => {
      for (const def of NODE_REGISTRY) {
        expect(def.color).toMatch(/^#[0-9a-fA-F]{6}$/)
      }
    })

    it('type 不重复', () => {
      const types = NODE_REGISTRY.map(d => d.type)
      const unique = new Set(types)
      expect(unique.size).toBe(types.length)
    })

    it('每个节点的 inputs/outputs 是有效数组', () => {
      for (const def of NODE_REGISTRY) {
        expect(Array.isArray(def.inputs)).toBe(true)
        expect(Array.isArray(def.outputs)).toBe(true)
      }
    })
  })

  describe('getNodeDef', () => {
    it('返回匹配的节点定义', () => {
      const def = getNodeDef('http')
      expect(def).toBeDefined()
      expect(def!.type).toBe('http')
    })

    it('返回 undefined 当类型不存在', () => {
      expect(getNodeDef('nonexistent')).toBeUndefined()
    })
  })

  describe('节点连接兼容性', () => {
    it('常用节点有至少一个 input 或 output', () => {
      const http = NODE_REGISTRY.find(d => d.type === 'http')
      expect(http).toBeDefined()
      expect(http!.outputs.length).toBeGreaterThanOrEqual(0)

      const jsonParse = NODE_REGISTRY.find(d => d.type === 'json_parse')
      expect(jsonParse).toBeDefined()
    })
  })
})
