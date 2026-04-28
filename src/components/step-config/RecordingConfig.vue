<script setup lang="ts">
import { computed } from 'vue'
import type { WorkflowStep } from '../../types/workflow'
import { getNodeFields } from '../../config/node-fields'
import ConfigFieldRenderer from './ConfigFieldRenderer.vue'

const props = defineProps<{ allSteps?: WorkflowStep[] }>()
const config = defineModel<Record<string, unknown>>('config', { required: true })

const fields = computed(() => {
  const allFields = getNodeFields(props.allSteps || [])
  return (allFields['recording'] || []).filter(f => !f.show || f.show(config.value))
})
</script>

<template>
  <ConfigFieldRenderer v-model:config="config" :fields="fields" />
</template>
