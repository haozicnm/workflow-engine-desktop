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

const bgClass = {
  success: 'bg-[#238636] border-[#2ea043]',
  error: 'bg-danger border-danger',
  info: 'bg-[#1f6feb] border-[#388bfd]',
}

const iconMap = {
  success: '✅',
  error: '❌',
  info: 'ℹ️',
}
</script>

<template>
  <div
    :class="[
      'fixed top-4 right-4 z-[200] flex items-center gap-2 px-4 py-2.5 rounded-lg border text-sm font-medium text-white shadow-lg max-w-[400px]',
      'transition-transform duration-300 ease-in-out',
      visible ? 'translate-x-0' : 'translate-x-[120%]',
      bgClass[type || 'info'],
    ]"
  >
    <span class="text-base shrink-0">{{ iconMap[type || 'info'] }}</span>
    <span class="flex-1">{{ message }}</span>
  </div>
</template>
