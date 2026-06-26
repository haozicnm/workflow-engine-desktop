<script setup lang="ts">
import { TabsTrigger } from 'radix-vue'
import { cn } from '@/lib/utils'

defineProps<{
  value: string
  disabled?: boolean
}>()
</script>

<template>
  <TabsTrigger
    :value="value"
    :disabled="disabled"
    :class="cn(
      'inline-flex items-center justify-center whitespace-nowrap rounded-md px-3 py-1 text-sm font-medium',
      'ring-offset-[var(--bg-base-default)] transition-all',
      'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--border-contrast)] focus-visible:ring-offset-2',
      'disabled:pointer-events-none disabled:opacity-50',
      'data-[state=active]:bg-[var(--bg-base-default)] data-[state=active]:text-[var(--text-default)] data-[state=active]:shadow',
    )"
  >
    <slot />
  </TabsTrigger>
</template>
