<script setup lang="ts">
import { inject, type Ref } from 'vue'
import { cn } from '@/lib/utils'

const sidebar = inject<{ open: Ref<boolean>; toggle: () => void }>('sidebar')
</script>

<template>
  <aside
    aria-label="主导航"
    :class="cn(
      'relative flex flex-col bg-[var(--bg-base-secondary)] border-r border-[var(--border-neutral-l1)] transition-[width] duration-200 ease-in-out shrink-0 overflow-hidden',
      sidebar?.open.value ? 'w-[280px]' : 'w-[52px]',
      $attrs.class as string,
    )"
    v-bind="$attrs"
  >
    <slot />
  </aside>
</template>
