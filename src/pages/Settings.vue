<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { safeInvoke } from '../utils/tauri'
import { useToast } from '../composables/useToast'
import { useTheme, type Theme } from '../composables/useTheme'
import { setStoredLocale, type Locale } from '@/i18n'
import pkg from '../../package.json'
import Button from '../components/ui/button/Button.vue'
import Input from '../components/ui/input/Input.vue'
import Label from '../components/ui/label/Label.vue'
import Switch from '../components/ui/switch/Switch.vue'
import Badge from '../components/ui/badge/Badge.vue'
import Card from '../components/ui/card/Card.vue'
import ActionIcon from '../components/ActionIcon.vue'
import Select from '../components/ui/select/Select.vue'
import { cn } from '@/lib/utils'
import SKILL_CONTENT from '../assets/workflow-engine-cli.SKILL.md?raw'

const emit = defineEmits<{ 'back': [] }>()

const { t, locale } = useI18n()
const toast = useToast()
const APP_VERSION = pkg.version
const { theme: currentTheme, setTheme } = useTheme()

const localeOptions: { value: Locale; label: string }[] = [
  { value: 'zh-CN', label: '简体中文' },
  { value: 'en-US', label: 'English' },
]

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
  { value: 'auto', label: 'Auto-detect', desc: 'Priority: Edge → Chrome → Playwright Chromium' },
  { value: 'msedge', label: 'Microsoft Edge', desc: 'Built into Windows' },
  { value: 'chrome', label: 'Google Chrome', desc: 'Requires Chrome browser' },
  { value: 'chromium', label: 'Playwright Chromium', desc: 'Use Playwright bundled Chromium' },
]

const logLevelOptions = [
  { value: 'debug', label: 'Debug' },
  { value: 'info', label: 'Info' },
  { value: 'warn', label: 'Warn' },
  { value: 'error', label: 'Error' },
]

const themeOptions: { value: Theme; label: string; icon: string; desc: string }[] = [
  { value: 'light', label: 'Light', icon: 'Sun', desc: 'Light theme, bright environments' },
  { value: 'dark', label: 'Dark', icon: 'Moon', desc: 'Dark theme, eye comfort' },
  { value: 'system', label: 'System', icon: 'Monitor', desc: 'Follow system preference' },
]

onMounted(async () => {
  loading.value = true
  try {
    const s = await safeInvoke<any>('settings_get').catch(() => ({}))
    settings.value = { ...settings.value, ...(s || {}) }
  } catch (e: any) {
    console.warn('Settings load failed, using defaults:', e)
  }
  try {
    sysInfo.value = await safeInvoke('system_check_browser')
  } catch (e: any) {
    console.warn('System info check failed:', e)
  }
  loading.value = false
})

function setLocale(val: Locale) {
  locale.value = val
  setStoredLocale(val)
  settings.value.language = val
}

async function save() {
  saving.value = true
  try {
    await safeInvoke('settings_update', { settings: settings.value })
    toast.success(t('toast.saved'))
  } catch (e: any) {
    toast.error(t('error.saveFailed') + ': ' + e)
  } finally {
    saving.value = false
  }
}

async function openLogDir() {
  try {
    await safeInvoke('open_log_dir')
  } catch (e: any) {
    toast.error('Open log dir failed: ' + e)
  }
}

async function clearLogs() {
  try {
    await safeInvoke('clear_logs')
    toast.success('Logs cleared')
  } catch (e: any) {
    toast.error('Clear logs failed: ' + e)
  }
}

function truncatePath(path: string, maxLen: number): string {
  if (path.length <= maxLen) return path
  return '...' + path.slice(-(maxLen - 3))
}

