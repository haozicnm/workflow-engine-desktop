<script setup lang="ts">
/**
 * ActionIcon — 将 node-registry 中的图标名渲染为 Lucide 图标
 * 支持：kebab-case（如 'git-branch'）、PascalCase（如 'GitBranch'）、单字符 fallback
 */
import { computed } from 'vue'
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
  Terminal, Network, FileCheck,
  CodeXml, Braces, ListFilter, Replace, GitMerge,
  Palette, RotateCcw,
  /* 新增：node-schema 中缺失的图标 */
  Binary, Database, Folder, FileSpreadsheet, Shield, ShieldCheck,
  Sigma, Video, Printer, MapPin, Map, ScanText, LayoutPanelTop,
  Move,
} from 'lucide-vue-next'

const iconMap: Record<string, any> = {
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
  Terminal, Network, FileCheck,
  CodeXml, Braces, ListFilter, Replace, GitMerge,
  Palette, RotateCcw,
  Binary, Database, Folder, FileSpreadsheet, Shield, ShieldCheck,
  Sigma, Video, Printer, MapPin, Map, ScanText, LayoutPanelTop,
  Move,
  /* 别名：schema 中的短名 → Lucide 组件 */
  Code: CodeXml,
  Filter: ListFilter,
  Pagination: List,
  Container: Boxes,
}

/** kebab-case → PascalCase */
function toPascal(s: string): string {
  if (!s.includes('-') && s[0] === s[0].toUpperCase()) return s // 已是 PascalCase
  return s.split('-').map(w => w.charAt(0).toUpperCase() + w.slice(1)).join('')
}

const props = withDefaults(defineProps<{
  name: string
  cls?: string
}>(), {
  cls: 'w-4 h-4'
})

const iconComp = computed(() => {
  const pascal = toPascal(props.name)
  return iconMap[pascal] ?? iconMap[props.name] ?? null
})
</script>

<template>
  <component v-if="iconComp" :is="iconComp" :class="cls" />
  <span v-else :class="cls || 'text-base'" class="inline-flex items-center justify-center">{{ name }}</span>
</template>
