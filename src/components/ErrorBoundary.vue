<script setup lang="ts">
import { ref, onErrorCaptured } from 'vue'
import Button from './ui/button/Button.vue'

const error = ref<string | null>(null)
const errorInfo = ref('')

onErrorCaptured((err, instance, info) => {
  error.value = err.message || String(err)
  errorInfo.value = info
  console.error('[ErrorBoundary]', err, info)
  return false // prevent propagation
})

function reload() {
  error.value = null
  errorInfo.value = ''
  window.location.reload()
}

function dismiss() {
  error.value = null
  errorInfo.value = ''
}
</script>

<template>
  <slot v-if="!error" />
  <div v-else class="h-screen flex items-center justify-center bg-background text-foreground font-sans">
    <div class="text-center p-10 bg-card border border-border rounded-xl max-w-[480px]">
      <div class="text-5xl mb-4">⚠️</div>
      <h2 class="text-lg font-semibold mb-3">出现了一个错误</h2>
      <p class="text-sm font-mono bg-background text-destructive p-3 rounded-md break-all">{{ error }}</p>
      <p v-if="errorInfo" class="text-xs text-muted-foreground mt-2">组件: {{ errorInfo }}</p>
      <div class="mt-6 flex gap-3 justify-center">
        <Button variant="default" size="sm" class="bg-success text-success-foreground" @click="reload">重新加载</Button>
        <Button variant="outline" size="sm" @click="dismiss">忽略</Button>
      </div>
    </div>
  </div>
</template>
