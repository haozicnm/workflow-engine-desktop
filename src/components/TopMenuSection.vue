<template>
  <header class="top-menu">
    <div class="menu-left">
      <span class="workflow-name">{{ name }}</span>
      <span class="stat-badge">{{ nodeCount }} 节点</span>
      <span class="stat-badge">{{ edgeCount }} 连线</span>
      <span v-if="dirty" class="stat-badge dirty">● 已修改</span>
    </div>
    <div class="menu-center">
      <button class="menu-btn primary" :disabled="disableRun" @click="$emit('run')">
        ▶ 运行
      </button>
      <button class="menu-btn" :disabled="disableRun" @click="$emit('step')">
        ⏯ 单步
      </button>
      <button class="menu-btn" :disabled="!running" @click="$emit('stop')">
        ■ 停止
      </button>
      <button class="menu-btn" @click="$emit('save')" title="保存到工作流">
        💾 保存
      </button>
    </div>
    <div class="menu-right">
      <button
        :class="['menu-btn', recording ? 'recording' : '']"
        @click="$emit('record')"
        title="录制浏览器操作"
      >
        {{ recording ? '⏹ 停止录制' : '🔴 录制' }}
      </button>
      <button class="menu-btn" @click="$emit('pick')" title="拾取浏览器元素">
        🎯 拾取
      </button>
      <button class="menu-btn" @click="$emit('import')" title="导入 JSON">
        📥 导入
      </button>
      <button class="menu-btn" @click="$emit('export')" title="导出 JSON">
        📤 导出
      </button>
      <button class="menu-btn" @click="$emit('clear')">
        🗑 清空
      </button>
    </div>
  </header>
</template>

<script setup lang="ts">
defineProps<{
  name: string
  nodeCount: number
  edgeCount: number
  dirty: boolean
  running: boolean
  recording: boolean
  disableRun: boolean
}>()

defineEmits<{
  run: []
  step: []
  stop: []
  save: []
  record: []
  pick: []
  import: []
  export: []
  clear: []
}>()
</script>

<style scoped>
.top-menu {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 40px;
  padding: 0 12px;
  background: var(--color-surface);
  border-bottom: 1px solid var(--color-border);
  flex-shrink: 0;
  gap: 12px;
}
.menu-left {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}
.workflow-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.stat-badge {
  font-size: 11px;
  color: #8b949e;
  padding: 2px 8px;
  background: #21262d;
  border-radius: 10px;
  white-space: nowrap;
}
.stat-badge.dirty { color: #d29922; }
.menu-center {
  display: flex;
  gap: 4px;
}
.menu-right {
  display: flex;
  gap: 4px;
}
.menu-btn {
  height: 28px;
  padding: 0 10px;
  border: 1px solid #30363d;
  border-radius: 6px;
  background: #21262d;
  color: #c9d1d9;
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s;
}
.menu-btn:hover { background: #30363d; }
.menu-btn.primary {
  background: #238636;
  border-color: #238636;
  color: #fff;
}
.menu-btn.primary:hover { background: #2ea043; }
.menu-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.menu-btn.recording {
  background: #da3633;
  border-color: #da3633;
  color: #fff;
  animation: pulse 1.5s ease-in-out infinite;
}
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.7; }
}
</style>
