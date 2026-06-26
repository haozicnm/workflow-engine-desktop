<script setup lang="ts">
import { cn } from '@/lib/utils'
</script>

<template>
  <div :class="cn('mb-4', $attrs.class as string)" v-bind="$attrs">
    <slot />
  </div>
</template>
