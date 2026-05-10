<script setup lang="ts">
import { DialogTitle } from 'radix-vue'
import { cn } from '@/lib/utils'
</script>

<template>
  <DialogTitle
    :class="cn('text-lg font-semibold leading-none tracking-tight text-foreground', $attrs.class)"
  >
    <slot />
  </DialogTitle>
</template>
