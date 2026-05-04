// ─── LiteGraph Custom Node Definitions (v4.1: 动作框左进右出) ───
// 容器 = 环境参数壳子，无自身端口
// 每个 action = 动作框，左入右出，连线直接走动作框之间

import {
  LGraphNode,
  LiteGraph,
} from '@comfyorg/litegraph'

// ═══════════════════════════════════════════
// Color constants (dark theme)
// ═══════════════════════════════════════════
const COLOR_DATA = '#3fb950'
const COLOR_CONTROL = '#d29922'
const COLOR_OUTPUT = '#f78166'
const COLOR_EXCEL = '#58a6ff'
const COLOR_WORD = '#bc8cff'
const COLOR_BROWSER = '#79c0ff'
const COLOR_DEFAULT = '#8b949e'

const BG_DARK = '#1c2129'
const BOX_DARK = '#2a3040'

// ═══════════════════════════════════════════
// Base class shared by all workflow nodes
// ═══════════════════════════════════════════
class WorkflowNode extends LGraphNode {
  constructor(title: string, type?: string) {
    super(title, type)
    this.bgcolor = BG_DARK
    this.boxcolor = BOX_DARK
  }

  onConnectOutput(slot: number, inputType: unknown, _input: unknown, _targetNode: unknown, _targetSlot: number): boolean {
    const outputType = this.outputs?.[slot]?.type || (this.outputs?.[slot] as any)?.name || 'any'
    const compatible = isTypeCompatible(String(outputType), String(inputType || 'any'))
    if (!compatible) {
      const origColor = this.color
      const origBox = this.boxcolor
      this.color = '#f85149'
      this.boxcolor = '#da3633'
      setTimeout(() => {
        this.color = origColor
        this.boxcolor = origBox
      }, 200)
    }
    return compatible
  }

  onConnectInput(targetSlot: number, outputType: unknown, _output: unknown, _sourceNode: unknown, _sourceSlot: number): boolean {
    const inputType = this.inputs?.[targetSlot]?.type || (this.inputs?.[targetSlot] as any)?.name || 'any'
    const compatible = isTypeCompatible(String(outputType || 'any'), String(inputType))
    if (!compatible) {
      const origColor = this.color
      const origBox = this.boxcolor
      this.color = '#f85149'
      this.boxcolor = '#da3633'
      setTimeout(() => {
        this.color = origColor
        this.boxcolor = origBox
      }, 200)
    }
    return compatible
  }
}

function isTypeCompatible(sourceType: string, targetType: string): boolean {
  if (sourceType === 'any' || targetType === 'any') return true
  if (sourceType === targetType) return true
  if (sourceType === 'string' && targetType === 'object') return true
  if (sourceType === 'number' && targetType === 'string') return true
  return false
}

// ═══════════════════════════════════════════
// 容器节点基类 — 容器本身无端口，actions 各有 {id}_in {id}_out
// ═══════════════════════════════════════════
class ContainerNode extends WorkflowNode {
  constructor(title: string, type: string) {
    super(title, type)
    this.properties = this.properties || {}
    if (!(this.properties as any).actions) {
      (this.properties as any).actions = []
    }
  }

  get actions(): ContainerAction[] {
    return (this.properties as any).actions || []
  }

  addAction(type: string, label: string, config: Record<string, unknown> = {}) {
    const actions = (this.properties as any).actions
    const action: ContainerAction = {
      id: `a${Date.now()}`,
      type,
      label,
      config: { ...config },
    }
    actions.push(action)
    this.rebuildPorts()
    this.setDirtyCanvas(true, true)
  }

  removeAction(actionId: string) {
    const actions = (this.properties as any).actions
    const idx = actions.findIndex((a: ContainerAction) => a.id === actionId)
    if (idx >= 0) {
      actions.splice(idx, 1)
      this.rebuildPorts()
      this.setDirtyCanvas(true, true)
    }
  }

  // v4.1: 每个 action 生成一对端口：左入 {id}_in ，右出 {id}_out
  rebuildPorts() {
    while (this.inputs.length > 0) this.removeInput(0)
    while (this.outputs.length > 0) this.removeOutput(0)

    const actions: ContainerAction[] = (this.properties as any).actions || []
    for (const action of actions) {
      this.addInput(`${action.id}_in`, 'any')
      this.addOutput(`${action.id}_out`, 'any')
    }
  }

  protected onAdded(graph: LGraph): void {
    super.onAdded?.(graph)
    this.rebuildPorts()
  }
}

// Action 类型定义
interface ContainerAction {
  id: string
  type: string
  label: string
  config: Record<string, unknown>
}

// ═══════════════════════════════════════════
// 动作选项列表
// ═══════════════════════════════════════════

export const BROWSER_ACTIONS = [
  { type: 'navigate', label: '导航', icon: '🧭', desc: '打开网页' },
  { type: 'wait', label: '等待', icon: '⏳', desc: '等待元素出现' },
  { type: 'input', label: '输入', icon: '⌨️', desc: '填写输入框' },
  { type: 'click', label: '点击', icon: '👆', desc: '点击元素' },
  { type: 'extract', label: '提取文本', icon: '⛏️', desc: '提取页面文本' },
  { type: 'screenshot', label: '截图', icon: '📸', desc: '页面截图' },
  { type: 'evaluate', label: '执行JS', icon: '⚡', desc: '执行 JavaScript' },
  { type: 'scroll', label: '滚动', icon: '📜', desc: '滚动页面' },
  { type: 'get_title', label: '获取标题', icon: '📝', desc: '获取页面标题' },
  { type: 'pdf', label: '导出PDF', icon: '📄', desc: '导出为 PDF' },
]

