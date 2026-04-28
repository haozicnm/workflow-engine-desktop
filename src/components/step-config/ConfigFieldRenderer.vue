<script setup lang="ts">
import type { Field } from '../../config/node-fields'

defineProps<{ fields: Field[] }>()
const config = defineModel<Record<string, unknown>>('config', { required: true })
</script>

<template>
  <div v-if="fields.length === 0" class="no-fields">
    此类型无特定配置项，切换到 JSON 模式手动编辑
  </div>
  <div v-for="field in fields" :key="field.key" class="form-field">
    <label>{{ field.label }}</label>

    <select
      v-if="field.type === 'select'"
      class="input"
      v-model="config[field.key]"
    >
      <option v-if="field.placeholder" value="" disabled>{{ field.placeholder }}</option>
      <option v-for="opt in field.options" :key="opt.value" :value="opt.value">
        {{ opt.label }}
      </option>
    </select>

    <textarea
      v-else-if="field.type === 'textarea'"
      class="input textarea"
      :value="typeof config[field.key] === 'string' ? config[field.key] : JSON.stringify(config[field.key], null, 2)"
      @input="(e: Event) => config[field.key] = (e.target as HTMLTextAreaElement).value"
      :placeholder="field.placeholder"
      rows="4"
    ></textarea>

    <input
      v-else-if="field.type === 'number'"
      class="input"
      type="number"
      v-model="config[field.key]"
      :placeholder="field.placeholder"
    />

    <label v-else-if="field.type === 'checkbox'" class="checkbox-label">
      <input type="checkbox" v-model="config[field.key]" />
      <span>{{ field.label }}</span>
    </label>

    <div v-else-if="field.type === 'json'" class="json-field">
      <textarea
        class="input textarea small"
        :value="typeof config[field.key] === 'string' ? config[field.key] : JSON.stringify(config[field.key], null, 2)"
        :placeholder="field.placeholder"
        @blur="(e: Event) => { try { config[field.key] = JSON.parse((e.target as HTMLTextAreaElement).value) } catch { config[field.key] = (e.target as HTMLTextAreaElement).value } }"
      ></textarea>
    </div>

    <input
      v-else
      class="input"
      v-model="config[field.key]"
      :placeholder="field.placeholder"
    />
  </div>
</template>

<style scoped>
.form-field { display: flex; flex-direction: column; gap: 4px; }
.form-field label { font-size: 11px; color: #8b949e; }
.input {
  width: 100%; background: #0d1117; border: 1px solid #30363d; color: #e1e4e8;
  padding: 6px 10px; border-radius: 6px; font-size: 13px; box-sizing: border-box;
}
.input:focus { outline: none; border-color: #58a6ff; }
select.input { cursor: pointer; }
.textarea {
  font-family: 'Cascadia Code', 'Fira Code', monospace;
  font-size: 12px; line-height: 1.5; resize: vertical;
}
.textarea.small { min-height: 60px; }
.checkbox-label {
  display: flex; align-items: center; gap: 8px;
  font-size: 13px; color: #c9d1d9; cursor: pointer;
}
.checkbox-label input { width: 16px; height: 16px; accent-color: #58a6ff; }
.json-field { display: flex; flex-direction: column; }
.no-fields { color: #6e7681; font-size: 12px; padding: 12px 0; text-align: center; }
</style>
