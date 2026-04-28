<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '../composables/useToast'
import { describeCron } from '../utils/cron'

const toast = useToast()

interface ScheduleInfo {
  id: string
  workflow_id: string
  workflow_name: string
  cron_expr: string
  enabled: boolean
  last_run_at: string | null
  created_at: string
}

interface WorkflowItem {
  id: string
  name: string
}

const show = ref(false)
const editingId = ref<string | null>(null)
const workflowId = ref('')
const cronExpr = ref('')
const saving = ref(false)

const workflows = ref<WorkflowItem[]>([])

const presets = [
  { label: '每小时', value: '0 * * * *' },
  { label: '每天 09:00', value: '0 9 * * *' },
  { label: '工作日 09:00', value: '0 9 * * 1-5' },
  { label: '每周一 09:00', value: '0 9 * * 1' },
  { label: '每天 00:00', value: '0 0 * * *' },
  { label: '每 30 分钟', value: '*/30 * * * *' },
  { label: '每 5 分钟', value: '*/5 * * * *' },
]

const emit = defineEmits<{
  saved: []
}>()

async function open(wfs: WorkflowItem[], schedule?: ScheduleInfo) {
  workflows.value = wfs
  if (schedule) {
    editingId.value = schedule.id
    workflowId.value = schedule.workflow_id
    cronExpr.value = schedule.cron_expr
  } else {
    editingId.value = null
    workflowId.value = wfs.length > 0 ? wfs[0].id : ''
    cronExpr.value = ''
  }
  show.value = true
}

function close() {
  show.value = false
}

function applyPreset(value: string) {
  cronExpr.value = value
}

async function save() {
  if (!workflowId.value) { toast.error('请选择工作流'); return }
  if (!cronExpr.value.trim()) { toast.error('请输入 cron 表达式'); return }

  saving.value = true
  try {
    if (editingId.value) {
      await invoke('schedule_update', { id: editingId.value, cronExpr: cronExpr.value.trim() })
      toast.success('计划已更新')
    } else {
      await invoke('schedule_create', { workflowId: workflowId.value, cronExpr: cronExpr.value.trim() })
      toast.success('计划已创建')
    }
    show.value = false
    emit('saved')
  } catch (e: any) {
    toast.error(e.message || e)
  } finally {
    saving.value = false
  }
}

defineExpose({ open })
</script>

<template>
  <Teleport to="body">
    <div v-if="show" class="dialog-overlay" @click.self="close">
      <div class="dialog">
        <div class="dialog-header">
          <h3>{{ editingId ? '✏️ 编辑计划' : '⏰ 新建定时计划' }}</h3>
          <button class="close-btn" @click="close">✕</button>
        </div>

        <div class="dialog-body">
          <!-- 选择工作流 -->
          <div class="form-group">
            <label>工作流</label>
            <select v-model="workflowId" :disabled="!!editingId">
              <option v-for="wf in workflows" :key="wf.id" :value="wf.id">{{ wf.name }}</option>
            </select>
          </div>

          <!-- Cron 表达式 -->
          <div class="form-group">
            <label>Cron 表达式 <span class="label-hint">（5 字段：分 时 日 月 周）</span></label>
            <input
              v-model="cronExpr"
              type="text"
              placeholder="0 9 * * *"
              class="cron-input"
            />
            <div v-if="describeCron(cronExpr)" class="cron-preview">
              📅 {{ describeCron(cronExpr) }}
            </div>
          </div>

          <!-- 快捷预设 -->
          <div class="form-group">
            <label>快捷预设</label>
            <div class="presets">
              <button
                v-for="p in presets"
                :key="p.value"
                class="preset-btn"
                :class="{ active: cronExpr === p.value }"
                @click="applyPreset(p.value)"
              >
                {{ p.label }}
                <span class="preset-expr">{{ p.value }}</span>
              </button>
            </div>
          </div>
        </div>

        <div class="dialog-footer">
          <button class="btn" @click="close">取消</button>
          <button class="btn btn-primary" @click="save" :disabled="saving">
            {{ saving ? '保存中...' : '保存' }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.dialog-overlay {
  position: fixed; inset: 0; background: rgba(0,0,0,0.6);
  display: flex; align-items: center; justify-content: center; z-index: 1000;
}
.dialog {
  background: #161b22; border: 1px solid #30363d; border-radius: 12px;
  width: 520px; max-height: 80vh; overflow-y: auto;
  box-shadow: 0 8px 32px rgba(0,0,0,0.4);
}
.dialog-header {
  display: flex; justify-content: space-between; align-items: center;
  padding: 16px 20px; border-bottom: 1px solid #21262d;
}
.dialog-header h3 { margin: 0; font-size: 16px; color: #e1e4e8; }
.close-btn {
  background: none; border: none; color: #8b949e; font-size: 16px;
  cursor: pointer; padding: 4px 8px; border-radius: 4px;
}
.close-btn:hover { background: #21262d; color: #e1e4e8; }

.dialog-body { padding: 20px; }

.form-group { margin-bottom: 16px; }
.form-group label {
  display: block; font-size: 13px; font-weight: 600; color: #c9d1d9;
  margin-bottom: 6px;
}
.label-hint { font-weight: 400; color: #6e7681; }

.form-group select, .form-group input {
  width: 100%; padding: 8px 12px; background: #0d1117;
  border: 1px solid #30363d; border-radius: 6px; color: #c9d1d9;
  font-size: 14px; font-family: 'Cascadia Code', monospace;
  box-sizing: border-box;
}
.form-group select:focus, .form-group input:focus {
  outline: none; border-color: #58a6ff;
}

.cron-input { font-size: 16px; letter-spacing: 1px; }
.cron-preview {
  margin-top: 6px; font-size: 12px; color: #3fb950; background: #23863615;
  padding: 4px 8px; border-radius: 4px;
}

.presets { display: flex; flex-wrap: wrap; gap: 6px; }
.preset-btn {
  background: #21262d; border: 1px solid #30363d; border-radius: 6px;
  padding: 6px 10px; cursor: pointer; font-size: 12px; color: #c9d1d9;
  display: flex; flex-direction: column; gap: 2px; transition: all 0.15s;
}
.preset-btn:hover { border-color: #58a6ff; background: #1f6feb15; }
.preset-btn.active { border-color: #58a6ff; background: #1f6feb22; }
.preset-expr { font-size: 10px; color: #6e7681; font-family: 'Cascadia Code', monospace; }

.dialog-footer {
  display: flex; justify-content: flex-end; gap: 8px;
  padding: 12px 20px; border-top: 1px solid #21262d;
}
.btn {
  padding: 6px 16px; border-radius: 6px; font-size: 13px; font-weight: 500;
  cursor: pointer; border: 1px solid #30363d; background: #21262d; color: #c9d1d9;
  transition: all 0.15s;
}
.btn:hover { background: #30363d; }
.btn-primary { background: #238636; border-color: #238636; color: #fff; }
.btn-primary:hover { background: #2ea043; }
.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
