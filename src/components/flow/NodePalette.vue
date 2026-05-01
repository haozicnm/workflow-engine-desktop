<template>
  <aside class="node-palette">
    <div class="palette-search">
      <svg class="search-icon" width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
        <path d="M11.742 10.344a6.5 6.5 0 1 0-1.397 1.398h-.001l3.85 3.85a1 1 0 0 0 1.415-1.414l-3.85-3.85zm-5.242.156a5 5 0 1 1 0-10 5 5 0 0 1 0 10z"/>
      </svg>
      <input
        v-model="search"
        type="text"
        placeholder="搜索节点..."
        class="search-input"
      />
    </div>

    <div class="category-list">
      <div
        v-for="cat in filteredCategories"
        :key="cat.name"
        class="category"
      >
        <button
          class="category-toggle"
          @click="toggleCategory(cat.name)"
        >
          <span class="chevron" :class="{ expanded: isExpanded(cat.name) }">▸</span>
          <span class="category-name">{{ cat.label }}</span>
          <span class="category-count">{{ cat.items.length }}</span>
        </button>

        <div v-show="isExpanded(cat.name)" class="category-items">
          <div
            v-for="def in cat.items"
            :key="def.type"
            class="palette-item"
            :style="{ '--accent': def.color }"
            draggable="true"
            @dragstart="onDrag($event, def)"
          >
            <span class="item-icon">{{ def.icon }}</span>
            <div class="item-info">
              <span class="item-name">{{ def.label }}</span>
              <span v-if="def.description" class="item-desc">{{ def.description }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-if="filteredCategories.length === 0" class="no-results">
      没有匹配的节点
    </div>
  </aside>
</template>

<script setup lang="ts">
import { ref, computed, reactive } from 'vue'
import type { NodeDefinition } from './pinTypes'
import { NODE_REGISTRY } from './pinTypes'

// ─── 分类定义 ───
const CATEGORY_DEFS = [
  { name: 'source', label: '📥 数据源' },
  { name: 'process', label: '🔧 处理' },
  { name: 'ai', label: '🤖 AI' },
  { name: 'automation', label: '🖥️ 浏览器' },
  { name: 'output', label: '📤 输出' },
] as const

// ─── 搜索 ───
const search = ref('')

// ─── 折叠状态 ───
const expanded = reactive(new Set<string>(['source', 'process', 'output']))

function toggleCategory(name: string) {
  if (expanded.has(name)) {
    expanded.delete(name)
  } else {
    expanded.add(name)
  }
}

function isExpanded(name: string): boolean {
  return expanded.has(name)
}

// ─── 过滤分类 ───
const filteredCategories = computed(() => {
  const q = search.value.trim().toLowerCase()

  return CATEGORY_DEFS
    .map(catDef => {
      const items = q
        ? NODE_REGISTRY.filter(
            d =>
              d.category === catDef.name &&
              (d.label.toLowerCase().includes(q) ||
                d.type.toLowerCase().includes(q) ||
                (d.description || '').toLowerCase().includes(q))
          )
        : NODE_REGISTRY.filter(d => d.category === catDef.name)

      return { ...catDef, items }
    })
    .filter(c => c.items.length > 0)
})

// ─── 拖拽 ───
const emit = defineEmits<{
  'drag-start': [def: NodeDefinition, event: DragEvent]
}>()

function onDrag(event: DragEvent, def: NodeDefinition) {
  event.dataTransfer?.setData('application/flow-node-type', def.type)
  event.dataTransfer!.effectAllowed = 'move'
  emit('drag-start', def, event)
}
</script>

<style scoped>
.node-palette {
  width: 220px;
  min-width: 220px;
  height: 100%;
  background: #0d1117;
  border-right: 1px solid #21262d;
  overflow-y: auto;
  user-select: none;
  display: flex;
  flex-direction: column;
}

/* ─── 搜索栏 ─── */
.palette-search {
  position: relative;
  padding: 10px;
  border-bottom: 1px solid #21262d;
  flex-shrink: 0;
}

.search-icon {
  position: absolute;
  left: 18px;
  top: 50%;
  transform: translateY(-50%);
  color: #484f58;
  pointer-events: none;
}

.search-input {
  width: 100%;
  padding: 6px 8px 6px 28px;
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 6px;
  color: #c9d1d9;
  font-size: 12px;
  outline: none;
  transition: border-color 0.15s;
}

.search-input::placeholder {
  color: #484f58;
}

.search-input:focus {
  border-color: #58a6ff;
}

/* ─── 分类列表 ─── */
.category-list {
  flex: 1;
  overflow-y: auto;
}

.category-toggle {
  display: flex;
  align-items: center;
  gap: 4px;
  width: 100%;
  padding: 6px 8px;
  background: none;
  border: none;
  color: #8b949e;
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  text-align: left;
  transition: background 0.1s;
}

.category-toggle:hover {
  background: #161b22;
}

.chevron {
  display: inline-block;
  width: 14px;
  font-size: 10px;
  transition: transform 0.15s;
  flex-shrink: 0;
}

.chevron.expanded {
  transform: rotate(90deg);
}

.category-name {
  flex: 1;
}

.category-count {
  font-size: 10px;
  background: #21262d;
  padding: 1px 6px;
  border-radius: 10px;
  color: #8b949e;
  font-weight: 400;
}

/* ─── 节点项 ─── */
.category-items {
  padding: 2px 0;
}

.palette-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px 6px 24px;
  font-size: 12px;
  cursor: grab;
  border-left: 2px solid transparent;
  transition: background 0.1s;
}

.palette-item:hover {
  background: #161b22;
  border-left-color: var(--accent, #58a6ff);
}

.palette-item:active {
  cursor: grabbing;
}

.item-icon {
  font-size: 16px;
  flex-shrink: 0;
}

.item-info {
  overflow: hidden;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.item-name {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: #c9d1d9;
}

.item-desc {
  font-size: 10px;
  color: #484f58;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ─── 空状态 ─── */
.no-results {
  padding: 24px;
  text-align: center;
  color: #484f58;
  font-size: 13px;
}
</style>
