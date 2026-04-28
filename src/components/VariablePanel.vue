<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { useWorkflowStore } from '../stores/workflow'
import { useToast } from '../composables/useToast'

const store = useWorkflowStore()
const toast = useToast()

// ─── 变量条目（内部编辑用，value 一律存为字符串） ───

interface VarEntry {
  key: string
  type: 'string' | 'number' | 'boolean' | 'array' | 'object'
  value: string
}

const entries = ref<VarEntry[]>([])
const showPanel = ref(true)

// ─── 添加状态 ───

const isAdding = ref(false)
const newKey = ref('')
const newType = ref<'string' | 'number' | 'boolean' | 'array' | 'object'>('string')
const newValue = ref('')
const addError = ref('')
const editingEntryIndex = ref<number | null>(null)
const editKey = ref('')
const editValue = ref('')
const editError = ref('')

// ─── 类型推断 ───

function inferType(val: unknown): VarEntry['type'] {
  if (val === null || val === undefined) return 'string'
  if (Array.isArray(val)) return 'array'
  if (typeof val === 'object') return 'object'
  if (typeof val === 'number') return 'number'
  if (typeof val === 'boolean') return 'boolean'
  return 'string'
}

// ─── 值→字符串（用于编辑框） ───

function valueToString(val: unknown, type: VarEntry['type']): string {
  if (val === null || val === undefined) return ''
  if (type === 'array' || type === 'object') {
    try { return JSON.stringify(val, null, 2) }
    catch { return String(val) }
  }
  if (type === 'boolean') return val ? 'true' : 'false'
  return String(val)
}

// ─── 字符串→值（存回 store） ───

function stringToValue(s: string, type: VarEntry['type']): unknown {
  switch (type) {
    case 'string': return s
    case 'number': {
      const n = Number(s)
      return Number.isNaN(n) ? 0 : n
    }
    case 'boolean': return s.toLowerCase() === 'true'
    case 'array':
    case 'object': {
      try {
        const parsed = JSON.parse(s)
        if (type === 'array' && !Array.isArray(parsed)) throw new Error('应为数组')
        if (type === 'object' && (Array.isArray(parsed) || typeof parsed !== 'object')) throw new Error('应为对象')
        return parsed
      } catch {
        return type === 'array' ? [] : {}
      }
    }
  }
}

// ─── 引用语法显示 ───

function refSyntax(key: string) { return '{{' + key + '}}' }
function refTitle(key: string) { return '引用: {{' + key + '}}' }

// ─── 从 store 同步到 entries ───

function syncFromStore() {
  const vars = store.variables || {}
  entries.value = Object.entries(vars).map(([key, val]) => ({
    key,
    type: inferType(val),
    value: valueToString(val, inferType(val)),
  }))
}

// ─── 从 entries 写回 store ───

function syncToStore() {
  const newVars: Record<string, unknown> = {}
  for (const e of entries.value) {
    newVars[e.key] = stringToValue(e.value, e.type)
  }
  // Clear existing keys (delete is reactive in Vue 3)
  for (const k of Object.keys(store.variables)) {
    delete store.variables[k]
  }
  Object.assign(store.variables, newVars)
  store.syncToYaml()
}

// ─── 初始化 + 监听 store ───

let updatingFromStore = false

onMounted(() => { syncFromStore() })

watch(() => store.variables, () => {
  // 不打扰正在编辑/添加的用户
  if (editingEntryIndex.value !== null || isAdding.value) return
  updatingFromStore = true
  syncFromStore()
  updatingFromStore = false
}, { deep: true })

// ─── 添加变量 ───

function startAdd() {
  isAdding.value = true
  newKey.value = ''
  newValue.value = ''
  newType.value = 'string'
  addError.value = ''
}

function cancelAdd() {
  isAdding.value = false
  newKey.value = ''
  newValue.value = ''
}

function confirmAdd() {
  addError.value = ''
  const key = newKey.value.trim()
  if (!key) { addError.value = '变量名不能为空'; return }
  if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(key)) { addError.value = '变量名只能包含字母、数字和下划线，且不能以数字开头'; return }
  if (entries.value.some(e => e.key === key)) { addError.value = '变量名已存在'; return }

  const val = newType.value === 'boolean' ? (newValue.value || 'false') : newValue.value
  entries.value.push({ key, type: newType.value, value: val })
  syncToStore()
  cancelAdd()
}

