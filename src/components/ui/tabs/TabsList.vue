<script setup lang="ts">
import { TabsList } from 'radix-vue'
import { cn } from '@/lib/utils'
</script>

<template>
  <TabsList
    :class="cn(
      'inline-flex h-9 items-center justify-center rounded-lg bg-[var(--bg-overlay-l1)] p-1 text-[var(--text-tertiary)]',
      $attrs.class as string,
    )"
  >
    <slot />
  </TabsList>
</template>
