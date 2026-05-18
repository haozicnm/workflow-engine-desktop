<script setup lang="ts">
/**
 * ActionIcon — 将 node-registry 中的图标名渲染为 Lucide 图标
 * 支持：Lucide 图标名（如 'Globe'）、单字符 fallback
 */
import {
  Globe, BarChart3, FileText, GitBranch, Clock, Bell, ScrollText,
  ClipboardList, Repeat, RefreshCw, Hand, Link, MousePointerClick,
  Keyboard, Camera, Zap, Tag, ChevronDown, CheckSquare, MousePointer,
  Cookie, Radio, Plus, X, BookOpen, ArrowLeft, ArrowRight, Activity,
  Download, Upload, MoveHorizontal, Frame, MessageSquare, Target,
  Pencil, Search, ArrowUpDown, Calculator, Paperclip, Ban, Equal,
  TrendingUp, TrendingDown, ArrowUpRight, ArrowDownRight, CircleOff,
  CheckCircle, Asterisk, Settings, Play, Pause, Square, Trash2,
  ChevronRight, ChevronUp, MoreHorizontal, GripVertical, Save,
  FolderOpen, File, List, Grid3x3, Table, Type, Hash, ToggleLeft,
  Workflow, Boxes, Eye, EyeOff, Lock, Unlock, Info, AlertTriangle,
  HelpCircle, ExternalLink, Copy, Clipboard, FolderPlus,
  FilePlus, FileUp, MoveRight, FolderTree, ScanSearch,
  Sun, Moon, Monitor, XCircle, Loader,
  Circle, Merge, CircleStop, CircleAlert, ArrowRightLeft,
  Terminal,
  Network,
  FileCheck,
} from 'lucide-vue-next'



withDefaults(defineProps<{
  name: string
  cls?: string
}>(), {
  cls: 'w-4 h-4'
})
</script>

