<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'
import { DropdownMenuSeparator, type DropdownMenuSeparatorProps } from 'radix-vue'

const props = defineProps<DropdownMenuSeparatorProps & { class?: HTMLAttributes['class'] }>()
</script>

<template>
  <DropdownMenuSeparator v-bind="props" :class="cn('-mx-1 my-1 h-px bg-muted', props.class)" />
</template>
