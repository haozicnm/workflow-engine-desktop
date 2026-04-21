<template>
  <div class="p-6">
    <h2 class="text-2xl font-bold text-gray-800 dark:text-white mb-6">
      编辑器
      <span v-if="route.params.id" class="text-sm font-normal text-gray-400 ml-2">
        ID: {{ route.params.id }}
      </span>
    </h2>

    <div class="grid grid-cols-2 gap-6">
      <!-- YAML 编辑器 -->
      <div>
        <h3 class="text-sm font-medium text-gray-600 dark:text-gray-300 mb-2">YAML 定义</h3>
        <textarea
          v-model="yamlContent"
          class="w-full h-96 font-mono text-sm p-4 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-white resize-none"
          placeholder="在此输入工作流 YAML..."
        ></textarea>
        <button
          @click="validate"
          class="mt-2 px-4 py-1.5 bg-green-600 text-white rounded-lg hover:bg-green-700 text-sm"
        >
          校验
        </button>
        <span v-if="validationResult" class="ml-3 text-sm" :class="validationResult.valid ? 'text-green-600' : 'text-red-600'">
          {{ validationResult.valid ? '✓ 有效' : `✗ ${validationResult.error}` }}
        </span>
      </div>

      <!-- 预览/节点列表 -->
      <div>
        <h3 class="text-sm font-medium text-gray-600 dark:text-gray-300 mb-2">节点预览</h3>
        <div class="border rounded-lg p-4 h-96 overflow-auto dark:border-gray-600 bg-gray-50 dark:bg-gray-800">
          <div v-if="validationResult?.valid" class="space-y-2">
            <div class="font-medium text-gray-800 dark:text-white">
              {{ validationResult.workflow.name }}
            </div>
            <div class="text-sm text-gray-500">
              {{ validationResult.workflow.step_count }} 个步骤
            </div>
          </div>
          <div v-else class="text-gray-400 text-sm text-center mt-20">
            编写 YAML 后点击「校验」查看预览
          </div>
        </div>
      </div>
    </div>

    <!-- 节点类型参考 -->
    <div class="mt-6 p-4 bg-gray-50 dark:bg-gray-800 rounded-lg">
      <h3 class="text-sm font-medium text-gray-600 dark:text-gray-300 mb-3">可用节点类型</h3>
      <div class="flex flex-wrap gap-2">
        <span v-for="node in nodeTypes" :key="node.name"
          class="px-3 py-1 bg-white dark:bg-gray-700 border dark:border-gray-600 rounded-full text-xs text-gray-600 dark:text-gray-300">
          {{ node.icon }} {{ node.name }}
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useRoute } from "vue-router";
import { validateWorkflow } from "../services/tauri";

const route = useRoute();
const yamlContent = ref(`name: 我的工作流
description: 示例工作流
steps:
  - id: step1
    name: 发送 HTTP 请求
    type: http
    config:
      method: GET
      url: https://httpbin.org/get
    next: step2
  - id: step2
    name: 输出结果
    type: script
    config:
      script: '"完成！"'
`);

const validationResult = ref<any>(null);

const nodeTypes = [
  { name: "http", icon: "🌐" },
  { name: "data", icon: "📊" },
  { name: "script", icon: "📝" },
  { name: "condition", icon: "🔀" },
  { name: "loop", icon: "🔄" },
  { name: "excel", icon: "📗" },
  { name: "word", icon: "📘" },
  { name: "notify", icon: "🔔" },
  { name: "approval", icon: "✅" },
  { name: "browser", icon: "🌍" },
  { name: "parallel", icon: "⚡" },
];

async function validate() {
  try {
    validationResult.value = await validateWorkflow(yamlContent.value);
  } catch (e: any) {
    validationResult.value = { valid: false, error: e.toString() };
  }
}
</script>
