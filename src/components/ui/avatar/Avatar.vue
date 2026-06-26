<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'
import { AvatarRoot, type AvatarRootProps } from 'radix-vue'

const props = withDefaults(defineProps<AvatarRootProps & { class?: HTMLAttributes['class'] }>(), {
  as: 'span',
})
</script>

<template>
  <AvatarRoot
    v-bind="props"
    :class="cn('relative flex h-10 w-10 shrink-0 overflow-hidden rounded-full', props.class)"
  >
    <slot />
  </AvatarRoot>
</template>
