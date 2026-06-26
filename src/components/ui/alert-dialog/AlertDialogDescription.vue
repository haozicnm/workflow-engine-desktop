<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'
import { AlertDialogDescription, type AlertDialogDescriptionProps } from 'radix-vue'

const props = defineProps<AlertDialogDescriptionProps & { class?: HTMLAttributes['class'] }>()
</script>

<template>
  <AlertDialogDescription v-bind="props" :class="cn('text-sm text-[var(--text-tertiary)]', props.class)">
    <slot />
  </AlertDialogDescription>
</template>
