<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Step } from '../types/types'
import { getContainerDef } from '../types/node-registry'
import { useVariableRefs } from '../composables/useVariableRefs'
import ActionIcon from './ActionIcon.vue'
import { X } from 'lucide-vue-next'
import ParamField from './ParamField.vue'
import Button from './ui/button/Button.vue'
import Card from './ui/card/Card.vue'
import CardHeader from './ui/card/CardHeader.vue'
import CardTitle from './ui/card/CardTitle.vue'
import CardContent from './ui/card/CardContent.vue'



const props = withDefaults(defineProps<{
  step: Step
  steps?: Step[]
}>(), {
  steps: () => [],
})

const { t } = useI18n()

const emit = defineEmits<{
  'update-config': [config: { [key: string]: unknown }]
  close: []
}>()

const containerDef = computed(() => getContainerDef(props.step.type))
const localConfig = ref<Record<string, unknown>>({ ...props.step.config })

watch(
  () => props.step.id,
  () => {
    localConfig.value = { ...props.step.config }
  },
)

// ─── Variable refs ───
const { groupedRefs } = useVariableRefs(
  () => props.steps || [],
  () => props.step.id,
)

function onParamChange(key: string, value: unknown) {
  localConfig.value[key] = value
  emit('update-config', { ...localConfig.value })
}
</script>

<template>
  <Card class="max-w-[400px]">
    <CardHeader class="flex flex-row items-center gap-2 p-4 pb-3">
      <ActionIcon :name="containerDef.icon" cls="w-5 h-5" />
      <CardTitle class="flex-1 text-sm">
        {{ step.label }} - {{ t('containerConfig.title') }}
      </CardTitle>
      <Button variant="ghost" size="icon" class="h-6 w-6" :aria-label="t('common.close')" @click="emit('close')">
        <X class="w-3.5 h-3.5" />
      </Button>
    </CardHeader>

    <CardContent class="px-4 pb-4 pt-0">
      <!-- Param fields (with variable refs!) -->
      <div v-if="containerDef.params.length > 0" class="space-y-3">
        <ParamField
          v-for="param in containerDef.params"
          :key="param.key"
          :param="param"
          :model-value="localConfig[param.key] ?? param.default"
          :grouped-refs="groupedRefs"
          :sibling-values="localConfig"
          @update:model-value="v => onParamChange(param.key, v)"
        />
      </div>

      <!-- No params -->
      <div v-else class="text-center text-muted-foreground text-sm py-3">
        {{ t('containerConfig.noParams') }}
      </div>
    </CardContent>
  </Card>
</template>
