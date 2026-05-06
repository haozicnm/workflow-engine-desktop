<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import type { Step, ContainerType } from '../types/workflow'
import { getContainerDef } from '../types/workflow'
import Button from './ui/button/Button.vue'
import Input from './ui/input/Input.vue'
import Label from './ui/label/Label.vue'
import Checkbox from './ui/checkbox/Checkbox.vue'
import Card from './ui/card/Card.vue'
import CardHeader from './ui/card/CardHeader.vue'
import CardTitle from './ui/card/CardTitle.vue'
import CardContent from './ui/card/CardContent.vue'
import Select from './ui/select/Select.vue'

const props = defineProps<{
  step: Step
}>()

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

function onParamChange(key: string, value: unknown) {
  localConfig.value[key] = value
  emit('update-config', { ...localConfig.value })
}

function onTextInput(key: string, e: Event) {
  onParamChange(key, (e.target as HTMLInputElement).value)
}

function onNumberInput(key: string, e: Event) {
  const v = (e.target as HTMLInputElement).value
  onParamChange(key, v === '' ? '' : Number(v))
}

function onSelectChange(key: string, e: Event) {
  onParamChange(key, (e.target as HTMLSelectElement).value)
}

function onCheckboxChange(key: string, val: boolean) {
  onParamChange(key, val)
}
</script>

<template>
  <Card class="max-w-[400px]">
    <CardHeader class="flex flex-row items-center gap-2 p-4 pb-3">
      <span class="text-lg">{{ containerDef.icon }}</span>
      <CardTitle class="flex-1 text-sm">
        {{ step.label }} - 容器参数
      </CardTitle>
      <Button variant="ghost" size="icon" class="h-6 w-6" @click="emit('close')">
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
      </Button>
    </CardHeader>

    <CardContent class="px-4 pb-4 pt-0">
      <!-- Param fields -->
      <div v-if="containerDef.params.length > 0" class="space-y-3">
        <div
          v-for="param in containerDef.params"
          :key="param.key"
        >
          <Label class="text-xs text-muted-foreground block mb-1.5">{{ param.label }}</Label>

          <!-- Text input -->
          <Input
            v-if="param.type === 'text'"
            type="text"
            :model-value="(localConfig[param.key] as string) ?? (param.default as string) ?? ''"
            :placeholder="param.placeholder"
            class="h-8 text-xs"
            @input="onTextInput(param.key, $event)"
          />

          <!-- Number input -->
          <Input
            v-else-if="param.type === 'number'"
            type="number"
            :model-value="(localConfig[param.key] as string) ?? (param.default as string) ?? ''"
            :placeholder="param.placeholder"
            class="h-8 text-xs"
            @input="onNumberInput(param.key, $event)"
          />

          <!-- Select -->
          <Select
            v-else-if="param.type === 'select'"
            :model-value="(localConfig[param.key] as string) ?? (param.default as string) ?? ''"
            :options="param.options"
            @update:model-value="v => onParamChange(param.key, v)"
          />

          <!-- Checkbox -->
          <div v-else-if="param.type === 'checkbox'" class="flex items-center gap-2">
            <Checkbox
              :model-value="!!(localConfig[param.key] ?? param.default)"
              @update:model-value="(v) => onCheckboxChange(param.key, v)"
            />
            <span class="text-xs text-foreground">
              {{ localConfig[param.key] ? '已启用' : '未启用' }}
            </span>
          </div>
        </div>
      </div>

      <!-- No params -->
      <div v-else class="text-center text-muted-foreground text-sm py-3">
        此容器没有可配置的参数
      </div>
    </CardContent>
  </Card>
</template>
