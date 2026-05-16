<script setup lang="ts">
import { ref, onUnmounted, computed } from 'vue'
import { Hand } from 'lucide-vue-next'
import { safeListen, safeInvoke } from '../utils/tauri'
import Button from '@/components/ui/button/Button.vue'
import Badge from '@/components/ui/badge/Badge.vue'
import Textarea from '@/components/ui/textarea/Textarea.vue'

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
  if (secs < 60) return `${secs}秒前`
  if (secs < 3600) return `${Math.floor(secs / 60)}分钟前`
  return `${Math.floor(secs / 3600)}小时前`
}

// 格式化超时倒计时
function formatTimeout(entry: PendingItem): string {
  if (entry.timeout_secs <= 0) return '无超时'
  const remaining = entry.timeout_secs - entry.elapsed
  if (remaining <= 0) return '即将超时'
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
    console.error('[ApprovalCenter] 加载待审批失败:', e)
  }
}

// 提交审批决策
async function decide(item: PendingItem, option: string) {
  if (item.deciding) return
  item.deciding = true
  try {
    await safeInvoke('approval_response', {
      approvalId: item.id,
      approved: option === '同意' || option === item.recommended,
      comment: item.comment || null,
      option,
    })
    // 从列表移除
    if (item.timer) clearInterval(item.timer)
    pending.value = pending.value.filter(p => p.id !== item.id)
  } catch (e) {
    console.error('[ApprovalCenter] 审批决策失败:', e)
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

init()

onUnmounted(() => {
  unlisten?.()
  pending.value.forEach(p => { if (p.timer) clearInterval(p.timer) })
})
</script>

<template>
  <!-- 浮动按钮：显示待审批数量 -->
  <button
    v-if="pendingCount > 0"
    class="fixed bottom-6 right-6 z-50 flex items-center gap-2 rounded-full bg-primary px-4 py-3 text-primary-foreground shadow-lg hover:bg-primary/90 transition-colors cursor-pointer"
    @click="visible = !visible"
  >
    <Hand class="w-5 h-5" />
    <span class="font-medium">待审批</span>
    <Badge variant="secondary" class="ml-1">{{ pendingCount }}</Badge>
  </button>

  <!-- 侧边面板 -->
  <Teleport to="body">
    <Transition name="slide">
      <div
        v-if="visible && pendingCount > 0"
        class="fixed inset-y-0 right-0 z-50 w-[420px] bg-background border-l border-border shadow-2xl flex flex-col"
      >
        <!-- 头部 -->
        <div class="flex items-center justify-between px-5 py-4 border-b border-border">
          <div class="flex items-center gap-2">
            <Hand class="w-5 h-5" />
            <h2 class="text-base font-semibold text-foreground">待审批</h2>
            <Badge variant="secondary">{{ pendingCount }}</Badge>
          </div>
          <button
            class="text-muted-foreground hover:text-foreground transition-colors p-1 cursor-pointer"
            @click="visible = false"
          >
            ✕
          </button>
        </div>

        <!-- 审批列表 -->
        <div class="flex-1 overflow-y-auto p-4 space-y-4">
          <div
            v-for="item in pending"
            :key="item.id"
            class="rounded-lg border border-border bg-card p-4 space-y-3"
          >
            <!-- 标题 + 时间 -->
            <div class="flex items-start justify-between">
              <div>
                <h3 class="font-medium text-foreground">{{ item.title }}</h3>
                <p class="text-xs text-muted-foreground mt-0.5">
                  步骤: {{ item.step_id }} · {{ formatElapsed(item.elapsed) }}
                </p>
              </div>
              <div
                v-if="item.timeout_secs > 0"
                class="text-xs px-2 py-1 rounded"
                :class="timeoutPercent(item) > 80 ? 'bg-destructive/10 text-destructive' : 'bg-muted text-muted-foreground'"
              >
                {{ formatTimeout(item) }}
              </div>
            </div>

            <!-- 超时进度条 -->
            <div
              v-if="item.timeout_secs > 0"
              class="h-1 rounded-full bg-muted overflow-hidden"
            >
              <div
                class="h-full rounded-full transition-all duration-1000"
                :class="timeoutPercent(item) > 80 ? 'bg-destructive' : 'bg-primary'"
                :style="{ width: `${timeoutPercent(item)}%` }"
              />
            </div>

            <!-- 审批内容 -->
            <div class="rounded-md bg-muted p-3 text-sm text-foreground whitespace-pre-wrap break-words">
              {{ item.message }}
            </div>

            <!-- 上游数据（可展开） -->
            <div v-if="item.item">
              <button
                class="text-xs text-primary hover:underline py-0.5 cursor-pointer"
                @click="item.showItem = !item.showItem"
              >
                {{ item.showItem ? '▼' : '▶' }} 查看数据
              </button>
              <pre
                v-if="item.showItem"
                class="mt-1 rounded-md border border-border bg-muted p-2 text-xs text-muted-foreground font-mono max-h-[150px] overflow-y-auto whitespace-pre-wrap break-all"
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
                <span v-if="opt === item.recommended" class="mr-1">⭐</span>
                {{ opt }}
              </Button>
            </div>

            <!-- 推荐提示 -->
            <p
              v-if="item.recommended"
              class="text-xs text-muted-foreground"
            >
              💡 推荐: {{ item.recommended }}
            </p>

            <!-- 审批意见 -->
            <Textarea
              v-model="item.comment"
              :rows="1"
              placeholder="审批意见（可选）"
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
