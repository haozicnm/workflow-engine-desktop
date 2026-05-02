<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { safeInvoke, safeListen } from '../utils/tauri'
import { useToast } from '../composables/useToast'
import pkg from '../package.json'

const toast = useToast()
const APP_VERSION = pkg.version

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
<div class="settings-page">
  <header class="page-header">
    <h1>⚙️ 设置</h1>
    <p class="page-desc">配置应用行为和浏览器节点</p>
  </header>

  <div v-if="loading" class="loading">加载中...</div>

  <div v-else class="settings-content">
    <!-- 浏览器设置 -->
    <section class="settings-section">
      <h2>🌐 浏览器节点</h2>
      <p class="section-desc">选择浏览器自动化使用的浏览器。内网环境建议选择 Edge。</p>

      <div class="form-group">
        <label>浏览器通道</label>
        <div class="radio-group">
          <label v-for="opt in browserOptions" :key="opt.value"
            class="radio-option" :class="{ active: settings.browser_channel === opt.value }">
            <input type="radio" v-model="settings.browser_channel" :value="opt.value" />
            <div class="radio-content">
              <span class="radio-label">{{ opt.label }}</span>
              <span class="radio-desc">{{ opt.desc }}</span>
            </div>
          </label>
        </div>
      </div>

      <!-- 系统检测 -->
      <div v-if="sysInfo" class="sys-info">
        <h3>系统检测</h3>
        <div class="info-grid">
          <div class="info-item">
            <span class="info-label">Python 环境</span>
            <span :class="sysInfo.python_available ? 'tag-ok' : 'tag-miss'">
              {{ sysInfo.python_available ? '✅ 已检测到' : '❌ 未检测到' }}
            </span>
          </div>
          <div class="info-item" v-if="sysInfo.system_python">
            <span class="info-label">　↳ 系统 Python</span>
            <span class="tag-ok" :title="sysInfo.system_python">{{ truncatePath(sysInfo.system_python, 40) }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">pip 离线安装包</span>
            <span :class="sysInfo.has_wheels ? 'tag-ok' : 'tag-miss'">
              {{ sysInfo.has_wheels ? '✅ 已内置（离线可用）' : '—（需联网安装）' }}
            </span>
          </div>
          <div class="info-item">
            <span class="info-label">Playwright Chromium</span>
            <span :class="sysInfo.has_playwright_chromium ? 'tag-ok' : 'tag-miss'">
              {{ sysInfo.has_playwright_chromium ? '✅ 已内置（离线可用）' : '⚠ 需手动安装' }}
            </span>
          </div>
          <div class="info-item">
            <span class="info-label">　↳ 备用: Edge</span>
            <span :class="sysInfo.has_edge ? 'tag-ok' : 'tag-miss'">
              {{ sysInfo.has_edge ? '✅ 可用' : '—' }}
            </span>
          </div>
          <div class="info-item">
            <span class="info-label">　↳ 备用: Chrome</span>
            <span :class="sysInfo.has_chrome ? 'tag-ok' : 'tag-miss'">
              {{ sysInfo.has_chrome ? '✅ 可用' : '—' }}
            </span>
          </div>
        </div>
      </div>
    </section>

    <!-- 高级设置 -->
    <section class="settings-section">
      <h2>🔧 高级</h2>

      <div class="form-group">
        <label>Python 路径 (可选)</label>
        <input type="text" v-model="settings.python_path"
          placeholder="留空使用自动检测" class="input-field" />
        <p class="field-hint">指定 Python 可执行文件完整路径。留空则自动检测。</p>
      </div>

      <div class="form-group">
        <label>日志级别</label>
        <select v-model="settings.log_level" class="select-field">
          <option v-for="opt in logLevelOptions" :key="opt.value" :value="opt.value">
            {{ opt.label }}
          </option>
        </select>
      </div>

      <div class="form-group">
        <label class="toggle">
          <input type="checkbox" v-model="settings.auto_start" />
          <span class="toggle-slider"></span>
          <span class="toggle-label">开机自启</span>
        </label>
      </div>
    </section>

    <!-- 日志管理 -->
    <section class="settings-section">
      <h2>📄 日志</h2>
      <p class="section-desc">查看和清理应用运行日志。</p>
      <div class="log-actions">
        <button class="btn-log" @click="openLogDir">
          📂 查看日志文件
        </button>
        <button class="btn-log btn-log-danger" @click="clearLogs">
          🗑 清空日志
        </button>
      </div>
    </section>

    <!-- 版本信息 -->
    <section class="settings-section">
      <h2>📦 版本信息</h2>
      <p class="section-desc">当前版本及更新记录。</p>
      <div class="version-info">
        <span class="version-tag">v{{ APP_VERSION }}</span>
      </div>
      <div class="changelog">
        <h3>更新明细</h3>
        <div class="changelog-item"><strong>v2.3</strong> — Grid 布局 · MiniMap · 右键菜单 · 画布搜索 · 预览面板 · 交互式元素选择</div>
        <div class="changelog-item"><strong>v2.2</strong> — 单窗口统一架构 · FloatingPanel · Widget 属性面板 · 浏览器自动化 v2</div>
        <div class="changelog-item"><strong>v2.1</strong> — LiteGraph 画布迁移 · 叠加层架构 · 统一设计令牌 · 模板系统</div>
        <div class="changelog-item"><strong>v2.0</strong> — Rust DAG 引擎重写 · Tauri 2.0 桌面应用 · 25+ 节点类型</div>
        <div class="changelog-item"><strong>v1.x</strong> — YAML 工作流引擎原型 · Web 前端 · Playwright 自动化</div>
      </div>
    </section>

    <div class="save-bar">
      <button class="btn-save" @click="save" :disabled="saving">
        {{ saving ? '保存中...' : '💾 保存设置' }}
      </button>
    </div>
  </div>
</div>
</template>

<style scoped>
.settings-page { max-width: 640px; margin: 0 auto; padding: 24px 20px; }
.page-header { margin-bottom: 24px; }
.page-header h1 { font-size: 20px; color: #e6edf3; margin: 0 0 4px 0; }
.page-desc { font-size: 13px; color: #8b949e; margin: 0; }
.loading { text-align: center; padding: 40px; color: #8b949e; }
.settings-section {
  background: #161b22; border: 1px solid #30363d; border-radius: 8px;
  padding: 20px; margin-bottom: 16px;
}
.settings-section h2 { font-size: 15px; color: #e6edf3; margin: 0 0 6px 0; }
.section-desc { font-size: 12px; color: #8b949e; margin: 0 0 16px 0; }
.form-group { margin-bottom: 16px; }
.form-group > label { display: block; font-size: 12px; color: #8b949e; margin-bottom: 6px; font-weight: 600; }

.radio-group { display: flex; flex-direction: column; gap: 8px; }
.radio-option {
  display: flex; align-items: flex-start; gap: 10px; padding: 10px 12px;
  border: 1px solid #30363d; border-radius: 6px; cursor: pointer; transition: all 0.15s;
}
.radio-option:hover { border-color: #58a6ff; }
.radio-option.active { border-color: #58a6ff; background: rgba(88,166,255,0.08); }
.radio-option input { margin-top: 3px; accent-color: #58a6ff; }
.radio-content { display: flex; flex-direction: column; gap: 2px; }
.radio-label { font-size: 13px; color: #e6edf3; font-weight: 600; }
.radio-desc { font-size: 11px; color: #8b949e; }

.sys-info { margin-top: 16px; padding: 12px; background: #0d1117; border-radius: 6px; }
.sys-info h3 { font-size: 12px; color: #8b949e; margin: 0 0 10px 0; }
.info-grid { display: flex; flex-direction: column; gap: 6px; }
.info-item { display: flex; justify-content: space-between; align-items: center; }
.info-label { font-size: 12px; color: #e6edf3; }
.tag-ok { font-size: 11px; color: #3fb950; }
.tag-miss { font-size: 11px; color: #8b949e; }

.input-field, .select-field {
  width: 100%; padding: 8px 10px; background: #0d1117; border: 1px solid #30363d;
  border-radius: 6px; color: #e6edf3; font-size: 13px; outline: none;
}
.input-field:focus, .select-field:focus { border-color: #58a6ff; }
.field-hint { font-size: 11px; color: #6e7681; margin: 4px 0 0 0; }

.toggle { display: flex; align-items: center; gap: 10px; cursor: pointer; }
.toggle input { accent-color: #58a6ff; }
.toggle-slider { display: none; }
.toggle-label { font-size: 13px; color: #e6edf3; }

.save-bar { text-align: right; margin-top: 8px; }
.btn-save {
  padding: 8px 20px; background: #238636; border: none; border-radius: 6px;
  color: #fff; font-size: 13px; font-weight: 600; cursor: pointer;
}
.btn-save:hover { background: #2ea043; }
.btn-save:disabled { opacity: 0.5; cursor: not-allowed; }

.log-actions { display: flex; gap: 10px; flex-wrap: wrap; }
.btn-log {
  padding: 8px 16px; background: #21262d; border: 1px solid #30363d;
  border-radius: 6px; color: #e6edf3; font-size: 13px; cursor: pointer;
  transition: all 0.15s;
}
.btn-log:hover { border-color: #58a6ff; background: #1f2a37; }
.btn-log-danger:hover { border-color: #f85149; background: #2a1215; color: #f85149; }

.version-info { margin-bottom: 16px; }
.version-tag {
  display: inline-block; padding: 4px 12px; background: #1f6feb22;
  border: 1px solid #1f6feb; border-radius: 20px;
  color: #58a6ff; font-size: 14px; font-weight: 700;
}
.changelog h3 { font-size: 13px; color: #e6edf3; margin: 0 0 8px 0; }
.changelog-item {
  font-size: 12px; color: #8b949e; padding: 6px 0;
  border-bottom: 1px solid #21262d;
}
.changelog-item:last-child { border-bottom: none; }
.changelog-item strong { color: #e6edf3; }
</style>
