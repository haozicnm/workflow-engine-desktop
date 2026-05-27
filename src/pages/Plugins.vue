<script setup lang="ts">
// pages/Plugins.vue — 插件管理页面（覆盖层面板）
// 独立服务器模式：使用 HTTP API + multipart 文件上传
import { ref, onMounted } from 'vue'
import { useToast } from '@/composables/useToast'
import ActionIcon from '@/components/ActionIcon.vue'
import Button from '@/components/ui/button/Button.vue'

const { show: toast } = useToast()

interface PluginInfo {
  name: string
  version: string
  title: string
  description: string
  author: string
  icon: string
  mcp_count: number
  template_count: number
}

const plugins = ref<PluginInfo[]>([])
const loading = ref(false)
const installing = ref(false)
const fileInput = ref<HTMLInputElement | null>(null)

async function loadPlugins() {
  loading.value = true
  try {
    const res = await fetch('/api/plugins')
    const data = await res.json()
    if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`)
    plugins.value = data.plugins || []
  } catch (e: any) {
    toast(`加载插件列表失败: ${e.message}`, 'error')
  } finally {
    loading.value = false
  }
}

function triggerFilePick() {
  fileInput.value?.click()
}

async function onFileSelected(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  input.value = '' // 重置以便再次选择同一文件

  if (!file.name.endsWith('.wfplug')) {
    toast('请选择 .wfplug 格式的插件文件', 'error')
    return
  }

  installing.value = true
  try {
    const form = new FormData()
    form.append('file', file)

    const res = await fetch('/api/plugins/upload', { method: 'POST', body: form })
    const data = await res.json()
    if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`)

    toast(`插件 ${data.plugin.title} v${data.plugin.version} 安装成功`, 'success')
    await loadPlugins()
  } catch (e: any) {
    toast(`安装失败: ${e.message}`, 'error')
  } finally {
    installing.value = false
  }
}

async function uninstallPlugin(plugin: PluginInfo) {
  if (!confirm(`确定要删除插件「${plugin.title}」吗？\n此操作将移除所有 MCP 节点和模板。`)) return

  try {
    const res = await fetch('/api/plugins/uninstall', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: plugin.name }),
    })
    const data = await res.json()
    if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`)

    toast(`插件 ${plugin.title} 已卸载`, 'success')
    await loadPlugins()
  } catch (e: any) {
    toast(`卸载失败: ${e.message}`, 'error')
  }
}

function getIcon(name: string): string {
  const map: Record<string, string> = {
    'Server': 'Server',
    'Package': 'Package',
    'Puzzle': 'Puzzle',
    'Box': 'Box',
    'Database': 'Database',
    'Globe': 'Globe',
  }
  return map[name] || 'Package'
}

onMounted(loadPlugins)
</script>

<template>
  <div class="h-full flex flex-col bg-card">
    <!-- Header -->
    <div class="flex items-center justify-between px-6 py-4 border-b border-border shrink-0">
      <div>
        <h2 class="text-lg font-semibold text-foreground">插件管理</h2>
        <p class="text-xs text-muted-foreground mt-0.5">安装和管理 workflow 功能插件</p>
      </div>
      <div class="flex items-center gap-2">
        <input
          ref="fileInput"
          type="file"
          accept=".wfplug"
          class="hidden"
          @change="onFileSelected"
        />
        <Button
          :disabled="installing"
          @click="triggerFilePick"
        >
          {{ installing ? '安装中...' : '+ 安装插件' }}
        </Button>
      </div>
    </div>

    <!-- Plugin list -->
    <div class="flex-1 overflow-y-auto p-4">
      <!-- Loading -->
      <div v-if="loading" class="flex items-center justify-center py-12">
        <span class="text-sm text-muted-foreground">加载中...</span>
      </div>

      <!-- Empty -->
      <div v-else-if="plugins.length === 0" class="flex flex-col items-center justify-center py-16 text-center">
        <ActionIcon name="Package" cls="w-12 h-12 text-muted-foreground mb-4" />
        <h3 class="text-base font-medium text-foreground mb-1">暂无已安装插件</h3>
        <p class="text-sm text-muted-foreground mb-4 max-w-sm">
          点击「安装插件」选择 .wfplug 文件，即可快速添加新功能包
        </p>
        <Button @click="triggerFilePick">
          + 安装插件
        </Button>
      </div>

      <!-- Cards -->
      <div v-else class="space-y-3">
        <div
          v-for="p in plugins"
          :key="p.name"
          class="rounded-lg border border-border bg-background p-4 transition-colors hover:border-primary/30"
        >
          <div class="flex items-start justify-between gap-3">
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 mb-1">
                <ActionIcon :name="getIcon(p.icon)" cls="w-4 h-4 text-primary" />
                <h3 class="text-sm font-semibold text-foreground truncate">{{ p.title }}</h3>
                <span class="text-[10px] text-muted-foreground bg-secondary rounded px-1.5 py-0.5 shrink-0">v{{ p.version }}</span>
              </div>
              <p class="text-xs text-muted-foreground line-clamp-2 mb-2">{{ p.description }}</p>
              <div class="flex items-center gap-3 text-[10px] text-muted-foreground">
                <span v-if="p.author" class="flex items-center gap-1">
                  {{ p.author }}
                </span>
                <span class="flex items-center gap-1" title="MCP 节点数">
                  {{ p.mcp_count }} 个节点
                </span>
                <span class="flex items-center gap-1" title="模板数">
                  {{ p.template_count }} 个模板
                </span>
              </div>
            </div>
            <Button
              variant="destructive"
              size="sm"
              class="shrink-0 text-[11px]"
              @click="uninstallPlugin(p)"
            >
              删除
            </Button>
          </div>
        </div>
      </div>
    </div>

    <!-- Footer hint -->
    <div class="px-6 py-3 border-t border-border shrink-0">
      <p class="text-[10px] text-muted-foreground">
        插件安装后会自动注册 MCP 节点并导入模板。删除时自动清理。.wfplug 文件可在项目
        <code class="bg-secondary rounded px-1 text-[10px]">samba-web-manager/</code> 目录找到。
      </p>
    </div>
  </div>
</template>
