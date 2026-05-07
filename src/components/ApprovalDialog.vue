<script setup lang="ts">
import { ref, onUnmounted } from 'vue'
import { safeListen, safeInvoke } from '../utils/tauri'

interface ApprovalData {
  run_id: string
  step_id: string
  approval_id: string
  title: string
  message: string
  options?: string[]
}

const visible = ref(false)
const approvalData = ref<ApprovalData | null>(null)
const deciding = ref(false)
const comment = ref('')

let unlistenA: (() => void) | null = null
let unlistenS: (() => void) | null = null

async function init() {
  // 监听 approval-required 事件（scheduler 触发）
  unlistenA = await safeListen<any>('approval-required', (event) => {
    approvalData.value = {
      run_id: event.payload.run_id,
      step_id: event.payload.step_id,
      approval_id: event.payload.approval_id,
      title: '人工审批',
      message: event.payload.message || '请审批此操作',
    }
    visible.value = true
  })

  // 监听 step-update，当审批步骤输出 awaiting_approval 时展示数据
  unlistenS = await safeListen<any>('step-update', (event) => {
    const output = event.payload?.output
    if (output?.status === 'awaiting_approval') {
      approvalData.value = {
        run_id: event.payload.run_id,
        step_id: event.payload.step_id,
        approval_id: `approval:${event.payload.step_id}`,
        title: output.title || '人工审批',
        message: output.message || '请审批此操作',
      }
      visible.value = true
    }
  })
}

async function decide(approved: boolean) {
  if (!approvalData.value || deciding.value) return
  deciding.value = true
  try {
    await safeInvoke('approval_response', {
      approvalId: approvalData.value.approval_id,
      approved,
      comment: comment.value || null,
    })
  } catch (e) {
    console.error('[ApprovalDialog] 审批响应失败:', e)
  }
  visible.value = false
  approvalData.value = null
  comment.value = ''
  deciding.value = false
}

init()

onUnmounted(() => {
  unlistenA?.()
  unlistenS?.()
})
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="approval-overlay">
      <div class="approval-card">
        <div class="approval-header">
          <span class="approval-icon">✋</span>
          <span class="approval-title">{{ approvalData?.title || '人工审批' }}</span>
        </div>
        <div class="approval-body">
          <div class="approval-message">{{ approvalData?.message || '请审批此操作' }}</div>
          <div class="approval-comment">
            <input
              v-model="comment"
              type="text"
              placeholder="审批意见（可选）"
              class="comment-input"
            />
          </div>
          <div class="approval-meta" v-if="approvalData">
            <span>步骤: {{ approvalData.step_id }}</span>
            <span>运行: {{ approvalData.run_id }}</span>
          </div>
        </div>
        <div class="approval-actions">
          <button class="btn btn-reject" :disabled="deciding" @click="decide(false)">
            ❌ 拒绝
          </button>
          <button class="btn btn-approve" :disabled="deciding" @click="decide(true)">
            ✅ 同意
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.approval-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  backdrop-filter: blur(4px);
  background: rgba(0, 0, 0, 0.45);
  animation: fadeIn 0.15s ease;
}
.approval-card {
  background: #1e1e2e;
  border: 1px solid #f778ba;
  border-radius: 14px;
  padding: 28px 32px;
  width: 440px;
  max-width: 90vw;
  box-shadow: 0 8px 40px rgba(247, 120, 186, 0.2);
  animation: slideUp 0.2s ease;
}
.approval-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 18px;
}
.approval-icon { font-size: 24px; }
.approval-title { font-size: 18px; font-weight: 600; color: #f778ba; }
.approval-body { margin-bottom: 24px; }
.approval-message {
  background: #2a2a3c;
  border-radius: 8px;
  padding: 14px 16px;
  font-size: 14px;
  line-height: 1.6;
  color: #cdd6f4;
  white-space: pre-wrap;
  word-break: break-word;
}
.approval-comment { margin-top: 12px; }
.comment-input {
  width: 100%;
  padding: 10px 12px;
  border-radius: 8px;
  border: 1px solid #45475a;
  background: #1e1e2e;
  color: #cdd6f4;
  font-size: 13px;
  outline: none;
}
.comment-input:focus { border-color: #f778ba; }
.comment-input::placeholder { color: #585b70; }
.approval-meta {
  display: flex;
  gap: 16px;
  margin-top: 10px;
  font-size: 11px;
  color: #6c7086;
}
.approval-actions {
  display: flex;
  gap: 12px;
  justify-content: flex-end;
}
.btn {
  padding: 10px 24px;
  border: none;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: opacity 0.15s;
}
.btn:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-reject {
  background: #3a1f2a;
  color: #f38ba8;
  border: 1px solid #f38ba8;
}
.btn-reject:hover:not(:disabled) { background: #4a2532; }
.btn-approve {
  background: #a6e3a1;
  color: #1e1e2e;
}
.btn-approve:hover:not(:disabled) { background: #94d89f; }

@keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
@keyframes slideUp { from { opacity: 0; transform: translateY(20px); } to { opacity: 1; transform: translateY(0); } }
</style>
