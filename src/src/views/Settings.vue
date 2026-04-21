<template>
  <div class="p-6 max-w-2xl">
    <h2 class="text-2xl font-bold text-gray-800 dark:text-white mb-6">设置</h2>

    <div class="space-y-6">
      <!-- 主题 -->
      <div>
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">主题</label>
        <select
          v-model="settings.theme"
          class="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-white"
        >
          <option value="system">跟随系统</option>
          <option value="light">浅色</option>
          <option value="dark">深色</option>
        </select>
      </div>

      <!-- 语言 -->
      <div>
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">语言</label>
        <select
          v-model="settings.language"
          class="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-white"
        >
          <option value="zh-CN">中文</option>
          <option value="en">English</option>
        </select>
      </div>

      <!-- 日志级别 -->
      <div>
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">日志级别</label>
        <select
          v-model="settings.log_level"
          class="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-white"
        >
          <option value="error">Error</option>
          <option value="warn">Warn</option>
          <option value="info">Info</option>
          <option value="debug">Debug</option>
        </select>
      </div>

      <!-- 开机自启 -->
      <div class="flex items-center gap-3">
        <input
          type="checkbox"
          v-model="settings.auto_start"
          id="auto-start"
          class="rounded"
        />
        <label for="auto-start" class="text-sm text-gray-700 dark:text-gray-300">开机自启</label>
      </div>

      <!-- Python 路径 -->
      <div>
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          Python 路径
          <span class="text-gray-400 font-normal">（浏览器节点需要）</span>
        </label>
        <input
          v-model="settings.python_path"
          placeholder="自动检测"
          class="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-white"
        />
        <button
          @click="checkPython"
          class="mt-2 text-sm text-blue-600 hover:underline"
        >
          检测 Python 环境
        </button>
        <span v-if="pythonStatus" class="ml-3 text-sm" :class="pythonStatus.available ? 'text-green-600' : 'text-red-500'">
          {{ pythonStatus.available ? `✓ ${pythonStatus.python_path}` : '✗ 未找到 Python' }}
        </span>
      </div>

      <!-- 保存 -->
      <button
        @click="save"
        class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 text-sm"
      >
        保存设置
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { getSettings, updateSettings, checkBrowser } from "../services/tauri";
import type { AppSettings } from "../services/tauri";

const settings = ref<AppSettings>({
  theme: "system",
  language: "zh-CN",
  auto_start: false,
  log_level: "info",
  python_path: null,
});

const pythonStatus = ref<{ available: boolean; python_path: string | null } | null>(null);

onMounted(async () => {
  try {
    settings.value = await getSettings();
  } catch (e) {
    console.error("加载设置失败:", e);
  }
});

async function checkPython() {
  pythonStatus.value = await checkBrowser();
  if (pythonStatus.value.available && pythonStatus.value.python_path) {
    settings.value.python_path = pythonStatus.value.python_path;
  }
}

async function save() {
  try {
    await updateSettings(settings.value);
    alert("设置已保存");
  } catch (e) {
    alert("保存失败: " + e);
  }
}
</script>
