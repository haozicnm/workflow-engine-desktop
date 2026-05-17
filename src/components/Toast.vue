<script setup lang="ts">
import { ref, onMounted } from 'vue'
import ActionIcon from './ActionIcon.vue'

const props = withDefaults(defineProps<{
  message: string
  type?: 'success' | 'error' | 'info'
  duration?: number
  index?: number
}>(), {
  type: 'info',
  duration: 3000,
  index: 0,
})

const emit = defineEmits<{ close: [] }>()
const visible = ref(false)
let timer: ReturnType<typeof setTimeout> | null = null
let remaining = 0

function startTimer(ms: number) {
  remaining = ms
  timer = setTimeout(() => {
    visible.value = false
    setTimeout(() => emit('close'), 300)
  }, ms)
}

function pauseDismiss() {
  if (timer) { clearTimeout(timer); remaining = Math.max(0, remaining - 300) }
}

function resumeDismiss() {
  if (timer) { clearTimeout(timer); startTimer(remaining) }
}

function dismissNow() {
  if (timer) clearTimeout(timer)
  visible.value = false
  setTimeout(() => emit('close'), 300)
}

onMounted(() => {
  requestAnimationFrame(() => { visible.value = true })
  startTimer(props.duration || 3000)
})

const bgClass = {
  success: 'bg-success border-success/80',
  error: 'bg-danger border-danger/80',
  info: 'bg-primary border-primary/80',
}

const iconMap = {
  success: 'CheckCircle',
  error: 'XCircle',
  info: 'Info',
}
</script>

<template>
  <div
    role="alert"
    aria-live="polite"
    :class="[
      'fixed z-[200] flex items-center gap-2 px-4 py-2.5 rounded-lg border text-sm font-medium text-primary-foreground shadow-lg max-w-[400px]',
      'transition-all duration-300 ease-in-out',
      visible ? 'opacity-100 translate-x-0' : 'opacity-0 translate-x-[120%]',
      bgClass[type || 'info'],
    ]"
    :style="{ top: `${16 + (index || 0) * 52}px`, right: '16px' }"
    @mouseenter="pauseDismiss"
    @mouseleave="resumeDismiss"
  >
    <ActionIcon :name="iconMap[type || 'info']" cls="w-4 h-4 shrink-0" />
    <span class="flex-1">{{ message }}</span>
    <button
      class="shrink-0 w-5 h-5 flex items-center justify-center rounded hover:bg-white/20 transition-colors"
      @click="dismissNow"
      aria-label="关闭通知"
    >✕</button>
  </div>
</template>
