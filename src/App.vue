<script setup lang="ts">
// App.vue — Single-page layout: Sidebar (workflow list) + Main content (editor/settings/history)
// + Unified operation console at bottom
import { ref, provide, onMounted, onUnmounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import Editor from './pages/Editor.vue'
import Dashboard from './pages/Dashboard.vue'
import Settings from './pages/Settings.vue'
import RunHistory from './pages/RunHistory.vue'
import Plugins from './pages/Plugins.vue'
import ActionIcon from './components/ActionIcon.vue'
import SchedulePanel from './components/SchedulePanel.vue'
import StatusBar from './components/StatusBar.vue'
import ErrorBoundary from "./components/ErrorBoundary.vue"
import Toast from './components/Toast.vue'
import Button from './components/ui/button/Button.vue'
import ApprovalCenter from './components/ApprovalCenter.vue'
import { useToast } from './composables/useToast'
import { useGlobalStatus } from './composables/useGlobalStatus'
import { useOpsConsole } from './composables/useOpsConsole'
import { syncNodeSchema } from './composables/useNodeSchema'
import SidebarProvider from './components/ui/sidebar/SidebarProvider.vue'
import Sidebar from './components/ui/sidebar/Sidebar.vue'
import SidebarInset from './components/ui/sidebar/SidebarInset.vue'

const { t } = useI18n()
const { toasts, remove } = useToast()
const globalStatus = useGlobalStatus()
const ops = useOpsConsole()

onMounted(() => {
  globalStatus.startSchedulePolling()
  syncNodeSchema() // P3: 拉取后端节点目录，扩展前端可用节点类型
})

onUnmounted(() => {
  globalStatus.stopSchedulePolling()
  ops.unsubscribe()
})

type MainView = 'welcome' | 'editor' | 'settings' | 'history' | 'template' | 'plugins'

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

function onOpenPlugins() {
  currentView.value = 'plugins'
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

// ─── Console panel helpers ───
const consoleStatusCounts = computed(() => {
  let ok = 0, fail = 0, start = 0
  for (const l of ops.logs.value) {
    if (l.status === 'ok') ok++
    else if (l.status === 'fail') fail++
    else start++
  }
  return { ok, fail, start }
})

function statusIcon(status: string): string {
  if (status === 'start') return '▶'
  if (status === 'ok') return '✓'
  if (status === 'fail') return '✗'
  return '·'
}

function statusColor(status: string): string {
  if (status === 'start') return 'text-primary'
  if (status === 'ok') return 'text-success'
  if (status === 'fail') return 'text-destructive'
  return 'text-muted-foreground'
}
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
        <div class="flex h-dvh w-full overflow-hidden bg-background">
          <!-- Sidebar: workflow list -->
          <Sidebar>
            <Dashboard
              ref="dashboardRef"
              :selected-id="selectedWorkflowId"
              @open-workflow="onOpenWorkflow"
              @open-settings="onOpenSettings"
              @open-history="onOpenHistory"
              @open-plugins="onOpenPlugins"
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
                <h2 class="text-2xl font-bold tracking-tight text-foreground">{{ t('dashboard.createWorkflowToStart') }}</h2>
                <p class="text-muted-foreground max-w-md">
                  {{ t('empty.createWorkflow') }}
                </p>
              </div>
            </div>

            <!-- Background overlay -->
            <Transition name="fade">
              <div
                v-if="currentView === 'settings' || currentView === 'history' || currentView === 'plugins'"
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

            <!-- Plugins panel overlay -->
            <Transition name="slide-right">
              <div
                v-if="currentView === 'plugins'"
                class="fixed top-0 right-0 bottom-0 w-[520px] bg-card border-l border-border z-50 shadow-xl overflow-y-auto"
              >
                <Plugins @back="onBackToMain" />
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

            <!-- ─── Unified Operation Console ─── -->
            <div class="border-t border-border bg-background shrink-0 select-none">
              <button
                class="w-full flex items-center justify-between px-3 py-1.5 cursor-pointer hover:bg-secondary/50 transition-colors"
                @click="ops.toggle()"
                :aria-expanded="ops.visible.value"
                aria-label="切换操作控制台"
              >
                <div class="flex items-center gap-2">
                  <span class="text-[11px] text-muted-foreground">{{ ops.visible.value ? '▼' : '▶' }}</span>
                  <span class="text-xs text-foreground">{{ t('nav.dashboard') }} {{ t('common.actions') }}</span>
                  <span v-if="ops.logs.value.length" class="text-[10px] text-muted-foreground bg-secondary rounded px-1.5 py-0.5">
                    {{ ops.logs.value.length }}
                  </span>
                  <span class="text-[10px] text-success ml-0.5">✓{{ consoleStatusCounts.ok }}</span>
                  <span v-if="consoleStatusCounts.fail" class="text-[10px] text-destructive ml-0.5">✗{{ consoleStatusCounts.fail }}</span>
                </div>
                <Button
                  v-if="ops.visible.value"
                  variant="ghost"
                  size="sm"
                  class="h-5 text-[10px] px-1.5"
                  @click.stop="ops.clearLogs()"
                >{{ t('common.clear') }}</Button>
              </button>
              <Transition name="collapse">
                <div v-if="ops.visible.value" class="max-h-[200px] overflow-y-auto border-t border-border">
                  <div v-if="!ops.logs.value.length" class="text-muted-foreground text-xs p-3">
                    {{ t('empty.noHistory') }}
                  </div>
                  <div
                    v-for="(log, i) in ops.logs.value"
                    :key="i"
                    class="flex items-baseline gap-2 px-3 py-0.5 text-[11px] font-mono hover:bg-card transition-colors"
                    :class="statusColor(log.status)"
                  >
                    <span class="text-muted-foreground/50 shrink-0 w-[70px]">{{ log.time }}</span>
                    <span class="shrink-0 w-[12px]">{{ statusIcon(log.status) }}</span>
                    <span class="text-muted-foreground/60 shrink-0 w-[32px] text-[10px]">{{ log.source === 'gui' ? 'GUI' : 'Agent' }}</span>
                    <span class="text-foreground flex-1 truncate">{{ log.name }}</span>
                    <span v-if="log.detail" class="text-muted-foreground/50 text-[10px] max-w-[180px] truncate">{{ log.detail }}</span>
                    <span v-if="log.elapsed" class="text-muted-foreground/40 text-[10px] shrink-0">{{ log.elapsed }}ms</span>
                  </div>
                </div>
              </Transition>
            </div>

            <StatusBar />
          </div>
        </div>
      </SidebarProvider>

      <!-- Drag import overlay -->
      <Transition name="fade">
        <div v-if="isDragging" class="fixed inset-0 bg-background/90 z-[100] flex items-center justify-center">
          <div class="text-center p-10 border-2 border-dashed border-primary rounded-2xl bg-primary/10">
            <span class="text-5xl text-muted-foreground">↓</span>
            <div class="text-base text-foreground mt-2">{{ t('editor.importJson') }}</div>
            <div class="text-xs text-muted-foreground">.json / .yaml</div>
          </div>
        </div>
      </Transition>

      <Toast
        v-for="(t, i) in toasts"
        :key="t.id"
        :message="t.message"
        :type="t.type"
        :duration="t.duration"
        :index="i"
        @close="remove(t.id)"
      />

      <ApprovalCenter />
    </ErrorBoundary>
  </div>
</template>

<style scoped>
.app-shell {
  height: 100dvh;
  background: var(--color-background);
  color: var(--color-foreground);
  overflow: hidden;
}

.fade-enter-active, .fade-leave-active { transition: opacity 0.15s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }

.slide-right-enter-active, .slide-right-leave-active { transition: transform 0.25s ease, opacity 0.25s ease; }
.slide-right-enter-from, .slide-right-leave-to { transform: translateX(100%); opacity: 0; }

.collapse-enter-active,
.collapse-leave-active {
  transition: all 0.15s ease;
  overflow: hidden;
}
.collapse-enter-from,
.collapse-leave-to {
  max-height: 0;
  opacity: 0;
}
</style>
