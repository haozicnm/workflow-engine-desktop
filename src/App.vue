<script setup lang="ts">
// App.vue — Single-page layout: Sidebar (workflow list) + Main content (editor/settings/history)
import { ref, provide, onMounted, onUnmounted } from 'vue'
import Editor from './pages/Editor.vue'
import Dashboard from './pages/Dashboard.vue'
import Settings from './pages/Settings.vue'
import ActionIcon from './components/ActionIcon.vue'
import RunHistory from './pages/RunHistory.vue'
import SchedulePanel from './components/SchedulePanel.vue'
import StatusBar from './components/StatusBar.vue'
import ErrorBoundary from "./components/ErrorBoundary.vue"
import Toast from './components/Toast.vue'
import ApprovalCenter from './components/ApprovalCenter.vue'
import { useToast } from './composables/useToast'
import { useGlobalStatus } from './composables/useGlobalStatus'
import SidebarProvider from './components/ui/sidebar/SidebarProvider.vue'
import Sidebar from './components/ui/sidebar/Sidebar.vue'
import SidebarInset from './components/ui/sidebar/SidebarInset.vue'

const { toasts, remove } = useToast()
const globalStatus = useGlobalStatus()

onMounted(() => {
  globalStatus.startSchedulePolling()
})

onUnmounted(() => {
  globalStatus.stopSchedulePolling()
})

type MainView = 'welcome' | 'editor' | 'settings' | 'history'

const currentView = ref<MainView>('welcome')
const selectedWorkflowId = ref<string | null>(null)
const showSchedule = ref(false)
const scheduleWorkflowId = ref<string | null>(null)
const dashboardRef = ref<InstanceType<typeof Dashboard> | null>(null)

// Drag-drop import state
const isDragging = ref(false)
let dragCounter = 0

function onDragEnter(e: DragEvent) {
  e.preventDefault()
  dragCounter++
  isDragging.value = true
}
function onDragLeave(e: DragEvent) {
  e.preventDefault()
  dragCounter--
  if (dragCounter <= 0) { isDragging.value = false; dragCounter = 0 }
}
function onDragOver(e: DragEvent) {
  e.preventDefault()
}
function onDrop(e: DragEvent) {
  e.preventDefault()
  isDragging.value = false
  dragCounter = 0
  // File import is handled by Dashboard's drop handler at the sidebar level
}

// Sidebar click → set selected workflow ID → Editor loads that workflow
function onOpenWorkflow(id?: string) {
  selectedWorkflowId.value = id ?? null
  currentView.value = 'editor'
}

function onWorkflowCreated(id: string) {
  selectedWorkflowId.value = id
  currentView.value = 'editor'
}

function onOpenSettings() {
  currentView.value = 'settings'
}

function onOpenHistory() {
  currentView.value = 'history'
}

function onBackToMain() {
  currentView.value = selectedWorkflowId.value ? 'editor' : 'welcome'
}

function onSchedule(workflowId: string) {
  scheduleWorkflowId.value = workflowId
  showSchedule.value = true
}

function onWorkflowUpdated() {
  dashboardRef.value?.loadList()
}

function onWorkflowDeleted() {
  dashboardRef.value?.loadList()
  selectedWorkflowId.value = null
  currentView.value = 'welcome'
}

// Provide for any child that still uses inject (legacy compatibility)
provide('app:openEditor', (id?: string) => {
  selectedWorkflowId.value = id ?? null
  currentView.value = 'editor'
})
provide('app:backToDashboard', () => {
  currentView.value = selectedWorkflowId.value ? 'editor' : 'welcome'
})
provide('globalStatus', globalStatus)
</script>

