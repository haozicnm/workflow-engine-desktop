<script setup lang="ts">
import { cn } from '@/lib/utils'
</script>

<template>
  <div :class="cn('p-4 border-t border-[var(--border-neutral-l1)]', $attrs.class as string)" v-bind="$attrs">
    <slot />
  </div>
</template>
