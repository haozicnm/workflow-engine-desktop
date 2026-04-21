<template>
  <div class="p-6">
    <div class="flex items-center justify-between mb-6">
      <h2 class="text-2xl font-bold text-gray-800 dark:text-white">工作流</h2>
      <button
        @click="showCreate = true"
        class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors text-sm"
      >
        + 新建工作流
      </button>
    </div>

    <!-- 加载状态 -->
    <div v-if="store.loading" class="text-center py-12 text-gray-400">
      加载中...
    </div>

    <!-- 空状态 -->
    <div v-else-if="store.workflows.length === 0" class="text-center py-12">
      <div class="text-4xl mb-3">📋</div>
      <p class="text-gray-500 mb-4">还没有工作流</p>
      <button
        @click="showCreate = true"
        class="text-blue-600 hover:underline text-sm"
      >
        创建第一个工作流
      </button>
    </div>

    <!-- 工作流列表 -->
    <div v-else class="grid gap-4">
      <div
        v-for="wf in store.workflows"
        :key="wf.id"
        class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-4 hover:shadow-md transition-shadow"
      >
        <div class="flex items-center justify-between">
          <div>
            <h3 class="font-medium text-gray-800 dark:text-white">{{ wf.name }}</h3>
            <p class="text-sm text-gray-500 mt-1">{{ wf.description || '暂无描述' }}</p>
            <p class="text-xs text-gray-400 mt-2">更新于 {{ formatDate(wf.updated_at) }}</p>
          </div>
          <div class="flex items-center gap-2">
            <span
              :class="wf.enabled ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-500'"
              class="px-2 py-1 rounded text-xs"
            >
              {{ wf.enabled ? '已启用' : '已禁用' }}
            </span>
            <router-link
              :to="`/editor/${wf.id}`"
              class="text-blue-600 hover:underline text-sm"
            >
              编辑
            </router-link>
            <button
              @click="confirmDelete(wf)"
              class="text-red-500 hover:underline text-sm"
            >
              删除
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 新建对话框 -->
    <div v-if="showCreate" class="fixed inset-0 bg-black/40 flex items-center justify-center z-50">
      <div class="bg-white dark:bg-gray-800 rounded-xl p-6 w-96 shadow-xl">
        <h3 class="text-lg font-bold mb-4 text-gray-800 dark:text-white">新建工作流</h3>
        <input
          v-model="newName"
          placeholder="工作流名称"
          class="w-full px-3 py-2 border rounded-lg mb-3 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
          @keydown.enter="createWorkflow"
          autofocus
        />
        <input
          v-model="newDesc"
          placeholder="描述（可选）"
          class="w-full px-3 py-2 border rounded-lg mb-4 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
        />
        <div class="flex justify-end gap-2">
          <button
            @click="showCreate = false"
            class="px-4 py-2 text-gray-600 hover:bg-gray-100 rounded-lg text-sm"
          >
            取消
          </button>
          <button
            @click="createWorkflow"
            :disabled="!newName.trim()"
            class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 text-sm"
          >
            创建
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useWorkflowStore } from "../stores/workflow";

const store = useWorkflowStore();
const showCreate = ref(false);
const newName = ref("");
const newDesc = ref("");

onMounted(() => {
  store.fetchWorkflows();
});

function formatDate(iso: string) {
  return new Date(iso).toLocaleString("zh-CN");
}

async function createWorkflow() {
  if (!newName.value.trim()) return;
  await store.create(newName.value.trim(), newDesc.value.trim() || undefined);
  showCreate.value = false;
  newName.value = "";
  newDesc.value = "";
}

function confirmDelete(wf: any) {
  if (confirm(`确定删除工作流「${wf.name}」？`)) {
    store.remove(wf.id);
  }
}
</script>
