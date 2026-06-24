<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import ActionIcon from './ActionIcon.vue'
import Button from './ui/button/Button.vue'
import Input from './ui/input/Input.vue'
import { Search } from 'lucide-vue-next'
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
  core: '🔧 核心',
  data: '📊 数据',
  flow: '🔄 流程控制',
  browser: '🌐 浏览器',
  office: '📋 办公',
  system: '⚙️ 系统',
  ai: '🤖 AI',
  trigger: '⚡ 触发器',
  integration: '🔗 集成',
  other: '📦 其他',
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') emit('close')
}
</script>

<template>
  <Teleport to="body">
    <Transition name="fade">
      <div v-if="show" class="fixed inset-0 z-[100]" role="dialog" aria-modal="true" @click="emit('close')" @keydown="onKeydown">
        <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-card border border-border rounded-lg min-w-[320px] max-w-[400px] max-h-[70vh] overflow-hidden flex flex-col" @click.stop>
          <!-- 搜索框 -->
          <div class="p-2 border-b border-border">
            <div class="relative">
              <Search class="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
              <Input
                v-model="searchQuery"
                :placeholder="t('editor.searchNodes', '搜索节点...')"
                class="pl-8 h-8 text-sm"
                autofocus
              />
            </div>
          </div>

          <!-- 节点列表 -->
          <div class="overflow-y-auto flex-1 p-1">
            <!-- 搜索结果为空 -->
            <div v-if="searchQuery && groupedContainerDefs.baseNodes.length === 0 && groupedContainerDefs.mcpNodes.length === 0"
              class="py-8 text-center text-sm text-muted-foreground">
              {{ t('editor.noNodesFound', '未找到匹配的节点') }}
            </div>

            <!-- 按 category 分组显示 -->
            <template v-if="!searchQuery">
              <div v-for="[cat, nodes] in groupedContainerDefs.categories" :key="cat" class="mb-1">
                <div class="px-3 py-1 text-[10px] uppercase tracking-wide text-muted-foreground font-medium">
                  {{ categoryLabels[cat] || cat }}
                </div>
                <div
                  v-for="def in nodes"
                  :key="def.type"
                  class="flex items-center gap-2.5 px-3 py-2 rounded-md cursor-pointer transition-colors hover:bg-secondary"
                  @click="emit('select', def.type)"
                >
                  <ActionIcon :name="def.icon" cls="w-5 h-5 shrink-0" />
                  <div class="flex-1 min-w-0">
                    <div class="text-sm font-medium text-foreground">{{ def.label }}</div>
                    <div class="text-[11px] text-muted-foreground truncate">{{ def.description }}</div>
                  </div>
                </div>
              </div>
            </template>

            <!-- 搜索模式：扁平列表 -->
            <template v-else>
              <div
                v-for="def in groupedContainerDefs.baseNodes"
                :key="def.type"
                class="flex items-center gap-2.5 px-3 py-2 rounded-md cursor-pointer transition-colors hover:bg-secondary"
                @click="emit('select', def.type)"
              >
                <ActionIcon :name="def.icon" cls="w-5 h-5 shrink-0" />
                <div class="flex-1 min-w-0">
                  <div class="text-sm font-medium text-foreground">{{ def.label }}</div>
                  <div class="text-[11px] text-muted-foreground truncate">{{ def.description }}</div>
                </div>
              </div>
            </template>

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
