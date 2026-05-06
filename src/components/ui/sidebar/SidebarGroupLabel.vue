<script setup lang="ts">
import { inject, type Ref } from 'vue'
import { cn } from '@/lib/utils'

defineProps<{
  label: string
}>()

const sidebar = inject<{ open: Ref<boolean>; toggle: () => void }>('sidebar')
</script>

<template>
  <span
    v-if="sidebar?.open.value"
    :class="cn(
      'text-[11px] font-semibold text-muted-foreground uppercase tracking-wider px-2 mb-1.5 block',
      $attrs.class as string,
    )"
    v-bind="$attrs"
  >
    {{ label }}
  </span>
  <div v-else class="mx-2 mb-2 h-px bg-border" />
</template>
