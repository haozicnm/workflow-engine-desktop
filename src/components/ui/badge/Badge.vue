<script setup lang="ts">
import { cn } from '@/lib/utils'

interface Props {
  variant?: 'default' | 'secondary' | 'destructive' | 'outline' | 'success' | 'warning'
  class?: string
}

const props = withDefaults(defineProps<Props>(), {
  variant: 'default',
})

const variantClasses: Record<string, string> = {
  default: 'bg-[var(--bg-brand)]/20 text-[var(--text-brand)] border-[var(--bg-brand)]/30',
  secondary: 'bg-[var(--bg-overlay-l1)] text-[var(--text-secondary)] border-[var(--border-neutral-l1)]',
  destructive: 'bg-[var(--status-error-default)]/15 text-[var(--status-error-default)] border-[var(--status-error-default)]/30',
  outline: 'text-[var(--text-default)] border-[var(--border-neutral-l1)]',
  success: 'bg-[var(--status-success-default)]/20 text-[var(--status-success-default)] border-[var(--status-success-default)]/30',
  warning: 'bg-[var(--status-warning-default)]/20 text-[var(--status-warning-default)] border-[var(--status-warning-default)]/30',
}
</script>

<template>
  <div
    :class="cn(
      'inline-flex items-center gap-1 rounded-md border px-2 py-0.5 text-xs font-semibold transition-colors',
      variantClasses[variant],
      props.class,
    )"
  >
    <slot />
  </div>
</template>
