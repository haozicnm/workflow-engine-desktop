<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { safeInvoke } from '../utils/tauri'
import { useToast } from '../composables/useToast'
import Button from '../components/ui/button/Button.vue'
import Input from '../components/ui/input/Input.vue'
import Badge from '../components/ui/badge/Badge.vue'
import Separator from '../components/ui/separator/Separator.vue'

interface TemplateMeta {
  id: string
  name: string
  description: string
  step_count: number
  source: string
  category: string
  file_path?: string
}

interface WorkflowStep {
  id: string
  name: string
  step_type: string
  config?: Record<string, unknown>
  actions?: { type: string; label?: string; params?: Record<string, unknown> }[]
}

interface TemplateDetail {
  name: string
  description?: string
  steps: WorkflowStep[]
  variables?: Record<string, unknown>
}

const props = defineProps<{
  template: TemplateMeta
}>()

const emit = defineEmits<{
  back: []
  instantiated: [workflowId: string]
}>()

const toast = useToast()
const loading = ref(false)
const detail = ref<TemplateDetail | null>(null)
const error = ref<string | null>(null)

// Parameter detection from template YAML
const detectedParams = computed(() => {
  if (!detail.value) return []
  const params = new Set<string>()
  const json = JSON.stringify(detail.value)
  const matches = json.matchAll(/\{\{params\.(\w+)\}\}/g)
  for (const m of matches) {
    params.add(m[1])
  }
  return Array.from(params).sort()
})

const paramValues = ref<Record<string, string>>({})

function setParam(key: string, value: string) {
  paramValues.value = { ...paramValues.value, [key]: value }
}

onMounted(loadDetail)

async function loadDetail() {
  loading.value = true
  error.value = null
  try {
    const yaml = await safeInvoke<string>('load_template', { id: props.template.id })
    if (!yaml) {
      error.value = '模板内容为空'
      return
    }
    // Parse YAML-like JSON (workflows are stored as JSON)
    try {
      // Try JSON first (builtin templates)
      const json = JSON.parse(yaml)
      detail.value = {
        name: json.name || props.template.name,
        description: json.description,
        steps: json.steps || [],
        variables: json.variables,
      }
    } catch {
      // Try as parsed workflow object
      detail.value = {
        name: props.template.name,
        description: props.template.description,
        steps: [],
      }
    }
  } catch (e: any) {
    error.value = `加载模板失败: ${e}`
  } finally {
    loading.value = false
  }
}

async function instantiate() {
  loading.value = true
  try {
    // Load template content
    const yaml = await safeInvoke<string>('load_template', { id: props.template.id })
    if (!yaml) {
      toast.error('模板内容为空')
      return
    }

    // Substitute parameters
    let content = yaml
    for (const [key, val] of Object.entries(paramValues.value)) {
      if (val) {
        const placeholder = `{{params.${key}}}`
        content = content.split(placeholder).join(val)
      }
    }

    // Create workflow
    const wfId = await safeInvoke<string>('workflow_create', {
      name: `${props.template.name} (${new Date().toLocaleDateString('zh-CN')})`,
      description: props.template.description,
    })
    if (!wfId) {
      toast.error('创建工作流失败')
      return
    }

    // Save YAML
    await safeInvoke('workflow_save_yaml', { id: wfId, yaml: content })
    
    toast.success('模板已实例化')
    emit('instantiated', wfId)
  } catch (e: any) {
    toast.error(`实例化失败: ${e}`)
  } finally {
    loading.value = false
  }
}

const stepTypeLabel = (t: string) => {
  const labels: Record<string, string> = {
    excel_container: '📊 Excel', word_container: '📝 Word',
    browser_container: '🌐 浏览器', shell: '💻 Shell',
    http: '🌍 HTTP', condition: '🔀 条件',
    file_container: '📁 文件', loop: '🔁 循环',
    delay: '⏱ 延迟', notify: '🔔 通知',
    clipboard_container: '📋 剪贴板', approval: '✋ 审批',
    web_scrape: '🕷 网页抓取', cursor: '👆 游标',
  }
  return labels[t] || t
}
</script>

<template>
  <div class="template-preview flex-1 flex flex-col overflow-hidden min-h-0 bg-background">
    <!-- Header -->
    <div class="flex items-center gap-3 px-4 py-3 border-b border-border shrink-0">
      <button
        @click="emit('back')"
        class="inline-flex items-center justify-center w-8 h-8 rounded-md hover:bg-accent transition-colors"
        title="返回"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"/></svg>
      </button>
      <div class="flex-1 min-w-0">
        <h1 class="text-lg font-semibold truncate">{{ template.name }}</h1>
        <p class="text-xs text-muted-foreground truncate">{{ template.description }}</p>
      </div>
      <Badge :variant="template.source === 'builtin' ? 'default' : 'secondary'" size="sm">
        {{ template.source === 'builtin' ? '内置' : '本地' }}
      </Badge>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-4 space-y-4">
      <!-- Loading -->
      <div v-if="loading" class="flex items-center justify-center py-20">
        <div class="flex items-center gap-2 text-muted-foreground">
          <svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/></svg>
          加载中...
        </div>
      </div>

      <!-- Error -->
      <div v-else-if="error" class="p-4 rounded-lg bg-destructive/10 text-destructive text-sm">
        {{ error }}
      </div>

      <!-- Detail -->
      <template v-else-if="detail">
        <!-- Steps preview -->
        <div>
          <h3 class="text-sm font-medium mb-2">步骤预览 ({{ detail.steps.length }})</h3>
          <div class="space-y-2">
            <div
              v-for="step in detail.steps"
              :key="step.id"
              class="flex items-center gap-2 p-2 rounded-md bg-muted/50 text-sm"
            >
              <span class="text-xs">{{ stepTypeLabel(step.step_type) }}</span>
              <span class="font-medium">{{ step.name }}</span>
              <span v-if="step.actions?.length" class="text-xs text-muted-foreground ml-auto">
                {{ step.actions.length }} 个动作
              </span>
            </div>
          </div>
        </div>

        <!-- Parameters (if any) -->
        <template v-if="detectedParams.length > 0">
          <Separator />
          <div>
            <h3 class="text-sm font-medium mb-2">参数填写</h3>
            <p class="text-xs text-muted-foreground mb-3">
              以下参数将从模板中检测到，填写后点击「实例化」生成工作流。
            </p>
            <div class="space-y-3">
              <div v-for="param in detectedParams" :key="param" class="flex flex-col gap-1">
                <label :for="`param-${param}`" class="text-xs font-medium text-muted-foreground">
                  {{ param }}
                </label>
                <Input
                  :id="`param-${param}`"
                  :model-value="paramValues[param] || ''"
                  :placeholder="`输入 ${param}`"
                  @update:model-value="(v: string) => setParam(param, v)"
                />
              </div>
            </div>
          </div>
        </template>

        <template v-else>
          <Separator />
          <div class="text-center py-4 text-sm text-muted-foreground">
            此模板无需参数，可直接实例化。
          </div>
        </template>
      </template>
    </div>

    <!-- Footer -->
    <div class="flex items-center gap-2 px-4 py-3 border-t border-border shrink-0">
      <Button variant="outline" @click="emit('back')">返回</Button>
      <div class="flex-1" />
      <Button :disabled="loading" @click="instantiate">
        <template v-if="loading">
          <svg class="animate-spin h-4 w-4 mr-1" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/></svg>
        </template>
        实例化工作流
      </Button>
    </div>
  </div>
</template>
