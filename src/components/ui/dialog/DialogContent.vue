<script setup lang="ts">
import { DialogPortal, DialogOverlay, DialogContent, DialogClose } from 'radix-vue'
import { cn } from '@/lib/utils'

defineProps<{
  class?: string
}>()
</script>

<template>
  <DialogPortal>
    <DialogOverlay
      :class="cn(
        'fixed inset-0 z-50 bg-[var(--bg-overlay-l4)] backdrop-blur-sm',
        'data-[state=open]:animate-in data-[state=open]:fade-in-0',
        'data-[state=closed]:animate-out data-[state=closed]:fade-out-0',
      )"
    />
    <DialogContent
      :class="cn(
        'fixed left-1/2 top-1/2 z-50 -translate-x-1/2 -translate-y-1/2',
        'w-full max-w-lg p-6',
        'bg-[var(--bg-base-default)] border border-[var(--border-neutral-l1)] rounded-xl shadow-[0_24px_64px_rgba(0,0,0,0.14)]',
        'data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95 data-[state=open]:slide-in-from-left-1/2 data-[state=open]:slide-in-from-top-[48%]',
        'data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95 data-[state=closed]:slide-out-to-left-1/2 data-[state=closed]:slide-out-to-top-[48%]',
        $props.class,
      )"
    >
      <slot />

      <DialogClose
        :class="cn(
          'absolute right-4 top-4 rounded-md p-1',
          'text-[var(--text-tertiary)] hover:text-[var(--text-default)] hover:bg-[var(--bg-overlay-l1)]',
          'transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-contrast)] focus:ring-offset-2 focus:ring-offset-[var(--bg-base-default)]',
        )"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M18 6 6 18" /><path d="m6 6 12 12" />
        </svg>
        <span class="sr-only">Close</span>
      </DialogClose>
    </DialogContent>
  </DialogPortal>
</template>
