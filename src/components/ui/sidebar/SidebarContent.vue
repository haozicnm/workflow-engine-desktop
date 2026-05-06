<script setup lang="ts">
import { cn } from '@/lib/utils'
</script>

<template>
  <div :class="cn('flex-1 overflow-y-auto px-3 py-2', $attrs.class as string)" v-bind="$attrs">
    <slot />
  </div>
</template>
