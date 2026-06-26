<script setup lang="ts">
import { cn } from '@/lib/utils'

interface Props {
  modelValue?: boolean
  disabled?: boolean
  class?: string
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: false,
  disabled: false,
})

const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>()

function toggle() {
  if (!props.disabled) {
    emit('update:modelValue', !props.modelValue)
  }
}
</script>

<template>
  <button
    type="button"
    role="checkbox"
    :aria-checked="modelValue"
    :disabled="disabled"
    :class="cn(
      'peer h-4 w-4 shrink-0 rounded-sm border border-[var(--border-neutral-l1)] shadow-none transition-colors',
      'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--border-contrast)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--bg-base-default)]',
      'disabled:cursor-not-allowed disabled:opacity-50',
      modelValue ? 'bg-[var(--bg-brand)] border-[var(--bg-brand)] text-[var(--text-onbrand)]' : 'bg-[var(--bg-base-default)]',
      props.class,
    )"
    @click="toggle"
  >
    <svg v-if="modelValue" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" class="mx-auto">
      <path d="M20 6 9 17l-5-5" />
    </svg>
  </button>
</template>
