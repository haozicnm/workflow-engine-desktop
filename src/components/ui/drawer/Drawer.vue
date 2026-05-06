<script setup lang="ts">
import { ref, watch } from 'vue'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  open?: boolean
  direction?: 'left' | 'right' | 'top' | 'bottom'
}>(), {
  direction: 'right',
})

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const isOpen = ref(props.open ?? false)

watch(() => props.open, (val) => {
  if (val !== undefined) isOpen.value = val
})

watch(isOpen, (val) => {
  emit('update:open', val)
})

function close() {
  isOpen.value = false
}
</script>

<template>
  <slot name="trigger" :open="() => isOpen = true" />

  <Teleport to="body">
    <Transition name="drawer-overlay">
      <div
        v-if="isOpen"
        :class="cn('fixed inset-0 z-50 bg-black/60 backdrop-blur-sm')"
        @click="close"
      />
    </Transition>

    <Transition :name="`drawer-${direction}`">
      <div
        v-if="isOpen"
        :class="cn(
          'fixed z-50 bg-[#161b22] border-[#30363d] shadow-2xl flex flex-col',
          direction === 'right' && 'right-0 top-0 h-full w-[360px] border-l',
          direction === 'left' && 'left-0 top-0 h-full w-[360px] border-r',
          direction === 'top' && 'top-0 left-0 w-full h-auto border-b',
          direction === 'bottom' && 'bottom-0 left-0 w-full h-auto border-t',
        )"
      >
        <!-- Header -->
        <div :class="cn('flex items-center justify-between p-4 border-b border-[#30363d]')">
          <slot name="header" />
          <button
            :class="cn(
              'ml-auto rounded-md p-1.5',
              'text-[#8b949e] hover:text-white hover:bg-[#30363d]',
              'transition-colors focus:outline-none focus:ring-2 focus:ring-[#58a6ff]',
            )"
            @click="close"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M18 6 6 18" /><path d="m6 6 12 12" />
            </svg>
            <span class="sr-only">Close</span>
          </button>
        </div>

        <!-- Body -->
        <div :class="cn('flex-1 overflow-y-auto p-4')">
          <slot />
        </div>

        <!-- Footer -->
        <div v-if="$slots.footer" :class="cn('border-t border-[#30363d] p-4')">
          <slot name="footer" />
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.drawer-overlay-enter-active,
.drawer-overlay-leave-active {
  transition: opacity 0.3s ease;
}
.drawer-overlay-enter-from,
.drawer-overlay-leave-to {
  opacity: 0;
}

/* Right drawer */
.drawer-right-enter-active,
.drawer-right-leave-active {
  transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
}
.drawer-right-enter-from,
.drawer-right-leave-to {
  transform: translateX(100%);
}

/* Left drawer */
.drawer-left-enter-active,
.drawer-left-leave-active {
  transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
}
.drawer-left-enter-from,
.drawer-left-leave-to {
  transform: translateX(-100%);
}

/* Top drawer */
.drawer-top-enter-active,
.drawer-top-leave-active {
  transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
}
.drawer-top-enter-from,
.drawer-top-leave-to {
  transform: translateY(-100%);
}

/* Bottom drawer */
.drawer-bottom-enter-active,
.drawer-bottom-leave-active {
  transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
}
.drawer-bottom-enter-from,
.drawer-bottom-leave-to {
  transform: translateY(100%);
}
</style>
