// ─── LiteGraph Custom Node Definitions (v3: No action dispatch) ───
// All nodes have dedicated types — one operation per node.
// 70+ workflow node types total.

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

  /** v3: 连接类型校验 — 类型不匹配时拒绝连线并有视觉反馈 */
  onConnectOutput(slot: number, inputType: unknown, _input: unknown, _targetNode: unknown, _targetSlot: number): boolean {
    const outputType = this.outputs?.[slot]?.type || (this.outputs?.[slot] as any)?.name || 'any'
    const compatible = isTypeCompatible(String(outputType), String(inputType || 'any'))
    if (!compatible) {
      // 短暂闪红以示拒绝
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

/** 判断两个针脚类型是否兼容 */
function isTypeCompatible(sourceType: string, targetType: string): boolean {
  if (sourceType === 'any' || targetType === 'any') return true
  if (sourceType === targetType) return true
  // action 触发器兼容所有类型
  if (sourceType === 'action' || targetType === 'action') return true
  // 字符串可连到 object (JSON 解析)
  if (sourceType === 'string' && targetType === 'object') return true
  // number 可连到 string
  if (sourceType === 'number' && targetType === 'string') return true
  return false
}

// ═══════════════════════════════════════════
// P0 核心节点
// ═══════════════════════════════════════════

class HttpNode extends WorkflowNode {
  static title = 'HTTP 请求'
  static type = 'http'
  constructor(title?: string) {
    super(title || 'HTTP 请求')
    this.color = COLOR_DATA
    this.addOutput('data', 'object')
    this.addWidget('combo', 'method', 'GET', null, { property: "method",
      values: ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS'],
    })
    this.addWidget('string', 'url', 'https://httpbin.org/get', 'url')
  }
  onExecute(): void {}
}

class ScriptNode extends WorkflowNode {
  static title = '脚本'
  static type = 'script'
  constructor(title?: string) {
    super(title || '脚本')
    this.color = COLOR_DEFAULT
    this.addInput('trigger', 'action')
    this.addOutput('trigger', 'action')
    this.addWidget('combo', 'language', 'javascript', null, { property: "language",
      values: ['javascript', 'python', 'bash', 'powershell', 'lua'],
    })
    this.addWidget('text', 'script', '// 在此编写脚本\n1 + 1', 'script', { multiline: true })
  }
  onExecute(): void {}
}

class ConditionNode extends WorkflowNode {
  static title = '条件判断'
  static type = 'condition'
  constructor(title?: string) {
    super(title || '条件判断')
    this.color = COLOR_CONTROL
    this.addInput('trigger', 'action')
    this.addOutput('true', 'action')
    this.addOutput('false', 'action')
    this.addWidget('combo', 'op', '==', null, { property: "op", values: [
      '==', '!=', '>', '<', '>=', '<=',
      'contains', 'starts_with', 'ends_with',
      'regex', 'is_empty', 'is_truthy',
    ]})
    this.addWidget('string', 'left', '', 'left')
    this.addWidget('string', 'right', '', 'right')
  }
  onExecute(): void {}
}

// ═══════════════════════════════════════════
// 数据处理节点 (v3: 独立类型)
// ═══════════════════════════════════════════

class DataSetNode extends WorkflowNode {
  static title = '设置变量'
  static type = 'data_set'
  constructor(title?: string) {
    super(title || '设置变量')
    this.color = COLOR_DATA
    this.addInput('value', 'any')
    this.addOutput('data', 'any')
    this.addWidget('string', 'key', '', 'key')
    this.addWidget('string', 'value', '', 'value')
  }
  onExecute(): void {}
}

class DataGetNode extends WorkflowNode {
  static title = '读取变量'
  static type = 'data_get'
  constructor(title?: string) {
    super(title || '读取变量')
    this.color = COLOR_DATA
    this.addOutput('data', 'any')
    this.addWidget('string', 'key', '', 'key')
  }
  onExecute(): void {}
}

class DataLengthNode extends WorkflowNode {
  static title = '获取长度'
  static type = 'data_length'
  constructor(title?: string) {
    super(title || '获取长度')
    this.color = COLOR_DATA
    this.addOutput('length', 'number')
    this.addWidget('string', 'source', '', 'source')
    this.addWidget('string', 'key', '', 'key')
  }
  onExecute(): void {}
}

class DataDefaultNode extends WorkflowNode {
  static title = '默认值'
  static type = 'data_default'
  constructor(title?: string) {
    super(title || '默认值')
    this.color = COLOR_DATA
    this.addOutput('data', 'any')
    this.addWidget('string', 'key', '', 'key')
    this.addWidget('string', 'value', '', 'value')
  }
  onExecute(): void {}
}

class DataMergeNode extends WorkflowNode {
  static title = '合并变量'
  static type = 'data_merge'
  constructor(title?: string) {
    super(title || '合并变量')
    this.color = COLOR_DATA
    this.addOutput('data', 'object')
    this.addWidget('string', 'target', '', 'target')
    this.addWidget('string', 'source', '', 'source')
  }
  onExecute(): void {}
}

class JsonParseNode extends WorkflowNode {
  static title = 'JSON 解析'
  static type = 'json_parse'
  constructor(title?: string) {
    super(title || 'JSON 解析')
    this.color = COLOR_DATA
    this.addInput('data', 'string')
    this.addOutput('result', 'any')
    this.addWidget('string', 'expression', '$', 'expression')
  }
  onExecute(): void {}
}

class TextTemplateNode extends WorkflowNode {
  static title = '文本拼接'
  static type = 'text_template'
  constructor(title?: string) {
    super(title || '文本拼接')
    this.color = COLOR_DATA
    this.addInput('data', 'string')
    this.addOutput('result', 'string')
    this.addWidget('string', 'template', '', 'template')
    this.addWidget('string', 'output_key', '', 'output_key')
  }
  onExecute(): void {}
}

// ═══════════════════════════════════════════
// 文件节点 (v3: 独立类型)
// ═══════════════════════════════════════════

class FileReadNode extends WorkflowNode {
  static title = '读取文件'
  static type = 'file_read'
  constructor(title?: string) {
    super(title || '读取文件')
    this.color = COLOR_DATA
    this.addOutput('data', 'string')
    this.addWidget('string', 'path', '', 'path')
    this.addWidget('combo', 'encoding', 'text', null, { property: "encoding",
      values: ['text', 'base64'],
    })
  }
  onExecute(): void {}
}

class FileWriteNode extends WorkflowNode {
  static title = '写入文件'
  static type = 'file_write'
  constructor(title?: string) {
    super(title || '写入文件')
    this.color = COLOR_DATA
    this.addInput('content', 'any')
    this.addOutput('data', 'object')
    this.addWidget('string', 'path', '', 'path')
    this.addWidget('combo', 'encoding', 'text', null, { property: "encoding",
      values: ['text', 'base64'],
    })
  }
  onExecute(): void {}
}

class FileListNode extends WorkflowNode {
  static title = '列出文件'
  static type = 'file_list'
  constructor(title?: string) {
    super(title || '列出文件')
    this.color = COLOR_DATA
    this.addOutput('files', 'array')
    this.addWidget('string', 'path', '', 'path')
    this.addWidget('toggle', 'recursive', false, 'recursive')
  }
  onExecute(): void {}
}

class FileDeleteNode extends WorkflowNode {
  static title = '删除文件'
  static type = 'file_delete'
  constructor(title?: string) {
    super(title || '删除文件')
    this.color = COLOR_DATA
    this.addOutput('result', 'object')
    this.addWidget('string', 'path', '', 'path')
    this.addWidget('toggle', 'recursive', false, 'recursive')
  }
  onExecute(): void {}
}

class FileExistsNode extends WorkflowNode {
  static title = '存在检查'
  static type = 'file_exists'
  constructor(title?: string) {
    super(title || '存在检查')
    this.color = COLOR_DATA
    this.addOutput('exists', 'boolean')
    this.addWidget('string', 'path', '', 'path')
  }
  onExecute(): void {}
}

class FileSaveNode extends WorkflowNode {
  static title = '保存文件'
  static type = 'file_save'
  constructor(title?: string) {
    super(title || '保存文件')
    this.color = COLOR_OUTPUT
    this.addInput('data', 'any')
    this.addOutput('trigger', 'action')
    this.addWidget('string', 'path', './output.txt', 'path')
    this.addWidget('combo', 'format', 'auto', null, { property: "format",
      values: ['auto', 'json', 'yaml', 'csv', 'txt', 'binary'],
    })
    this.addWidget('combo', 'encoding', 'utf-8', null, { property: "encoding",
      values: ['utf-8', 'base64'],
    })
  }
  onExecute(): void {}
}

// ═══════════════════════════════════════════
// 剪贴板节点 (v3: 独立类型)
// ═══════════════════════════════════════════

class ClipboardReadNode extends WorkflowNode {
  static title = '读取剪贴板'
  static type = 'clipboard_read'
  constructor(title?: string) {
    super(title || '读取剪贴板')
    this.color = COLOR_DATA
    this.addOutput('text', 'string')
  }
  onExecute(): void {}
}

class ClipboardWriteNode extends WorkflowNode {
  static title = '写入剪贴板'
  static type = 'clipboard_write'
  constructor(title?: string) {
    super(title || '写入剪贴板')
    this.color = COLOR_DATA
    this.addInput('text', 'string')
    this.addOutput('trigger', 'action')
    this.addWidget('string', 'text', '', 'text')
  }
  onExecute(): void {}
}

// ═══════════════════════════════════════════
// 正则节点 (v3: 独立类型)
// ═══════════════════════════════════════════

class RegexExtractNode extends WorkflowNode {
  static title = '正则提取'
  static type = 'regex_extract'
  constructor(title?: string) {
    super(title || '正则提取')
    this.color = COLOR_DATA
    this.addInput('input', 'string')
    this.addOutput('captures', 'array')
    this.addWidget('string', 'pattern', '', 'pattern')
    this.addWidget('toggle', 'global', true, 'global')
  }
  onExecute(): void {}
}

class RegexReplaceNode extends WorkflowNode {
  static title = '正则替换'
  static type = 'regex_replace'
  constructor(title?: string) {
    super(title || '正则替换')
    this.color = COLOR_DATA
    this.addInput('input', 'string')
    this.addOutput('result', 'string')
    this.addWidget('string', 'pattern', '', 'pattern')
    this.addWidget('string', 'replacement', '', 'replacement')
    this.addWidget('toggle', 'global', true, 'global')
  }
  onExecute(): void {}
}

class RegexMatchNode extends WorkflowNode {
  static title = '正则匹配'
  static type = 'regex_match'
  constructor(title?: string) {
    super(title || '正则匹配')
    this.color = COLOR_DATA
    this.addInput('input', 'string')
    this.addOutput('matches', 'array')
    this.addWidget('string', 'pattern', '', 'pattern')
    this.addWidget('toggle', 'global', true, 'global')
  }
  onExecute(): void {}
}

// ═══════════════════════════════════════════
// 数组节点 (v3: 独立类型)
// ═══════════════════════════════════════════

class ArrayFilterNode extends WorkflowNode {
  static title = '数组过滤'
  static type = 'array_filter'
  constructor(title?: string) {
    super(title || '数组过滤')
    this.color = COLOR_DATA
    this.addInput('source', 'array')
    this.addOutput('result', 'array')
    this.addWidget('string', 'source', '', 'source')
    this.addWidget('string', 'field', '', 'field')
    this.addWidget('combo', 'op', '==', null, { property: "op",
      values: ['==', '!=', '>', '>=', '<', '<=', 'contains', 'starts_with', 'ends_with', 'is_null', 'is_not_null', 'is_empty', 'is_not_empty'],
    })
    this.addWidget('string', 'value', '', 'value')
  }
  onExecute(): void {}
}

class ArraySortNode extends WorkflowNode {
  static title = '数组排序'
  static type = 'array_sort'
  constructor(title?: string) {
    super(title || '数组排序')
    this.color = COLOR_DATA
    this.addInput('source', 'array')
    this.addOutput('result', 'array')
    this.addWidget('string', 'source', '', 'source')
    this.addWidget('string', 'field', '', 'field')
    this.addWidget('combo', 'order', 'asc', null, { property: "order",
      values: ['asc', 'desc'],
    })
  }
  onExecute(): void {}
}

class ArrayDedupNode extends WorkflowNode {
  static title = '数组去重'
  static type = 'array_dedup'
  constructor(title?: string) {
    super(title || '数组去重')
    this.color = COLOR_DATA
    this.addInput('source', 'array')
    this.addOutput('result', 'array')
    this.addWidget('string', 'source', '', 'source')
    this.addWidget('string', 'field', '', 'field')
  }
  onExecute(): void {}
}

class ArrayPaginateNode extends WorkflowNode {
  static title = '数组分页'
  static type = 'array_paginate'
  constructor(title?: string) {
    super(title || '数组分页')
    this.color = COLOR_DATA
    this.addInput('source', 'array')
    this.addOutput('result', 'array')
    this.addWidget('string', 'source', '', 'source')
    this.addWidget('number', 'page', 1, 'page', { min: 1, max: 9999, step2: 1 })
    this.addWidget('number', 'page_size', 10, 'page_size', { min: 1, max: 500, step2: 5 })
  }
  onExecute(): void {}
}

class ArrayMapNode extends WorkflowNode {
  static title = '数组映射'
  static type = 'array_map'
  constructor(title?: string) {
    super(title || '数组映射')
    this.color = COLOR_DATA
    this.addInput('source', 'array')
    this.addOutput('result', 'array')
    this.addWidget('string', 'source', '', 'source')
    this.addWidget('string', 'template', '{{__item}}', 'template')
  }
  onExecute(): void {}
}

class ArrayJoinNode extends WorkflowNode {
  static title = '数组连接'
  static type = 'array_join'
  constructor(title?: string) {
    super(title || '数组连接')
    this.color = COLOR_DATA
    this.addInput('source', 'array')
    this.addOutput('result', 'string')
    this.addWidget('string', 'source', '', 'source')
    this.addWidget('string', 'separator', ',', 'separator')
    this.addWidget('string', 'field', '', 'field')
  }
  onExecute(): void {}
}

class ArrayReduceNode extends WorkflowNode {
  static title = '数组聚合'
  static type = 'array_reduce'
  constructor(title?: string) {
    super(title || '数组聚合')
    this.color = COLOR_DATA
    this.addInput('source', 'array')
    this.addOutput('result', 'number')
    this.addWidget('string', 'source', '', 'source')
    this.addWidget('combo', 'aggregator', 'count', null, { property: "aggregator",
      values: ['count', 'sum', 'avg', 'min', 'max', 'first', 'last'],
    })
    this.addWidget('string', 'field', '', 'field')
  }
  onExecute(): void {}
}

// ═══════════════════════════════════════════
// 转换节点 (v3: 独立类型)
// ═══════════════════════════════════════════

class ConvertToTextNode extends WorkflowNode {
  static title = '转文本'
  static type = 'convert_to_text'
  constructor(title?: string) {
    super(title || '转文本')
    this.color = COLOR_DATA
    this.addInput('input', 'any')
    this.addOutput('result', 'string')
    this.addWidget('string', 'input', '', 'input')
  }
  onExecute(): void {}
}

class ConvertToNumberNode extends WorkflowNode {
  static title = '转数字'
  static type = 'convert_to_number'
  constructor(title?: string) {
    super(title || '转数字')
    this.color = COLOR_DATA
    this.addInput('input', 'any')
    this.addOutput('result', 'number')
    this.addWidget('string', 'input', '', 'input')
  }
  onExecute(): void {}
}

class ConvertToJsonNode extends WorkflowNode {
  static title = '转 JSON'
  static type = 'convert_to_json'
  constructor(title?: string) {
    super(title || '转 JSON')
    this.color = COLOR_DATA
    this.addInput('input', 'string')
    this.addOutput('result', 'object')
    this.addWidget('string', 'input', '', 'input')
  }
  onExecute(): void {}
}

class ConvertToCsvNode extends WorkflowNode {
  static title = '转 CSV'
  static type = 'convert_to_csv'
  constructor(title?: string) {
    super(title || '转 CSV')
    this.color = COLOR_DATA
    this.addInput('input', 'array')
    this.addOutput('result', 'string')
    this.addWidget('string', 'input', '', 'input')
    this.addWidget('string', 'delimiter', ',', 'delimiter')
    this.addWidget('toggle', 'include_header', true, 'include_header')
  }
  onExecute(): void {}
}

class ConvertToHtmlNode extends WorkflowNode {
  static title = '转 HTML'
  static type = 'convert_to_html'
  constructor(title?: string) {
    super(title || '转 HTML')
    this.color = COLOR_DATA
    this.addInput('input', 'any')
    this.addOutput('result', 'string')
    this.addWidget('string', 'input', '', 'input')
    this.addWidget('string', 'template', '', 'template')
  }
  onExecute(): void {}
}

class ConvertToBase64Node extends WorkflowNode {
  static title = '转 Base64'
  static type = 'convert_to_base64'
  constructor(title?: string) {
    super(title || '转 Base64')
    this.color = COLOR_DATA
    this.addInput('input', 'any')
    this.addOutput('result', 'string')
    this.addWidget('string', 'input', '', 'input')
  }
  onExecute(): void {}
}

// ═══════════════════════════════════════════
// 流程控制节点
// ═══════════════════════════════════════════

class DelayNode extends WorkflowNode {
  static title = '延时'
  static type = 'delay'
  constructor(title?: string) {
    super(title || '延时')
    this.color = COLOR_CONTROL
    this.addInput('trigger', 'action')
    this.addOutput('trigger', 'action')
    this.addWidget('number', 'duration_ms', 1000, 'duration_ms', { min: 0, max: 300000, step2: 100 })
  }
  onExecute(): void {}
}

class LoopNode extends WorkflowNode {
  static title = '循环'
  static type = 'loop'
  constructor(title?: string) {
    super(title || '循环')
    this.color = COLOR_CONTROL
    this.addInput('list', 'array')
    this.addOutput('item', 'any')
    this.addOutput('done', 'action')
    this.addWidget('number', 'max_iterations', 1000, 'max_iterations', { min: 1, max: 100000, step2: 100 })
    this.addWidget('combo', 'on_error', 'fail', null, { property: "on_error", values: ['fail', 'skip', 'retry'] })
  }
  onExecute(): void {}
}

class WhileNode extends WorkflowNode {
  static title = 'While 循环'
  static type = 'while'
  constructor(title?: string) {
    super(title || 'While 循环')
    this.color = COLOR_CONTROL
    this.addInput('trigger', 'action')
    this.addOutput('body', 'action')
    this.addOutput('done', 'action')
    this.addWidget('combo', 'condition_op', 'not_empty', null, { property: "condition_op",
      values: ['not_empty', 'is_empty', 'equals', 'contains', 'gt', 'lt'],
    })
    this.addWidget('number', 'max_iterations', 1000, 'max_iterations', { min: 1, max: 100000, step2: 100 })
  }
  onExecute(): void {}
}

class SubWorkflowNode extends WorkflowNode {
  static title = '子流程'
  static type = 'sub_workflow'
  constructor(title?: string) {
    super(title || '子流程')
    this.color = COLOR_CONTROL
    this.addInput('trigger', 'action')
    this.addOutput('trigger', 'action')
    this.addWidget('string', 'output_key', 'result', 'output_key')
  }
  onExecute(): void {}
}

class ApprovalNode extends WorkflowNode {
  static title = '审批'
  static type = 'approval'
  constructor(title?: string) {
    super(title || '审批')
    this.color = COLOR_CONTROL
    this.addInput('trigger', 'action')
    this.addOutput('approved', 'action')
    this.addOutput('rejected', 'action')
    this.addWidget('string', 'message', '请审批此操作', 'message')
    this.addWidget('number', 'timeout', 300, 'timeout', { min: 0, max: 86400, step2: 60 })
  }
  onExecute(): void {}
}

class ParallelNode extends WorkflowNode {
  static title = '并行'
  static type = 'parallel'
  constructor(title?: string) {
    super(title || '并行')
    this.color = COLOR_CONTROL
    this.addInput('trigger', 'action')
    this.addOutput('done', 'action')
    this.addWidget('number', 'branch_count', 2, 'branch_count', { min: 2, max: 16, step2: 1 })
  }
  onExecute(): void {}
}

// ═══════════════════════════════════════════
// 输出节点
// ═══════════════════════════════════════════

class PrintNode extends WorkflowNode {
  static title = '控制台打印'
  static type = 'print'
  constructor(title?: string) {
    super(title || '控制台打印')
    this.color = COLOR_OUTPUT
    this.addInput('data', 'any')
    this.addOutput('trigger', 'action')
    this.addWidget('string', 'message', '', 'message')
    this.addWidget('combo', 'level', 'info', null, { property: "level",
      values: ['info', 'warn', 'error'],
    })
  }
  onExecute(): void {}
}

class NotifyNode extends WorkflowNode {
  static title = '通知'
  static type = 'notify'
  constructor(title?: string) {
    super(title || '通知')
    this.color = COLOR_OUTPUT
    this.addInput('trigger', 'action')
    this.addOutput('trigger', 'action')
    this.addWidget('combo', 'notify_type', 'system', null, { property: "notify_type",
      values: ['system', 'email', 'feishu', 'wechat', 'sms', 'webhook'],
    })
    this.addWidget('string', 'title', '通知标题', 'title')
    this.addWidget('string', 'body', '通知内容', 'body')
  }
  onExecute(): void {}
}

// ═══════════════════════════════════════════
// 桌面自动化节点
// ═══════════════════════════════════════════

class MouseKeyboardNode extends WorkflowNode {
  static title = '鼠标/键盘'
  static type = 'mouse_keyboard'
  constructor(title?: string) {
    super(title || '鼠标/键盘')
    this.color = COLOR_DEFAULT
    this.addInput('trigger', 'action')
    this.addOutput('trigger', 'action')
    this.addWidget('combo', 'action', 'click', null, { property: "action",
      values: ['click', 'dblclick', 'move', 'drag', 'scroll', 'type', 'keypress', 'hotkey'],
    })
    this.addWidget('number', 'x', 0, 'x')
    this.addWidget('number', 'y', 0, 'y')
    this.addWidget('combo', 'button', 'left', null, { property: "button", values: ['left', 'right', 'middle'] })
  }
  onExecute(): void {}
}

class WindowNode extends WorkflowNode {
  static title = '窗口管理'
  static type = 'window'
  constructor(title?: string) {
    super(title || '窗口管理')
    this.color = COLOR_DEFAULT
    this.addInput('trigger', 'action')
    this.addOutput('trigger', 'action')
    this.addWidget('combo', 'action', 'find', null, { property: "action",
      values: ['find', 'focus', 'minimize', 'maximize', 'close', 'resize', 'move', 'screenshot'],
    })
    this.addWidget('string', 'title', '', 'title')
    this.addWidget('number', 'x', 0, 'x')
    this.addWidget('number', 'y', 0, 'y')
    this.addWidget('number', 'width', 800, 'width')
    this.addWidget('number', 'height', 600, 'height')
  }
  onExecute(): void {}
}

class RecordingNode extends WorkflowNode {
  static title = '操作录制'
  static type = 'recording'
  constructor(title?: string) {
    super(title || '操作录制')
    this.color = COLOR_DEFAULT
    this.addInput('trigger', 'action')
    this.addOutput('trigger', 'action')
    this.addWidget('combo', 'action', 'start', null, { property: "action",
      values: ['start', 'stop', 'play', 'save', 'load'],
    })
    this.addWidget('toggle', 'headless', false, 'headless')
    this.addWidget('string', 'filename', 'recording.json', 'filename')
  }
  onExecute(): void {}
}

class OcrNode extends WorkflowNode {
  static title = 'OCR 识别'
  static type = 'ocr'
  constructor(title?: string) {
    super(title || 'OCR 识别')
    this.color = COLOR_DATA
    this.addInput('image', 'string')
    this.addOutput('text', 'string')
    this.addWidget('combo', 'action', 'read', null, { property: "action",
      values: ['read', 'read_screen', 'read_region', 'read_clipboard'],
    })
    this.addWidget('combo', 'language', 'auto', null, { property: "language",
      values: ['auto', 'chi_sim', 'chi_tra', 'eng', 'jpn', 'kor', 'fra', 'deu', 'spa'],
    })
  }
  onExecute(): void {}
}

// ═══════════════════════════════════════════
// 浏览器节点 (v3: 所有独立类型)
// ═══════════════════════════════════════════

class BrowserNode extends WorkflowNode {
  static title = '浏览器(万能)'
  static type = 'browser'
  constructor(title?: string) {
    super(title || '浏览器')
    this.color = COLOR_BROWSER
    this.addInput('input', 'any')
    this.addOutput('data', 'any')
    this.addOutput('error', 'string')
    this.addWidget('string', 'action', 'navigate', 'action')
    this.addWidget('string', 'url', 'https://example.com', 'url')
    this.addWidget('string', 'selector', '', 'selector')
    this.addWidget('string', 'value', '', 'value')
    this.addWidget('number', 'timeout', 30000, 'timeout', { min: 1000, max: 300000, step2: 1000 })
  }
  onExecute(): void {}
}

class BrowserNavigateNode extends WorkflowNode {
  static title = '浏览器导航'
  static type = 'browser_navigate'
  constructor(title?: string) {
    super(title || '导航')
    this.color = COLOR_BROWSER
    this.addInput('url', 'string')
    this.addOutput('page', 'object')
    this.addOutput('error', 'string')
    this.addWidget('string', 'url', 'https://example.com', 'url')
    this.addWidget('combo', 'wait_until', 'load', null, { property: "wait_until",
      values: ['load', 'domcontentloaded', 'networkidle'],
    })
  }
  onExecute(): void {}
}

class BrowserClickNode extends WorkflowNode {
  static title = '浏览器点击'
  static type = 'browser_click'
  constructor(title?: string) {
    super(title || '点击')
    this.color = COLOR_BROWSER
    this.addInput('selector', 'string')
    this.addOutput('data', 'object')
    this.addOutput('error', 'string')
    this.addWidget('string', 'selector', '', 'selector')
  }
  onExecute(): void {}
}

class BrowserFillNode extends WorkflowNode {
  static title = '浏览器填写'
  static type = 'browser_fill'
  constructor(title?: string) {
    super(title || '填写')
    this.color = COLOR_BROWSER
    this.addInput('selector', 'string')
    this.addInput('value', 'string')
    this.addOutput('data', 'object')
    this.addOutput('error', 'string')
    this.addWidget('string', 'selector', '', 'selector')
    this.addWidget('string', 'value', '', 'value')
    this.addWidget('toggle', 'clear', true, 'clear')
  }
  onExecute(): void {}
}

class BrowserExtractNode extends WorkflowNode {
  static title = '浏览器提取'
  static type = 'browser_extract'
  constructor(title?: string) {
    super(title || '提取')
    this.color = COLOR_BROWSER
    this.addInput('selector', 'string')
    this.addOutput('data', 'array')
    this.addOutput('error', 'string')
    this.addWidget('combo', 'mode', 'text', null, { property: "mode",
      values: ['text', 'html', 'table', 'links', 'attribute'],
    })
    this.addWidget('string', 'selector', '', 'selector')
    this.addWidget('string', 'attribute', 'href', 'attribute')
  }
  onExecute(): void {}
}

class BrowserScreenshotNode extends WorkflowNode {
  static title = '浏览器截图'
  static type = 'browser_screenshot'
  constructor(title?: string) {
    super(title || '截图')
    this.color = COLOR_BROWSER
    this.addOutput('path', 'string')
    this.addOutput('error', 'string')
    this.addWidget('string', 'path', 'screenshot.png', 'path')
    this.addWidget('toggle', 'full_page', false, 'full_page')
  }
  onExecute(): void {}
}

class BrowserEvaluateNode extends WorkflowNode {
  static title = '浏览器执行JS'
  static type = 'browser_evaluate'
  constructor(title?: string) {
    super(title || '执行JS')
    this.color = COLOR_BROWSER
    this.addInput('script', 'string')
    this.addOutput('result', 'any')
    this.addOutput('error', 'string')
    this.addWidget('text', 'script', 'document.title', 'script')
  }
  onExecute(): void {}
}

class BrowserScrollNode extends WorkflowNode {
  static title = '浏览器滚动'
  static type = 'browser_scroll'
  constructor(title?: string) {
    super(title || '滚动')
    this.color = COLOR_BROWSER
    this.addOutput('data', 'object')
    this.addWidget('combo', 'direction', 'bottom', null, { property: "direction",
      values: ['bottom', 'top'],
    })
    this.addWidget('number', 'times', 1, 'times', { min: 1, max: 100, step2: 1 })
    this.addWidget('number', 'delay_ms', 500, 'delay_ms', { min: 0, max: 10000, step2: 100 })
  }
  onExecute(): void {}
}

class BrowserWaitNode extends WorkflowNode {
  static title = '浏览器等待'
  static type = 'browser_wait'
  constructor(title?: string) {
    super(title || '等待')
    this.color = COLOR_BROWSER
    this.addInput('selector', 'string')
    this.addOutput('found', 'boolean')
    this.addWidget('string', 'selector', '', 'selector')
    this.addWidget('number', 'timeout_ms', 30000, 'timeout_ms', { min: 1000, max: 300000, step2: 1000 })
  }
  onExecute(): void {}
}

class BrowserPdfNode extends WorkflowNode {
  static title = '浏览器PDF'
  static type = 'browser_pdf'
  constructor(title?: string) {
    super(title || '生成PDF')
    this.color = COLOR_BROWSER
    this.addOutput('path', 'string')
    this.addWidget('string', 'path', 'output.pdf', 'path')
  }
  onExecute(): void {}
}

class WebScrapeNode extends WorkflowNode {
  static title = '网页抓取'
  static type = 'web_scrape'
  constructor(title?: string) {
    super(title || '网页抓取')
    this.color = COLOR_DATA
    this.addInput('url', 'string')
    this.addOutput('data', 'string')
    this.addWidget('string', 'url', 'https://example.com', 'url')
  }
  onExecute(): void {}
}

class MapNode extends WorkflowNode {
  static title = '数据映射'
  static type = 'map'
  constructor(title?: string) {
    super(title || '数据映射')
    this.color = COLOR_DATA
    this.addInput('list', 'array')
    this.addOutput('results', 'array')
    this.addWidget('string', 'source', '', 'source')
    this.addWidget('string', 'template', '', 'template')
  }
  onExecute(): void {}
}

// ═══════════════════════════════════════════
// Excel 节点
// ═══════════════════════════════════════════

class ExcelNode extends WorkflowNode {
  static title = 'Excel(通用)'
  static type = 'excel'
  constructor(title?: string) {
    super(title || 'Excel')
    this.color = COLOR_EXCEL
    this.addInput('file', 'string')
    this.addOutput('data', 'object')
    this.addWidget('combo', 'action', 'read', null, { property: 'action',
      values: ['read', 'write', 'append', 'create', 'merge'],
    })
    this.addWidget('string', 'path', './input.xlsx', 'path')
    this.addWidget('string', 'sheet', 'Sheet1', 'sheet')
    this.addWidget('string', 'range', 'A1', 'range')
  }
  onExecute(): void {}
}

class ExcelReadNode extends WorkflowNode {
  static title = '读取表格'
  static type = 'excel_read'
  constructor(title?: string) {
    super(title || '读取表格')
    this.color = COLOR_EXCEL
    this.addOutput('data', 'array')
    this.addOutput('sheet', 'string')
    this.addWidget('string', 'path', '', 'path')
    this.addWidget('string', 'sheet', '', 'sheet')
  }
  onExecute(): void {}
}

class ExcelWriteNode extends WorkflowNode {
  static title = '写入表格'
  static type = 'excel_write'
  constructor(title?: string) {
    super(title || '写入表格')
    this.color = COLOR_EXCEL
    this.addInput('data', 'array')
    this.addOutput('path', 'string')
    this.addWidget('string', 'path', '', 'path')
    this.addWidget('string', 'sheet', 'Sheet1', 'sheet')
    this.addWidget('combo', 'write_mode', 'overwrite', null, { property: 'write_mode',
      values: ['overwrite', 'append'],
    })
  }
  onExecute(): void {}
}

class ExcelCreateNode extends WorkflowNode {
  static title = '创建表格'
  static type = 'excel_create'
  constructor(title?: string) {
    super(title || '创建表格')
    this.color = COLOR_EXCEL
    this.addInput('data', 'array')
    this.addOutput('path', 'string')
    this.addWidget('string', 'path', '新表格.xlsx', 'path')
    this.addWidget('string', 'sheet', 'Sheet1', 'sheet')
    this.addWidget('string', 'headers', '', 'headers')
  }
  onExecute(): void {}
}

class ExcelFilterNode extends WorkflowNode {
  static title = '筛选数据'
  static type = 'excel_filter'
  constructor(title?: string) {
    super(title || '筛选数据')
    this.color = COLOR_EXCEL
    this.addInput('data', 'array')
    this.addInput('condition', 'object')
    this.addOutput('result', 'array')
    this.addWidget('string', 'column', '', 'column')
    this.addWidget('combo', 'op', '==', null, { property: 'op',
      values: ['==', '!=', '>', '<', '>=', '<=', 'contains', 'starts_with'],
    })
    this.addWidget('string', 'value', '', 'value')
  }
  onExecute(): void {}
}

class ExcelSortNode extends WorkflowNode {
  static title = '排序数据'
  static type = 'excel_sort'
  constructor(title?: string) {
    super(title || '排序数据')
    this.color = COLOR_EXCEL
    this.addInput('data', 'array')
    this.addOutput('result', 'array')
    this.addWidget('string', 'column', '', 'column')
    this.addWidget('combo', 'order', 'asc', null, { property: 'order',
      values: ['asc', 'desc'],
    })
  }
  onExecute(): void {}
}

class ExcelAppendNode extends WorkflowNode {
  static title = '追加行'
  static type = 'excel_append'
  constructor(title?: string) {
    super(title || '追加行')
    this.color = COLOR_EXCEL
    this.addInput('data', 'array')
    this.addOutput('path', 'string')
    this.addWidget('string', 'path', '', 'path')
    this.addWidget('string', 'sheet', 'Sheet1', 'sheet')
  }
  onExecute(): void {}
}

class ExcelCsvNode extends WorkflowNode {
  static title = 'CSV 互转'
  static type = 'excel_csv'
  constructor(title?: string) {
    super(title || 'CSV 互转')
    this.color = COLOR_EXCEL
    this.addOutput('path', 'string')
    this.addWidget('string', 'path', '', 'path')
    this.addWidget('combo', 'direction', 'csv_to_xlsx', null, { property: 'direction',
      values: ['csv_to_xlsx', 'xlsx_to_csv'],
    })
    this.addWidget('string', 'delimiter', ',', 'delimiter')
    this.addWidget('string', 'output', '', 'output')
  }
  onExecute(): void {}
}

// ═══════════════════════════════════════════
// Word 节点
// ═══════════════════════════════════════════

class WordNode extends WorkflowNode {
  static title = 'Word(通用)'
  static type = 'word'
  constructor(title?: string) {
    super(title || 'Word')
    this.color = COLOR_WORD
    this.addInput('file', 'string')
    this.addOutput('data', 'string')
    this.addWidget('combo', 'action', 'read', null, { property: 'action',
      values: ['read', 'write', 'create', 'convert', 'merge'],
    })
    this.addWidget('string', 'path', './input.docx', 'path')
    this.addWidget('combo', 'format', 'text', null, { property: 'format',
      values: ['text', 'html', 'markdown', 'docx'],
    })
  }
  onExecute(): void {}
}

class WordReadNode extends WorkflowNode {
  static title = '读取文档'
  static type = 'word_read'
  constructor(title?: string) {
    super(title || '读取文档')
    this.color = COLOR_WORD
    this.addOutput('text', 'string')
    this.addOutput('paragraphs', 'array')
    this.addWidget('string', 'path', '', 'path')
  }
  onExecute(): void {}
}

class WordWriteNode extends WorkflowNode {
  static title = '写入文档'
  static type = 'word_write'
  constructor(title?: string) {
    super(title || '写入文档')
    this.color = COLOR_WORD
    this.addInput('content', 'string')
    this.addOutput('path', 'string')
    this.addWidget('string', 'path', '', 'path')
    this.addWidget('combo', 'mode', 'overwrite', null, { property: 'mode',
      values: ['overwrite', 'append'],
    })
  }
  onExecute(): void {}
}

class WordCreateNode extends WorkflowNode {
  static title = '创建文档'
  static type = 'word_create'
  constructor(title?: string) {
    super(title || '创建文档')
    this.color = COLOR_WORD
    this.addInput('content', 'string')
    this.addOutput('path', 'string')
    this.addWidget('string', 'path', '新文档.docx', 'path')
    this.addWidget('string', 'title', '', 'title')
  }
  onExecute(): void {}
}

class WordReplaceNode extends WorkflowNode {
  static title = '查找替换'
  static type = 'word_replace'
  constructor(title?: string) {
    super(title || '查找替换')
    this.color = COLOR_WORD
    this.addInput('content', 'string')
    this.addOutput('result', 'string')
    this.addWidget('string', 'path', '', 'path')
    this.addWidget('string', 'find', '', 'find')
    this.addWidget('string', 'replace', '', 'replace')
    this.addWidget('number', 'count', 0, 'count', { min: 0, max: 9999, step2: 1 })
  }
  onExecute(): void {}
}

class WordMergeNode extends WorkflowNode {
  static title = '合并文档'
  static type = 'word_merge'
  constructor(title?: string) {
    super(title || '合并文档')
    this.color = COLOR_WORD
    this.addInput('files', 'array')
    this.addOutput('path', 'string')
    this.addWidget('string', 'paths', '', 'paths')
    this.addWidget('string', 'output', '合并文档.docx', 'output')
  }
  onExecute(): void {}
}

// ═══════════════════════════════════════════
// Registry: register all node types
// ═══════════════════════════════════════════

export function registerAllNodes(): void {
  // P0 核心
  LiteGraph.registerNodeType('http', HttpNode)
  LiteGraph.registerNodeType('script', ScriptNode)
  LiteGraph.registerNodeType('condition', ConditionNode)

  // 数据处理
  LiteGraph.registerNodeType('data_set', DataSetNode)
  LiteGraph.registerNodeType('data_get', DataGetNode)
  LiteGraph.registerNodeType('data_length', DataLengthNode)
  LiteGraph.registerNodeType('data_default', DataDefaultNode)
  LiteGraph.registerNodeType('data_merge', DataMergeNode)
  LiteGraph.registerNodeType('json_parse', JsonParseNode)
  LiteGraph.registerNodeType('text_template', TextTemplateNode)

  // 文件
  LiteGraph.registerNodeType('file_read', FileReadNode)
  LiteGraph.registerNodeType('file_write', FileWriteNode)
  LiteGraph.registerNodeType('file_list', FileListNode)
  LiteGraph.registerNodeType('file_delete', FileDeleteNode)
  LiteGraph.registerNodeType('file_exists', FileExistsNode)
  LiteGraph.registerNodeType('file_save', FileSaveNode)

  // 剪贴板
  LiteGraph.registerNodeType('clipboard_read', ClipboardReadNode)
  LiteGraph.registerNodeType('clipboard_write', ClipboardWriteNode)

  // 正则
  LiteGraph.registerNodeType('regex_extract', RegexExtractNode)
  LiteGraph.registerNodeType('regex_replace', RegexReplaceNode)
  LiteGraph.registerNodeType('regex_match', RegexMatchNode)

  // 数组
  LiteGraph.registerNodeType('array_filter', ArrayFilterNode)
  LiteGraph.registerNodeType('array_sort', ArraySortNode)
  LiteGraph.registerNodeType('array_dedup', ArrayDedupNode)
  LiteGraph.registerNodeType('array_paginate', ArrayPaginateNode)
  LiteGraph.registerNodeType('array_map', ArrayMapNode)
  LiteGraph.registerNodeType('array_join', ArrayJoinNode)
  LiteGraph.registerNodeType('array_reduce', ArrayReduceNode)

  // 转换
  LiteGraph.registerNodeType('convert_to_text', ConvertToTextNode)
  LiteGraph.registerNodeType('convert_to_number', ConvertToNumberNode)
  LiteGraph.registerNodeType('convert_to_json', ConvertToJsonNode)
  LiteGraph.registerNodeType('convert_to_csv', ConvertToCsvNode)
  LiteGraph.registerNodeType('convert_to_html', ConvertToHtmlNode)
  LiteGraph.registerNodeType('convert_to_base64', ConvertToBase64Node)

  // 流程控制
  LiteGraph.registerNodeType('delay', DelayNode)
  LiteGraph.registerNodeType('loop', LoopNode)
  LiteGraph.registerNodeType('while', WhileNode)
  LiteGraph.registerNodeType('sub_workflow', SubWorkflowNode)
  LiteGraph.registerNodeType('approval', ApprovalNode)
  LiteGraph.registerNodeType('parallel', ParallelNode)

  // 输出
  LiteGraph.registerNodeType('print', PrintNode)
  LiteGraph.registerNodeType('notify', NotifyNode)

  // 桌面自动化
  LiteGraph.registerNodeType('mouse_keyboard', MouseKeyboardNode)
  LiteGraph.registerNodeType('window', WindowNode)
  LiteGraph.registerNodeType('recording', RecordingNode)
  LiteGraph.registerNodeType('ocr', OcrNode)

  // 浏览器
  LiteGraph.registerNodeType('browser', BrowserNode)
  LiteGraph.registerNodeType('browser_navigate', BrowserNavigateNode)
  LiteGraph.registerNodeType('browser_click', BrowserClickNode)
  LiteGraph.registerNodeType('browser_fill', BrowserFillNode)
  LiteGraph.registerNodeType('browser_extract', BrowserExtractNode)
  LiteGraph.registerNodeType('browser_screenshot', BrowserScreenshotNode)
  LiteGraph.registerNodeType('browser_evaluate', BrowserEvaluateNode)
  LiteGraph.registerNodeType('browser_scroll', BrowserScrollNode)
  LiteGraph.registerNodeType('browser_wait', BrowserWaitNode)
  LiteGraph.registerNodeType('browser_pdf', BrowserPdfNode)

  // Web
  LiteGraph.registerNodeType('web_scrape', WebScrapeNode)
  LiteGraph.registerNodeType('map', MapNode)

  // Excel
  LiteGraph.registerNodeType('excel', ExcelNode)
  LiteGraph.registerNodeType('excel_read', ExcelReadNode)
  LiteGraph.registerNodeType('excel_write', ExcelWriteNode)
  LiteGraph.registerNodeType('excel_create', ExcelCreateNode)
  LiteGraph.registerNodeType('excel_filter', ExcelFilterNode)
  LiteGraph.registerNodeType('excel_sort', ExcelSortNode)
  LiteGraph.registerNodeType('excel_append', ExcelAppendNode)
  LiteGraph.registerNodeType('excel_csv', ExcelCsvNode)

  // Word
  LiteGraph.registerNodeType('word', WordNode)
  LiteGraph.registerNodeType('word_read', WordReadNode)
  LiteGraph.registerNodeType('word_write', WordWriteNode)
  LiteGraph.registerNodeType('word_create', WordCreateNode)
  LiteGraph.registerNodeType('word_replace', WordReplaceNode)
  LiteGraph.registerNodeType('word_merge', WordMergeNode)
}

// Export all node classes for direct usage
export {
  WorkflowNode,
  HttpNode,
  ScriptNode,
  ConditionNode,
  DataSetNode,
  DataGetNode,
  DataLengthNode,
  DataDefaultNode,
  DataMergeNode,
  JsonParseNode,
  TextTemplateNode,
  FileReadNode,
  FileWriteNode,
  FileListNode,
  FileDeleteNode,
  FileExistsNode,
  FileSaveNode,
  ClipboardReadNode,
  ClipboardWriteNode,
  RegexExtractNode,
  RegexReplaceNode,
  RegexMatchNode,
  ArrayFilterNode,
  ArraySortNode,
  ArrayDedupNode,
  ArrayPaginateNode,
  ArrayMapNode,
  ArrayJoinNode,
  ArrayReduceNode,
  ConvertToTextNode,
  ConvertToNumberNode,
  ConvertToJsonNode,
  ConvertToCsvNode,
  ConvertToHtmlNode,
  ConvertToBase64Node,
  DelayNode,
  LoopNode,
  WhileNode,
  SubWorkflowNode,
  ApprovalNode,
  ParallelNode,
  PrintNode,
  NotifyNode,
  MouseKeyboardNode,
  WindowNode,
  RecordingNode,
  OcrNode,
  BrowserNode,
  BrowserNavigateNode,
  BrowserClickNode,
  BrowserFillNode,
  BrowserExtractNode,
  BrowserScreenshotNode,
  BrowserEvaluateNode,
  BrowserScrollNode,
  BrowserWaitNode,
  BrowserPdfNode,
  WebScrapeNode,
  MapNode,
  ExcelNode,
  ExcelReadNode,
  ExcelWriteNode,
  ExcelCreateNode,
  ExcelFilterNode,
  ExcelSortNode,
  ExcelAppendNode,
  ExcelCsvNode,
  WordNode,
  WordReadNode,
  WordWriteNode,
  WordCreateNode,
  WordReplaceNode,
  WordMergeNode,
}