// ─── 编辑变量 ───

function startEdit(index: number) {
  const e = entries.value[index]
  editingEntryIndex.value = index
  editKey.value = e.key
  editValue.value = e.value
  editError.value = ''
}

function confirmEdit() {
  if (editingEntryIndex.value === null) return
  editError.value = ''
  const key = editKey.value.trim()
  if (!key) { editError.value = '变量名不能为空'; return }
  if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(key)) { editError.value = '变量名只能包含字母、数字和下划线，且不能以数字开头'; return }
  const dup = entries.value.find((e, i) => e.key === key && i !== editingEntryIndex.value)
  if (dup) { editError.value = '变量名已存在'; return }

  entries.value[editingEntryIndex.value!].key = key
  entries.value[editingEntryIndex.value!].value = editValue.value
  syncToStore()
  cancelEdit()
}

function cancelEdit() {
  editingEntryIndex.value = null
  editKey.value = ''
  editValue.value = ''
  editError.value = ''
}

function deleteEntry(index: number) {
  entries.value.splice(index, 1)
  if (editingEntryIndex.value === index) cancelEdit()
  syncToStore()
  toast.success('变量已删除')
}

// ─── 类型切换 ───

function onTypeChange(index: number, newType: VarEntry['type']) {
  const e = entries.value[index]
  const oldType = e.type
  if (oldType === newType) return

  // 转换值时尝试保留数据
  if (oldType === 'string' && (newType === 'number' || newType === 'boolean')) {
    // 保留字符串作为值
  } else if (newType === 'array' || newType === 'object') {
    if (oldType === 'array' || oldType === 'object') {
      // 已经是 JSON 字符串，保持不变
    } else {
      // 将简单值包装成 JSON
      e.value = JSON.stringify(stringToValue(e.value, oldType), null, 2)
    }
  } else if ((oldType === 'array' || oldType === 'object') && (newType === 'string' || newType === 'number' || newType === 'boolean')) {
    try {
      const parsed = JSON.parse(e.value)
      e.value = valueToString(parsed, newType)
    } catch {
      e.value = ''
    }
  }

  e.type = newType
  syncToStore()
}

// ─── 数值/布尔值输入变化 ───

function onValueChange(index: number) {
  syncToStore()
}

// ─── 检测是否为 JSON 类型 ───

function isJsonType(type: string): boolean {
  return type === 'array' || type === 'object'
}

// ─── 工具函数 ───

function truncate(s: string, len: number): string {
  if (!s) return ''
  return s.length > len ? s.substring(0, len) + '...' : s
}

// ─── JSON 验证 ───

function validateJson(index: number): string {
  const e = entries.value[index]
  if (!e.value.trim()) return ''
  try {
    const parsed = JSON.parse(e.value)
    if (e.type === 'array' && !Array.isArray(parsed)) return 'JSON 不是数组'
    if (e.type === 'object' && (Array.isArray(parsed) || typeof parsed !== 'object')) return 'JSON 不是对象'
    return ''
  } catch (err: any) {
    return err.message || '无效 JSON'
  }
}
</script>

