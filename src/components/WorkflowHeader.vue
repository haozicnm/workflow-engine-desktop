<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import Button from '../components/ui/button/Button.vue'
import Input from '../components/ui/input/Input.vue'
import Card from '../components/ui/card/Card.vue'
import type { Workflow } from '../types/types'
import { Lock, Unlock, Ellipsis, Save, FileDown, FileOutput, Clock, Trash } from 'lucide-vue-next'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '../components/ui/dropdown-menu'

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
</script>

<template>
  <Card color="var(--color-muted-foreground)" class="mx-[var(--spacing-section-padding-x)] mt-6 shrink-0">
    <!-- Loading skeleton -->
    <div v-if="loading" class="px-4 py-3 space-y-3 animate-pulse">
      <div class="h-5 bg-[var(--bg-overlay-l1)]/50 rounded w-1/3" />
      <div class="h-3 bg-[var(--bg-overlay-l1)]/30 rounded w-2/3" />
    </div>
    <div v-else class="px-4 py-3">
      <!-- Row 1: Title + Actions -->
      <div class="flex items-center gap-3">
        <!-- Name -->
        <div v-if="!editingName" class="flex-1 min-w-0">
          <span
            class="text-base font-semibold text-[var(--text-default)] cursor-text hover:text-[var(--text-brand)] transition-colors"
            :class="{ 'cursor-not-allowed opacity-60': isLocked }"
            :title="isLocked ? t('editor.lockedHint2') : t('editor.clickToEditName')"
            @click="isLocked ? undefined : emit('update:editingName', true)"
          >
            {{ workflow?.name || t('editor.untitled') }}
          </span>
          <span v-if="dirty" class="text-[var(--status-warning-default)] text-xs ml-1">●</span>
          <span v-if="lastSavedAt" class="text-xs text-[var(--text-tertiary)] ml-2">{{ lastSavedAt }}</span>
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
          :class="isLocked ? 'text-[var(--status-warning-default)]' : 'text-[var(--text-tertiary)]/30 hover:text-[var(--text-tertiary)]'"
          :title="isLocked ? t('editor.unlockToEdit') : t('editor.lockToPrevent')"
          @click="emit('toggle-lock')"
        ><Lock v-if="isLocked" class="w-3.5 h-3.5" /><Unlock v-else class="w-3.5 h-3.5" /></Button>

        <!-- Primary action -->
        <Button v-if="!isRunning" variant="default" size="sm" class="h-8 bg-[var(--status-success-default)] hover:bg-[var(--status-success-default)]/90 text-white shrink-0" @click="emit('run')">{{ t('editor.run') }}</Button>
        <Button v-else variant="destructive" size="sm" class="h-8 shrink-0" @click="emit('stop')">{{ t('editor.stop') }}</Button>

        <!-- ⋯ Menu → DropdownMenu -->
        <DropdownMenu v-model:open="showCardMenu">
          <DropdownMenuTrigger as-child>
            <Button variant="ghost" size="icon" class="h-8 w-8 opacity-50 hover:opacity-100"><Ellipsis class="w-4 h-4" /></Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent class="w-44" align="end">
            <DropdownMenuItem @click="emit('save'); showCardMenu = false"><Save class="w-4 h-4 mr-2" /> {{ t('common.save') }}</DropdownMenuItem>
            <DropdownMenuItem @click="emit('save-as'); showCardMenu = false"><FileOutput class="w-4 h-4 mr-2" /> {{ t('editor.saveAs') }}</DropdownMenuItem>
            <DropdownMenuItem @click="emit('export'); showCardMenu = false"><FileDown class="w-4 h-4 mr-2" /> {{ t('common.export') }}</DropdownMenuItem>
            <DropdownMenuItem @click="emit('schedule'); showCardMenu = false"><Clock class="w-4 h-4 mr-2" /> {{ t('editor.schedule') }}</DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem class="text-[var(--status-error-default)] focus:bg-[var(--status-error-default)]/10 focus:text-[var(--status-error-default)]" @click="emit('delete'); showCardMenu = false"><Trash class="w-4 h-4 mr-2" /> {{ t('common.delete') }}</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      <!-- Row 2: Description -->
      <div class="mt-1.5">
        <input
          v-if="workflow"
          :value="workflow.description"
          :placeholder="t('editor.descPlaceholder')"
          class="w-full text-xs text-[var(--text-tertiary)] bg-transparent border-0 outline-none placeholder:text-[var(--text-tertiary)]/40 hover:text-[var(--text-default)] transition-colors"
          @input="workflow.description = ($event.target as HTMLInputElement).value"
        />
      </div>
    </div>
  </Card>

</template>
