<script setup lang="ts">
import { DialogDescription } from 'radix-vue'
import { cn } from '@/lib/utils'
</script>

<template>
  <DialogDescription
    :class="cn('text-sm text-[#8b949e]', $attrs.class)"
  >
    <slot />
  </DialogDescription>
</template>
