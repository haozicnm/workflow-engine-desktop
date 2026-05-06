<script setup lang="ts">
import { cn } from '@/lib/utils'

interface Props {
  class?: string
}
</script>

<template>
  <div
    :class="cn(
      'relative overflow-auto',
      $props.class,
    )"
    style="scrollbar-width: thin; scrollbar-color: #30363d transparent;"
  >
    <slot />
  </div>
</template>
