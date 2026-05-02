<template>
  <Teleport to="body">
    <transition name="panel-fade">
      <div v-if="visible" class="floating-panel" :style="panelStyle">
        <div class="panel-header" @mousedown="onDragStart">
          <span class="panel-title">{{ title }}</span>
          <button class="panel-close" @click="$emit('close')" title="关闭">✕</button>
        </div>
        <div class="panel-body">
          <slot />
        </div>
        <div class="panel-resize-handle" @mousedown="onResizeStart"></div>
      </div>
    </transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'

const props = withDefaults(defineProps<{
  visible: boolean
  title?: string
  width?: number
  height?: number
  anchorEl?: HTMLElement | null
}>(), {
  title: '',
  width: 260,
  height: 400,
})

const emit = defineEmits<{
  'close': []
}>()

const pos = ref({ x: 60, y: 80 })
const size = ref({ w: props.width, h: props.height })

onMounted(() => {
  if (props.anchorEl) {
    const rect = props.anchorEl.getBoundingClientRect()
    pos.value = { x: rect.right + 4, y: rect.top }
  }
})

const panelStyle = computed(() => ({
  left: `${pos.value.x}px`,
  top: `${pos.value.y}px`,
  width: `${size.value.w}px`,
  height: `${size.value.h}px`,
}))

// ─── 拖拽移动 ───
let dragOff = { x: 0, y: 0 }
let dragging = false

function onDragStart(e: MouseEvent) {
  dragging = true
  dragOff = { x: e.clientX - pos.value.x, y: e.clientY - pos.value.y }
  document.addEventListener('mousemove', onDrag)
  document.addEventListener('mouseup', onDragEnd)
}

function onDrag(e: MouseEvent) {
  if (!dragging) return
  pos.value = { x: e.clientX - dragOff.x, y: e.clientY - dragOff.y }
}

function onDragEnd() {
  dragging = false
  document.removeEventListener('mousemove', onDrag)
  document.removeEventListener('mouseup', onDragEnd)
}

// ─── 右下角缩放 ───
let resizing = false
let resizeStart = { x: 0, y: 0, w: 0, h: 0 }

function onResizeStart(e: MouseEvent) {
  e.preventDefault()
  e.stopPropagation()
  resizing = true
  resizeStart = { x: e.clientX, y: e.clientY, w: size.value.w, h: size.value.h }
  document.addEventListener('mousemove', onResize)
  document.addEventListener('mouseup', onResizeEnd)
}

function onResize(e: MouseEvent) {
  if (!resizing) return
  size.value = {
    w: Math.max(200, resizeStart.w + e.clientX - resizeStart.x),
    h: Math.max(150, resizeStart.h + e.clientY - resizeStart.y),
  }
}

function onResizeEnd() {
  resizing = false
  document.removeEventListener('mousemove', onResize)
  document.removeEventListener('mouseup', onResizeEnd)
}
</script>

<style scoped>
.floating-panel {
  position: fixed;
  z-index: 9998;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  pointer-events: auto;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  background: var(--color-elevated);
  border-bottom: 1px solid var(--color-border);
  cursor: move;
  user-select: none;
  flex-shrink: 0;
}

.panel-title {
  font-size: 11px;
  font-weight: 600;
  color: var(--color-text-dim);
}

.panel-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--color-text-dim);
  font-size: 12px;
  cursor: pointer;
}
.panel-close:hover {
  background: var(--color-danger-bg);
  color: #fff;
}

.panel-body {
  flex: 1;
  overflow: auto;
  min-height: 0;
}

.panel-resize-handle {
  position: absolute;
  bottom: 0;
  right: 0;
  width: 14px;
  height: 14px;
  cursor: nwse-resize;
  background: linear-gradient(135deg, transparent 50%, var(--color-border) 50%);
  border-radius: 0 0 8px 0;
}

/* 过渡动画 */
.panel-fade-enter-active,
.panel-fade-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.panel-fade-enter-from,
.panel-fade-leave-to {
  opacity: 0;
  transform: scale(0.95);
}
</style>
