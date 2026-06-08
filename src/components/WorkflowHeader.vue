<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import Button from '../components/ui/button/Button.vue'
import Input from '../components/ui/input/Input.vue'
import Card from '../components/ui/card/Card.vue'
import type { Workflow } from '../types/types'
import { Lock, Unlock } from 'lucide-vue-next'

const { t } = useI18n()

defineProps<{
  workflow: Workflow | null
  loading: boolean
  dirty: boolean
  isLocked: boolean
  isRunning: boolean
  editingName: boolean
  nameInput: string
  lastSavedAt?: string
}>()

const emit = defineEmits<{
  'update:editingName': [val: boolean]
  'update:nameInput': [val: string]
  'finish-edit-name': []
  'save': []
  'save-as': []
  'export': []
  'schedule': []
  'delete': []
  'toggle-lock': []
  'run': []
  'stop': []
}>()

const showCardMenu = ref(false)
const cardMenuBtnRef = ref<HTMLElement | null>(null)
const cardMenuPosStyle = ref<Record<string, string>>({})

function toggleCardMenu() {
  showCardMenu.value = !showCardMenu.value
  if (showCardMenu.value && cardMenuBtnRef.value && typeof cardMenuBtnRef.value.getBoundingClientRect === 'function') {
    const rect = cardMenuBtnRef.value.getBoundingClientRect()
    cardMenuPosStyle.value = {
      top: `${rect.bottom + 4}px`,
      left: `${rect.right - 176}px`,
    }
  }
}
</script>

<template>
  <Card color="#6e7681" class="mx-[var(--spacing-section-padding-x)] mt-6 shrink-0">
    <!-- Loading skeleton -->
    <div v-if="loading" class="px-4 py-3 space-y-3 animate-pulse">
      <div class="h-5 bg-secondary/50 rounded w-1/3" />
      <div class="h-3 bg-secondary/30 rounded w-2/3" />
    </div>
    <div v-else class="px-4 py-3">
      <!-- Row 1: Title + Actions -->
      <div class="flex items-center gap-3">
        <!-- Name -->
        <div v-if="!editingName" class="flex-1 min-w-0">
          <span
            class="text-base font-semibold text-foreground cursor-text hover:text-primary transition-colors"
            :class="{ 'cursor-not-allowed opacity-60': isLocked }"
            :title="isLocked ? t('editor.lockedHint2') : t('editor.clickToEditName')"
            @click="isLocked ? undefined : emit('update:editingName', true)"
          >
            {{ workflow?.name || t('editor.untitled') }}
          </span>
          <span v-if="dirty" class="text-warning text-xs ml-1">●</span>
          <span v-if="lastSavedAt" class="text-xs text-muted-foreground ml-2">{{ lastSavedAt }}</span>
        </div>
        <div v-else class="flex-1 min-w-0">
          <Input
            :value="nameInput"
            class="h-7 max-w-[300px] text-sm font-semibold"
            @input="emit('update:nameInput', ($event.target as HTMLInputElement).value)"
            @blur="emit('finish-edit-name')"
            @keydown.enter="emit('finish-edit-name')"
            @keydown.escape="emit('update:editingName', false)"
          />
        </div>

        <!-- Lock toggle -->
        <Button
          variant="ghost"
          size="icon"
          class="h-7 w-7 shrink-0"
          :class="isLocked ? 'text-warning' : 'text-muted-foreground/30 hover:text-muted-foreground'"
          :title="isLocked ? t('editor.unlockToEdit') : t('editor.lockToPrevent')"
          @click="emit('toggle-lock')"
        ><Lock v-if="isLocked" class="w-3.5 h-3.5" /><Unlock v-else class="w-3.5 h-3.5" /></Button>

        <!-- Primary action -->
        <Button v-if="!isRunning" variant="default" size="sm" class="h-8 bg-success hover:bg-success/90 text-success-foreground shrink-0" @click="emit('run')">{{ t('editor.run') }}</Button>
        <Button v-else variant="destructive" size="sm" class="h-8 shrink-0" @click="emit('stop')">{{ t('editor.stop') }}</Button>

        <!-- ⋯ Menu -->
        <div class="relative shrink-0" @click.stop>
          <Button ref="cardMenuBtnRef" variant="ghost" size="icon" class="h-8 w-8 opacity-50 hover:opacity-100" @click="toggleCardMenu">⋯</Button>
        </div>
      </div>

      <!-- Row 2: Description -->
      <div class="mt-1.5">
        <input
          v-if="workflow"
          :value="workflow.description"
          :placeholder="t('editor.descPlaceholder')"
          class="w-full text-xs text-muted-foreground bg-transparent border-0 outline-none placeholder:text-muted-foreground/40 hover:text-foreground transition-colors"
          @input="workflow.description = ($event.target as HTMLInputElement).value"
        />
      </div>
    </div>
  </Card>

  <!-- Card ⋯ Dropdown (teleported) -->
  <Teleport to="body">
    <div v-if="showCardMenu" class="fixed inset-0 z-40" @click="showCardMenu = false" @keydown.escape="showCardMenu = false" />
    <div v-if="showCardMenu" class="fixed z-50 w-44 bg-background border border-border rounded-md shadow-lg py-1" :style="cardMenuPosStyle">
      <Button variant="ghost" class="w-full justify-start px-3 py-2 text-sm" @click="emit('save'); showCardMenu = false">{{ t('common.save') }}</Button>
      <Button variant="ghost" class="w-full justify-start px-3 py-2 text-sm" @click="emit('save-as'); showCardMenu = false">{{ t('editor.saveAs') }}</Button>
      <Button variant="ghost" class="w-full justify-start px-3 py-2 text-sm" @click="emit('export'); showCardMenu = false">{{ t('common.export') }}</Button>
      <Button variant="ghost" class="w-full justify-start px-3 py-2 text-sm" @click="emit('schedule'); showCardMenu = false">{{ t('editor.schedule') }}</Button>
      <div class="border-t border-border my-1" />
      <Button variant="ghost" class="w-full justify-start px-3 py-2 text-sm text-destructive hover:bg-destructive/10 hover:text-destructive" @click="emit('delete'); showCardMenu = false">{{ t('common.delete') }}</Button>
    </div>
  </Teleport>
</template>
