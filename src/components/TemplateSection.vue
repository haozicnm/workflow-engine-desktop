<script setup lang="ts">
import { ref } from 'vue'

interface TemplateItem {
  id: string
  name: string
  description: string
}

const props = defineProps<{
  templates: TemplateItem[]
  creatingFromTemplate: string | null
}>()

const emit = defineEmits<{
  'create-from-template': [tpl: TemplateItem]
}>()

const showTemplates = ref(false)
</script>

<template>
  <div v-if="templates.length > 0" class="templates-section">
    <button class="progress-toggle" @click="showTemplates = !showTemplates">
      <span>📦 内置模板 · {{ templates.length }} 个可用</span>
      <span class="toggle-arrow" :class="{ open: showTemplates }">▸</span>
    </button>
    <div v-if="showTemplates" class="templates-body">
      <div class="template-grid">
        <div v-for="tpl in templates" :key="tpl.id" class="tpl-card">
          <div class="tpl-info">
            <div class="tpl-name">{{ tpl.name }}</div>
            <div class="tpl-desc">{{ tpl.description }}</div>
          </div>
          <button
            class="btn btn-primary btn-sm"
            @click="emit('create-from-template', tpl)"
            :disabled="creatingFromTemplate === tpl.id"
          >
            {{ creatingFromTemplate === tpl.id ? '创建中...' : '使用此模板' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.templates-section { margin-top: 8px; border: 1px solid #30363d; border-radius: 10px; overflow: hidden; background: #161b22; }
.progress-toggle { display: flex; justify-content: space-between; align-items: center; width: 100%; padding: 12px 16px; background: none; border: none; color: #8b949e; font-size: 13px; cursor: pointer; transition: background 0.15s; }
.progress-toggle:hover { background: #1c2128; }
.toggle-arrow { font-size: 14px; transition: transform 0.2s; }
.toggle-arrow.open { transform: rotate(90deg); }
.templates-body { padding: 0 16px 16px; border-top: 1px solid #21262d; }
.template-grid { display: flex; flex-direction: column; gap: 10px; margin-top: 12px; }
.tpl-card { display: flex; justify-content: space-between; align-items: center; padding: 12px 14px; background: #0d1117; border: 1px solid #21262d; border-radius: 8px; transition: border-color 0.15s; }
.tpl-card:hover { border-color: #484f58; }
.tpl-info { flex: 1; margin-right: 12px; }
.tpl-name { font-size: 13px; font-weight: 600; color: #e1e4e8; margin-bottom: 4px; }
.tpl-desc { font-size: 11px; color: #6e7681; }

.btn { padding: 6px 14px; border-radius: 6px; font-size: 13px; font-weight: 500; cursor: pointer; border: 1px solid #30363d; background: #21262d; color: #c9d1d9; transition: all 0.15s; }
.btn:hover { background: #30363d; }
.btn-sm { padding: 5px 12px; font-size: 12px; }
.btn-primary { background: #1f6feb; border-color: #388bfd; color: #fff; }
.btn-primary:hover { background: #388bfd; }
</style>
