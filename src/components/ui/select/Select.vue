<script setup lang="ts">
import {
  SelectRoot,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectViewport,
  SelectItem,
  SelectItemText,
  SelectItemIndicator,
  SelectIcon,
} from 'radix-vue'
import { cn } from '@/lib/utils'

export interface SelectOption {
  value: string
  label: string
  disabled?: boolean
}

defineProps<{
  modelValue?: string
  options?: SelectOption[]
  placeholder?: string
  disabled?: boolean
}>()

defineEmits<{
  'update:modelValue': [value: string]
}>()
</script>

<template>
  <SelectRoot :model-value="modelValue" @update:model-value="(v: string) => $emit('update:modelValue', v)" :disabled="disabled">
    <SelectTrigger
      :class="cn(
        'flex h-9 w-full items-center justify-between gap-2',
        'rounded-md border border-border bg-background px-3 py-2 text-sm',
        'text-foreground placeholder:text-muted-foreground',
        'hover:border-ring hover:bg-popover',
        'focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 focus:ring-offset-background',
        'disabled:cursor-not-allowed disabled:opacity-50',
        '[&>span]:line-clamp-1',
      )"
    >
      <SelectValue :placeholder="placeholder ?? 'Select...'" />
      <SelectIcon as-child>
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-muted-foreground">
          <path d="m6 9 6 6 6-6" />
        </svg>
      </SelectIcon>
    </SelectTrigger>

    <SelectContent
      position="popper"
      :side-offset="4"
      :class="cn(
        'relative z-50 max-h-96 min-w-[8rem] overflow-hidden',
        'rounded-md border border-border bg-popover shadow-xl',
        'data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95',
        'data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95',
        'data-[side=bottom]:slide-in-from-top-2 data-[side=top]:slide-in-from-bottom-2',
      )"
    >
      <SelectViewport :class="cn('p-1')">
        <SelectItem
          v-for="opt in options"
          :key="opt.value"
          :value="opt.value"
          :disabled="opt.disabled"
          :class="cn(
            'relative flex w-full cursor-pointer select-none items-center rounded-sm py-1.5 pl-8 pr-2 text-sm text-foreground',
            'outline-none transition-colors',
            'hover:bg-accent hover:text-foreground',
            'focus:bg-accent focus:text-foreground',
            'data-[disabled]:pointer-events-none data-[disabled]:opacity-50',
          )"
        >
          <span :class="cn('absolute left-2 flex h-3.5 w-3.5 items-center justify-center')">
            <SelectItemIndicator>
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M20 6 9 17l-5-5" />
              </svg>
            </SelectItemIndicator>
          </span>
          <SelectItemText>{{ opt.label }}</SelectItemText>
        </SelectItem>
      </SelectViewport>
    </SelectContent>
  </SelectRoot>
</template>
