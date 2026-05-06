<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { safeInvoke } from '../utils/tauri'
import { useToast } from '../composables/useToast'
import { useTheme, type Theme } from '../composables/useTheme'
import pkg from '../package.json'
import Button from '../components/ui/button/Button.vue'
import Input from '../components/ui/input/Input.vue'
import Label from '../components/ui/label/Label.vue'
import Switch from '../components/ui/switch/Switch.vue'
import Badge from '../components/ui/badge/Badge.vue'
import Card from '../components/ui/card/Card.vue'
import Select from '../components/ui/select/Select.vue'
import Separator from '../components/ui/separator/Separator.vue'
import { cn } from '@/lib/utils'

const emit = defineEmits<{ 'back': [] }>()

const toast = useToast()
const APP_VERSION = pkg.version
const { theme: currentTheme, setTheme } = useTheme()

const settings = ref({
  theme: 'system',
  language: 'zh-CN',
  auto_start: false,
  log_level: 'info',
  python_path: '',
  browser_channel: 'auto',
})

const sysInfo = ref<any>(null)
const saving = ref(false)
const loading = ref(true)

const browserOptions = [
  { value: 'auto', label: '自动检测', desc: '优先 Edge → Chrome → Playwright Chromium' },
  { value: 'msedge', label: 'Microsoft Edge', desc: 'Windows 自带，无需额外安装' },
  { value: 'chrome', label: 'Google Chrome', desc: '需要预装 Chrome 浏览器' },
  { value: 'chromium', label: 'Playwright Chromium', desc: '使用 Playwright 内置 Chromium' },
]

const logLevelOptions = [
  { value: 'debug', label: 'Debug' },
  { value: 'info', label: 'Info' },
  { value: 'warn', label: 'Warn' },
  { value: 'error', label: 'Error' },
]

const themeOptions: { value: Theme; label: string; icon: string; desc: string }[] = [
  { value: 'light', label: '浅色', icon: '☀️', desc: '浅色主题，适合明亮环境' },
  { value: 'dark', label: '深色', icon: '🌙', desc: '深色主题，护眼舒适' },
  { value: 'system', label: '跟随系统', icon: '💻', desc: '自动匹配系统设置' },
]

onMounted(async () => {
  loading.value = true
  try {
    const s = await safeInvoke<any>('settings_get').catch(() => ({}))
    settings.value = { ...settings.value, ...(s || {}) }
  } catch (e: any) {
    console.warn('加载设置失败，使用默认值:', e)
  }
  try {
    sysInfo.value = await safeInvoke('system_check_browser')
  } catch (e: any) {
    console.warn('获取系统信息失败:', e)
  }
  loading.value = false
})

async function save() {
  saving.value = true
  try {
    await safeInvoke('settings_update', { settings: settings.value })
    toast.success('设置已保存')
  } catch (e: any) {
    toast.error('保存失败: ' + e)
  } finally {
    saving.value = false
  }
}

async function openLogDir() {
  try {
    await safeInvoke('open_log_dir')
  } catch (e: any) {
    toast.error('打开日志目录失败: ' + e)
  }
}

async function clearLogs() {
  try {
    await safeInvoke('clear_logs')
    toast.success('日志已清空')
  } catch (e: any) {
    toast.error('清空日志失败: ' + e)
  }
}

function truncatePath(path: string, maxLen: number): string {
  if (path.length <= maxLen) return path
  return '...' + path.slice(-(maxLen - 3))
}
</script>

