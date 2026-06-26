<script setup lang="ts">
import { ref } from 'vue'
import { useToast } from '../composables/useToast'
import { safeInvoke } from '../utils/tauri'
import Button from './ui/button/Button.vue'

const toast = useToast()

const emit = defineEmits<{
  select: [ref: string]
  close: []
}>()

interface RefNode {
  role: string
  name?: string
  ref?: string
  children?: RefNode[]
}

const loading = ref(false)
const tree = ref<RefNode[]>([])
const refs = ref<Record<string, { role: string; name: string }>>({})
const pageTitle = ref('')
const pageUrl = ref('')
const selectedRef = ref('')

// ─── 扁平化交互元素列表 ───
interface FlatElement {
  ref: string
  role: string
  name: string
  depth: number
}

function flattenTree(nodes: RefNode[], depth = 0): FlatElement[] {
  const result: FlatElement[] = []
  for (const node of nodes) {
    if (node.ref) {
      result.push({
        ref: node.ref,
        role: node.role,
        name: node.name || '',
        depth,
      })
    }
    if (node.children?.length) {
      result.push(...flattenTree(node.children, depth + 1))
    }
  }
  return result
}

const flatElements = ref<FlatElement[]>([])

// ─── 角色图标映射 ───
const roleIcon: Record<string, string> = {
  button: '🔘',
  link: '🔗',
  textbox: '📝',
  combobox: '📋',
  checkbox: '☑️',
  radio: '📻',
  tab: '📑',
  menuitem: '📋',
  img: '🖼️',
  heading: '📌',
  list: '📋',
  listitem: '•',
  table: '📊',
  row: '➡️',
  cell: '⬜',
  navigation: '🧭',
  search: '🔍',
  toolbar: '🔧',
  dialog: '💬',
  alert: '⚠️',
}

function getRoleIcon(role: string): string {
  return roleIcon[role] || '▪️'
}

// ─── 获取 snapshot ───
async function takeSnapshot() {
  loading.value = true
  try {
    const result = await safeInvoke<{
      success?: boolean
      data?: {
        url?: string
        title?: string
        tree?: RefNode[]
        refs?: Record<string, { role: string; name: string }>
      }
      error?: string
    }>('browser_snapshot')

    if (!result?.success) {
      toast.error(result?.error || 'Snapshot 失败')
      return
    }

    const data = result.data || {}
    pageUrl.value = data.url || ''
    pageTitle.value = data.title || ''
    tree.value = data.tree || []
    refs.value = data.refs || {}
    flatElements.value = flattenTree(data.tree || [])
    selectedRef.value = ''
  } catch (e) {
    toast.error('Snapshot 失败: ' + (e instanceof Error ? e.message : String(e)))
  } finally {
    loading.value = false
  }
}

function selectElement(refId: string) {
  selectedRef.value = refId
}

function confirmSelect() {
  if (selectedRef.value) {
    emit('select', selectedRef.value)
  }
}
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-[var(--bg-overlay-l4)] backdrop-blur-sm" @click.self="emit('close')">
    <div class="bg-[var(--bg-base-default)] border border-[var(--border-neutral-l1)] rounded-lg shadow-[0_24px_64px_rgba(0,0,0,0.14)] w-[480px] max-h-[80vh] flex flex-col">
      <!-- Header -->
      <div class="flex items-center justify-between px-4 py-3 border-b border-[var(--border-neutral-l1)]">
        <h3 class="text-sm font-medium">
          元素选择器
          <span v-if="pageTitle" class="text-[var(--text-tertiary)] ml-2 text-xs">— {{ pageTitle }}</span>
        </h3>
        <button class="text-[var(--text-tertiary)] hover:text-[var(--text-default)] text-lg leading-none" @click="emit('close')">×</button>
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-y-auto min-h-0">
        <!-- No snapshot yet -->
        <div v-if="!flatElements.length && !loading" class="p-6 text-center">
          <p class="text-sm text-[var(--text-tertiary)] mb-4">
            点击按钮获取当前页面的交互元素列表
          </p>
          <Button size="sm" @click="takeSnapshot">
            📸 获取页面快照
          </Button>
        </div>

        <!-- Loading -->
        <div v-if="loading" class="p-6 text-center">
          <div class="animate-spin inline-block w-5 h-5 border-2 border-[var(--bg-brand)] border-t-transparent rounded-full mb-2" />
          <p class="text-sm text-[var(--text-tertiary)]">正在获取页面元素...</p>
        </div>

        <!-- Element list -->
        <div v-if="flatElements.length" class="py-2">
          <div class="px-3 py-1.5 flex items-center justify-between">
            <span class="text-xs text-[var(--text-tertiary)]">
              {{ flatElements.length }} 个交互元素
              <span v-if="pageUrl" class="ml-1 opacity-60">| {{ pageUrl }}</span>
            </span>
            <button class="text-xs text-[var(--text-brand)] hover:underline" @click="takeSnapshot">刷新</button>
          </div>
          <div
            v-for="(el, i) in flatElements"
            :key="i"
            class="flex items-center gap-2 px-3 py-1.5 cursor-pointer hover:bg-[var(--bg-overlay-l2)]/60 transition-colors"
            :class="{ 'bg-[var(--bg-overlay-l2)]': selectedRef === el.ref }"
            :style="{ paddingLeft: `${12 + el.depth * 16}px` }"
            @click="selectElement(el.ref)"
            @dblclick="selectElement(el.ref); confirmSelect()"
          >
            <span class="text-xs leading-none">{{ getRoleIcon(el.role) }}</span>
            <span class="text-xs font-mono text-[var(--text-brand)] min-w-[2.5rem]">@{{ el.ref }}</span>
            <span class="text-xs font-medium text-[var(--text-default)]/80">{{ el.role }}</span>
            <span v-if="el.name" class="text-xs text-[var(--text-tertiary)] truncate flex-1">{{ el.name }}</span>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-between px-4 py-3 border-t border-[var(--border-neutral-l1)]">
        <span v-if="selectedRef" class="text-xs font-mono text-[var(--text-brand)]">
          已选: @{{ selectedRef }}
          <span v-if="refs[selectedRef]" class="text-[var(--text-tertiary)] ml-1">
            ({{ refs[selectedRef].role }}{{ refs[selectedRef].name ? ': ' + refs[selectedRef].name : '' }})
          </span>
        </span>
        <span v-else class="text-xs text-[var(--text-tertiary)]">点击元素选择</span>
        <div class="flex gap-2">
          <Button variant="outline" size="sm" @click="emit('close')">取消</Button>
          <Button size="sm" :disabled="!selectedRef" @click="confirmSelect">确认选择</Button>
        </div>
      </div>
    </div>
  </div>
</template>
