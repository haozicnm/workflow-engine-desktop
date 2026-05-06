<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { safeInvoke } from '../utils/tauri'
import { useToast } from '../composables/useToast'
import Button from './ui/button/Button.vue'
import Input from './ui/input/Input.vue'
import Label from './ui/label/Label.vue'
import Switch from './ui/switch/Switch.vue'
import Badge from './ui/badge/Badge.vue'
import Select from './ui/select/Select.vue'
import Separator from './ui/separator/Separator.vue'
import { cn } from '@/lib/utils'

const emit = defineEmits<{ close: [] }>()
const toast = useToast()

const settings = ref({
  theme: 'system',
  language: 'zh-CN',
  auto_start: false,
  log_level: 'info',
  python_path: '',
  browser_channel: 'auto',
  working_dir: '',
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

async function browseWorkingDir() {
  try {
    const result = await safeInvoke<string | null>('dialog_open_dir')
    if (result) {
      settings.value.working_dir = result
    }
  } catch {
    // dialog may not be available
  }
}
</script>

<template>
  <div class="w-[360px] h-full bg-card border-l border-border flex flex-col overflow-hidden">
    <!-- Header -->
    <div class="flex items-center justify-between px-4 py-3.5 border-b border-border shrink-0">
      <span class="text-sm font-semibold text-foreground">⚙️ 设置</span>
      <Button variant="ghost" size="icon" class="h-7 w-7" @click="emit('close')">×</Button>
    </div>

    <div v-if="loading" class="text-center text-muted-foreground py-10">加载中...</div>

    <div v-else class="flex-1 overflow-y-auto px-4 py-3 space-y-5">
      <!-- File directory -->
      <section>
        <h3 class="text-sm text-foreground font-medium mb-3">📁 文件目录</h3>
        <div class="space-y-1.5">
          <Label class="text-xs text-muted-foreground font-semibold">工作目录</Label>
          <div class="flex gap-1">
            <Input v-model="settings.working_dir" placeholder="留空使用默认目录" class="flex-1 h-8 text-xs" />
            <Button variant="outline" size="sm" class="h-8 w-8 p-0 shrink-0" @click="browseWorkingDir" title="浏览...">📂</Button>
          </div>
          <p class="text-[11px] text-muted-foreground/60">模板、导出文件等存储位置。留空则使用程序默认目录。</p>
        </div>
      </section>

      <Separator />

      <!-- Browser settings -->
      <section>
        <h3 class="text-sm text-foreground font-medium mb-3">🌐 浏览器</h3>
        <div class="space-y-1.5">
          <Label class="text-xs text-muted-foreground font-semibold">浏览器通道</Label>
          <div class="flex flex-col gap-1.5">
            <label
              v-for="opt in browserOptions"
              :key="opt.value"
              :class="cn(
                'flex items-start gap-2 px-2.5 py-2 border rounded-md cursor-pointer transition-colors',
                settings.browser_channel === opt.value
                  ? 'border-primary bg-primary/5'
                  : 'border-border hover:border-primary',
              )"
            >
              <div :class="cn('w-3.5 h-3.5 rounded-full border-2 flex items-center justify-center mt-0.5 shrink-0', settings.browser_channel === opt.value ? 'border-primary bg-primary' : 'border-muted-foreground/40')">
                <div v-if="settings.browser_channel === opt.value" class="w-1.5 h-1.5 rounded-full bg-primary-foreground" />
              </div>
              <div>
                <span class="text-xs text-foreground font-semibold block">{{ opt.label }}</span>
                <span class="text-[11px] text-muted-foreground">{{ opt.desc }}</span>
              </div>
            </label>
          </div>
        </div>

        <!-- System check -->
        <div v-if="sysInfo" class="mt-3 p-2.5 bg-background rounded-md">
          <div class="flex justify-between items-center py-1 text-xs text-foreground">
            <span>Python</span>
            <span :class="sysInfo.python_available ? 'text-success' : 'text-danger'">
              {{ sysInfo.python_available ? '✅' : '❌' }}
            </span>
          </div>
          <div class="flex justify-between items-center py-1 text-xs text-foreground">
            <span>Playwright</span>
            <span :class="sysInfo.has_playwright_pkg ? 'text-success' : 'text-muted-foreground'">
              {{ sysInfo.has_playwright_pkg ? '✅' : '⏳ 首次自动安装' }}
            </span>
          </div>
          <div class="flex justify-between items-center py-1 text-xs text-foreground">
            <span>浏览器</span>
            <span :class="sysInfo.has_browser ? 'text-success' : 'text-muted-foreground'">
              {{ sysInfo.has_browser ? '✅' : '—' }}
            </span>
          </div>
        </div>
      </section>

      <Separator />

      <!-- Advanced -->
      <section>
        <h3 class="text-sm text-foreground font-medium mb-3">🔧 高级</h3>
        <div class="space-y-3">
          <div class="space-y-1.5">
            <Label class="text-xs text-muted-foreground font-semibold">Python 路径</Label>
            <Input v-model="settings.python_path" placeholder="留空自动检测" class="h-8 text-xs" />
          </div>
          <div class="space-y-1.5">
            <Label class="text-xs text-muted-foreground font-semibold">日志级别</Label>
            <Select
              :model-value="settings.log_level"
              @update:model-value="v => settings.log_level = v"
              :options="logLevelOptions"
            />
          </div>
          <div class="flex items-center gap-2">
            <Switch v-model="settings.auto_start" />
            <Label class="text-sm text-foreground cursor-pointer">开机自启</Label>
          </div>
        </div>
      </section>
    </div>

    <!-- Footer -->
    <div class="px-4 py-3 border-t border-border flex justify-end shrink-0">
      <Button
        variant="default"
        class="bg-[#238636] hover:bg-[#2ea043] text-white text-xs font-semibold"
        :disabled="saving"
        @click="save"
      >
        {{ saving ? '保存中...' : '💾 保存' }}
      </Button>
    </div>
  </div>
</template>
