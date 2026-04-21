<template>
  <div class="flex h-screen bg-gray-50 dark:bg-gray-900">
    <!-- 侧边栏 -->
    <aside class="w-56 bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 flex flex-col">
      <div class="p-4 border-b border-gray-200 dark:border-gray-700">
        <h1 class="text-lg font-bold text-gray-800 dark:text-white">⚙️ Workflow Engine</h1>
      </div>
      <nav class="flex-1 p-3 space-y-1">
        <router-link
          to="/"
          class="nav-link"
          :class="{ 'nav-active': $route.path === '/' }"
        >
          📋 工作流
        </router-link>
        <router-link
          to="/editor"
          class="nav-link"
          :class="{ 'nav-active': $route.path.startsWith('/editor') }"
        >
          🎨 编辑器
        </router-link>
        <router-link
          to="/monitor"
          class="nav-link"
          :class="{ 'nav-active': $route.path === '/monitor' }"
        >
          📊 监控
        </router-link>
        <router-link
          to="/settings"
          class="nav-link"
          :class="{ 'nav-active': $route.path === '/settings' }"
        >
          ⚙️ 设置
        </router-link>
      </nav>
      <div class="p-3 border-t border-gray-200 dark:border-gray-700 text-xs text-gray-400">
        v0.1.0 · P0
      </div>
    </aside>

    <!-- 主内容区 -->
    <main class="flex-1 overflow-auto">
      <router-view />
    </main>
  </div>
</template>

<script setup lang="ts">
// App 根组件
</script>

<style scoped>
.nav-link {
  @apply block px-3 py-2 rounded-md text-sm text-gray-600 dark:text-gray-300
         hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors;
}
.nav-active {
  @apply bg-blue-50 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400 font-medium;
}
</style>
