<script setup lang="ts">
import { cn } from '@/lib/utils'
</script>

<template>
  <div
    :class="cn(
      'rounded-xl border border-border bg-card text-card-foreground shadow-sm',
      $attrs.class,
    )"
  >
    <slot />
  </div>
</template>
