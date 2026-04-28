<script setup lang="ts">
import { computed, ref } from 'vue'

interface Milestone {
  id: string
  label: string
  desc: string
  done: boolean
}

const props = defineProps<{
  milestones: Milestone[]
  version: string
}>()

const showProgress = ref(false)

const doneCount = computed(() => props.milestones.filter(m => m.done).length)
const progressPct = computed(() => Math.round((doneCount.value / props.milestones.length) * 100))
</script>

<template>
  <div class="progress-section">
    <button class="progress-toggle" @click="showProgress = !showProgress">
      <span>🚀 v{{ version }} · 开发进度 {{ progressPct }}%</span>
      <span class="toggle-arrow" :class="{ open: showProgress }">▸</span>
    </button>

    <div v-if="showProgress" class="progress-body">
      <div class="progress-bar-wrap">
        <div class="progress-bar" :style="{ width: progressPct + '%' }"></div>
      </div>
      <div class="milestones">
        <div v-for="m in milestones" :key="m.id" class="ms-item" :class="{ done: m.done, current: !m.done }">
          <div class="ms-head">
            <span class="ms-badge" :class="m.done ? 'done' : 'todo'">{{ m.id }}</span>
            <span class="ms-label">{{ m.label }}</span>
            <span v-if="m.done" class="ms-check">✅</span>
            <span v-else class="ms-pending">⬜</span>
          </div>
          <div class="ms-desc">{{ m.desc }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.progress-section { margin-top: 8px; border: 1px solid #30363d; border-radius: 10px; overflow: hidden; background: #161b22; }
.progress-toggle { display: flex; justify-content: space-between; align-items: center; width: 100%; padding: 12px 16px; background: none; border: none; color: #8b949e; font-size: 13px; cursor: pointer; transition: background 0.15s; }
.progress-toggle:hover { background: #1c2128; }
.toggle-arrow { font-size: 14px; transition: transform 0.2s; }
.toggle-arrow.open { transform: rotate(90deg); }
.progress-body { padding: 0 16px 16px; border-top: 1px solid #21262d; }
.progress-bar-wrap { height: 6px; background: #21262d; border-radius: 3px; margin: 12px 0 16px; overflow: hidden; }
.progress-bar { height: 100%; background: linear-gradient(90deg, #238636, #3fb950); border-radius: 3px; transition: width 0.3s; }

.milestones { display: flex; flex-direction: column; gap: 8px; margin-bottom: 16px; }
.ms-item { padding: 8px 10px; border-radius: 6px; background: #0d1117; border: 1px solid #21262d; }
.ms-item.done { border-color: #23863644; }
.ms-item.current { border-color: #1f6feb44; }
.ms-head { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
.ms-badge { font-size: 10px; font-weight: 700; padding: 1px 6px; border-radius: 4px; letter-spacing: 0.3px; }
.ms-badge.done { background: #23863622; color: #3fb950; }
.ms-badge.todo { background: #1f6feb22; color: #58a6ff; }
.ms-label { font-size: 13px; font-weight: 600; color: #e1e4e8; flex: 1; }
.ms-check { font-size: 12px; }
.ms-pending { font-size: 12px; opacity: 0.5; }
.ms-desc { font-size: 11px; color: #6e7681; line-height: 1.5; padding-left: 54px; }
</style>
