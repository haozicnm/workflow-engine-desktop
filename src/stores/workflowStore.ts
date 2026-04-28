import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import yaml from 'js-yaml'
import { useEditorStore } from './editorStore'
import type { WorkflowListItem, WorkflowFull } from '../types/workflow'

export const useWorkflowStore = defineStore('workflow', () => {
  const workflowList = ref<WorkflowListItem[]>([])
  const currentId = ref<string | null>(null)
  const loading = ref(false)
  const saving = ref(false)

  // ─── List ───

  async function fetchList() {
    loading.value = true
    try {
      workflowList.value = await invoke<WorkflowListItem[]>('workflow_list')
    } catch (e) {
      console.error('获取工作流列表失败:', e)
    } finally {
      loading.value = false
    }
  }

  // ─── Load ───

  async function loadWorkflow(id: string) {
    loading.value = true
    try {
      const wf = await invoke<WorkflowFull | null>('workflow_get', { id })
      if (wf) {
        currentId.value = wf.id
        const editor = useEditorStore()
        editor.setMetadata(wf.name, wf.description || '')
        const content = wf.yaml || editor.createDefaultYaml()
        editor.parseYaml(content)
      }
    } catch (e) {
      console.error('加载工作流失败:', e)
    } finally {
      loading.value = false
    }
  }

  // ─── Save ───

  async function saveWorkflow(): Promise<boolean> {
    saving.value = true
    try {
      const editor = useEditorStore()
      if (!currentId.value) {
        const id = await invoke<string>('workflow_create', {
          name: editor.workflowName,
          description: editor.workflowDesc,
        })
        currentId.value = id
      }
      await invoke('workflow_update', {
        id: currentId.value,
        name: editor.workflowName,
        description: editor.workflowDesc,
        enabled: true,
      })
      await invoke('workflow_save_yaml', {
        id: currentId.value,
        yaml: editor.yamlText,
      })
      return true
    } catch (e) {
      console.error('保存失败:', e)
      return false
    } finally {
      saving.value = false
    }
  }

  // ─── Clone ───

  async function cloneWorkflow(id: string): Promise<string | null> {
    try {
      const wf = await invoke<WorkflowFull | null>('workflow_get', { id })
      if (!wf) return null
      const newId = await invoke<string>('workflow_create', {
        name: wf.name + '（副本）',
        description: wf.description,
      })
      if (wf.yaml) {
        await invoke('workflow_save_yaml', { id: newId, yaml: wf.yaml })
      }
      return newId
    } catch (e) {
      console.error('克隆失败:', e)
      return null
    }
  }

  // ─── Export/Import ───

  function exportYaml(yamlContent: string, name: string) {
    const blob = new Blob([yamlContent], { type: 'text/yaml' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = (name || 'workflow') + '.yaml'
    a.click()
    URL.revokeObjectURL(url)
  }

  async function importYaml(file: File): Promise<{ name: string; yaml: string } | null> {
    try {
      const text = await file.text()
      const data = yaml.load(text, { schema: yaml.JSON_SCHEMA })
      if (!data || typeof data !== 'object' || !(data as Record<string, unknown>).steps) {
        throw new Error('无效的工作流 YAML')
      }
      return { name: (data as Record<string, string>).name || file.name.replace(/\.ya?ml$/i, ''), yaml: text }
    } catch (e) {
      console.error('导入失败:', e)
      return null
    }
  }

  return {
    workflowList, currentId, loading, saving,
    fetchList, loadWorkflow, saveWorkflow, cloneWorkflow, exportYaml, importYaml,
  }
})
