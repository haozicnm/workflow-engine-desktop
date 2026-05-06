<script setup lang="ts">
import { DialogClose } from 'radix-vue'
import { cn } from '@/lib/utils'

defineProps<{
  asChild?: boolean
}>()
</script>

<template>
  <DialogClose :as-child="asChild">
    <slot />
  </DialogClose>
</template>
