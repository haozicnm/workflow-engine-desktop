<script setup lang="ts">
import { cn } from '@/lib/utils'
</script>

<template>
  <div :class="cn('p-4 border-t border-border', $attrs.class as string)" v-bind="$attrs">
    <slot />
  </div>
</template>
