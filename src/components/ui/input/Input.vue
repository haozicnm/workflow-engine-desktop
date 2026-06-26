<script setup lang="ts">
import { cn } from '@/lib/utils'

interface Props {
  type?: string
  placeholder?: string
  disabled?: boolean
  modelValue?: string | number
  class?: string
}

withDefaults(defineProps<Props>(), {
  type: 'text',
  disabled: false,
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

function onInput(event: Event) {
  const target = event.target as HTMLInputElement
  emit('update:modelValue', target.value)
}
</script>

<template>
  <input
    :type="type"
    :placeholder="placeholder"
    :disabled="disabled"
    :value="modelValue"
    :class="
      cn(
        'flex h-9 w-full rounded-md border border-[var(--border-neutral-l1)] bg-[var(--bg-base-default)] px-3 py-1 text-sm text-[var(--text-default)] shadow-none transition-colors',
        'placeholder:text-[var(--text-tertiary)]',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--border-contrast)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--bg-base-default)]',
        'disabled:cursor-not-allowed disabled:opacity-50',
        $props.class,
      )
    "
    @input="onInput"
  />
</template>
