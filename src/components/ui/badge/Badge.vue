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
  default: 'bg-primary/20 text-primary border-primary/30',
  secondary: 'bg-secondary text-secondary-foreground border-border',
  destructive: 'bg-destructive/15 text-destructive border-destructive/30',
  outline: 'text-foreground border-border',
  success: 'bg-success/20 text-success border-success/30',
  warning: 'bg-warning/20 text-warning border-warning/30',
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
