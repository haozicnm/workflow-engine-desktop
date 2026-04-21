import { defineStore } from "pinia";
import { ref } from "vue";
import * as api from "../services/tauri";
import type { WorkflowListItem } from "../services/tauri";

export const useWorkflowStore = defineStore("workflow", () => {
  const workflows = ref<WorkflowListItem[]>([]);
  const loading = ref(false);

  async function fetchWorkflows() {
    loading.value = true;
    try {
      workflows.value = await api.listWorkflows();
    } catch (e) {
      console.error("加载工作流列表失败:", e);
    } finally {
      loading.value = false;
    }
  }

  async function create(name: string, description?: string) {
    const id = await api.createWorkflow(name, description);
    await fetchWorkflows();
    return id;
  }

  async function remove(id: string) {
    await api.deleteWorkflow(id);
    await fetchWorkflows();
  }

  return { workflows, loading, fetchWorkflows, create, remove };
});