<template>
  <div class="variable-panel" :class="{ collapsed: !showPanel }">
    <!-- 标题栏 -->
    <div class="vp-header" @click="showPanel = !showPanel">
      <span class="vp-title">
        <span class="toggle-arrow" :class="{ open: showPanel }">▸</span>
        📦 全局变量
      </span>
      <span class="vp-count">{{ entries.length }}</span>
      <button
        v-if="showPanel"
        class="btn btn-icon btn-add"
        title="添加变量"
        @click.stop="startAdd"
      >+</button>
    </div>

    <div v-if="showPanel" class="vp-body">
      <!-- 添加表单 -->
      <div v-if="isAdding" class="vp-add-form">
        <div class="vp-add-row">
          <input
            v-model="newKey"
            class="vp-input vp-key-input"
            placeholder="变量名"
            @keyup.enter="confirmAdd"
            @keyup.escape="cancelAdd"
          />
          <select v-model="newType" class="vp-select">
            <option value="string">String</option>
            <option value="number">Number</option>
            <option value="boolean">Boolean</option>
            <option value="array">Array</option>
            <option value="object">Object</option>
          </select>
        </div>
        <div class="vp-add-row">
          <label class="vp-input-label">值:</label>
          <input
            v-if="!isJsonType(newType)"
            v-model="newValue"
            :type="newType === 'number' ? 'number' : (newType === 'boolean' ? 'text' : 'text')"
            :placeholder="newType === 'boolean' ? 'true / false' : ''"
            class="vp-input vp-value-input"
            @keyup.enter="confirmAdd"
            @keyup.escape="cancelAdd"
          />
          <textarea
            v-else
            v-model="newValue"
            class="vp-json-input"
            rows="3"
            placeholder="输入 JSON..."
          ></textarea>
        </div>
        <div v-if="addError" class="vp-error">{{ addError }}</div>
        <div class="vp-add-actions">
          <button class="btn btn-xs btn-primary" @click="confirmAdd">✓ 确定</button>
          <button class="btn btn-xs" @click="cancelAdd">✗ 取消</button>
        </div>
      </div>

      <!-- 变量列表 -->
      <div v-if="entries.length === 0 && !isAdding" class="vp-empty">
        暂无全局变量。点击 + 添加。
      </div>
      <div v-else class="vp-list">
        <template v-for="(entry, index) in entries" :key="index">
          <!-- 编辑模式 -->
          <div v-if="editingEntryIndex === index" class="vp-item vp-item-editing">
            <div class="vp-item-row">
              <input
                v-model="editKey"
                class="vp-input vp-key-input"
                placeholder="变量名"
                @keyup.enter="confirmEdit"
                @keyup.escape="cancelEdit"
              />
              <select
                v-model="entry.type"
                class="vp-select"
                @change="onTypeChange(index, ($event.target as HTMLSelectElement).value as VarEntry['type'])"
              >
                <option value="string">String</option>
                <option value="number">Number</option>
                <option value="boolean">Boolean</option>
                <option value="array">Array</option>
                <option value="object">Object</option>
              </select>
              <button class="btn btn-icon btn-delete" title="删除" @click="deleteEntry(index)">🗑</button>
            </div>
            <div class="vp-item-row">
              <label class="vp-input-label">值:</label>
              <input
                v-if="!isJsonType(entry.type)"
                v-model="editValue"
                :type="entry.type === 'number' ? 'number' : 'text'"
                class="vp-input vp-value-input"
                @keyup.enter="confirmEdit"
                @change="onValueChange(index)"
              />
              <textarea
                v-else
                v-model="editValue"
                class="vp-json-input"
                rows="3"
                @change="onValueChange(index)"
              ></textarea>
            </div>
            <div v-if="isJsonType(entry.type) && validateJson(index)" class="vp-json-error">
              ⚠ {{ validateJson(index) }}
            </div>
            <div v-if="editError" class="vp-error">{{ editError }}</div>
            <div class="vp-add-actions">
              <button class="btn btn-xs btn-primary" @click="confirmEdit">✓ 确定</button>
              <button class="btn btn-xs" @click="cancelEdit">✗ 取消</button>
            </div>
          </div>

          <!-- 显示模式 -->
          <div v-else class="vp-item" @click.stop="startEdit(index)">
            <div class="vp-item-main">
              <div class="vp-item-key">{{ entry.key }}</div>
              <div class="vp-item-meta">
                <span class="vp-type-badge">{{ entry.type }}</span>
                <span class="vp-value-preview">{{ truncate(entry.value, 40) }}</span>
              </div>
            </div>
            <div class="vp-item-ref" :title="refTitle(entry.key)">
              <code v-text="refSyntax(entry.key)"></code>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.variable-panel {
  border-top: 1px solid #30363d;
  user-select: none;
}
.variable-panel.collapsed { border-bottom: 1px solid #30363d; }

.vp-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  cursor: pointer;
  font-size: 11px;
  font-weight: 600;
  color: #8b949e;
}
.vp-header:hover { background: #21262d; }
.vp-title {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 4px;
}
.toggle-arrow {
  font-size: 10px;
  transition: transform 0.15s;
  display: inline-block;
  font-family: monospace;
}
.toggle-arrow.open { transform: rotate(90deg); }
.vp-count {
  font-size: 10px;
  color: #6e7681;
  background: #21262d;
  padding: 1px 6px;
  border-radius: 8px;
}

/* ─── 按钮 ─── */
.btn-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: 4px;
  border: 1px solid #30363d;
  background: #21262d;
  color: #c9d1d9;
  cursor: pointer;
  font-size: 13px;
  line-height: 1;
  padding: 0;
  flex-shrink: 0;
}
.btn-icon:hover { background: #30363d; }
.btn-add { font-size: 14px; font-weight: 700; }
.btn-delete { color: #f85149; border-color: #f8514933; }
.btn-delete:hover { background: #f8514922; }

/* ─── Body ─── */
.vp-body {
  padding: 0 6px 8px;
  max-height: 320px;
  overflow-y: auto;
}

/* ─── 空状态 ─── */
.vp-empty {
  font-size: 11px;
  color: #484f58;
  padding: 8px 6px;
  text-align: center;
}

/* ─── 添加表单 ─── */
.vp-add-form {
  background: #1a1e24;
  border: 1px solid #1f6feb44;
  border-radius: 6px;
  padding: 8px;
  margin-bottom: 6px;
}
.vp-add-row {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-bottom: 4px;
}
.vp-add-row:last-child { margin-bottom: 0; }
.vp-input-label {
  font-size: 10px;
  color: #8b949e;
  flex-shrink: 0;
  width: 22px;
}
.vp-input {
  background: #0d1117;
  border: 1px solid #30363d;
  color: #c9d1d9;
  border-radius: 4px;
  padding: 3px 6px;
  font-size: 11px;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
  outline: none;
  width: 100%;
  min-width: 0;
}
.vp-input:focus { border-color: #58a6ff; }
.vp-key-input { flex: 1; }
.vp-value-input { flex: 2; }
.vp-select {
  background: #0d1117;
  border: 1px solid #30363d;
  color: #c9d1d9;
  border-radius: 4px;
  padding: 3px 4px;
  font-size: 10px;
  flex-shrink: 0;
  outline: none;
  cursor: pointer;
}
.vp-select:focus { border-color: #58a6ff; }

.vp-json-input {
  flex: 2;
  background: #0d1117;
  border: 1px solid #30363d;
  color: #c9d1d9;
  border-radius: 4px;
  padding: 4px 6px;
  font-size: 10px;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
  outline: none;
  resize: vertical;
  min-height: 40px;
}
.vp-json-input:focus { border-color: #58a6ff; }

.vp-error {
  font-size: 10px;
  color: #f85149;
  margin-top: 2px;
}
.vp-json-error {
  font-size: 9px;
  color: #d29922;
  margin-top: 2px;
  font-family: monospace;
}
.vp-add-actions {
  display: flex;
  gap: 4px;
  margin-top: 4px;
}

/* ─── 变量列表项 ─── */
.vp-list {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.vp-item {
  display: flex;
  align-items: flex-start;
  gap: 4px;
  padding: 4px 6px;
  border-radius: 4px;
  cursor: pointer;
  border: 1px solid transparent;
}
.vp-item:hover { background: #21262d; border-color: #30363d; }
.vp-item-editing {
  background: #1a1e24;
  border: 1px solid #1f6feb33;
  border-radius: 6px;
  padding: 6px;
  margin-bottom: 2px;
  cursor: default;
  flex-direction: column;
}

.vp-item-main {
  flex: 1;
  min-width: 0;
}
.vp-item-key {
  font-size: 11px;
  font-weight: 600;
  color: #c9d1d9;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
}
.vp-item-meta {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-top: 1px;
}
.vp-type-badge {
  font-size: 9px;
  color: #58a6ff;
  background: #1f6feb18;
  padding: 0 4px;
  border-radius: 3px;
  flex-shrink: 0;
}
.vp-value-preview {
  font-size: 10px;
  color: #6e7681;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.vp-item-ref {
  flex-shrink: 0;
  max-width: 100px;
  overflow: hidden;
}
.vp-item-ref code {
  font-size: 9px;
  color: #79c0ff;
  background: #1f6feb18;
  padding: 1px 4px;
  border-radius: 3px;
  white-space: nowrap;
}

/* ─── 行内编辑 ─── */
.vp-item-row {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-bottom: 3px;
}
.vp-item-row:last-child { margin-bottom: 0; }
</style>
