<script setup lang="ts">
import { ref, watch } from 'vue'
import type { Workflow } from '../types/types'
import yaml from 'js-yaml'
import Button from './ui/button/Button.vue'
import Textarea from './ui/textarea/Textarea.vue'



const props = withDefaults(defineProps<{
  workflow: Workflow
}>(), {})

const emit = defineEmits<{
  'update:workflow': [wf: Workflow]
}>()

type ViewFormat = 'json' | 'yaml'
const format = ref<ViewFormat>('json')
const codeText = ref('')
const error = ref('')
const isEditing = ref(false)

function serialize() {
  const data = {
    name: props.workflow.name,
    description: props.workflow.description || '',
    steps: props.workflow.steps,
  }
  if (format.value === 'json') {
    codeText.value = JSON.stringify(data, null, 2)
  } else {
    codeText.value = yaml.dump(data, { indent: 2, lineWidth: 120, noRefs: true })
  }
  error.value = ''
}

function applyChanges() {
  try {
    let data: Record<string, unknown>
    if (format.value === 'json') {
      data = JSON.parse(codeText.value)
    } else {
      data = yaml.load(codeText.value) as Record<string, unknown>
    }
    if (!data || typeof data !== 'object') throw new Error('无效的数据格式')
    if (!Array.isArray(data.steps)) throw new Error('缺少 steps 数组')

    emit('update:workflow', {
      name: (data.name as string) || props.workflow.name,
      description: (data.description as string) || '',
      steps: data.steps as Workflow['steps'],
    })
    error.value = ''
  } catch (e: unknown) {
    error.value = (e as Error).message || '解析失败'
  }
}

watch(() => [props.workflow.name, props.workflow.steps], () => {
  if (!isEditing.value) serialize()
}, { deep: true })

watch(format, () => {
  if (!isEditing.value) serialize()
})

serialize()

function onFocus() { isEditing.value = true }
function onBlur() {
  isEditing.value = false
  applyChanges()
  serialize()
}
</script>

<template>
  <div class="flex flex-col flex-1 min-h-0">
    <div class="flex items-center gap-3 px-3 py-1.5 border-b border-[var(--border-neutral-l1)] bg-[var(--bg-base-secondary)] shrink-0">
      <div class="flex gap-0.5">
        <Button
          v-for="f in (['json', 'yaml'] as ViewFormat[])"
          :key="f"
          :variant="format === f ? 'default' : 'outline'"
          size="sm"
          class="h-7 px-2.5 text-xs font-mono uppercase"
          @click="format = f"
        >
          {{ f }}
        </Button>
      </div>
      <span v-if="error" class="text-xs text-[var(--status-error-default)] flex items-center gap-1">
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><path d="M12 9v4"/><path d="M12 17h.01"/></svg>
        {{ error }}
      </span>
      <span v-else class="text-xs text-[var(--text-tertiary)]">编辑后切换回可视化视图自动应用</span>
    </div>
    <Textarea
      :model-value="codeText"
      @update:model-value="v => codeText = v"
      class="flex-1 p-3 text-xs font-mono leading-relaxed border-none outline-none resize-none tab-size-2"
      spellcheck="false"
      @focus="onFocus"
      @blur="onBlur"
    />
  </div>
</template>
