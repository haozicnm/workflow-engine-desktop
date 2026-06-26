<script setup lang="ts">
import { cn } from '@/lib/utils'
</script>

<template>
  <div :class="cn('', $attrs.class as string)" v-bind="$attrs">
    <slot />
  </div>
</template>
