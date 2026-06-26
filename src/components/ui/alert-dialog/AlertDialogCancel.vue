<script setup lang="ts">
import { type HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'
import { AlertDialogCancel, type AlertDialogCancelProps } from 'radix-vue'

const props = defineProps<AlertDialogCancelProps & { class?: HTMLAttributes['class'] }>()
</script>

<template>
  <AlertDialogCancel v-bind="props" :class="cn('inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium ring-offset-[var(--bg-base-default)] transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--border-contrast)] focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 border border-[var(--border-neutral-l1)] bg-transparent text-[var(--text-default)] shadow-none hover:bg-[var(--bg-overlay-l1)] hover:text-[var(--text-default)] h-10 px-4 py-2 mt-2 sm:mt-0', props.class)">
    <slot />
  </AlertDialogCancel>
</template>