function downloadSkill() {
  const blob = new Blob([SKILL_CONTENT], { type: 'text/markdown;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = 'workflow-engine-cli.SKILL.md'
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  URL.revokeObjectURL(url)
  toast.success('SKILL.md downloaded')
}
</script>

<template>
  <div class="max-w-[640px] mx-auto px-5 py-6">
    <!-- Header -->
    <header class="mb-6">
      <Button variant="outline" size="sm" class="mb-2 text-xs" @click="emit('back')">← {{ t('common.back') }}</Button>
      <h1 class="text-xl font-bold text-foreground">{{ t('settingsPage.title') }}</h1>
      <p class="text-sm text-muted-foreground">{{ t('settingsPage.general') }}</p>
    </header>

    <div v-if="loading" class="text-center py-10 text-muted-foreground">{{ t('common.loading') }}</div>

    <div v-else class="space-y-4">
      <!-- Appearance -->
      <Card>
        <div class="p-5">
          <h2 class="text-sm font-semibold text-foreground mb-1.5">🎨 {{ t('settingsPage.appearance') }}</h2>
          <p class="text-xs text-muted-foreground mb-4">{{ t('settingsPage.theme') }}</p>
          <div class="grid grid-cols-3 gap-3">
            <Button
              v-for="opt in themeOptions"
              :key="opt.value"
              variant="outline"
              :aria-pressed="currentTheme === opt.value"
              :class="cn(
                'flex flex-col items-center gap-2 p-4 h-auto border-2',
                currentTheme === opt.value
                  ? 'border-primary bg-primary/5 shadow-sm'
                  : 'border-border hover:border-primary/50',
              )"
              @click="setTheme(opt.value)"
            >
              <ActionIcon :name="opt.icon" cls="w-6 h-6" />
              <span class="text-sm font-semibold text-foreground">{{ opt.label }}</span>
              <span class="text-[10px] text-muted-foreground text-center leading-tight">{{ opt.desc }}</span>
            </Button>
          </div>
        </div>
      </Card>

      <!-- Language -->
      <Card>
        <div class="p-5">
          <h2 class="text-sm font-semibold text-foreground mb-1.5">🌐 {{ t('settingsPage.language') }}</h2>
          <div class="flex gap-2">
            <Button
              v-for="opt in localeOptions"
              :key="opt.value"
              variant="outline"
              size="sm"
              :aria-pressed="locale === opt.value"
              :class="cn(
                'px-4',
                locale === opt.value
                  ? 'border-primary bg-primary/5 text-primary'
                  : 'border-border',
              )"
              @click="setLocale(opt.value)"
            >
              {{ opt.label }}
            </Button>
          </div>
        </div>
      </Card>

      <!-- Browser settings -->
      <Card>
        <div class="p-5">
          <h2 class="text-sm font-semibold text-foreground mb-1.5">{{ t('settingsPage.browserNode') }}</h2>
          <p class="text-xs text-muted-foreground mb-4">Select browser for automation. Edge recommended for intranet.</p>

          <div class="space-y-2 mb-4">
            <Label class="text-xs text-muted-foreground font-semibold">Browser channel</Label>
            <div class="flex flex-col gap-2" role="radiogroup" aria-label="Browser channel">
              <button
                v-for="(opt, idx) in browserOptions"
                :key="opt.value"
                role="radio"
                :aria-checked="settings.browser_channel === opt.value"
                :tabindex="settings.browser_channel === opt.value ? 0 : -1"
                :class="cn(
                  'flex items-start gap-2.5 px-3 py-2.5 border rounded-md cursor-pointer transition-colors text-left',
                  settings.browser_channel === opt.value
                    ? 'border-primary bg-primary/5'
                    : 'border-border hover:border-primary',
                )"
                @click="settings.browser_channel = opt.value"
                @keydown.up.prevent="browserOptions[(idx - 1 + browserOptions.length) % browserOptions.length].value !== opt.value && (settings.browser_channel = browserOptions[(idx - 1 + browserOptions.length) % browserOptions.length].value)"
                @keydown.down.prevent="browserOptions[(idx + 1) % browserOptions.length].value !== opt.value && (settings.browser_channel = browserOptions[(idx + 1) % browserOptions.length].value)"
              >
                <div :class="cn('w-3.5 h-3.5 rounded-full border-2 flex items-center justify-center mt-0.5 shrink-0', settings.browser_channel === opt.value ? 'border-primary bg-primary' : 'border-muted-foreground/40')">
                  <div v-if="settings.browser_channel === opt.value" class="w-1.5 h-1.5 rounded-full bg-primary-foreground" />
                </div>
                <div class="flex flex-col gap-0.5">
                  <span class="text-sm text-foreground font-semibold">{{ opt.label }}</span>
                  <span class="text-[11px] text-muted-foreground">{{ opt.desc }}</span>
                </div>
              </button>
            </div>
          </div>

          <!-- System check -->
          <div v-if="sysInfo" class="mt-4 p-3 bg-background rounded-md">
            <h3 class="text-xs text-muted-foreground mb-2.5 flex items-center gap-2">
              Environment
              <Badge :variant="sysInfo.ready ? 'success' : 'warning'" class="text-[10px]">
                {{ sysInfo.ready ? '✓ Ready' : '⊘ Not set up' }}
              </Badge>
            </h3>
            <div class="flex flex-col gap-1.5">
              <!-- Python -->
              <div class="flex justify-between items-center text-xs">
                <span class="text-foreground">Python</span>
                <span :class="sysInfo.python_available ? 'text-success' : 'text-danger'">
                  {{ sysInfo.python_available ? '✓ detected' : '✗ not found' }}
                </span>
              </div>
              <div v-if="sysInfo.system_python" class="flex justify-between items-center text-xs">
                <span class="text-foreground pl-3">↳ Path</span>
                <span class="text-success text-[11px] truncate max-w-[200px]" :title="sysInfo.system_python">{{ truncatePath(sysInfo.system_python, 40) }}</span>
              </div>
              <div v-if="!sysInfo.python_available" class="text-xs text-destructive">
                {{ t('settingsPage.installPython') }}
                <a href="https://www.python.org/downloads/" target="_blank" class="text-primary ml-1 hover:underline">Download</a>
              </div>

              <!-- Playwright -->
              <div class="flex justify-between items-center text-xs">
                <span class="text-foreground">Playwright</span>
                <span :class="sysInfo.has_playwright_pkg ? 'text-success' : 'text-muted-foreground'">
                  {{ sysInfo.has_playwright_pkg ? '✓ installed' : '◷ auto-install on first use' }}
                </span>
              </div>

              <!-- Browser -->
              <div class="flex justify-between items-center text-xs">
                <span class="text-foreground">Browser</span>
                <span :class="sysInfo.has_browser ? 'text-success' : 'text-muted-foreground'">
                  {{ sysInfo.has_browser ? '✓ available' : t('settingsPage.autoDownloadNote') }}
                </span>
              </div>
              <div v-if="sysInfo.has_system_browser" class="flex justify-between items-center text-xs">
                <span class="text-foreground pl-3">{{ t('settingsPage.systemBrowser') }}</span>
                <span class="text-success text-[11px]">
                  {{ [sysInfo.has_edge ? 'Edge' : '', sysInfo.has_chrome ? 'Chrome' : ''].filter(Boolean).join(' + ') }}{{ t('settingsPage.preferred') }}
                </span>
              </div>
              <div v-if="sysInfo.has_playwright_chromium" class="flex justify-between items-center text-xs">
                <span class="text-foreground pl-3">↳ Bundled Chromium</span>
                <span class="text-success text-[11px]">✓ bundled</span>
              </div>
              <div v-if="sysInfo.has_playwright_cache" class="flex justify-between items-center text-xs">
                <span class="text-foreground pl-3">{{ t('settingsPage.playwrightCache') }}</span>
                <span class="text-success text-[11px]">✓ downloaded</span>
              </div>
            </div>
          </div>
        </div>
      </Card>

      <!-- Advanced settings -->
      <Card>
        <div class="p-5">
          <h2 class="text-sm font-semibold text-foreground mb-4">{{ t('settingsPage.advanced') }}</h2>

          <div class="space-y-4">
            <div class="space-y-1.5">
              <Label class="text-xs text-muted-foreground font-semibold">Python Path (Optional)</Label>
              <Input v-model="settings.python_path" :placeholder="t('settingsPage.pythonPathPlaceholder')" class="h-8 text-xs" />
              <p class="text-[11px] text-muted-foreground/60">{{ t('settingsPage.pythonPathHint') }}</p>
            </div>

            <div class="space-y-1.5">
              <Label class="text-xs text-muted-foreground font-semibold">Log level</Label>
              <Select
                :model-value="settings.log_level"
                @update:model-value="v => settings.log_level = v"
                :options="logLevelOptions"
              />
            </div>

            <div class="flex items-center gap-2.5">
              <Switch v-model="settings.auto_start" />
              <Label class="text-sm text-foreground cursor-pointer">Auto-start</Label>
            </div>
          </div>
        </div>
      </Card>

      <!-- Agent integration -->
      <Card>
        <div class="p-5">
          <h2 class="text-sm font-semibold text-foreground mb-1.5">{{ t('settingsPage.agentIntegration') }}</h2>
          <p class="text-xs text-muted-foreground mb-4">
            {{ t('settingsPage.agentDesc') }}
          </p>

          <!-- CLI command preview -->
          <div class="bg-muted rounded-md p-3 mb-4 font-mono text-xs space-y-1">
            <div class="text-muted-foreground">{{ t('settingsPage.cliComment1') }}</div>
            <div class="text-foreground">wf-cli list --json</div>
            <div class="text-muted-foreground mt-2">{{ t('settingsPage.cliComment2') }}</div>
            <div class="text-foreground">wf-cli run &lt;id&gt; --var url=https://example.com</div>
            <div class="text-muted-foreground mt-2">{{ t('settingsPage.cliComment3') }}</div>
            <div class="text-foreground">wf-cli status &lt;run_id&gt; --json</div>
            <div class="text-muted-foreground mt-2">{{ t('settingsPage.cliComment4') }}</div>
            <div class="text-foreground">wf-cli schedule list --json</div>
          </div>

          <p class="text-xs text-muted-foreground mb-4">
            {{ t('settingsPage.cliDocNote') }}
          </p>

          <Button variant="outline" size="sm" @click="downloadSkill">{{ t('settingsPage.downloadSkill') }}</Button>
        </div>
      </Card>

      <!-- Log management -->
      <Card>
        <div class="p-5">
          <h2 class="text-sm font-semibold text-foreground mb-1.5">{{ t('settingsPage.logSection') }}</h2>
          <p class="text-xs text-muted-foreground mb-4">{{ t('settingsPage.logHint') }}</p>
          <div class="flex gap-2.5 flex-wrap">
            <Button variant="outline" size="sm" @click="openLogDir">{{ t('settingsPage.viewLogFile') }}</Button>
            <Button variant="outline" size="sm" class="text-destructive border-destructive/30 hover:bg-destructive/10" @click="clearLogs">{{ t('settingsPage.clearLogs') }}</Button>
          </div>
        </div>
      </Card>

      <!-- Version info -->
      <Card>
        <div class="p-5">
          <h2 class="text-sm font-semibold text-foreground mb-1.5">{{ t('settingsPage.versionInfo') }}</h2>
          <p class="text-xs text-muted-foreground mb-4">{{ t('settingsPage.versionHint') }}</p>
          <div class="mb-4">
            <Badge variant="default" class="text-sm px-3 py-1">v{{ APP_VERSION }}</Badge>
          </div>
          <h3 class="text-sm text-foreground mb-2">{{ t('settingsPage.changelog') }}</h3>
          <div class="space-y-0">
            <div v-for="(item, i) in [
              { version: 'v6.9.0', desc: '修复 runCondition 被 parser 抹除 · cursor/loop items 变量解析 · 模板库前端浏览+实例化 · IPC 守护进程状态指示' },
              { version: 'v6.8.0', desc: 'Shell Command 节点(万能原语) · File Operations 统一容器(10操作+grep) · 节点标准化 · ABCD 四轮交付(35测试)' },
              { version: 'v6.7.0', desc: 'CLI 执行器升级：支持条件分支、错误恢复(Ignore/Branch)、重试机制、步骤延迟、游标迭代 · Import 读取工作流名称' },
              { version: 'v6.6.0', desc: 'GitHub 迁移 · 项目结构整理 · CLI 双模入口(独立二进制) · 调度管理' },
              { version: 'v6.5.0', desc: 'Browser容器新增 8 种动作：上传文件/键盘操作/双击/拖拽/右键菜单/iframe切换/弹窗处理/滚动到元素' },
              { version: 'v6.4.0', desc: '生产风险修复：启动清理/事务保护/HTTP超时/空选择器校验/整体超时 · 帮助文档' },
              { version: 'v6.3.0', desc: '变量选择器改版（树形分组+点击插入）· 容器内数据流可视化' },
              { version: 'v6.2.0', desc: '引用系统简化（短ID+稳定引用+端口key统一）' },
              { version: 'v6.1.1', desc: '审批系统重构（channel暂停/恢复+推荐选项+全局审批队列）· SQLite 持久化' },
              { version: 'v5.1.1', desc: 'shadcn-vue 全组件化 · 浅色/深色主题切换 · 单页 Sidebar 布局 · 动作行 Card 重设计' },
              { version: 'v5.1.0', desc: 'v5 步骤编辑器正式版 · shadcn-vue 组件体系 · 容器模板系统 · 多容器类型' },
              { version: 'v5.0', desc: '去掉 LiteGraph · 自研步骤编辑器 · Steps→Actions 模型 · Vue Draggable' },
              { version: 'v2.x', desc: 'Grid 布局 · LiteGraph 画布 · 模板系统 · Browser自动化' },
              { version: 'v1.x', desc: 'YAML 工作流引擎原型 · Web 前端 · Playwright 自动化' },
            ]" :key="item.version"
              class="text-xs text-muted-foreground py-1.5"
              :class="i < 11 ? 'border-b border-border' : ''"
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
          {{ saving ? t('settingsPage.saving') : t('settingsPage.saveSettings') }}
        </Button>
      </div>
    </div>
  </div>
</template>
