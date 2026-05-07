<script setup lang="ts">
import { ref, onUnmounted } from 'vue'
import { safeListen, safeInvoke } from '../utils/tauri'

interface ApprovalEvent {
  run_id: string
  step_id: string
  approval_id: string
  message: string
  options: string[]
}

const visible = ref(false)
const approvalData = ref<ApprovalEvent | null>(null)
const deciding = ref(false)

let unlisten: (() => void) | null = null

async function init() {
  unlisten = await safeListen<ApprovalEvent>('approval-required', (event) => {
    approvalData.value = event.payload
    visible.value = true
  })
}

async function decide(approved: boolean) {
  if (!approvalData.value || deciding.value) return
  deciding.value = true
  try {
    await safeInvoke('approval_response', {
      approvalId: approvalData.value.approval_id,
      approved,
    })
  } catch (e) {
    console.error('[ApprovalDialog] 审批响应失败:', e)
  }
  visible.value = false
  approvalData.value = null
  deciding.value = false
}

init()

onUnmounted(() => {
  unlisten?.()
})
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="approval-overlay" @click.self="() => {}">
      <div class="approval-card">
        <div class="approval-header">
          <span class="approval-icon">✋</span>
          <span class="approval-title">人工审批</span>
        </div>
        <div class="approval-body">
          <div class="approval-message">{{ approvalData?.message || '请审批此操作' }}</div>
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
.approval-body {
  margin-bottom: 24px;
}
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
