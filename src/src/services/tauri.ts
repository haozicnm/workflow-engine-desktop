import { invoke } from "@tauri-apps/api/core";

// 类型定义
export interface WorkflowListItem {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface AppSettings {
  theme: string;
  language: string;
  auto_start: boolean;
  log_level: string;
  python_path: string | null;
}

// 工作流 API
export async function listWorkflows(): Promise<WorkflowListItem[]> {
  return invoke("workflow_list");
}

export async function createWorkflow(name: string, description?: string): Promise<string> {
  return invoke("workflow_create", { name, description });
}

export async function getWorkflow(id: string): Promise<any> {
  return invoke("workflow_get", { id });
}

export async function updateWorkflow(
  id: string,
  data: { name?: string; description?: string; enabled?: boolean }
): Promise<void> {
  return invoke("workflow_update", { id, ...data });
}

export async function deleteWorkflow(id: string): Promise<void> {
  return invoke("workflow_delete", { id });
}

export async function validateWorkflow(yaml: string): Promise<any> {
  return invoke("workflow_validate", { yaml });
}

// 系统 API
export async function getSettings(): Promise<AppSettings> {
  return invoke("settings_get");
}

export async function updateSettings(settings: AppSettings): Promise<void> {
  return invoke("settings_update", { settings });
}

export async function checkBrowser(): Promise<{ available: boolean; python_path: string | null }> {
  return invoke("system_check_browser");
}
