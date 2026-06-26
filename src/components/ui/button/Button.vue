<script setup lang="ts">
import { computed } from 'vue'
import { cn } from '@/lib/utils'

interface Props {
  variant?: 'default' | 'destructive' | 'outline' | 'secondary' | 'ghost' | 'link'
  size?: 'default' | 'sm' | 'lg' | 'icon'
  as?: string
  disabled?: boolean
  class?: string
}

const props = withDefaults(defineProps<Props>(), {
  variant: 'default',
  size: 'default',
  as: 'button',
  disabled: false,
})

const variantClasses: Record<string, string> = {
  default:
    'bg-[var(--bg-brand)] text-[var(--text-onbrand)] shadow-none hover:bg-[var(--bg-brand)]/90',
  destructive:
    'bg-[var(--status-error-default)] text-white shadow-none hover:bg-[var(--status-error-default)]/90',
  outline:
    'border border-[var(--border-neutral-l1)] bg-transparent text-[var(--text-default)] shadow-none hover:bg-[var(--bg-overlay-l1)] hover:text-[var(--text-default)]',
  secondary:
    'bg-[var(--bg-overlay-l1)] text-[var(--text-secondary)] shadow-none hover:bg-[var(--bg-overlay-l1)]/80',
  ghost:
    'text-[var(--text-default)] hover:bg-[var(--bg-overlay-l1)] hover:text-[var(--text-default)]',
  link:
    'text-[var(--text-brand)] underline-offset-4 hover:underline',
}

const sizeClasses: Record<string, string> = {
  default: 'h-9 px-4 py-2 text-sm',
  sm: 'h-8 rounded-md px-3 text-xs',
  lg: 'h-10 rounded-md px-6 text-sm',
  icon: 'h-9 w-9',
}

const classes = computed(() =>
  cn(
    'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-all',
    'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--border-contrast)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--bg-base-default)]',
    'disabled:pointer-events-none disabled:opacity-50',
    'active:scale-[0.98]',
    variantClasses[props.variant],
    sizeClasses[props.size],
    props.class,
  )
)
</script>

<template>
  <component
    :is="as"
    :class="classes"
    :disabled="disabled"
  >
    <slot />
  </component>
</template>
