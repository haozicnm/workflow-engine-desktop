<script setup lang="ts">
import { cn } from '@/lib/utils'
</script>

<template>
  <div :class="cn('flex flex-col gap-2 p-4', $attrs.class as string)" v-bind="$attrs">
    <slot />
  </div>
</template>
