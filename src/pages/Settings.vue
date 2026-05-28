<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { safeInvoke } from '../utils/tauri'
import { useToast } from '../composables/useToast'
import { useTheme, type Theme } from '../composables/useTheme'
import { setStoredLocale, type Locale } from '@/i18n'
import pkg from '../../package.json'
import changelogData from '../assets/changelog.json'
import Button from '../components/ui/button/Button.vue'
import Input from '../components/ui/input/Input.vue'
import Label from '../components/ui/label/Label.vue'
import Switch from '../components/ui/switch/Switch.vue'
import Badge from '../components/ui/badge/Badge.vue'
import Card from '../components/ui/card/Card.vue'
import CardHeader from '../components/ui/card/CardHeader.vue'
import CardTitle from '../components/ui/card/CardTitle.vue'
import CardDescription from '../components/ui/card/CardDescription.vue'
import CardContent from '../components/ui/card/CardContent.vue'
import Separator from '../components/ui/separator/Separator.vue'
import ActionIcon from '../components/ActionIcon.vue'
import Select from '../components/ui/select/Select.vue'
import { cn } from '@/lib/utils'
import SKILL_CONTENT from '../assets/workflow-engine-cli.SKILL.md?raw'

const emit = defineEmits<{ 'back': [] }>()

const { t, locale } = useI18n()
const toast = useToast()
const APP_VERSION = pkg.version
const { theme: currentTheme, setTheme } = useTheme()

const localeOptions = computed<{ value: Locale; label: string }[]>(() => [
  { value: 'zh-CN', label: t('settingsPage.langZh') },
  { value: 'en-US', label: t('settingsPage.langEn') },
])

// ── Settings data ──
interface TimeoutCfg { http_request_ms: number; browser_page_ms: number; workflow_total_ms: number; node_exec_ms: number }
interface LogCfg { max_size_mb: number; max_files: number; auto_clean_days: number }
interface ExecCfg { max_concurrent_runs: number; default_retries: number; retry_delay_ms: number }

const settings = ref({
  theme: 'system',
  language: 'zh-CN',
  auto_start: false,
  log_level: 'info',
  python_path: '',
  browser_channel: 'auto',
  browser_executable_path: '',
  working_dir: '',
  timeouts: { http_request_ms: 30000, browser_page_ms: 60000, workflow_total_ms: 600000, node_exec_ms: 120000 } as TimeoutCfg,
  logging: { max_size_mb: 50, max_files: 10, auto_clean_days: 30 } as LogCfg,
  execution: { max_concurrent_runs: 3, default_retries: 0, retry_delay_ms: 1000 } as ExecCfg,
})

const savedSnapshot = ref('')
const isDirty = computed(() => {
  try { return JSON.stringify(settings.value) !== savedSnapshot.value } catch { return false }
})

const sysInfo = ref<any>(null)
const saving = ref(false)
const loading = ref(true)
const changelog = changelogData as { version: string; desc: string }[]
const showChangelog = ref(false)

const browserOptions = computed(() => [
  { value: 'auto', label: t('settingsPage.browserAuto'), desc: t('settingsPage.browserAutoDesc') },
  { value: 'msedge', label: t('settingsPage.browserEdge'), desc: t('settingsPage.browserEdgeDesc') },
  { value: 'chrome', label: t('settingsPage.browserChrome'), desc: t('settingsPage.browserChromeDesc') },
  { value: 'chromium', label: t('settingsPage.browserChromium'), desc: t('settingsPage.browserChromiumDesc') },
])

const logLevelOptions = computed(() => [
  { value: 'debug', label: t('settingsPage.logDebug') },
  { value: 'info', label: t('settingsPage.logInfo') },
  { value: 'warn', label: t('settingsPage.logWarn') },
  { value: 'error', label: t('settingsPage.logError') },
])

const themeOptions = computed<{ value: Theme; label: string; icon: string; desc: string }[]>(() => [
  { value: 'light', label: t('settingsPage.themeLight'), icon: 'Sun', desc: t('settingsPage.themeLightDesc') },
  { value: 'dark', label: t('settingsPage.themeDark'), icon: 'Moon', desc: t('settingsPage.themeDarkDesc') },
  { value: 'system', label: t('settingsPage.themeSystem'), icon: 'Monitor', desc: t('settingsPage.themeSystemDesc') },
])

