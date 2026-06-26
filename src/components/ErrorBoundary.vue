<script setup lang="ts">
import { ref, onErrorCaptured, onMounted, onUnmounted } from 'vue'
import { AlertTriangle } from 'lucide-vue-next'
import Button from './ui/button/Button.vue'

const error = ref<string | null>(null)
const errorInfo = ref('')

// 捕获 Vue 组件错误
onErrorCaptured((err, _instance, info) => {
  error.value = err.message || String(err)
  errorInfo.value = info
  console.error('[ErrorBoundary]', err)
  return false
})

// 捕获未处理的异步错误和全局错误
function onGlobalError(event: ErrorEvent) {
  error.value = event.message || '未知错误'
  errorInfo.value = 'uncaught error'
  console.error('[GlobalError]', event.error)
}
function onUnhandledRejection(event: PromiseRejectionEvent) {
  error.value = String(event.reason) || '未处理的 Promise 拒绝'
  errorInfo.value = 'unhandledrejection'
  console.error('[UnhandledRejection]', event.reason)
}
onMounted(() => {
  window.addEventListener('error', onGlobalError)
  window.addEventListener('unhandledrejection', onUnhandledRejection)
})
onUnmounted(() => {
  window.removeEventListener('error', onGlobalError)
  window.removeEventListener('unhandledrejection', onUnhandledRejection)
})

onErrorCaptured((err, instance, info) => {
  error.value = err.message || String(err)
  errorInfo.value = info
  console.error('[ErrorBoundary]', err, instance, info)
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
  <div v-else class="min-h-[100dvh] flex items-center justify-center bg-[var(--bg-base-default)] text-[var(--text-default)] font-sans">
    <div class="text-center p-10 bg-[var(--bg-base-secondary)] border border-[var(--border-neutral-l1)] rounded-xl max-w-[480px]">
      <AlertTriangle class="w-12 h-12 text-[var(--status-warning-default)] mx-auto mb-4" />
      <h2 class="text-lg font-semibold mb-3">出现了一个错误</h2>
      <p class="text-sm font-mono bg-[var(--bg-base-default)] text-[var(--status-error-default)] p-3 rounded-md break-all">{{ error }}</p>
      <p v-if="errorInfo" class="text-xs text-[var(--text-tertiary)] mt-2">组件: {{ errorInfo }}</p>
      <div class="mt-6 flex gap-3 justify-center">
        <Button variant="default" size="sm" class="bg-[var(--status-success-default)] text-white" @click="reload">重新加载</Button>
        <Button variant="outline" size="sm" @click="dismiss">忽略</Button>
      </div>
    </div>
  </div>
</template>
