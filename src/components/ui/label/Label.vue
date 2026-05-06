<script setup lang="ts">
import { cn } from '@/lib/utils'

interface Props {
  htmlFor?: string
  class?: string
}

defineProps<Props>()
</script>

<template>
  <label
    :for="htmlFor"
    :class="
      cn(
        'text-sm font-medium leading-none text-foreground peer-disabled:cursor-not-allowed peer-disabled:opacity-70',
        $props.class,
      )
    "
  >
    <slot />
  </label>
</template>
