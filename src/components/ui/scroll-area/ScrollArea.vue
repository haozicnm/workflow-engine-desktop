<script setup lang="ts">
import { cn } from '@/lib/utils'

defineProps<{
  class?: string
}>()
</script>

<template>
  <div
    :class="cn(
      'relative overflow-auto',
      $props.class,
    )"
    style="scrollbar-width: thin; scrollbar-color: var(--color-border) transparent;"
  >
    <slot />
  </div>
</template>
