<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import ActionIcon from './ActionIcon.vue'
import Button from './ui/button/Button.vue'
import Input from './ui/input/Input.vue'
import { Search, ChevronRight } from 'lucide-vue-next'
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
const searchQuery = ref('')

const translatedContainerDefs = computed(() =>
  allContainerDefs().map(d => ({
    ...d,
    label: t(`nodeLabel.${d.type}`, d.label),
    description: t(`nodeDesc.${d.type}`, d.description),
  }))
)

const groupedContainerDefs = computed(() => {
  const q = searchQuery.value.toLowerCase().trim()
  let all = translatedContainerDefs.value
  if (q) {
    all = all.filter(d =>
      d.label.toLowerCase().includes(q) ||
      d.type.toLowerCase().includes(q) ||
      d.description.toLowerCase().includes(q) ||
      (d.category && d.category.toLowerCase().includes(q))
    )
  }
  const mcpNodes = all.filter(d => d.type.startsWith('mcp_'))
  const baseNodes = all.filter(d => !d.type.startsWith('mcp_'))

  // 按 category 分组
  const categories = new Map<string, typeof baseNodes>()
  for (const node of baseNodes) {
    const cat = node.category || 'other'
    if (!categories.has(cat)) categories.set(cat, [])
    categories.get(cat)!.push(node)
  }
  return { baseNodes, mcpNodes, categories }
})

const categoryLabels: Record<string, string> = {
  core: t('category.core', '核心'),
  data: t('category.data', '数据'),
  flow: t('category.flow', '流程控制'),
  browser: t('category.browser', '浏览器'),
  office: t('category.office', '办公'),
  system: t('category.system', '系统'),
  ai: t('category.ai', 'AI'),
  trigger: t('category.trigger', '触发器'),
  integration: t('category.integration', '集成'),
  other: t('category.other', '其他'),
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') emit('close')
}
</script>

<template>
  <Teleport to="body">
    <Transition name="fade">
      <div v-if="show" class="fixed inset-0 z-[100]" role="dialog" aria-modal="true" aria-labelledby="add-step-dialog-title" @click="emit('close')" @keydown="onKeydown">
        <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-[var(--bg-base-secondary)] border border-[var(--border-neutral-l1)] rounded-lg min-w-[320px] max-w-[400px] max-h-[70vh] overflow-hidden flex flex-col" @click.stop>
          <!-- 搜索框 -->
          <div class="p-2 border-b border-[var(--border-neutral-l1)]">
            <div class="relative">
              <Search class="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-[var(--text-tertiary)]" />
              <Input
                v-model="searchQuery"
                :placeholder="t('editor.searchNodes', '搜索节点...')"
                class="pl-8 h-8 text-sm"
                autofocus
                role="searchbox"
                aria-label="搜索节点"
              />
            </div>
          </div>

          <!-- 节点列表 -->
          <div class="overflow-y-auto flex-1 p-1">
            <!-- 搜索结果为空 -->
            <div v-if="searchQuery && groupedContainerDefs.baseNodes.length === 0 && groupedContainerDefs.mcpNodes.length === 0"
              class="py-8 text-center text-sm text-[var(--text-tertiary)]">
              {{ t('editor.noNodesFound', '未找到匹配的节点') }}
            </div>

            <!-- 按 category 分组显示 -->
            <template v-if="!searchQuery">
              <div v-for="[cat, nodes] in groupedContainerDefs.categories" :key="cat" class="mb-1">
                <div class="px-3 py-1 text-[10px] uppercase tracking-wide text-[var(--text-tertiary)] font-medium">
                  {{ categoryLabels[cat] || cat }}
                </div>
                <div
                  v-for="def in nodes"
                  :key="def.type"
                  class="flex items-center gap-2.5 px-3 py-2 rounded-md cursor-pointer transition-colors hover:bg-[var(--bg-overlay-l1)] focus-visible:bg-[var(--bg-overlay-l1)] focus-visible:outline-none"
                  role="option"
                  :aria-selected="false"
                  tabindex="0"
                  @click="emit('select', def.type)"
                  @keydown.enter.prevent="emit('select', def.type)"
                >
                  <ActionIcon :name="def.icon" cls="w-5 h-5 shrink-0" />
                  <div class="flex-1 min-w-0">
                    <div class="text-sm font-medium text-[var(--text-default)]">{{ def.label }}</div>
                    <div class="text-[11px] text-[var(--text-tertiary)] truncate">{{ def.description }}</div>
                  </div>
                </div>
              </div>
            </template>

            <!-- 搜索模式：扁平列表 -->
            <template v-else>
              <div
                v-for="def in groupedContainerDefs.baseNodes"
                :key="def.type"
                class="flex items-center gap-2.5 px-3 py-2 rounded-md cursor-pointer transition-colors hover:bg-[var(--bg-overlay-l1)] focus-visible:bg-[var(--bg-overlay-l1)] focus-visible:outline-none"
                role="option"
                tabindex="0"
                @click="emit('select', def.type)"
                @keydown.enter.prevent="emit('select', def.type)"
              >
                <ActionIcon :name="def.icon" cls="w-5 h-5 shrink-0" />
                <div class="flex-1 min-w-0">
                  <div class="text-sm font-medium text-[var(--text-default)]">{{ def.label }}</div>
                  <div class="text-[11px] text-[var(--text-tertiary)] truncate">{{ def.description }}</div>
                </div>
              </div>
            </template>

            <!-- MCP extension divider + collapse -->
            <div v-if="groupedContainerDefs.mcpNodes.length" class="border-t border-[var(--border-neutral-l1)] mt-1 pt-1">
              <Button
                variant="ghost"
                class="flex items-center gap-2 w-full px-3 py-2 rounded-md text-xs text-[var(--text-tertiary)] justify-start"
                :aria-expanded="showMcpNodes"
                @click="showMcpNodes = !showMcpNodes"
              >
                <ChevronRight class="w-3.5 h-3.5 transition-transform duration-150" :class="showMcpNodes ? 'rotate-90' : ''" />
                <span>MCP 扩展 ({{ groupedContainerDefs.mcpNodes.length }})</span>
              </Button>
              <div v-if="showMcpNodes" class="space-y-0.5">
                <div
                  v-for="def in groupedContainerDefs.mcpNodes"
                  :key="def.type"
                  class="flex items-center gap-2.5 px-3 py-2 rounded-md cursor-pointer transition-colors hover:bg-[var(--bg-overlay-l1)] focus-visible:bg-[var(--bg-overlay-l1)] focus-visible:outline-none ml-4"
                  role="option"
                  tabindex="0"
                  @click="emit('select', def.type)"
                  @keydown.enter.prevent="emit('select', def.type)"
                >
                  <ActionIcon :name="def.icon" cls="w-4 h-4 shrink-0" />
                  <div class="flex-1 min-w-0">
                    <div class="text-sm text-[var(--text-default)]">{{ def.label }}</div>
                    <div class="text-[10px] text-[var(--text-tertiary)] truncate">{{ def.description }}</div>
                  </div>
                </div>
              </div>
            </div>  <!-- /MCP border wrapper -->
          </div>  <!-- /node list -->
        </div>  <!-- /dialog box -->
      </div>  <!-- /outer overlay -->
    </Transition>
  </Teleport>
</template>