onMounted(async () => {
  loading.value = true
  try {
    const s = await safeInvoke<any>('settings_get').catch(() => ({}))
    settings.value = { ...settings.value, ...(s || {}) }
    if (s?.timeouts) settings.value.timeouts = { ...settings.value.timeouts, ...s.timeouts }
    if (s?.logging) settings.value.logging = { ...settings.value.logging, ...s.logging }
    if (s?.execution) settings.value.execution = { ...settings.value.execution, ...s.execution }
    savedSnapshot.value = JSON.stringify(settings.value)
  } catch (e: any) { console.warn('Settings load failed:', e) }
  try { sysInfo.value = await safeInvoke('system_check_browser') } catch (e: any) { console.warn('System check failed:', e) }
  loading.value = false
})

function setLocale(val: Locale) { locale.value = val; setStoredLocale(val); settings.value.language = val }

async function save() {
  saving.value = true
  try {
    await safeInvoke('settings_update', { settings: settings.value })
    savedSnapshot.value = JSON.stringify(settings.value)
    toast.success(t('toast.saved'))
  } catch (e: any) { toast.error(t('error.saveFailed') + ': ' + e) }
  finally { saving.value = false }
}

async function toggleAutoStart() { settings.value.auto_start = !settings.value.auto_start; await save() }
async function openLogDir() { try { await safeInvoke('open_log_dir') } catch (e: any) { toast.error(t('settingsPage.logOpenFailed') + ': ' + e) } }
async function clearLogs() { try { await safeInvoke('clear_logs'); toast.success(t('settingsPage.logCleared')) } catch (e: any) { toast.error(t('settingsPage.logClearFailed') + ': ' + e) } }
function truncatePath(p: string, n: number) { return p.length <= n ? p : '...' + p.slice(-(n - 3)) }
function downloadSkill() {
  const a = document.createElement('a')
  a.href = URL.createObjectURL(new Blob([SKILL_CONTENT], { type: 'text/markdown' }))
  a.download = 'workflow-engine-cli.SKILL.md'; document.body.appendChild(a); a.click()
  document.body.removeChild(a); toast.success(t('settingsPage.skillDownloaded'))
}
function resetTimeouts() { settings.value.timeouts = { http_request_ms: 30000, browser_page_ms: 60000, workflow_total_ms: 600000, node_exec_ms: 120000 } }
function resetLogging() { settings.value.logging = { max_size_mb: 50, max_files: 10, auto_clean_days: 30 } }
function resetExecution() { settings.value.execution = { max_concurrent_runs: 3, default_retries: 0, retry_delay_ms: 1000 } }
</script>

