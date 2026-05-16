<script setup lang="ts">
import { DialogClose } from 'radix-vue'

defineProps<{
  asChild?: boolean
}>()
</script>

<template>
  <DialogClose :as-child="asChild">
    <slot />
  </DialogClose>
</template>
