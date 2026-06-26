<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Hand, Lightbulb, Search, Star } from 'lucide-vue-next'
import { useToast } from '../composables/useToast'
import { safeListen, safeInvoke } from '../utils/tauri'
import Button from '@/components/ui/button/Button.vue'
import Badge from '@/components/ui/badge/Badge.vue'
import Textarea from '@/components/ui/textarea/Textarea.vue'

const { t } = useI18n()
const toast = useToast()

interface ApprovalEntry {
  id: string
  run_id: string
  step_id: string
  title: string
  message: string
  item?: Record<string, unknown>
  options: string[]
  recommended: string
  timeout_secs: number
  timeout_action: string
  created_at: string
  recommendation_reason?: string
}

interface PendingItem extends ApprovalEntry {
  comment: string
  deciding: boolean
  showItem: boolean
  elapsed: number
  timer: ReturnType<typeof setInterval> | null
}

const visible = ref(false)
const pending = ref<PendingItem[]>([])

const pendingCount = computed(() => pending.value.length)

// 格式化时间
function formatElapsed(secs: number): string {
  if (secs < 60) return `${secs}s`
  if (secs < 3600) return `${Math.floor(secs / 60)}m`
  return `${Math.floor(secs / 3600)}h`
}

// 格式化超时倒计时
function formatTimeout(entry: PendingItem): string {
  if (entry.timeout_secs <= 0) return t('approval.timeout')
  const remaining = entry.timeout_secs - entry.elapsed
  if (remaining <= 0) return t('approval.timeout')
  const m = Math.floor(remaining / 60)
  const s = remaining % 60
  return `${m}:${String(s).padStart(2, '0')}`
}

// 获取超时进度百分比（用于进度条）
function timeoutPercent(entry: PendingItem): number {
  if (entry.timeout_secs <= 0) return 0
  return Math.min(100, (entry.elapsed / entry.timeout_secs) * 100)
}

// 加载待审批列表
async function loadPending() {
  try {
    const list = await safeInvoke<ApprovalEntry[]>('approval_list_pending') ?? []
    // 清理旧的 timer
    pending.value.forEach(p => { if (p.timer) clearInterval(p.timer) })
    pending.value = list.map(entry => {
      const item: PendingItem = {
        ...entry,
        comment: '',
        deciding: false,
        showItem: false,
        elapsed: Math.floor((Date.now() - new Date(entry.created_at).getTime()) / 1000),
        timer: null,
      }
      // 每秒更新 elapsed
      item.timer = setInterval(() => {
        item.elapsed++
        // 超时检查：如果超时了，自动刷新列表
        if (entry.timeout_secs > 0 && item.elapsed >= entry.timeout_secs + 2) {
          loadPending()
        }
      }, 1000)
      return item
    })
  } catch (e) {
    toast.error('Failed to load approvals: ' + (e as Error).message)
  }
}

// 提交审批决策
async function decide(item: PendingItem, option: string) {
  if (item.deciding) return
  item.deciding = true
  try {
    await safeInvoke('approval_response', {
      approvalId: item.id,
      approved: option === item.recommended,
      comment: item.comment || null,
      option,
    })
    // 从列表移除
    if (item.timer) clearInterval(item.timer)
    pending.value = pending.value.filter(p => p.id !== item.id)
  } catch (e) {
    toast.error('Failed to submit decision: ' + (e as Error).message)
  } finally {
    item.deciding = false
  }
}

// 监听 step-update 事件，检测新的待审批
let unlisten: (() => void) | null = null

async function init() {
  // 初始加载
  await loadPending()

  // 监听 step-update 中的 awaiting_approval 状态
  unlisten = await safeListen<{
    run_id: string
    step_id: string
    status: string
    output?: Record<string, unknown>
  }>('step-update', async (event) => {
    const output = event.payload?.output
    if (output?.status === 'awaiting_approval') {
      // 有新审批请求，刷新列表
      await loadPending()
    }
  })
}

onMounted(() => { init() })

onUnmounted(() => {
  unlisten?.()
  pending.value.forEach(p => { if (p.timer) clearInterval(p.timer) })
})
</script>

