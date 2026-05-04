<script setup lang="ts">
// App.vue — v4.0: Dashboard 为默认入口
import { ref, provide } from 'vue'
import LiteGraphEditor from './pages/LiteGraphEditor.vue'
import Dashboard from './pages/Dashboard.vue'
import Settings from './pages/Settings.vue'
import RunHistory from './pages/RunHistory.vue'
import Toast from './components/Toast.vue'
import { useToast } from './composables/useToast'

const { toasts, remove } = useToast()

type AppView = 'dashboard' | 'editor' | 'settings' | 'history'

const currentView = ref<AppView>('dashboard')
const editorWorkflowId = ref<string | undefined>(undefined)

provide('app:openEditor', (id?: string) => {
  editorWorkflowId.value = id
  currentView.value = 'editor'
})

provide('app:backToDashboard', () => {
  currentView.value = 'dashboard'
  editorWorkflowId.value = undefined
})

function onOpenWorkflow(id?: string) {
  editorWorkflowId.value = id
  currentView.value = 'editor'
}

function onOpenSettings() {
  currentView.value = 'settings'
}

function onOpenHistory() {
  currentView.value = 'history'
}

function onBackToDashboard() {
  currentView.value = 'dashboard'
  editorWorkflowId.value = undefined
}
</script>

<template>
  <div class="app-shell">
    <Transition name="page-fade">
      <Dashboard
        v-if="currentView === 'dashboard'"
        @open-workflow="onOpenWorkflow"
        @open-settings="onOpenSettings"
        @open-history="onOpenHistory"
      />
    </Transition>

    <Transition name="page-fade">
      <LiteGraphEditor
        v-if="currentView === 'editor'"
        :initial-workflow-id="editorWorkflowId"
        @back="onBackToDashboard"
      />
    </Transition>

    <Transition name="page-fade">
      <Settings
        v-if="currentView === 'settings'"
        @back="onBackToDashboard"
      />
    </Transition>

    <Transition name="page-fade">
      <RunHistory
        v-if="currentView === 'history'"
        @back="onBackToDashboard"
      />
    </Transition>

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
  overflow-y: auto;
}

.page-fade-enter-active,
.page-fade-leave-active {
  transition: opacity 0.15s ease;
}
.page-fade-enter-from,
.page-fade-leave-to {
  opacity: 0;
}
</style>
