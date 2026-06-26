<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'
import { DropdownMenuContent, DropdownMenuPortal, type DropdownMenuContentEmits, type DropdownMenuContentProps, useForwardPropsEmits } from 'radix-vue'
import { computed } from 'vue'

defineOptions({ inheritAttrs: false })

const props = withDefaults(defineProps<DropdownMenuContentProps & { class?: HTMLAttributes['class'] }>(), {
  sideOffset: 4,
})
const emits = defineEmits<DropdownMenuContentEmits>()
const delegatedProps = computed(() => {
  const { class: _, ...rest } = props
  return rest
})
const forwarded = useForwardPropsEmits(delegatedProps, emits)
</script>

<template>
  <DropdownMenuPortal>
    <DropdownMenuContent v-bind="{ ...forwarded, ...$attrs }" :class="cn('z-50 min-w-[8rem] overflow-hidden rounded-md border bg-[var(--bg-menu)] p-1 text-[var(--text-default)] shadow-[0_12px_32px_rgba(0,0,0,0.12)] data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2', props.class)">
      <slot />
    </DropdownMenuContent>
  </DropdownMenuPortal>
</template>
