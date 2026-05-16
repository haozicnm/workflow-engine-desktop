<script setup lang="ts">
import { computed } from 'vue'
import { DialogRoot } from 'radix-vue'

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
