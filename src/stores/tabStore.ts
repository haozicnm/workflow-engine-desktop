import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export interface WorkflowTab {
  id: string
  name: string
  /** 是否已修改（有未保存的更改） */
  dirty: boolean
}

export const useTabStore = defineStore('tabs', () => {
  const tabs = ref<WorkflowTab[]>([])
  const activeTabId = ref<string | null>(null)

  const activeTab = computed(() => tabs.value.find(t => t.id === activeTabId.value) ?? null)
  const tabCount = computed(() => tabs.value.length)

  function addTab(name = '未命名') {
    const id = `tab_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`
    const tab: WorkflowTab = { id, name, dirty: false }
    tabs.value.push(tab)
    activeTabId.value = id
    return id
  }

  function removeTab(id: string) {
    const idx = tabs.value.findIndex(t => t.id === id)
    if (idx === -1) return
    tabs.value.splice(idx, 1)
    if (activeTabId.value === id) {
      // 切换到相邻 tab
      if (tabs.value.length > 0) {
        activeTabId.value = tabs.value[Math.min(idx, tabs.value.length - 1)].id
      } else {
        activeTabId.value = null
      }
    }
  }

  function setActive(id: string) {
    if (tabs.value.some(t => t.id === id)) {
      activeTabId.value = id
    }
  }

  function renameTab(id: string, name: string) {
    const tab = tabs.value.find(t => t.id === id)
    if (tab) tab.name = name
  }

  function setDirty(id: string, dirty: boolean) {
    const tab = tabs.value.find(t => t.id === id)
    if (tab) tab.dirty = dirty
  }

  /** 确保至少有一个 tab */
  function ensureTab(): string {
    if (tabs.value.length === 0) return addTab('工作流 1')
    if (!activeTabId.value) activeTabId.value = tabs.value[0].id
    return activeTabId.value!
  }

  return {
    tabs,
    activeTabId,
    activeTab,
    tabCount,
    addTab,
    removeTab,
    setActive,
    renameTab,
    setDirty,
    ensureTab,
  }
})
