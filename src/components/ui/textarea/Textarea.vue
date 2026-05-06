<script setup lang="ts">
import { cn } from '@/lib/utils'

interface Props {
  modelValue?: string
  placeholder?: string
  disabled?: boolean
  rows?: number
  class?: string
}

withDefaults(defineProps<Props>(), {
  rows: 4,
  disabled: false,
})

const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

function onInput(event: Event) {
  emit('update:modelValue', (event.target as HTMLTextAreaElement).value)
}
</script>

<template>
  <textarea
    :placeholder="placeholder"
    :disabled="disabled"
    :rows="rows"
    :value="modelValue"
    :class="cn(
      'flex min-h-[60px] w-full rounded-md border border-border bg-background px-3 py-2 text-sm text-foreground shadow-sm transition-colors',
      'placeholder:text-muted-foreground',
      'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background',
      'disabled:cursor-not-allowed disabled:opacity-50',
      'font-mono resize-y',
      $props.class,
    )"
    @input="onInput"
  />
</template>
