<script setup lang="ts">
import LiteGraphEditor from './pages/LiteGraphEditor.vue'
import Toast from './components/Toast.vue'
import StatusBar from './components/StatusBar.vue'
import { useToast } from './composables/useToast'

const { toasts, remove } = useToast()
</script>

<template>
  <div class="app-shell">
    <LiteGraphEditor />
    <div class="status-bar-wrapper">
      <StatusBar />
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
