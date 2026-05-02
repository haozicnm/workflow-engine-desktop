<template>
  <nav class="side-toolbar">
    <!-- 上组：功能入口 -->
    <div class="tool-group top">
      <button class="tool-btn active" title="工作流" @click="$emit('navigate', '/')">
        ⚡
      </button>
      <button class="tool-btn" title="节点库" @click="$emit('toggle-palette')">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/></svg>
      </button>
    </div>

    <!-- 分隔 -->
    <div class="tool-spacer" />

    <!-- 下组：工具 -->
    <div class="tool-group bottom">
      <button class="tool-btn" :class="{ on: showConsole }" title="控制台" @click="$emit('toggle-console')">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>
      </button>
      <button class="tool-btn" title="设置" @click="$emit('navigate', '/settings')">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
      </button>
    </div>
  </nav>
</template>

<script setup lang="ts">
defineProps<{ showConsole?: boolean }>()
defineEmits<{
  'navigate': [path: string]
  'toggle-palette': []
  'toggle-console': []
}>()
</script>

<style scoped>
.side-toolbar {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 48px;
  height: 100%;
  background: var(--color-surface);
  border-right: 1px solid var(--color-border);
  padding: 8px 0;
  gap: 4px;
  flex-shrink: 0;
}
.tool-group {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}
.tool-spacer {
  flex: 1;
}
.tool-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: #8b949e;
  cursor: pointer;
  font-size: 16px;
  transition: all 0.15s;
}
.tool-btn:hover {
  background: var(--color-elevated);
  color: var(--color-text);
}
.tool-btn.active,
.tool-btn.on {
  background: #1f6feb22;
  color: var(--color-accent);
}
</style>
