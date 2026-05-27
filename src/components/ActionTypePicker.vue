<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import ActionIcon from './ActionIcon.vue'

const { t } = useI18n()

defineProps<{
  show: boolean
  options: { type: string; label: string; icon: string }[]
}>()

const emit = defineEmits<{
  close: []
  select: [type: string]
}>()
</script>

<template>
  <Teleport to="body">
    <Transition name="fade">
      <div
        v-if="show"
        class="fixed inset-0 bg-black/50 flex items-center justify-center z-[100]"
        @click="emit('close')"
      >
        <div class="bg-card border border-border rounded-xl p-4 min-w-[280px] max-h-[400px] overflow-y-auto shadow-2xl" @click.stop>
          <div class="text-sm font-semibold text-foreground mb-3 px-1">{{ t('editor.selectActionType') }}</div>
          <div
            v-for="opt in options"
            :key="opt.type"
            class="flex items-center gap-2.5 px-3 py-2.5 rounded-md cursor-pointer transition-colors hover:bg-secondary"
            @click="emit('select', opt.type)"
          >
            <ActionIcon :name="opt.icon" cls="w-4 h-4" />
            <span class="text-sm font-medium text-foreground">{{ opt.label }}</span>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
