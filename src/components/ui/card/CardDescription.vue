<script setup lang="ts">
import { cn } from '@/lib/utils'
</script>

<template>
  <p :class="cn('text-sm text-[var(--text-tertiary)]', $attrs.class)">
    <slot />
  </p>
</template>
