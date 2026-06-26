<script setup lang="ts">
import { type HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'
import { AlertDialogAction, type AlertDialogActionProps } from 'radix-vue'

const props = defineProps<AlertDialogActionProps & { class?: HTMLAttributes['class'] }>()
</script>

<template>
  <AlertDialogAction v-bind="props" :class="cn('inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium ring-offset-[var(--bg-base-default)] transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--border-contrast)] focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 bg-[var(--bg-brand)] text-[var(--text-onbrand)] shadow-none hover:bg-[var(--bg-brand)]/90 h-10 px-4 py-2', props.class)">
    <slot />
  </AlertDialogAction>
</template>
