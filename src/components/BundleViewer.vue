<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  content: string
  filename: string
}>()

const isJson = computed(() => props.filename.endsWith('.json'))
const isImage = computed(() => /\.(png|jpg|jpeg|gif|webp|svg)$/i.test(props.filename))

const truncated = computed(() => {
  if (props.content.length <= 5000) return props.content
  return props.content.slice(0, 5000) + '\n... (内容已截断)'
})
</script>

<template>
  <div class="max-h-[400px] overflow-auto">
    <!-- Text content -->
    <pre
      v-if="!isImage"
      class="text-[10px] font-mono p-2 m-0 whitespace-pre-wrap break-all"
      :class="isJson ? 'text-foreground' : 'text-muted-foreground'"
    >{{ truncated }}</pre>

    <!-- Image placeholder (Tauri asset protocol) -->
    <div v-else class="flex items-center justify-center p-4 text-muted-foreground/50 text-xs">
      [截图快照]
    </div>
  </div>
</template>
