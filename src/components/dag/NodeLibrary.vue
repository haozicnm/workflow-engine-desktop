<template>
  <div class="node-library">
    <div class="library-search">
      <input
        v-model="search"
        placeholder="🔍 搜索节点..."
        class="search-input"
      />
    </div>

    <div v-for="cat in filteredCategories" :key="cat.name" class="category">
      <div class="category-header" @click="cat.expanded = !cat.expanded">
        <span class="chevron">{{ cat.expanded ? '▼' : '▶' }}</span>
        <span>{{ cat.name }}</span>
        <span class="count">{{ cat.items.length }}</span>
      </div>
      <div v-show="cat.expanded" class="category-items">
        <div
          v-for="def in cat.items"
          :key="def.type"
          class="library-item"
          :style="{ borderLeftColor: def.color }"
          draggable="true"
          @dragstart="onDragStart($event, def)"
        >
          <span class="item-icon">{{ def.icon }}</span>
          <span class="item-label">{{ def.label }}</span>
        </div>
      </div>
    </div>

    <div v-if="filteredCategories.length === 0" class="no-results">
      没有匹配的节点
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue'
import type { DAGNodeDefinition } from '../../types/dag-node'
import { BASE_NODE_DEFINITIONS } from '../../types/dag-node'

const search = ref('')

const categories = reactive([
  {
    name: '📥 数据源',
    expanded: true,
    items: BASE_NODE_DEFINITIONS.filter(d => d.category === 'source'),
  },
  {
    name: '🔧 处理',
    expanded: true,
    items: BASE_NODE_DEFINITIONS.filter(d => d.category === 'process'),
  },
  {
    name: '🤖 AI',
    expanded: true,
    items: BASE_NODE_DEFINITIONS.filter(d => d.category === 'ai'),
  },
  {
    name: '📤 输出',
    expanded: true,
    items: BASE_NODE_DEFINITIONS.filter(d => d.category === 'output'),
  },
])

const filteredCategories = computed(() => {
  if (!search.value.trim()) return categories
  const q = search.value.toLowerCase()
  return categories
    .map(c => ({
      ...c,
      items: c.items.filter(
        item => item.label.toLowerCase().includes(q) || item.type.includes(q)
      ),
    }))
    .filter(c => c.items.length > 0)
})

function onDragStart(event: DragEvent, def: DAGNodeDefinition) {
  event.dataTransfer?.setData('application/dag-node-type', def.type)
  event.dataTransfer!.effectAllowed = 'move'
}
</script>

<style scoped>
.node-library {
  width: 220px;
  height: 100%;
  background: #181825;
  border-right: 1px solid #313244;
  overflow-y: auto;
  user-select: none;
}

.library-search {
  padding: 8px;
  position: sticky;
  top: 0;
  background: #181825;
  z-index: 10;
}

.search-input {
  width: 100%;
  padding: 6px 8px;
  background: #1e1e2e;
  border: 1px solid #313244;
  border-radius: 4px;
  color: #cdd6f4;
  font-size: 12px;
  outline: none;
}
.search-input:focus {
  border-color: #89b4fa;
}

.category-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 10px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  color: #a6adc8;
}
.category-header:hover {
  background: #1e1e2e;
}

.chevron {
  font-size: 10px;
  width: 12px;
}

.count {
  margin-left: auto;
  font-size: 10px;
  background: #313244;
  padding: 1px 6px;
  border-radius: 8px;
  color: #6c7086;
}

.library-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px 8px 24px;
  font-size: 12px;
  cursor: grab;
  border-left: 3px solid transparent;
  transition: background 0.1s;
}
.library-item:hover {
  background: #1e1e2e;
}
.library-item:active {
  cursor: grabbing;
}

.item-icon {
  font-size: 16px;
  flex-shrink: 0;
}

.item-label {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.no-results {
  padding: 20px;
  text-align: center;
  color: #6c7086;
  font-size: 13px;
}
</style>
