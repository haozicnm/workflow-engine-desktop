<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'
import { DropdownMenuItem, type DropdownMenuItemProps } from 'radix-vue'

const props = defineProps<DropdownMenuItemProps & { class?: HTMLAttributes['class']; inset?: boolean }>()
</script>

<template>
  <DropdownMenuItem v-bind="props" :class="cn('relative flex cursor-default select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none transition-colors focus:bg-[var(--bg-overlay-l2)] focus:text-[var(--text-default)] data-[disabled]:pointer-events-none data-[disabled]:opacity-50', props.inset && 'pl-8', props.class)">
    <slot />
  </DropdownMenuItem>
</template>