<template>
  <Globe v-if="name === 'Globe'" :class="cls" />
  <BarChart3 v-else-if="name === 'BarChart3'" :class="cls" />
  <FileText v-else-if="name === 'FileText'" :class="cls" />
  <GitBranch v-else-if="name === 'GitBranch'" :class="cls" />
  <Clock v-else-if="name === 'Clock'" :class="cls" />
  <Bell v-else-if="name === 'Bell'" :class="cls" />
  <ScrollText v-else-if="name === 'ScrollText'" :class="cls" />
  <ClipboardList v-else-if="name === 'ClipboardList'" :class="cls" />
  <Repeat v-else-if="name === 'Repeat'" :class="cls" />
  <RefreshCw v-else-if="name === 'RefreshCw'" :class="cls" />
  <Hand v-else-if="name === 'Hand'" :class="cls" />
  <Link v-else-if="name === 'Link'" :class="cls" />
  <MousePointerClick v-else-if="name === 'MousePointerClick'" :class="cls" />
  <Keyboard v-else-if="name === 'Keyboard'" :class="cls" />
  <Camera v-else-if="name === 'Camera'" :class="cls" />
  <Zap v-else-if="name === 'Zap'" :class="cls" />
  <Tag v-else-if="name === 'Tag'" :class="cls" />
  <ChevronDown v-else-if="name === 'ChevronDown'" :class="cls" />
  <CheckSquare v-else-if="name === 'CheckSquare'" :class="cls" />
  <MousePointer v-else-if="name === 'MousePointer'" :class="cls" />
  <Cookie v-else-if="name === 'Cookie'" :class="cls" />
  <Radio v-else-if="name === 'Radio'" :class="cls" />
  <Plus v-else-if="name === 'Plus'" :class="cls" />
  <X v-else-if="name === 'X'" :class="cls" />
  <BookOpen v-else-if="name === 'BookOpen'" :class="cls" />
  <ArrowLeft v-else-if="name === 'ArrowLeft'" :class="cls" />
  <ArrowRight v-else-if="name === 'ArrowRight'" :class="cls" />
  <Activity v-else-if="name === 'Activity'" :class="cls" />
  <Download v-else-if="name === 'Download'" :class="cls" />
  <Upload v-else-if="name === 'Upload'" :class="cls" />
  <MoveHorizontal v-else-if="name === 'MoveHorizontal'" :class="cls" />
  <Frame v-else-if="name === 'Frame'" :class="cls" />
  <MessageSquare v-else-if="name === 'MessageSquare'" :class="cls" />
  <Target v-else-if="name === 'Target'" :class="cls" />
  <Terminal v-else-if="name === 'Terminal'" :class="cls" />
  <Pencil v-else-if="name === 'Pencil'" :class="cls" />
  <Search v-else-if="name === 'Search'" :class="cls" />
  <ArrowUpDown v-else-if="name === 'ArrowUpDown'" :class="cls" />
  <Calculator v-else-if="name === 'Calculator'" :class="cls" />
  <Paperclip v-else-if="name === 'Paperclip'" :class="cls" />
  <Ban v-else-if="name === 'Ban'" :class="cls" />
  <Equal v-else-if="name === 'Equal'" :class="cls" />
  <TrendingUp v-else-if="name === 'TrendingUp'" :class="cls" />
  <TrendingDown v-else-if="name === 'TrendingDown'" :class="cls" />
  <ArrowUpRight v-else-if="name === 'ArrowUpRight'" :class="cls" />
  <ArrowDownRight v-else-if="name === 'ArrowDownRight'" :class="cls" />
  <CircleOff v-else-if="name === 'CircleOff'" :class="cls" />
  <CheckCircle v-else-if="name === 'CheckCircle'" :class="cls" />
  <Asterisk v-else-if="name === 'Asterisk'" :class="cls" />
  <Settings v-else-if="name === 'Settings'" :class="cls" />
  <Play v-else-if="name === 'Play'" :class="cls" />
  <Pause v-else-if="name === 'Pause'" :class="cls" />
  <Square v-else-if="name === 'Square'" :class="cls" />
  <Trash2 v-else-if="name === 'Trash2'" :class="cls" />
  <ChevronRight v-else-if="name === 'ChevronRight'" :class="cls" />
  <ChevronUp v-else-if="name === 'ChevronUp'" :class="cls" />
  <MoreHorizontal v-else-if="name === 'MoreHorizontal'" :class="cls" />
  <GripVertical v-else-if="name === 'GripVertical'" :class="cls" />
  <Save v-else-if="name === 'Save'" :class="cls" />
  <FolderOpen v-else-if="name === 'FolderOpen'" :class="cls" />
  <File v-else-if="name === 'File'" :class="cls" />
  <List v-else-if="name === 'List'" :class="cls" />
  <Grid3x3 v-else-if="name === 'Grid3x3'" :class="cls" />
  <Table v-else-if="name === 'Table'" :class="cls" />
  <Type v-else-if="name === 'Type'" :class="cls" />
  <Hash v-else-if="name === 'Hash'" :class="cls" />
  <ToggleLeft v-else-if="name === 'ToggleLeft'" :class="cls" />
  <Workflow v-else-if="name === 'Workflow'" :class="cls" />
  <Boxes v-else-if="name === 'Boxes'" :class="cls" />
  <Eye v-else-if="name === 'Eye'" :class="cls" />
  <EyeOff v-else-if="name === 'EyeOff'" :class="cls" />
  <Lock v-else-if="name === 'Lock'" :class="cls" />
  <Unlock v-else-if="name === 'Unlock'" :class="cls" />
  <Info v-else-if="name === 'Info'" :class="cls" />
  <AlertTriangle v-else-if="name === 'AlertTriangle'" :class="cls" />
  <HelpCircle v-else-if="name === 'HelpCircle'" :class="cls" />
  <ExternalLink v-else-if="name === 'ExternalLink'" :class="cls" />
  <Copy v-else-if="name === 'Copy'" :class="cls" />
  <Clipboard v-else-if="name === 'Clipboard'" :class="cls" />
  <FolderPlus v-else-if="name === 'FolderPlus'" :class="cls" />
  <FilePlus v-else-if="name === 'FilePlus'" :class="cls" />
  <FileUp v-else-if="name === 'FileUp'" :class="cls" />
  <MoveRight v-else-if="name === 'MoveRight'" :class="cls" />
  <FolderTree v-else-if="name === 'FolderTree'" :class="cls" />
  <ScanSearch v-else-if="name === 'ScanSearch'" :class="cls" />
  <Sun v-else-if="name === 'Sun'" :class="cls" />
  <Moon v-else-if="name === 'Moon'" :class="cls" />
  <Monitor v-else-if="name === 'Monitor'" :class="cls" />
  <XCircle v-else-if="name === 'XCircle'" :class="cls" />
  <Loader v-else-if="name === 'Loader'" :class="cls" />
  <Circle v-else-if="name === 'Circle'" :class="cls" />
  <Merge v-else-if="name === 'Merge'" :class="cls" />
  <CircleStop v-else-if="name === 'CircleStop'" :class="cls" />
  <CircleAlert v-else-if="name === 'CircleAlert'" :class="cls" />
  <ArrowRightLeft v-else-if="name === 'ArrowRightLeft'" :class="cls" />
  <Network v-else-if="name === 'Network'" :class="cls" />
  <FileCheck v-else-if="name === 'FileCheck'" :class="cls" />
  <span v-else :class="cls || 'text-base'" class="inline-flex items-center justify-center">{{ name }}</span>
</template>
