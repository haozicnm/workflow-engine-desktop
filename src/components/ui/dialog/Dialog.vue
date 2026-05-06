<script setup lang="ts">
import { computed } from 'vue'
import {
  DialogRoot,
  DialogTrigger,
  DialogPortal,
  DialogOverlay,
  DialogContent,
  DialogClose,
  DialogTitle,
  DialogDescription,
} from 'radix-vue'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  open?: boolean
  modal?: boolean
}>(), {
  modal: true,
})

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const isOpen = computed({
  get: () => props.open ?? false,
  set: (val) => emit('update:open', val),
})
</script>

<template>
  <DialogRoot v-model:open="isOpen" :modal="modal">
    <slot />
  </DialogRoot>
</template>