<template>
  <div class="max-w-[640px] mx-auto px-5 py-6">
    <!-- Header -->
    <header class="mb-6">
      <Button variant="outline" size="sm" class="mb-2 text-xs" @click="emit('back')">← 返回</Button>
      <h1 class="text-xl font-bold text-foreground">⚙️ 设置</h1>
      <p class="text-sm text-muted-foreground">配置应用行为和浏览器节点</p>
    </header>

    <div v-if="loading" class="text-center py-10 text-muted-foreground">加载中...</div>

    <div v-else class="space-y-4">
      <!-- Theme settings -->
      <Card>
        <div class="p-5">
          <h2 class="text-sm font-semibold text-foreground mb-1.5">🎨 外观</h2>
          <p class="text-xs text-muted-foreground mb-4">选择界面主题风格。</p>
          <div class="grid grid-cols-3 gap-3">
            <Button
              v-for="opt in themeOptions"
              :key="opt.value"
              variant="outline"
              :class="cn(
                'flex flex-col items-center gap-2 p-4 h-auto border-2',
                currentTheme === opt.value
                  ? 'border-primary bg-primary/5 shadow-sm'
                  : 'border-border hover:border-primary/50',
              )"
              @click="setTheme(opt.value)"
            >
              <span class="text-2xl">{{ opt.icon }}</span>
              <span class="text-sm font-semibold text-foreground">{{ opt.label }}</span>
              <span class="text-[10px] text-muted-foreground text-center leading-tight">{{ opt.desc }}</span>
            </Button>
          </div>
        </div>
      </Card>

      <!-- Browser settings -->
      <Card>
        <div class="p-5">
          <h2 class="text-sm font-semibold text-foreground mb-1.5">🌐 浏览器节点</h2>
          <p class="text-xs text-muted-foreground mb-4">选择浏览器自动化使用的浏览器。内网环境建议选择 Edge。</p>

          <div class="space-y-2 mb-4">
            <Label class="text-xs text-muted-foreground font-semibold">浏览器通道</Label>
            <div class="flex flex-col gap-2">
              <label
                v-for="opt in browserOptions"
                :key="opt.value"
                :class="cn(
                  'flex items-start gap-2.5 px-3 py-2.5 border rounded-md cursor-pointer transition-colors',
                  settings.browser_channel === opt.value
                    ? 'border-primary bg-primary/5'
                    : 'border-border hover:border-primary',
                )"
              >
                <div :class="cn('w-3.5 h-3.5 rounded-full border-2 flex items-center justify-center mt-0.5 shrink-0', settings.browser_channel === opt.value ? 'border-primary bg-primary' : 'border-muted-foreground/40')">
                  <div v-if="settings.browser_channel === opt.value" class="w-1.5 h-1.5 rounded-full bg-primary-foreground" />
                </div>
                <div class="flex flex-col gap-0.5">
                  <span class="text-sm text-foreground font-semibold">{{ opt.label }}</span>
                  <span class="text-[11px] text-muted-foreground">{{ opt.desc }}</span>
                </div>
              </label>
            </div>
          </div>

          <!-- System check -->
          <div v-if="sysInfo" class="mt-4 p-3 bg-background rounded-md">
            <h3 class="text-xs text-muted-foreground mb-2.5 flex items-center gap-2">
              环境检测
              <Badge :variant="sysInfo.ready ? 'success' : 'warning'" class="text-[10px]">
                {{ sysInfo.ready ? '✅ 就绪' : '⚠️ 待配置' }}
              </Badge>
            </h3>
            <div class="flex flex-col gap-1.5">
              <!-- Python -->
              <div class="flex justify-between items-center text-xs">
                <span class="text-foreground">Python 环境</span>
                <span :class="sysInfo.python_available ? 'text-success' : 'text-danger'">
                  {{ sysInfo.python_available ? '✅ 已检测到' : '❌ 未检测到' }}
                </span>
              </div>
              <div v-if="sysInfo.system_python" class="flex justify-between items-center text-xs">
                <span class="text-foreground pl-3">↳ 路径</span>
                <span class="text-success text-[11px] truncate max-w-[200px]" :title="sysInfo.system_python">{{ truncatePath(sysInfo.system_python, 40) }}</span>
              </div>
              <div v-if="!sysInfo.python_available" class="text-xs text-destructive">
                → 请安装 Python 3.8+
                <a href="https://www.python.org/downloads/" target="_blank" class="text-primary ml-1 hover:underline">下载 →</a>
              </div>

              <!-- Playwright -->
              <div class="flex justify-between items-center text-xs">
                <span class="text-foreground">Playwright 包</span>
                <span :class="sysInfo.has_playwright_pkg ? 'text-success' : 'text-muted-foreground'">
                  {{ sysInfo.has_playwright_pkg ? '✅ 已安装' : '⏳ 首次使用自动安装' }}
                </span>
              </div>

              <!-- Browser -->
              <div class="flex justify-between items-center text-xs">
                <span class="text-foreground">浏览器</span>
                <span :class="sysInfo.has_browser ? 'text-success' : 'text-muted-foreground'">
                  {{ sysInfo.has_browser ? '✅ 可用' : '—（首次使用自动下载）' }}
                </span>
              </div>
              <div v-if="sysInfo.has_system_browser" class="flex justify-between items-center text-xs">
                <span class="text-foreground pl-3">↳ 系统浏览器</span>
                <span class="text-success text-[11px]">
                  {{ [sysInfo.has_edge ? 'Edge' : '', sysInfo.has_chrome ? 'Chrome' : ''].filter(Boolean).join(' + ') }} （首选）
                </span>
              </div>
              <div v-if="sysInfo.has_playwright_chromium" class="flex justify-between items-center text-xs">
                <span class="text-foreground pl-3">↳ 内置 Chromium</span>
                <span class="text-success text-[11px]">✅ 安装包附带</span>
              </div>
              <div v-if="sysInfo.has_playwright_cache" class="flex justify-between items-center text-xs">
                <span class="text-foreground pl-3">↳ Playwright 缓存</span>
                <span class="text-success text-[11px]">✅ 已下载</span>
              </div>
            </div>
          </div>
        </div>
      </Card>

      <!-- Advanced settings -->
      <Card>
        <div class="p-5">
          <h2 class="text-sm font-semibold text-foreground mb-4">🔧 高级</h2>

          <div class="space-y-4">
            <div class="space-y-1.5">
              <Label class="text-xs text-muted-foreground font-semibold">Python 路径 (可选)</Label>
              <Input v-model="settings.python_path" placeholder="留空使用自动检测" class="h-8 text-xs" />
              <p class="text-[11px] text-muted-foreground/60">指定 Python 可执行文件完整路径。留空则自动检测。</p>
            </div>

            <div class="space-y-1.5">
              <Label class="text-xs text-muted-foreground font-semibold">日志级别</Label>
              <Select
                :model-value="settings.log_level"
                @update:model-value="v => settings.log_level = v"
                :options="logLevelOptions"
              />
            </div>

            <div class="flex items-center gap-2.5">
              <Switch v-model="settings.auto_start" />
              <Label class="text-sm text-foreground cursor-pointer">开机自启</Label>
            </div>
          </div>
        </div>
      </Card>

      <!-- Log management -->
      <Card>
        <div class="p-5">
          <h2 class="text-sm font-semibold text-foreground mb-1.5">📄 日志</h2>
          <p class="text-xs text-muted-foreground mb-4">查看和清理应用运行日志。</p>
          <div class="flex gap-2.5 flex-wrap">
            <Button variant="outline" size="sm" @click="openLogDir">📂 查看日志文件</Button>
            <Button variant="outline" size="sm" class="text-destructive border-destructive/30 hover:bg-destructive/10" @click="clearLogs">🗑 清空日志</Button>
          </div>
        </div>
      </Card>

      <!-- Version info -->
      <Card>
        <div class="p-5">
          <h2 class="text-sm font-semibold text-foreground mb-1.5">📦 版本信息</h2>
          <p class="text-xs text-muted-foreground mb-4">当前版本及更新记录。</p>
          <div class="mb-4">
            <Badge variant="default" class="text-sm px-3 py-1">v{{ APP_VERSION }}</Badge>
          </div>
          <h3 class="text-sm text-foreground mb-2">更新明细</h3>
          <div class="space-y-0">
            <div v-for="(item, i) in [
              { version: 'v5.1.1', desc: 'shadcn-vue 全组件化 · 浅色/深色主题切换 · 单页 Sidebar 布局 · 动作行 Card 重设计 · 滚轮/下拉修复 · 设置/历史右侧面板' },
              { version: 'v5.1.0', desc: 'v5 步骤编辑器正式版 · shadcn-vue 组件体系 · 容器模板系统 · 多容器类型' },
              { version: 'v5.0', desc: '去掉 LiteGraph · 自研步骤编辑器 · Steps→Actions 模型 · Vue Draggable' },
              { version: 'v2.x', desc: 'Grid 布局 · LiteGraph 画布 · 模板系统 · 浏览器自动化' },
              { version: 'v1.x', desc: 'YAML 工作流引擎原型 · Web 前端 · Playwright 自动化' },
            ]" :key="item.version"
              class="text-xs text-muted-foreground py-1.5"
              :class="i < 4 ? 'border-b border-border' : ''"
            >
              <strong class="text-foreground">{{ item.version }}</strong> — {{ item.desc }}
            </div>
          </div>
        </div>
      </Card>

      <!-- Save bar -->
      <div class="text-right">
        <Button
          class="bg-success hover:bg-success/90 text-success-foreground font-semibold"
          :disabled="saving"
          @click="save"
        >
          {{ saving ? '保存中...' : '💾 保存设置' }}
        </Button>
      </div>
    </div>
  </div>
</template>
