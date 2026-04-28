<script setup lang="ts">
import { ref, onMounted } from 'vue'

const props = defineProps<{
  message: string
  type?: 'success' | 'error' | 'info'
  duration?: number
}>()

const emit = defineEmits<{ close: [] }>()
const visible = ref(false)

onMounted(() => {
  requestAnimationFrame(() => { visible.value = true })
  setTimeout(() => {
    visible.value = false
    setTimeout(() => emit('close'), 300)
  }, props.duration || 3000)
})
</script>

<template>
  <div class="toast" :class="[type || 'info', { show: visible }]">
    <span class="toast-icon">
      {{ type === 'success' ? '✅' : type === 'error' ? '❌' : 'ℹ️' }}
    </span>
    <span class="toast-msg">{{ message }}</span>
  </div>
</template>

<style scoped>
.toast {
  position: fixed; top: 16px; right: 16px;
  display: flex; align-items: center; gap: 8px;
  padding: 10px 16px; border-radius: 8px;
  font-size: 13px; font-weight: 500; z-index: 200;
  transform: translateX(120%); transition: transform 0.3s ease;
  box-shadow: 0 4px 12px rgba(0,0,0,0.3);
  max-width: 400px;
}
.toast.show { transform: translateX(0); }
.toast.success { background: #238636; color: #fff; border: 1px solid #2ea043; }
.toast.error { background: #da3633; color: #fff; border: 1px solid #f85149; }
.toast.info { background: #1f6feb; color: #fff; border: 1px solid #388bfd; }
.toast-icon { font-size: 15px; flex-shrink: 0; }
.toast-msg { flex: 1; }
</style>
