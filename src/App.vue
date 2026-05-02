<script setup lang="ts">
import { RouterView } from 'vue-router'
import Toast from './components/Toast.vue'
import StatusBar from './components/StatusBar.vue'
import { useToast } from './composables/useToast'

const { toasts, remove } = useToast()
</script>

<template>
  <div class="app-shell">
    <RouterView v-slot="{ Component }">
      <KeepAlive :include="['LiteGraphEditor']">
        <component :is="Component" />
      </KeepAlive>
    </RouterView>
    <StatusBar />

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
</style>
