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
    role="switch"
    :aria-checked="modelValue"
    :disabled="disabled"
    :class="cn(
      'peer inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors',
      'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--border-contrast)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--bg-base-default)]',
      'disabled:cursor-not-allowed disabled:opacity-50',
      modelValue ? 'bg-[var(--bg-brand)]' : 'bg-input',
      props.class,
    )"
    @click="toggle"
  >
    <span
      :class="cn(
        'pointer-events-none block h-4 w-4 rounded-full bg-[var(--bg-base-default)] shadow-[0_12px_32px_rgba(0,0,0,0.12)] ring-0 transition-transform',
        modelValue ? 'translate-x-4' : 'translate-x-0',
      )"
    />
  </button>
</template>
