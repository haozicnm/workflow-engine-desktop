<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'
import { ProgressRoot, type ProgressRootProps } from 'radix-vue'
import { computed } from 'vue'
import { ProgressIndicator } from 'radix-vue'

const props = withDefaults(defineProps<ProgressRootProps & { class?: HTMLAttributes['class'] }>(), {
  modelValue: 0,
  max: 100,
})

const percentage = computed(() => Math.round((props.modelValue ?? 0) / (props.max ?? 100) * 100))
</script>

<template>
  <ProgressRoot
    v-bind="props"
    :class="cn(
      'relative h-4 w-full overflow-hidden rounded-full bg-[var(--bg-overlay-l1)]',
      props.class,
    )"
  >
    <ProgressIndicator
      class="h-full w-full flex-1 bg-[var(--bg-brand)] transition-all"
      :style="`transform: translateX(-${100 - percentage}%)`"
    />
  </ProgressRoot>
</template>