<template>
  <div
    class="app-shell"
    @dragenter="onDragEnter"
    @dragleave="onDragLeave"
    @dragover="onDragOver"
    @drop="onDrop"
  >
    <ErrorBoundary>
      <SidebarProvider :default-open="true">
        <div class="flex h-screen w-full overflow-hidden bg-background">
          <!-- Sidebar: workflow list -->
          <Sidebar>
            <Dashboard
              ref="dashboardRef"
              :selected-id="selectedWorkflowId"
              @open-workflow="onOpenWorkflow"
              @open-settings="onOpenSettings"
              @open-history="onOpenHistory"
              @workflow-created="onWorkflowCreated"
            />
          </Sidebar>

          <!-- Main content area -->
          <div class="flex-1 flex flex-col min-w-0 overflow-hidden">
            <SidebarInset>
            <!-- Editor view -->
            <Editor
              v-if="currentView === 'editor' || currentView === 'settings' || currentView === 'history'"
              :workflow-id="selectedWorkflowId"
              @schedule="onSchedule"
              @workflow-updated="onWorkflowUpdated"
              @workflow-deleted="onWorkflowDeleted"
            />

            <!-- Welcome / empty state -->
            <div v-else class="flex-1 flex items-center justify-center bg-background">
              <div class="text-center space-y-4">
                <div class="flex justify-center">
                  <ActionIcon name="Settings" cls="w-16 h-16 text-muted-foreground" />
                </div>
                <h2 class="text-2xl font-bold tracking-tight text-foreground">欢迎使用 WorkFlow</h2>
                <p class="text-muted-foreground max-w-md">
                  从左侧选择一个工作流开始编辑，或点击「＋ 新建」创建新的工作流。
                </p>
              </div>
            </div>

            <!-- Backdrop overlay for panels -->
            <Transition name="fade">
              <div
                v-if="currentView === 'settings' || currentView === 'history'"
                class="fixed inset-0 bg-black/20 z-40"
                @click="onBackToMain"
              />
            </Transition>

            <!-- Settings panel overlay -->
            <Transition name="slide-right">
              <div
                v-if="currentView === 'settings'"
                class="fixed top-0 right-0 bottom-0 w-[480px] bg-card border-l border-border z-50 shadow-xl overflow-y-auto"
              >
                <Settings @back="onBackToMain" />
              </div>
            </Transition>

            <!-- RunHistory panel overlay -->
            <Transition name="slide-right">
              <div
                v-if="currentView === 'history'"
                class="fixed top-0 right-0 bottom-0 w-[480px] bg-card border-l border-border z-50 shadow-xl overflow-y-auto"
              >
                <RunHistory @back="onBackToMain" />
              </div>
            </Transition>

            <!-- Schedule panel overlay -->
            <Transition name="slide-right">
              <div
                v-if="showSchedule"
                class="fixed top-0 right-0 bottom-0 w-[480px] bg-card border-l border-border z-50 shadow-xl overflow-y-auto"
              >
                <SchedulePanel
                  :workflow-id="scheduleWorkflowId || undefined"
                  @close="showSchedule = false"
                />
              </div>
            </Transition>
            </SidebarInset>
            <StatusBar />
          </div>
        </div>
      </SidebarProvider>

      <!-- Drag import overlay -->
      <Transition name="fade">
        <div v-if="isDragging" class="fixed inset-0 bg-background/90 z-[100] flex items-center justify-center">
          <div class="text-center p-10 border-2 border-dashed border-primary rounded-2xl bg-primary/10">
            <span class="text-5xl text-muted-foreground">↓</span>
            <div class="text-base text-foreground mt-2">松开导入工作流</div>
            <div class="text-xs text-muted-foreground">支持 .json / .yaml 文件</div>
          </div>
        </div>
      </Transition>

      <Toast
        v-for="t in toasts"
        :key="t.id"
        :message="t.message"
        :type="t.type"
        :duration="t.duration"
        @close="remove(t.id)"
      />

      <ApprovalCenter />
    </ErrorBoundary>
  </div>
</template>

<style scoped>
.app-shell {
  height: 100vh;
  background: var(--color-background);
  color: var(--color-foreground);
  overflow-y: auto;
}

.fade-enter-active, .fade-leave-active { transition: opacity 0.15s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }

.slide-right-enter-active, .slide-right-leave-active { transition: transform 0.25s ease, opacity 0.25s ease; }
.slide-right-enter-from, .slide-right-leave-to { transform: translateX(100%); opacity: 0; }
</style>
