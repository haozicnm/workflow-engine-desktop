<template>
  <div class="workflow-tabs">
    <div
      v-for="tab in store.tabs"
      :key="tab.id"
      :class="['tab-item', { active: tab.id === store.activeTabId }]"
      @click="store.setActive(tab.id)"
    >
      <span class="tab-name">{{ tab.name }}</span>
      <span v-if="tab.dirty" class="tab-dot">●</span>
      <button
        v-if="store.tabCount > 1"
        class="tab-close"
        @click.stop="store.removeTab(tab.id)"
        title="关闭"
      >×</button>
    </div>
    <button class="tab-add" @click="onAdd" title="新建工作流">+</button>
  </div>
</template>

<script setup lang="ts">
import { useTabStore } from '../stores/tabStore'

const store = useTabStore()

const emit = defineEmits<{
  'add': []
}>()

function onAdd() {
  emit('add')
}
</script>

<style scoped>
.workflow-tabs {
  display: flex;
  align-items: center;
  height: 32px;
  background: var(--color-bg);
  border-bottom: 1px solid var(--color-border);
  padding: 0 4px;
  gap: 2px;
  flex-shrink: 0;
  overflow-x: auto;
  scrollbar-width: none;
}
.workflow-tabs::-webkit-scrollbar { display: none; }

.tab-item {
  display: flex;
  align-items: center;
  gap: 4px;
  height: 26px;
  padding: 0 10px;
  border-radius: 6px 6px 0 0;
  background: var(--color-elevated);
  color: var(--color-text-dim);
  font-size: 11px;
  cursor: pointer;
  white-space: nowrap;
  user-select: none;
  border: 1px solid transparent;
  border-bottom: none;
  transition: all 0.12s;
}
.tab-item:hover {
  background: var(--color-border);
  color: var(--color-text);
}
.tab-item.active {
  background: var(--color-surface);
  color: var(--color-text);
  border-color: var(--color-border);
}

.tab-name {
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tab-dot {
  color: var(--color-warning);
  font-size: 8px;
  flex-shrink: 0;
}

.tab-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  border: none;
  border-radius: 3px;
  background: transparent;
  color: var(--color-text-dim);
  font-size: 12px;
  cursor: pointer;
  flex-shrink: 0;
  line-height: 1;
}
.tab-close:hover {
  background: var(--color-danger-bg);
  color: #fff;
}

.tab-add {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--color-text-dim);
  font-size: 16px;
  cursor: pointer;
  flex-shrink: 0;
  margin-left: 2px;
}
.tab-add:hover {
  background: var(--color-elevated);
  color: var(--color-text);
}
</style>
