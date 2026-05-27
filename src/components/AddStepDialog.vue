<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import ActionIcon from './ActionIcon.vue'
import Button from './ui/button/Button.vue'
import { allContainerDefs } from '../types/node-registry'
import type { ContainerType } from '../types/types'

const { t } = useI18n()

defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  close: []
  select: [type: ContainerType]
}>()

const showMcpNodes = ref(false)

const translatedContainerDefs = computed(() =>
  allContainerDefs().map(d => ({
    ...d,
    label: t(`nodeLabel.${d.type}`, d.label),
    description: t(`nodeDesc.${d.type}`, d.description),
  }))
)

const groupedContainerDefs = computed(() => {
  const all = translatedContainerDefs.value
  const mcpNodes = all.filter(d => d.type.startsWith('mcp_'))
  const baseNodes = all.filter(d => !d.type.startsWith('mcp_'))
  return { baseNodes, mcpNodes }
})
</script>

<template>
  <Teleport to="body">
    <Transition name="fade">
      <div v-if="show" class="fixed inset-0 z-[100]" role="dialog" aria-modal="true" @click="emit('close')" @keydown.escape="emit('close')">
        <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-card border border-border rounded-lg p-2 min-w-[280px] max-h-[70vh] overflow-y-auto shadow-xl" @click.stop>
          <!-- Base nodes -->
          <div
            v-for="def in groupedContainerDefs.baseNodes"
            :key="def.type"
            class="flex items-center gap-2.5 px-3 py-2.5 rounded-md cursor-pointer transition-colors hover:bg-secondary"
            @click="emit('select', def.type)"
          >
            <ActionIcon :name="def.icon" cls="w-5 h-5 shrink-0" />
            <div class="flex-1 min-w-0">
              <div class="text-sm font-medium text-foreground">{{ def.label }}</div>
              <div class="text-[11px] text-muted-foreground truncate">{{ def.description }}</div>
            </div>
          </div>

          <!-- MCP extension divider + collapse -->
          <div v-if="groupedContainerDefs.mcpNodes.length" class="border-t border-border mt-1 pt-1">
            <Button
              variant="ghost"
              class="flex items-center gap-2 w-full px-3 py-2 rounded-md text-xs text-muted-foreground justify-start"
              @click="showMcpNodes = !showMcpNodes"
            >
              <span class="transition-transform duration-150" :class="showMcpNodes ? 'rotate-90' : ''">▶</span>
              <span>MCP 扩展 ({{ groupedContainerDefs.mcpNodes.length }})</span>
            </Button>
            <div v-if="showMcpNodes" class="space-y-0.5">
              <div
                v-for="def in groupedContainerDefs.mcpNodes"
                :key="def.type"
                class="flex items-center gap-2.5 px-3 py-2 rounded-md cursor-pointer transition-colors hover:bg-secondary ml-4"
                @click="emit('select', def.type)"
              >
                <ActionIcon :name="def.icon" cls="w-4 h-4 shrink-0" />
                <div class="flex-1 min-w-0">
                  <div class="text-sm text-foreground">{{ def.label }}</div>
                  <div class="text-[10px] text-muted-foreground truncate">{{ def.description }}</div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
