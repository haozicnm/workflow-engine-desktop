<script setup lang="ts">
import { DialogDescription } from 'radix-vue'
import { cn } from '@/lib/utils'
</script>

<template>
  <DialogDescription
    :class="cn('text-sm text-[var(--text-tertiary)]', $attrs.class)"
  >
    <slot />
  </DialogDescription>
</template>
