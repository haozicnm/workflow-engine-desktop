<script setup lang="ts">
import { TabsRoot } from 'radix-vue'
import { cn } from '@/lib/utils'

defineProps<{
  defaultValue?: string
  modelValue?: string
}>()

defineEmits<{
  'update:modelValue': [value: string]
}>()
</script>

<template>
  <TabsRoot
    :default-value="defaultValue"
    :model-value="modelValue"
    :class="cn($attrs.class as string)"
    @update:model-value="(v: string) => $emit('update:modelValue', v)"
  >
    <slot />
  </TabsRoot>
</template>
