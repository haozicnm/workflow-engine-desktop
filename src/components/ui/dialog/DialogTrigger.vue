<script setup lang="ts">
import { DialogTrigger } from 'radix-vue'
import { cn } from '@/lib/utils'

const props = defineProps<{
  asChild?: boolean
}>()
</script>

<template>
  <DialogTrigger :as-child="asChild">
    <slot />
  </DialogTrigger>
</template>
