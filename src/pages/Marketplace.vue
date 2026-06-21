<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { safeInvoke } from '../utils/tauri'
import { useToast } from '../composables/useToast'
import { Search, Download, Tag, ExternalLink } from 'lucide-vue-next'
import Button from '../components/ui/button/Button.vue'
import Input from '../components/ui/input/Input.vue'
import Badge from '../components/ui/badge/Badge.vue'
import Card from '../components/ui/card/Card.vue'

const { t } = useI18n()
const { show: toast } = useToast()

interface Template {
  name: string
  label: string
  description: string
  category: string
  tags: string[]
  author: string
  version: string
  icon?: string
  source: 'builtin' | 'online'
}

const templates = ref<Template[]>([])
const loading = ref(true)
const searchQuery = ref('')
const selectedCategory = ref<string | null>(null)

const categories = computed(() => {
  const cats = new Set(templates.value.map(t => t.category))
  return Array.from(cats).sort()
})

const filteredTemplates = computed(() => {
  let result = templates.value
  if (selectedCategory.value) {
    result = result.filter(t => t.category === selectedCategory.value)
  }
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    result = result.filter(t =>
      t.label.toLowerCase().includes(q) ||
      t.description.toLowerCase().includes(q) ||
      t.tags.some(tag => tag.toLowerCase().includes(q))
    )
  }
  return result
})

const categoryIcons: Record<string, string> = {
  '数据采集': '🔍',
  '数据处理': '📊',
  '自动化办公': '📋',
  'AI 应用': '🤖',
  '电商运营': '🛒',
  '开发运维': '⚙️',
  '触发器示例': '⚡',
}

async function loadTemplates() {
  loading.value = true
  try {
    const result = await safeInvoke<Template[]>('template_list')
    if (result) templates.value = result
  } catch (e) {
    console.error('Failed to load templates:', e)
  } finally {
    loading.value = false
  }
}

async function installTemplate(name: string) {
  try {
    const result = await safeInvoke<{ id: string }>('template_import', { name })
    if (result?.id) {
      toast(t('marketplace.installed', { name }), 'success')
    }
  } catch (e) {
    toast(t('marketplace.installFailed') + ': ' + (e as Error).message, 'error')
  }
}

onMounted(loadTemplates)
</script>

<template>
  <div class="flex flex-col h-full bg-background">
    <!-- Header -->
    <div class="border-b px-6 py-4 shrink-0">
      <div class="flex items-center gap-3 mb-3">
        <h1 class="text-lg font-semibold">{{ t('marketplace.title') }}</h1>
        <Badge variant="outline" class="text-xs">{{ templates.length }} {{ t('marketplace.templates') }}</Badge>
      </div>
      <div class="flex items-center gap-3">
        <div class="relative flex-1 max-w-md">
          <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
          <Input
            v-model="searchQuery"
            :placeholder="t('marketplace.search')"
            class="pl-9"
          />
        </div>
      </div>
    </div>

    <div class="flex flex-1 overflow-hidden">
      <!-- Category sidebar -->
      <div class="w-48 border-r shrink-0 overflow-auto py-3 px-2">
        <button
          class="w-full text-left px-3 py-1.5 text-sm rounded-md transition-colors"
          :class="!selectedCategory ? 'bg-accent text-accent-foreground' : 'text-muted-foreground hover:bg-accent/50'"
          @click="selectedCategory = null"
        >
          {{ t('marketplace.allCategories') }}
        </button>
        <button
          v-for="cat in categories"
          :key="cat"
          class="w-full text-left px-3 py-1.5 text-sm rounded-md transition-colors flex items-center gap-2"
          :class="selectedCategory === cat ? 'bg-accent text-accent-foreground' : 'text-muted-foreground hover:bg-accent/50'"
          @click="selectedCategory = cat"
        >
          <span>{{ categoryIcons[cat] || '📦' }}</span>
          <span class="truncate">{{ cat }}</span>
        </button>
      </div>

      <!-- Template grid -->
      <div class="flex-1 overflow-auto p-6">
        <div v-if="loading" class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <Card v-for="i in 6" :key="i" class="p-4">
            <div class="h-5 w-3/4 bg-muted animate-pulse rounded mb-2" />
            <div class="h-4 w-full bg-muted animate-pulse rounded mb-1" />
            <div class="h-4 w-2/3 bg-muted animate-pulse rounded" />
          </Card>
        </div>

        <div v-else-if="filteredTemplates.length === 0" class="flex flex-col items-center justify-center py-20 text-muted-foreground">
          <Search class="w-12 h-12 mb-4 opacity-30" />
          <p class="text-sm">{{ t('marketplace.noResults') }}</p>
        </div>

        <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <Card
            v-for="tmpl in filteredTemplates"
            :key="tmpl.name"
            class="p-4 hover:border-primary/50 transition-colors group"
          >
            <div class="flex items-start gap-3 mb-3">
              <div class="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center text-lg shrink-0">
                {{ categoryIcons[tmpl.category] || '📦' }}
              </div>
              <div class="flex-1 min-w-0">
                <h3 class="text-sm font-medium truncate">{{ tmpl.label }}</h3>
                <p class="text-xs text-muted-foreground mt-0.5 line-clamp-2">{{ tmpl.description }}</p>
              </div>
            </div>
            <div class="flex flex-wrap gap-1 mb-3">
              <Badge v-for="tag in tmpl.tags" :key="tag" variant="secondary" class="text-[10px] px-1.5 py-0">
                {{ tag }}
              </Badge>
            </div>
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-1 text-xs text-muted-foreground">
                <Tag class="w-3 h-3" />
                <span>{{ tmpl.category }}</span>
                <span v-if="tmpl.source === 'online'" class="ml-1"><ExternalLink class="w-3 h-3 inline" /></span>
              </div>
              <Button variant="ghost" size="sm" class="opacity-0 group-hover:opacity-100 transition-opacity" @click="installTemplate(tmpl.name)">
                <Download class="w-3.5 h-3.5 mr-1" /> {{ t('marketplace.install') }}
              </Button>
            </div>
          </Card>
        </div>
      </div>
    </div>
  </div>
</template>
