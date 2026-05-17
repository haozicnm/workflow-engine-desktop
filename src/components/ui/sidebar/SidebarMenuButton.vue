<script setup lang="ts">
import { inject, type Ref } from 'vue'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  active?: boolean
  tooltip?: string
}>(), {
  active: false,
})

const sidebar = inject<{ open: Ref<boolean>; toggle: () => void }>('sidebar')
</script>

<template>
  <button
    :title="!sidebar?.open.value && tooltip ? tooltip : undefined"
    :aria-current="props.active ? 'page' : undefined"
    :class="cn(
      'flex items-center gap-2 w-full rounded-md px-2 py-1.5 text-sm transition-colors cursor-pointer',
      props.active
        ? 'bg-primary/10 text-primary font-medium'
        : 'hover:bg-secondary text-foreground',
      !sidebar?.open.value ? 'justify-center' : '',
      $attrs.class as string,
    )"
    v-bind="$attrs"
  >
    <!-- Icon slot - always visible -->
    <slot name="icon" />
    <!-- Content - hidden when collapsed -->
    <span v-if="sidebar?.open.value" class="flex-1 text-left truncate">
      <slot />
    </span>
  </button>
</template>
