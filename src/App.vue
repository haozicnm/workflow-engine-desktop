<script setup lang="ts">
// App.vue — v3.0: Dashboard 为默认入口
import { ref, provide } from 'vue'
import LiteGraphEditor from './pages/LiteGraphEditor.vue'
import Dashboard from './pages/Dashboard.vue'
import Toast from './components/Toast.vue'
import StatusBar from './components/StatusBar.vue'
import { useToast } from './composables/useToast'

const { toasts, remove } = useToast()

type AppView = 'dashboard' | 'editor'

const currentView = ref<AppView>('dashboard')
const editorWorkflowId = ref<string | undefined>(undefined)
const editorTemplate = ref<{ id: string; name: string; nodes: unknown[]; edges: unknown[] } | null>(null)

provide('app:openEditor', (id?: string) => {
  editorWorkflowId.value = id
  editorTemplate.value = null
  currentView.value = 'editor'
})

provide('app:openFromTemplate', (template: { id: string; name: string; nodes: unknown[]; edges: unknown[] }) => {
  editorTemplate.value = template
  editorWorkflowId.value = undefined
  currentView.value = 'editor'
})

provide('app:backToDashboard', () => {
  currentView.value = 'dashboard'
  editorWorkflowId.value = undefined
  editorTemplate.value = null
})

function onOpenWorkflow(id?: string) {
  editorWorkflowId.value = id
  editorTemplate.value = null
  currentView.value = 'editor'
}

function onCreateFromTemplate(template: { id: string; name: string; nodes: unknown[]; edges: unknown[] }) {
  editorTemplate.value = template
  editorWorkflowId.value = undefined
  currentView.value = 'editor'
}

function onBackToDashboard() {
  currentView.value = 'dashboard'
  editorWorkflowId.value = undefined
  editorTemplate.value = null
}
</script>

<template>
  <div class="app-shell">
    <!-- Dashboard 首页 -->
    <Dashboard
      v-if="currentView === 'dashboard'"
      @open-workflow="onOpenWorkflow"
      @create-from-template="onCreateFromTemplate"
      @navigate="(path: string) => {}"
    />

    <!-- 编辑器 -->
    <LiteGraphEditor
      v-if="currentView === 'editor'"
      :initial-workflow-id="editorWorkflowId"
      :initial-template="editorTemplate"
      @back="onBackToDashboard"
    />

    <!-- 全局底栏 -->
    <div class="status-bar-wrapper">
      <StatusBar :page="currentView" />
    </div>

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
  height: 100vh;
  background: #0f1117;
  color: #e1e4e8;
  overflow: hidden;
}

.status-bar-wrapper {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  z-index: 10000;
}
</style>
