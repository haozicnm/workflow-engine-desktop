<script setup lang="ts">
import { inject, type Ref } from 'vue'
import { cn } from '@/lib/utils'
import Button from '../button/Button.vue'
import { PanelLeft, PanelRight } from 'lucide-vue-next'

const sidebar = inject<{ open: Ref<boolean>; toggle: () => void }>('sidebar')
</script>

<template>
  <Button
    variant="ghost"
    size="icon"
    :class="cn('h-7 w-7 shrink-0', $attrs.class as string)"
    :aria-label="sidebar?.open.value ? 'Collapse sidebar' : 'Expand sidebar'"
    v-bind="$attrs"
    @click="sidebar?.toggle()"
  >
    <PanelLeft v-if="sidebar?.open.value" class="w-4 h-4" />
    <PanelRight v-else class="w-4 h-4" />
  </Button>
</template>