export const WORD_ACTIONS = [
  { type: 'read', label: '读取', icon: '📖', desc: '读取文档内容' },
  { type: 'write', label: '写入', icon: '✏️', desc: '写入内容到文档' },
  { type: 'replace', label: '替换', icon: '🔄', desc: '查找替换文本' },
  { type: 'create', label: '新建', icon: '📄', desc: '创建新文档' },
  { type: 'merge', label: '合并', icon: '🔗', desc: '合并多个文档' },
  { type: 'insert_table', label: '插入表格', icon: '📊', desc: '插入表格' },
]

export const EXCEL_ACTIONS = [
  { type: 'read', label: '读取', icon: '📖', desc: '读取单元格数据' },
  { type: 'write', label: '写入', icon: '✏️', desc: '写入数据到单元格' },
  { type: 'filter', label: '筛选', icon: '🔍', desc: '按条件筛选行' },
  { type: 'sort', label: '排序', icon: '📊', desc: '按列排序' },
  { type: 'create', label: '新建', icon: '📄', desc: '创建新文件' },
  { type: 'append', label: '追加', icon: '➕', desc: '追加行数据' },
  { type: 'formula', label: '公式', icon: '🧮', desc: '设置公式' },
  { type: 'csv', label: 'CSV互转', icon: '📄', desc: 'CSV ↔ Excel 格式互转' },
]

export const LOGIC_ACTIONS = [
  { type: 'equals', label: '等于', icon: '=', desc: '值等于指定内容' },
  { type: 'not_equals', label: '不等于', icon: '≠', desc: '值不等于指定内容' },
  { type: 'gt', label: '大于', icon: '>', desc: '数值大于指定值' },
  { type: 'gte', label: '大于等于', icon: '≥', desc: '数值大于等于指定值' },
  { type: 'lt', label: '小于', icon: '<', desc: '数值小于指定值' },
  { type: 'lte', label: '小于等于', icon: '≤', desc: '数值小于等于指定值' },
  { type: 'contains', label: '包含', icon: '⊃', desc: '字符串包含指定内容' },
  { type: 'not_contains', label: '不包含', icon: '⊅', desc: '字符串不包含指定内容' },
  { type: 'is_empty', label: '为空', icon: '∅', desc: '值为空' },
  { type: 'not_empty', label: '不为空', icon: '⦰', desc: '值不为空' },
  { type: 'regex', label: '正则匹配', icon: '.*', desc: '正则表达式匹配' },
]

// ═══════════════════════════════════════════
// 容器子类（纯环境参数，无端口）
// ═══════════════════════════════════════════

class BrowserContainerNode extends ContainerNode {
  static title = '🌐 浏览器'
  static type = 'browser_container'

  constructor(title?: string) {
    super(title || '浏览器', 'browser_container')
    this.color = COLOR_BROWSER
    this.addWidget('combo', 'browser', 'chromium', null, {
      property: 'browser',
      values: ['chromium', 'firefox', 'webkit'],
    })
    this.addWidget('toggle', 'headless', false, 'headless')
    this.addWidget('number', 'timeout', 30000, 'timeout', { min: 1000, max: 300000, step: 1000 })
  }

  onExecute(): void {}
}

class ExcelContainerNode extends ContainerNode {
  static title = '📊 Excel'
  static type = 'excel_container'

  constructor(title?: string) {
    super(title || 'Excel', 'excel_container')
    this.color = COLOR_EXCEL
    this.addWidget('string', 'file_path', '', 'file_path')
    this.addWidget('string', 'sheet', 'Sheet1', 'sheet')
  }

  onExecute(): void {}
}

class WordContainerNode extends ContainerNode {
  static title = '📄 Word'
  static type = 'word_container'

  constructor(title?: string) {
    super(title || 'Word', 'word_container')
    this.color = COLOR_WORD
    this.addWidget('string', 'file_path', '', 'file_path')
  }

  onExecute(): void {}
}

// ═══════════════════════════════════════════
// 🔀 逻辑判断容器 — 固定端口
// ═══════════════════════════════════════════

class LogicContainerNode extends ContainerNode {
  static title = '🔀 逻辑判断'
  static type = 'logic_container'

  constructor(title?: string) {
    super(title || '逻辑判断', 'logic_container')
    this.color = COLOR_CONTROL
  }

  rebuildPorts(): void {
    while (this.inputs.length > 0) this.removeInput(0)
    while (this.outputs.length > 0) this.removeOutput(0)
    this.addInput('输入', 'any')
    this.addOutput('true出口', 'any')
    this.addOutput('false出口', 'any')
  }

  onExecute(): void {}
}

// ═══════════════════════════════════════════
// Registry
// ═══════════════════════════════════════════

export function registerAllNodes(): void {
  LiteGraph.registerNodeType('browser_container', BrowserContainerNode)
  LiteGraph.registerNodeType('excel_container', ExcelContainerNode)
  LiteGraph.registerNodeType('word_container', WordContainerNode)
  LiteGraph.registerNodeType('logic_container', LogicContainerNode)
}

export {
  WorkflowNode,
  ContainerNode,
  BrowserContainerNode,
  ExcelContainerNode,
  WordContainerNode,
  LogicContainerNode,
}
export type { ContainerAction }
