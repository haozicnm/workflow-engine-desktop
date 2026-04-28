<script setup lang="ts">
import { ref, watch } from 'vue'

const props = defineProps<{
  yamlText: string
  error: string
}>()

const emit = defineEmits<{
  'change': [text: string]
  'sync-to-yaml': []
  'sync-from-yaml': []
}>()

const localText = ref(props.yamlText)
const lineCount = ref(1)

watch(() => props.yamlText, (val) => {
  localText.value = val
  updateLineCount()
})

function updateLineCount() {
  lineCount.value = localText.value.split('\n').length
}

function onInput(e: Event) {
  const target = e.target as HTMLTextAreaElement
  localText.value = target.value
  updateLineCount()
  emit('change', localText.value)
}
</script>

<template>
  <div class="yaml-panel">
    <div class="yaml-header">
      <span class="yaml-title">YAML</span>
      <div class="yaml-actions">
        <button class="btn btn-xs" @click="emit('sync-from-yaml')" title="YAML → 画布">➡ 同步到画布</button>
        <button class="btn btn-xs" @click="emit('sync-to-yaml')" title="画布 → YAML">⬅ 同步到YAML</button>
      </div>
    </div>

    <div v-if="error" class="yaml-error">{{ error }}</div>

    <div class="yaml-editor">
      <div class="line-numbers">
        <div v-for="n in lineCount" :key="n" class="line-num">{{ n }}</div>
      </div>
      <textarea
        :value="localText"
        @input="onInput"
        class="yaml-textarea"
        spellcheck="false"
        wrap="off"
      ></textarea>
    </div>
  </div>
</template>

<style scoped>
.yaml-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
}
.yaml-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background: #161b22;
  border-bottom: 1px solid #30363d;
}
.yaml-title {
  font-size: 12px;
  font-weight: 600;
  color: #8b949e;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.yaml-error {
  padding: 8px 12px;
  background: #da363322;
  color: #f85149;
  font-size: 12px;
  border-bottom: 1px solid #da363344;
}
.yaml-editor {
  flex: 1;
  display: flex;
  overflow: hidden;
}
.line-numbers {
  width: 36px;
  flex-shrink: 0;
  padding: 8px 4px;
  text-align: right;
  background: #161b22;
  border-right: 1px solid #21262d;
  overflow: hidden;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
  font-size: 12px;
  line-height: 1.5;
  color: #484f58;
}
.yaml-textarea {
  flex: 1;
  padding: 8px 12px;
  background: #0d1117;
  color: #c9d1d9;
  border: none;
  resize: none;
  font-family: 'Cascadia Code', 'Fira Code', monospace;
  font-size: 12px;
  line-height: 1.5;
  outline: none;
  white-space: pre;
  overflow-x: auto;
  tab-size: 2;
}
</style>
