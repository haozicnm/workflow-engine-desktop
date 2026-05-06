<script setup lang="ts">
import { cn } from '@/lib/utils'
</script>

<template>
  <main :class="cn('flex-1 flex flex-col overflow-hidden min-h-0', $attrs.class as string)" v-bind="$attrs">
    <slot />
  </main>
</template>
