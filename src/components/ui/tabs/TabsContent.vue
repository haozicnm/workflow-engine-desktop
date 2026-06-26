<script setup lang="ts">
import { TabsContent } from 'radix-vue'
import { cn } from '@/lib/utils'

defineProps<{
  value: string
}>()
</script>

<template>
  <TabsContent
    :value="value"
    :class="cn(
      'mt-2 ring-offset-[var(--bg-base-default)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--border-contrast)] focus-visible:ring-offset-2 flex flex-col min-h-0',
      $attrs.class as string,
    )"
  >
    <slot />
  </TabsContent>
</template>
