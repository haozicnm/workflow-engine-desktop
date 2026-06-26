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
      'flex min-h-[60px] w-full rounded-md border border-[var(--border-neutral-l1)] bg-[var(--bg-base-default)] px-3 py-2 text-sm text-[var(--text-default)] shadow-none transition-colors',
      'placeholder:text-[var(--text-tertiary)]',
      'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--border-contrast)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--bg-base-default)]',
      'disabled:cursor-not-allowed disabled:opacity-50',
      'font-mono resize-y',
      $props.class,
    )"
    @input="onInput"
  />
</template>
