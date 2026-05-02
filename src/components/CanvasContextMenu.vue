<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="context-menu-backdrop"
      @click="emit('close')"
      @contextmenu.prevent
    >
      <div
        ref="menuRef"
        class="context-menu"
        :style="menuStyle"
        @click.stop
        @contextmenu.prevent.stop
      >
        <template v-for="(item, index) in items" :key="index">
          <div v-if="item.divider" class="context-menu-divider" />
          <div
            v-else
            :class="['context-menu-item', { disabled: item.disabled }]"
            @click="handleItemClick(item)"
          >
            {{ item.label }}
          </div>
        </template>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'

export interface ContextMenuItem {
  label: string
  action: () => void
  disabled?: boolean
  divider?: boolean
}

const props = defineProps<{
  items: ContextMenuItem[]
  x: number
  y: number
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
}>()

const menuRef = ref<HTMLElement | null>(null)
const adjustedX = ref(0)
const adjustedY = ref(0)

watch(
  () => [props.visible, props.x, props.y] as const,
  async ([visible, x, y]) => {
    if (!visible) return
    adjustedX.value = x
    adjustedY.value = y
    await nextTick()
    const el = menuRef.value
    if (!el) return
    const rect = el.getBoundingClientRect()

    // 水平翻转：右侧超出视口时向左弹
    if (x + rect.width > window.innerWidth) {
      adjustedX.value = x - rect.width
    }
    // 垂直翻转：底部超出视口时向上弹
    if (y + rect.height > window.innerHeight) {
      adjustedY.value = y - rect.height
    }
    // 边界钳制：确保不超出左上角
    if (adjustedX.value < 0) adjustedX.value = 0
    if (adjustedY.value < 0) adjustedY.value = 0
  },
)

const menuStyle = computed(() => ({
  left: `${adjustedX.value}px`,
  top: `${adjustedY.value}px`,
}))

function handleItemClick(item: ContextMenuItem) {
  if (item.disabled) return
  item.action()
  emit('close')
}
</script>

<style scoped>
.context-menu-backdrop {
  position: fixed;
  inset: 0;
  z-index: 9998;
  background: transparent;
}

.context-menu {
  position: fixed;
  z-index: 9999;
  min-width: 160px;
  padding: 4px 0;
  background: #161b22;
  border: 1px solid #30363d;
  border-radius: 6px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.5);
  font-size: 12px;
  color: #c9d1d9;
  user-select: none;
}

.context-menu-item {
  padding: 6px 12px;
  cursor: pointer;
  white-space: nowrap;
  transition: background 0.1s;
}

.context-menu-item:hover {
  background: #21262d;
}

.context-menu-item.disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.context-menu-item.disabled:hover {
  background: transparent;
}

.context-menu-divider {
  border-top: 1px solid #30363d;
  margin: 2px 0;
}
</style>
