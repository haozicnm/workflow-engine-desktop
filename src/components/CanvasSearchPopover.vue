<template>
  <Teleport to="body">
    <!-- 透明遮罩层：点击外部关闭 -->
    <div
      v-if="visible"
      class="canvas-search-overlay"
      @click.self="emit('close')"
    >
      <div
        ref="popoverRef"
        class="canvas-search-popover"
        :style="{ left: `${x}px`, top: `${y}px` }"
      >
        <!-- 搜索输入框 -->
        <div class="search-header">
          <input
            ref="inputRef"
            v-model="query"
            type="text"
            placeholder="搜索节点..."
            class="search-input"
            @keydown="handleKeydown"
          />
        </div>

        <!-- 结果列表 -->
        <div ref="listRef" class="results-list">
          <div
            v-for="(item, index) in filteredResults"
            :key="item.type"
            class="result-item"
            :class="{ selected: index === selectedIndex }"
            @click="selectItem(item)"
            @mouseenter="selectedIndex = index"
          >
            <span class="item-icon">{{ item.icon }}</span>
            <div class="item-info">
              <span class="item-label">{{ item.label }}</span>
              <span v-if="item.description" class="item-desc">{{ item.description }}</span>
            </div>
            <span class="item-category-tag">{{ categoryLabel(item.category) }}</span>
          </div>
          <div v-if="filteredResults.length === 0" class="no-results">
            无匹配结果
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import type { NodeDefinition } from './flow/pinTypes'
import { NODE_REGISTRY } from './flow/pinTypes'

// ─── Props ───
const props = defineProps<{
  visible: boolean
  x: number
  y: number
  graph: any // LGraph | null — 父组件持有，此处仅透传类型
}>()

// ─── Emits ───
const emit = defineEmits<{
  close: []
  'node-added': [def: NodeDefinition]
}>()

// ─── 分类中文标签 ───
const CATEGORY_LABELS: Record<string, string> = {
  browser: '浏览器',
  excel: 'Excel',
  word: 'Word',
  data: '数据',
  logic: '逻辑',
  output: '输出',
  other: '其它',
}

function categoryLabel(cat: string): string {
  return CATEGORY_LABELS[cat] || cat
}

// ─── 搜索状态 ───
const query = ref('')
const selectedIndex = ref(0)
const inputRef = ref<HTMLInputElement | null>(null)
const listRef = ref<HTMLElement | null>(null)
const popoverRef = ref<HTMLElement | null>(null)

// ─── 过滤结果 ───
const filteredResults = computed<NodeDefinition[]>(() => {
  const q = query.value.trim().toLowerCase()
  if (!q) return NODE_REGISTRY

  return NODE_REGISTRY.filter(
    d =>
      d.label.toLowerCase().includes(q) ||
      d.type.toLowerCase().includes(q) ||
      (d.description || '').toLowerCase().includes(q)
  )
})

// ─── 可见性变化时重置状态 ───
watch(
  () => props.visible,
  val => {
    if (val) {
      query.value = ''
      selectedIndex.value = 0
      nextTick(() => {
        inputRef.value?.focus()
      })
    }
  }
)

// 结果变化时钳制选中索引
watch(filteredResults, results => {
  if (selectedIndex.value >= results.length) {
    selectedIndex.value = Math.max(0, results.length - 1)
  }
})

// ─── 滚动选中项到可见区域 ───
function scrollToSelected() {
  nextTick(() => {
    const list = listRef.value
    if (!list) return
    const selected = list.querySelector('.result-item.selected') as HTMLElement | null
    if (!selected) return

    const listRect = list.getBoundingClientRect()
    const itemRect = selected.getBoundingClientRect()

    if (itemRect.bottom > listRect.bottom) {
      selected.scrollIntoView({ block: 'nearest', behavior: 'auto' })
    } else if (itemRect.top < listRect.top) {
      selected.scrollIntoView({ block: 'nearest', behavior: 'auto' })
    }
  })
}

// ─── 选择项 ───
function selectItem(item: NodeDefinition) {
  emit('node-added', item)
  emit('close')
}

// ─── 键盘导航 ───
function handleKeydown(e: KeyboardEvent) {
  const results = filteredResults.value

  switch (e.key) {
    case 'ArrowDown':
      e.preventDefault()
      if (results.length > 0) {
        selectedIndex.value = (selectedIndex.value + 1) % results.length
        scrollToSelected()
      }
      break

    case 'ArrowUp':
      e.preventDefault()
      if (results.length > 0) {
        selectedIndex.value =
          (selectedIndex.value - 1 + results.length) % results.length
        scrollToSelected()
      }
      break

    case 'Enter':
      e.preventDefault()
      if (results.length > 0) {
        const item = results[selectedIndex.value]
        if (item) selectItem(item)
      }
      break

    case 'Escape':
      e.preventDefault()
      emit('close')
      break
  }
}
</script>

<style scoped>
/* ─── 遮罩层：覆盖全屏但不拦截点击 ─── */
.canvas-search-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  /* 自身不渲染背景色，仅用来捕获外部点击 */
}

/* ─── 弹窗本体 ─── */
.canvas-search-popover {
  position: fixed;
  width: 280px;
  max-height: 360px;
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  /* 默认 transform 基于 top-left，由 x/y 定位 */
}

/* ─── 搜索输入区 ─── */
.search-header {
  padding: 8px;
  border-bottom: 1px solid #30363d;
  flex-shrink: 0;
}

.search-input {
  width: 100%;
  padding: 6px 10px;
  background: #0d1117;
  border: 1px solid #30363d;
  border-radius: 6px;
  color: #c9d1d9;
  font-size: 13px;
  outline: none;
  transition: border-color 0.15s;
  box-sizing: border-box;
}

.search-input::placeholder {
  color: #484f58;
}

.search-input:focus {
  border-color: #58a6ff;
}

/* ─── 结果列表 ─── */
.results-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}

.result-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  cursor: pointer;
  border-left: 2px solid transparent;
  transition: background 0.1s, border-color 0.1s;
}

.result-item:hover {
  background: #1c2128;
}

.result-item.selected {
  background: #1f6feb22;
  border-left-color: #58a6ff;
}

/* ─── 图标 ─── */
.item-icon {
  font-size: 16px;
  flex-shrink: 0;
  line-height: 1;
}

/* ─── 名称 + 描述 ─── */
.item-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.item-label {
  font-size: 12px;
  color: #c9d1d9;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.item-desc {
  font-size: 10px;
  color: #484f58;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ─── 类别标签 ─── */
.item-category-tag {
  font-size: 10px;
  background: #21262d;
  border-radius: 4px;
  padding: 1px 6px;
  color: #8b949e;
  flex-shrink: 0;
  white-space: nowrap;
}

/* ─── 无结果 ─── */
.no-results {
  padding: 24px;
  text-align: center;
  color: #484f58;
  font-size: 13px;
}
</style>
