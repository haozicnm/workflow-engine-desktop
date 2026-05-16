<script setup lang="ts">
import { cn } from '@/lib/utils'

defineProps<{
  color?: string
}>()
</script>

<template>
  <div
    :class="cn(
      'rounded-xl border bg-card text-card-foreground shadow-sm overflow-hidden',
      color ? 'border-l-0' : 'border-border',
      !color && 'border-border',
      $attrs.class,
    )"
  >
    <div v-if="color" class="flex">
      <!-- 左侧色条 -->
      <div class="w-[3px] shrink-0" :style="{ background: color }" />
      <div class="flex-1 min-w-0">
        <slot />
      </div>
    </div>
    <slot v-if="!color" />
  </div>
</template>
