<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'
import { RadioGroupItem, type RadioGroupItemProps } from 'radix-vue'
import { Circle } from 'lucide-vue-next'

const props = defineProps<RadioGroupItemProps & { class?: HTMLAttributes['class'] }>()
</script>

<template>
  <RadioGroupItem
    v-bind="props"
    :class="cn(
      'aspect-square h-4 w-4 rounded-full border border-[var(--bg-brand)] text-[var(--text-brand)] ring-offset-[var(--bg-base-default)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--border-contrast)] focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50',
      props.class,
    )"
  >
    <Circle class="h-2.5 w-2.5 fill-current text-current" />
  </RadioGroupItem>
</template>
