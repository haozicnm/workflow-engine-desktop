<script setup lang="ts">
import { RouterView, RouterLink, useRoute } from 'vue-router'
import Toast from './components/Toast.vue'
import StatusBar from './components/StatusBar.vue'
import { useToast } from './composables/useToast'

const route = useRoute()
const { toasts, remove } = useToast()
</script>

<template>
  <div class="app-shell">
    <header class="app-header">
      <RouterLink to="/" class="app-brand">
        <span class="brand-icon">⚡</span>
        <span class="brand-text">Workflow Engine</span>
      </RouterLink>
      <nav class="app-nav">
        <RouterLink to="/" :class="{ active: route.path === '/' }">📋 工作流</RouterLink>
        <RouterLink to="/editor/new" :class="{ active: route.path.startsWith('/editor') }">📝 编辑器</RouterLink>
      </nav>
    </header>
    <main class="app-main">
      <RouterView v-slot="{ Component }">
        <KeepAlive :include="['LiteGraphEditor']">
          <component :is="Component" />
        </KeepAlive>
      </RouterView>
    </main>
    <StatusBar />

    <!-- 全局 Toast -->
    <Toast
      v-for="t in toasts"
      :key="t.id"
      :message="t.message"
      :type="t.type"
      :duration="t.duration"
      @close="remove(t.id)"
    />
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #0f1117;
  color: #e1e4e8;
}
.app-header {
  display: flex;
  align-items: center;
  gap: 24px;
  padding: 0 20px;
  height: 48px;
  background: #161b22;
  border-bottom: 1px solid #30363d;
  flex-shrink: 0;
}
.app-brand {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  font-size: 15px;
  color: #e1e4e8;
  text-decoration: none;
}
.app-brand:hover { color: #58a6ff; }
.brand-icon { font-size: 18px; }
.app-nav {
  display: flex;
  gap: 4px;
}
.app-nav :deep(a) {
  padding: 6px 12px;
  border-radius: 6px;
  text-decoration: none;
  color: #8b949e;
  font-size: 13px;
  transition: all 0.15s;
}
.app-nav :deep(a:hover) { background: #21262d; color: #e1e4e8; }
.app-nav :deep(a.active) { background: #1f6feb22; color: #58a6ff; }
.app-main {
  flex: 1;
  overflow: auto;
}
</style>
