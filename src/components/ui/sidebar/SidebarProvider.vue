<script setup lang="ts">
import { ref, provide, toRef } from 'vue'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  defaultOpen?: boolean
  open?: boolean
}>(), {
  defaultOpen: true,
})

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const openState = ref(props.defaultOpen)

function toggle() {
  const next = !openState.value
  openState.value = next
  emit('update:open', next)
}

provide('sidebar', {
  open: openState,
  toggle,
})
</script>

<template>
  <div
    :class="cn('group/sidebar-wrapper flex min-h-svh w-full', $attrs.class as string)"
    v-bind="$attrs"
  >
    <slot />
  </div>
</template>