<template>
  <div class="max-w-[640px] mx-auto px-5 py-6 pb-20">
    <!-- Header -->
    <header class="mb-6">
      <Button variant="outline" size="sm" class="mb-2 text-xs gap-1" @click="emit('back')">
        <ActionIcon name="ArrowLeft" cls="w-3.5 h-3.5" />
        {{ t('common.back') }}
      </Button>
      <h1 class="text-xl font-bold text-foreground">{{ t('settingsPage.title') }}</h1>
      <p class="text-sm text-muted-foreground">{{ t('settingsPage.general') }}</p>
    </header>

    <div v-if="loading" class="text-center py-10 text-muted-foreground">{{ t('common.loading') }}</div>

    <div v-else class="space-y-4">
      <!-- ═══ Appearance ═══ -->
      <Card>
        <CardHeader class="flex flex-row items-center gap-2 p-4 pb-3">
          <ActionIcon name="Palette" cls="w-4 h-4 text-muted-foreground" />
          <div class="flex-1">
            <CardTitle class="text-sm">{{ t('settingsPage.appearance') }}</CardTitle>
            <CardDescription class="text-xs">{{ t('settingsPage.theme') }}</CardDescription>
          </div>
        </CardHeader>
        <CardContent class="px-4 pb-4 pt-0">
          <div class="grid grid-cols-3 gap-3">
            <Button
              v-for="opt in themeOptions" :key="opt.value"
              variant="outline"
              :aria-pressed="currentTheme === opt.value"
              :class="cn(
                'flex flex-col items-center gap-2 p-4 h-auto border-2',
                currentTheme === opt.value ? 'border-primary bg-primary/5 shadow-sm' : 'border-border hover:border-primary/50',
              )"
              @click="setTheme(opt.value)"
            >
              <ActionIcon :name="opt.icon" cls="w-6 h-6" />
              <span class="text-sm font-semibold text-foreground">{{ opt.label }}</span>
              <span class="text-[10px] text-muted-foreground text-center leading-tight">{{ opt.desc }}</span>
            </Button>
          </div>
        </CardContent>
      </Card>

      <!-- ═══ Language ═══ -->
      <Card>
        <CardHeader class="flex flex-row items-center gap-2 p-4 pb-3">
          <ActionIcon name="Globe" cls="w-4 h-4 text-muted-foreground" />
          <CardTitle class="flex-1 text-sm">{{ t('settingsPage.language') }}</CardTitle>
        </CardHeader>
        <CardContent class="px-4 pb-4 pt-0">
          <div class="flex gap-2">
            <Button
              v-for="opt in localeOptions" :key="opt.value"
              variant="outline" size="sm"
              :aria-pressed="locale === opt.value"
              :class="cn('px-4', locale === opt.value ? 'border-primary bg-primary/5 text-primary' : 'border-border')"
              @click="setLocale(opt.value)"
            >{{ opt.label }}</Button>
          </div>
        </CardContent>
      </Card>

      <!-- ═══ Browser ═══ -->
      <Card>
        <CardHeader class="flex flex-row items-center gap-2 p-4 pb-3">
          <ActionIcon name="Globe" cls="w-4 h-4 text-muted-foreground" />
          <div class="flex-1">
            <CardTitle class="text-sm">{{ t('settingsPage.browserNode') }}</CardTitle>
            <CardDescription class="text-xs">{{ t('settingsPage.browserAutoDesc') }}</CardDescription>
          </div>
        </CardHeader>
        <CardContent class="px-4 pb-4 pt-0 space-y-4">
          <!-- Channel selection -->
          <div class="space-y-2">
            <Label class="text-xs text-muted-foreground font-semibold">{{ t('settingsPage.browserNode') }}</Label>
            <div class="flex flex-col gap-2" role="radiogroup" aria-label="Browser channel">
              <button
                v-for="(opt, idx) in browserOptions" :key="opt.value"
                role="radio" :aria-checked="settings.browser_channel === opt.value"
                :tabindex="settings.browser_channel === opt.value ? 0 : -1"
                :class="cn(
                  'flex items-start gap-2.5 px-3 py-2.5 border rounded-md cursor-pointer transition-colors text-left',
                  settings.browser_channel === opt.value ? 'border-primary bg-primary/5' : 'border-border hover:border-primary',
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

          <!-- Browser executable path -->
          <div class="space-y-1.5">
            <Label class="text-xs text-muted-foreground font-semibold">{{ t('settingsPage.browserExecPath') }}</Label>
            <Input v-model="settings.browser_executable_path" :placeholder="t('settingsPage.browserExecPathPlaceholder')" class="h-8 text-xs" />
            <p class="text-[11px] text-muted-foreground/60">{{ t('settingsPage.browserExecPathHint') }}</p>
          </div>

          <!-- System check -->
          <div v-if="sysInfo" class="p-3 bg-muted/50 rounded-md space-y-1.5">
            <h3 class="text-xs text-muted-foreground flex items-center gap-2">
              {{ t('settingsPage.envBrowser') }}
              <Badge :variant="sysInfo.ready ? 'success' : 'warning'" class="text-[10px]">
                {{ sysInfo.ready ? t('settingsPage.envReady') : t('settingsPage.envNotReady') }}
              </Badge>
            </h3>
            <div class="flex flex-col gap-1.5">
              <div class="flex justify-between items-center text-xs">
                <span class="text-foreground">{{ t('settingsPage.envPython') }}</span>
                <span :class="sysInfo.python_available ? 'text-success' : 'text-danger'">
                  {{ sysInfo.python_available ? t('settingsPage.envDetected') : t('settingsPage.envNotFound') }}
                </span>
              </div>
              <div v-if="sysInfo.system_python" class="flex justify-between items-center text-xs">
                <span class="text-foreground pl-3">{{ t('settingsPage.envPath') }}</span>
                <span class="text-success text-[11px] truncate max-w-[200px]" :title="sysInfo.system_python">{{ truncatePath(sysInfo.system_python, 40) }}</span>
              </div>
              <div v-if="!sysInfo.python_available" class="text-xs text-destructive">
                {{ t('settingsPage.installPython') }}
                <a href="https://www.python.org/downloads/" target="_blank" class="text-primary ml-1 hover:underline">{{ t('settingsPage.downloadLink') }}</a>
              </div>
              <div class="flex justify-between items-center text-xs">
                <span class="text-foreground">{{ t('settingsPage.envPlaywright') }}</span>
                <span :class="sysInfo.has_playwright_pkg ? 'text-success' : 'text-muted-foreground'">
                  {{ sysInfo.has_playwright_pkg ? t('settingsPage.envInstalled') : t('settingsPage.envAutoInstall') }}
                </span>
              </div>
              <div class="flex justify-between items-center text-xs">
                <span class="text-foreground">{{ t('settingsPage.envBrowser') }}</span>
                <span :class="sysInfo.has_browser ? 'text-success' : 'text-muted-foreground'">
                  {{ sysInfo.has_browser ? t('settingsPage.envAvailable') : t('settingsPage.autoDownloadNote') }}
                </span>
              </div>
              <div v-if="sysInfo.has_system_browser" class="flex justify-between items-center text-xs">
                <span class="text-foreground pl-3">{{ t('settingsPage.systemBrowser') }}</span>
                <span class="text-success text-[11px]">
                  {{ [sysInfo.has_edge ? 'Edge' : '', sysInfo.has_chrome ? 'Chrome' : ''].filter(Boolean).join(' + ') }}{{ t('settingsPage.preferred') }}
                </span>
              </div>
              <div v-if="sysInfo.has_playwright_chromium" class="flex justify-between items-center text-xs">
                <span class="text-foreground pl-3">{{ t('settingsPage.envBundledChromium') }}</span>
                <span class="text-success text-[11px]">{{ t('settingsPage.envBundled') }}</span>
              </div>
              <div v-if="sysInfo.has_playwright_cache" class="flex justify-between items-center text-xs">
                <span class="text-foreground pl-3">{{ t('settingsPage.playwrightCache') }}</span>
                <span class="text-success text-[11px]">{{ t('settingsPage.envDownloaded') }}</span>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      <!-- ═══ Execution Engine ═══ -->
      <Card>
        <CardHeader class="flex flex-row items-center gap-2 p-4 pb-3">
          <ActionIcon name="Settings" cls="w-4 h-4 text-muted-foreground" />
          <div class="flex-1">
            <CardTitle class="text-sm">{{ t('settingsPage.executionEngine') }}</CardTitle>
            <CardDescription class="text-xs">{{ t('settingsPage.executionEngineDesc') }}</CardDescription>
          </div>
        </CardHeader>
        <CardContent class="px-4 pb-4 pt-0 space-y-4">
          <!-- Timeouts -->
          <div class="space-y-3">
            <h3 class="text-xs text-muted-foreground font-semibold flex items-center gap-1.5">
              <ActionIcon name="Clock" cls="w-3.5 h-3.5" />
              {{ t('settingsPage.timeouts') }}
            </h3>
            <div class="grid grid-cols-2 gap-3">
              <div class="space-y-1">
                <Label class="text-[11px] text-muted-foreground">{{ t('settingsPage.timeoutHttpRequest') }}</Label>
                <div class="flex items-center gap-1.5">
                  <Input type="number" v-model.number="settings.timeouts.http_request_ms" class="h-8 text-xs" min="1000" step="1000" />
                  <span class="text-[11px] text-muted-foreground whitespace-nowrap">ms</span>
                </div>
              </div>
              <div class="space-y-1">
                <Label class="text-[11px] text-muted-foreground">{{ t('settingsPage.timeoutBrowserPage') }}</Label>
                <div class="flex items-center gap-1.5">
                  <Input type="number" v-model.number="settings.timeouts.browser_page_ms" class="h-8 text-xs" min="1000" step="1000" />
                  <span class="text-[11px] text-muted-foreground whitespace-nowrap">ms</span>
                </div>
              </div>
              <div class="space-y-1">
                <Label class="text-[11px] text-muted-foreground">{{ t('settingsPage.timeoutWorkflowTotal') }}</Label>
                <div class="flex items-center gap-1.5">
                  <Input type="number" v-model.number="settings.timeouts.workflow_total_ms" class="h-8 text-xs" min="0" step="60000" />
                  <span class="text-[11px] text-muted-foreground whitespace-nowrap">ms</span>
                </div>
              </div>
              <div class="space-y-1">
                <Label class="text-[11px] text-muted-foreground">{{ t('settingsPage.timeoutNodeExec') }}</Label>
                <div class="flex items-center gap-1.5">
                  <Input type="number" v-model.number="settings.timeouts.node_exec_ms" class="h-8 text-xs" min="1000" step="1000" />
                  <span class="text-[11px] text-muted-foreground whitespace-nowrap">ms</span>
                </div>
              </div>
            </div>
            <p class="text-[11px] text-muted-foreground/60">{{ t('settingsPage.timeoutZeroHint') }}</p>
            <Button variant="ghost" size="sm" class="text-xs gap-1" @click="resetTimeouts">
              <ActionIcon name="RotateCcw" cls="w-3.5 h-3.5" />
              {{ t('settingsPage.resetDefaults') }}
            </Button>
          </div>

          <Separator />

          <!-- Concurrency & Retry -->
          <div class="space-y-3">
            <h3 class="text-xs text-muted-foreground font-semibold flex items-center gap-1.5">
              <ActionIcon name="Repeat" cls="w-3.5 h-3.5" />
              {{ t('settingsPage.concurrencyRetry') }}
            </h3>
            <div class="grid grid-cols-3 gap-3">
              <div class="space-y-1">
                <Label class="text-[11px] text-muted-foreground">{{ t('settingsPage.maxConcurrentRuns') }}</Label>
                <Input type="number" v-model.number="settings.execution.max_concurrent_runs" class="h-8 text-xs" min="1" max="10" />
              </div>
              <div class="space-y-1">
                <Label class="text-[11px] text-muted-foreground">{{ t('settingsPage.defaultRetries') }}</Label>
                <Input type="number" v-model.number="settings.execution.default_retries" class="h-8 text-xs" min="0" max="10" />
              </div>
              <div class="space-y-1">
                <Label class="text-[11px] text-muted-foreground">{{ t('settingsPage.retryDelay') }}</Label>
                <div class="flex items-center gap-1.5">
                  <Input type="number" v-model.number="settings.execution.retry_delay_ms" class="h-8 text-xs" min="100" step="100" />
                  <span class="text-[11px] text-muted-foreground whitespace-nowrap">ms</span>
                </div>
              </div>
            </div>
            <Button variant="ghost" size="sm" class="text-xs gap-1" @click="resetExecution">
              <ActionIcon name="RotateCcw" cls="w-3.5 h-3.5" />
              {{ t('settingsPage.resetDefaults') }}
            </Button>
          </div>
        </CardContent>
      </Card>

      <!-- ═══ Log Management ═══ -->
      <Card>
        <CardHeader class="flex flex-row items-center gap-2 p-4 pb-3">
          <ActionIcon name="FileText" cls="w-4 h-4 text-muted-foreground" />
          <div class="flex-1">
            <CardTitle class="text-sm">{{ t('settingsPage.logSection') }}</CardTitle>
            <CardDescription class="text-xs">{{ t('settingsPage.logHint') }}</CardDescription>
          </div>
        </CardHeader>
        <CardContent class="px-4 pb-4 pt-0 space-y-4">
          <div class="flex items-center gap-2">
            <Label class="text-xs text-muted-foreground font-semibold min-w-[80px]">{{ t('settingsPage.logLevel') }}</Label>
            <Select :model-value="settings.log_level" @update:model-value="v => settings.log_level = v" :options="logLevelOptions" />
          </div>
          <div class="grid grid-cols-3 gap-3">
            <div class="space-y-1">
              <Label class="text-[11px] text-muted-foreground">{{ t('settingsPage.logMaxSize') }}</Label>
              <div class="flex items-center gap-1.5">
                <Input type="number" v-model.number="settings.logging.max_size_mb" class="h-8 text-xs" min="1" max="500" />
                <span class="text-[11px] text-muted-foreground whitespace-nowrap">MB</span>
              </div>
            </div>
            <div class="space-y-1">
              <Label class="text-[11px] text-muted-foreground">{{ t('settingsPage.logMaxFiles') }}</Label>
              <Input type="number" v-model.number="settings.logging.max_files" class="h-8 text-xs" min="1" max="100" />
            </div>
            <div class="space-y-1">
              <Label class="text-[11px] text-muted-foreground">{{ t('settingsPage.logAutoCleanDays') }}</Label>
              <div class="flex items-center gap-1.5">
                <Input type="number" v-model.number="settings.logging.auto_clean_days" class="h-8 text-xs" min="1" max="365" />
                <span class="text-[11px] text-muted-foreground whitespace-nowrap">{{ t('settingsPage.days') }}</span>
              </div>
            </div>
          </div>
          <Button variant="ghost" size="sm" class="text-xs gap-1" @click="resetLogging">
            <ActionIcon name="RotateCcw" cls="w-3.5 h-3.5" />
            {{ t('settingsPage.resetDefaults') }}
          </Button>

          <Separator />

          <div class="flex gap-2.5 flex-wrap">
            <Button variant="outline" size="sm" class="gap-1" @click="openLogDir">
              <ActionIcon name="FolderOpen" cls="w-3.5 h-3.5" />
              {{ t('settingsPage.viewLogFile') }}
            </Button>
            <Button variant="outline" size="sm" class="text-destructive border-destructive/30 hover:bg-destructive/10 gap-1" @click="clearLogs">
              <ActionIcon name="Trash2" cls="w-3.5 h-3.5" />
              {{ t('settingsPage.clearLogs') }}
            </Button>
          </div>
        </CardContent>
      </Card>

      <!-- ═══ Advanced ═══ -->
      <Card>
        <CardHeader class="flex flex-row items-center gap-2 p-4 pb-3">
          <ActionIcon name="Settings" cls="w-4 h-4 text-muted-foreground" />
          <CardTitle class="flex-1 text-sm">{{ t('settingsPage.advanced') }}</CardTitle>
        </CardHeader>
        <CardContent class="px-4 pb-4 pt-0 space-y-4">
          <div class="space-y-1.5">
            <Label class="text-xs text-muted-foreground font-semibold">{{ t('settingsPage.pythonPath') }}</Label>
            <Input v-model="settings.python_path" :placeholder="t('settingsPage.pythonPathPlaceholder')" class="h-8 text-xs" />
            <p class="text-[11px] text-muted-foreground/60">{{ t('settingsPage.pythonPathHint') }}</p>
          </div>
          <div class="space-y-1.5">
            <Label class="text-xs text-muted-foreground font-semibold">{{ t('settingsPage.workingDirectory') }}</Label>
            <Input v-model="settings.working_dir" :placeholder="t('settingsPage.workingDirPlaceholder')" class="h-8 text-xs" />
            <p class="text-[11px] text-muted-foreground/60">{{ t('settingsPage.workingDirHint') }}</p>
          </div>
          <div class="flex items-center gap-2.5">
            <Switch :model-value="settings.auto_start" @update:model-value="toggleAutoStart" />
            <Label class="text-sm text-foreground cursor-pointer">{{ t('settingsPage.autoStart') }}</Label>
          </div>
        </CardContent>
      </Card>

      <!-- ═══ Agent Integration ═══ -->
      <Card>
        <CardHeader class="flex flex-row items-center gap-2 p-4 pb-3">
          <ActionIcon name="Terminal" cls="w-4 h-4 text-muted-foreground" />
          <div class="flex-1">
            <CardTitle class="text-sm">{{ t('settingsPage.agentIntegration') }}</CardTitle>
            <CardDescription class="text-xs">{{ t('settingsPage.agentDesc') }}</CardDescription>
          </div>
        </CardHeader>
        <CardContent class="px-4 pb-4 pt-0 space-y-3">
          <div class="bg-muted rounded-md p-3 font-mono text-xs space-y-1">
            <div class="text-muted-foreground">{{ t('settingsPage.cliComment1') }}</div>
            <div class="text-foreground">wf-cli list --json</div>
            <div class="text-muted-foreground mt-2">{{ t('settingsPage.cliComment2') }}</div>
            <div class="text-foreground">wf-cli run &lt;id&gt; --var url=https://example.com</div>
            <div class="text-muted-foreground mt-2">{{ t('settingsPage.cliComment3') }}</div>
            <div class="text-foreground">wf-cli status &lt;run_id&gt; --json</div>
            <div class="text-muted-foreground mt-2">{{ t('settingsPage.cliComment4') }}</div>
            <div class="text-foreground">wf-cli schedule list --json</div>
          </div>
          <p class="text-xs text-muted-foreground">{{ t('settingsPage.cliDocNote') }}</p>
          <Button variant="outline" size="sm" class="gap-1" @click="downloadSkill">
            <ActionIcon name="Download" cls="w-3.5 h-3.5" />
            {{ t('settingsPage.downloadSkill') }}
          </Button>
        </CardContent>
      </Card>

      <!-- ═══ Version + Changelog ═══ -->
      <Card>
        <CardHeader class="flex flex-row items-center gap-2 p-4 pb-3">
          <ActionIcon name="Info" cls="w-4 h-4 text-muted-foreground" />
          <div class="flex-1">
            <CardTitle class="text-sm">{{ t('settingsPage.versionInfo') }}</CardTitle>
            <CardDescription class="text-xs">{{ t('settingsPage.versionHint') }}</CardDescription>
          </div>
        </CardHeader>
        <CardContent class="px-4 pb-4 pt-0 space-y-3">
          <Badge variant="default" class="text-sm px-3 py-1">v{{ APP_VERSION }}</Badge>
          <Button variant="ghost" size="sm" class="text-xs gap-1 -ml-2" @click="showChangelog = !showChangelog">
            <ActionIcon :name="showChangelog ? 'ChevronDown' : 'ChevronRight'" cls="w-3.5 h-3.5" />
            {{ t('settingsPage.changelog') }}
          </Button>
          <div v-if="showChangelog" class="space-y-0">
            <div
              v-for="(item, i) in changelog" :key="item.version"
              class="text-xs text-muted-foreground py-1.5"
              :class="i < changelog.length - 1 ? 'border-b border-border' : ''"
            >
              <strong class="text-foreground">{{ item.version }}</strong> — {{ item.desc }}
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  </div>

  <!-- ═══ Sticky save bar ═══ -->
  <div
    v-if="isDirty"
    class="fixed bottom-0 left-0 right-0 z-50 bg-background/95 backdrop-blur border-t border-border px-5 py-3"
    style="box-shadow: 0 -4px 12px rgba(0,0,0,0.1)"
  >
    <div class="max-w-[640px] mx-auto flex items-center justify-between">
      <span class="text-sm text-muted-foreground flex items-center gap-1.5">
        <ActionIcon name="Info" cls="w-4 h-4" />
        {{ t('settingsPage.unsavedChanges') }}
      </span>
      <div class="flex gap-2">
        <Button variant="outline" size="sm" @click="emit('back')">{{ t('common.cancel') }}</Button>
        <Button class="bg-success hover:bg-success/90 text-success-foreground font-semibold gap-1" :disabled="saving" @click="save">
          <ActionIcon v-if="!saving" name="Save" cls="w-3.5 h-3.5" />
          <ActionIcon v-else name="Loader" cls="w-3.5 h-3.5 animate-spin" />
          {{ saving ? t('settingsPage.saving') : t('settingsPage.saveSettings') }}
        </Button>
      </div>
    </div>
  </div>
</template>