<template>
  <!-- 浮动按钮：显示待审批数量 -->
  <Button
    v-if="pendingCount > 0"
    class="fixed bottom-6 right-6 z-[60] flex items-center gap-2 rounded-full px-4 py-3 shadow-[0_12px_32px_rgba(0,0,0,0.12)]"
    @click="visible = !visible"
  >
    <Hand class="w-5 h-5" />
    <span class="font-medium">{{ t('approval.pending') }}</span>
    <Badge variant="secondary" class="ml-1">{{ pendingCount }}</Badge>
  </Button>

  <!-- 侧边面板 -->
  <Teleport to="body">
    <Transition name="slide">
      <div
        v-if="visible && pendingCount > 0"
        class="fixed inset-y-0 right-0 z-[60] w-[420px] bg-[var(--bg-base-default)] border-l border-[var(--border-neutral-l1)] shadow-[0_24px_64px_rgba(0,0,0,0.14)] flex flex-col"
      >
        <!-- 头部 -->
        <div class="flex items-center justify-between px-5 py-4 border-b border-[var(--border-neutral-l1)]">
          <div class="flex items-center gap-2">
            <Hand class="w-5 h-5" />
            <h2 class="text-base font-semibold text-[var(--text-default)]">{{ t('approval.pending') }}</h2>
            <Badge variant="secondary">{{ pendingCount }}</Badge>
          </div>
          <Button
            variant="ghost"
            size="icon"
            class="h-7 w-7"
            @click="visible = false"
          >
            ✕
          </Button>
        </div>

        <!-- 审批列表 -->
        <div class="flex-1 overflow-y-auto p-4 space-y-4">
          <div
            v-for="item in pending"
            :key="item.id"
            class="rounded-lg border border-[var(--border-neutral-l1)] bg-[var(--bg-base-secondary)] p-4 space-y-3"
          >
            <!-- 标题 + 时间 -->
            <div class="flex items-start justify-between">
              <div>
                <h3 class="font-medium text-[var(--text-default)]">{{ item.title }}</h3>
                <p class="text-xs text-[var(--text-tertiary)] mt-0.5">
                  {{ t('approval.stepLabel') }}: {{ item.step_id }} · {{ formatElapsed(item.elapsed) }}
                </p>
              </div>
              <div
                v-if="item.timeout_secs > 0"
                class="text-xs px-2 py-1 rounded"
                :class="timeoutPercent(item) > 80 ? 'bg-[var(--status-error-default)]/10 text-[var(--status-error-default)]' : 'bg-[var(--bg-overlay-l1)] text-[var(--text-tertiary)]'"
              >
                {{ formatTimeout(item) }}
              </div>
            </div>

            <!-- 超时进度条 -->
            <div
              v-if="item.timeout_secs > 0"
              class="h-1 rounded-full bg-[var(--bg-overlay-l1)] overflow-hidden"
            >
              <div
                class="h-full rounded-full transition-all duration-1000"
                :class="timeoutPercent(item) > 80 ? 'bg-[var(--status-error-default)]' : 'bg-[var(--bg-brand)]'"
                :style="{ width: `${timeoutPercent(item)}%` }"
              />
            </div>

            <!-- 审批内容 -->
            <div class="rounded-md bg-[var(--bg-overlay-l1)] p-3 text-sm text-[var(--text-default)] whitespace-pre-wrap break-words">
              {{ item.message }}
            </div>

            <!-- 上游数据（可展开） -->
            <div v-if="item.item">
              <Button
                variant="link"
                class="text-xs px-0 py-0.5 h-auto"
                @click="item.showItem = !item.showItem"
              >
                {{ item.showItem ? '▼' : '▶' }} {{ t('approval.viewData') }}
              </Button>
              <pre
                v-if="item.showItem"
                class="mt-1 rounded-md border border-[var(--border-neutral-l1)] bg-[var(--bg-overlay-l1)] p-2 text-xs text-[var(--text-tertiary)] font-mono max-h-[150px] overflow-y-auto whitespace-pre-wrap break-all"
              >{{ JSON.stringify(item.item, null, 2) }}</pre>
            </div>

            <!-- 选项按钮 -->
            <div class="flex flex-wrap gap-2">
              <Button
                v-for="opt in item.options"
                :key="opt"
                :variant="opt === item.recommended ? 'default' : 'outline'"
                :disabled="item.deciding"
                size="sm"
                @click="decide(item, opt)"
              >
                <Star v-if="opt === item.recommended" class="w-3 h-3 mr-1 text-[var(--status-warning-default)] fill-warning" />
                {{ opt }}
              </Button>
            </div>

            <!-- 推荐提示 -->
            <p
              v-if="item.recommended"
              class="text-xs text-[var(--text-tertiary)]"
            >
              <Lightbulb class="w-3 h-3 inline-block mr-1 text-[var(--status-warning-default)]" />{{ t('approval.recommendedLabel') }}: {{ item.recommended }}
            </p>

            <!-- 条件推荐原因（新增） -->
            <p
              v-if="item.recommendation_reason"
              class="text-xs text-[var(--text-brand)] whitespace-pre-wrap"
            >
              <Search class="w-3 h-3 inline-block mr-1 text-[var(--text-brand)]" />{{ item.recommendation_reason }}
            </p>

            <!-- 审批意见 -->
            <Textarea
              v-model="item.comment"
              :rows="1"
              :placeholder="t('approval.commentPlaceholder')"
              class="text-sm"
            />
          </div>
        </div>
      </div>
    </Transition>

    <!-- 遮罩 -->
    <Transition name="fade">
      <div
        v-if="visible && pendingCount > 0"
        class="fixed inset-0 z-40 bg-black/30"
        @click="visible = false"
      />
    </Transition>
  </Teleport>
</template>

<style scoped>
.slide-enter-active,
.slide-leave-active {
  transition: transform 0.3s ease;
}
.slide-enter-from,
.slide-leave-to {
  transform: translateX(100%);
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
