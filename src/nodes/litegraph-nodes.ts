// ─── LiteGraph Custom Node Definitions ───
// All 34 workflow node types for the Workflow Engine Desktop

import {
  LGraphNode,
  LiteGraph,
} from '@comfyorg/litegraph'

// ═══════════════════════════════════════════
// Color constants (dark theme — improved contrast)
// ═══════════════════════════════════════════
const COLOR_AI = '#58a6ff'
const COLOR_DATA = '#3fb950'
const COLOR_CONTROL = '#d29922'
const COLOR_OUTPUT = '#f78166'
const COLOR_DEFAULT = '#8b949e'

// 节点背景：画布是 #0d1117，需要明显反差
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
}

// ═══════════════════════════════════════════
// SOURCE / DATA nodes
// ═══════════════════════════════════════════

/** HTTP 请求节点 - 发送 HTTP 请求获取数据 */
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

  onExecute(): void {
    // Execute HTTP request
  }
}

/** 文件操作节点 - 读取本地文件内容 */
class FileNode extends WorkflowNode {
  static title = '文件操作'
  static type = 'file'

  constructor(title?: string) {
    super(title || '文件操作')
    this.color = COLOR_DATA
    this.addOutput('data', 'string')
    this.addWidget('string', 'path', '', 'path')
    this.addWidget('combo', 'encoding', 'utf-8', null, { property: "encoding", 
      values: ['utf-8', 'ascii', 'latin1', 'utf-16', 'base64'],
    })
  }

  onExecute(): void {
    // Read file from disk
  }
}

/** 剪贴板节点 - 读取系统剪贴板内容 */
class ClipboardNode extends WorkflowNode {
  static title = '剪贴板'
  static type = 'clipboard'

  constructor(title?: string) {
    super(title || '剪贴板')
    this.color = COLOR_DATA
    this.addOutput('data', 'string')
    this.addWidget('combo', 'format', 'text', null, { property: "format", 
      values: ['text', 'html', 'image', 'files'],
    })
  }

  onExecute(): void {
    // Read clipboard content
  }
}

/** JSON 解析节点 - 用 JSONPath 提取字段 */
class JsonParseNode extends WorkflowNode {
  static title = 'JSON 解析'
  static type = 'json_parse'

  constructor(title?: string) {
    super(title || 'JSON 解析')
    this.color = COLOR_DATA
    this.addInput('data', 'string')
    this.addOutput('data', 'object')
    this.addWidget('string', 'expression', '$', 'expression')
    this.addWidget('string', 'target_field', '', 'target_field')
  }

  onExecute(): void {
    // Parse JSON and extract fields
  }
}

/** 正则处理节点 - 使用正则表达式提取/替换文本 */
class RegexNode extends WorkflowNode {
  static title = '正则处理'
  static type = 'regex'

  constructor(title?: string) {
    super(title || '正则处理')
    this.color = COLOR_DATA
    this.addInput('data', 'string')
    this.addOutput('data', 'string')
    this.addWidget('string', 'pattern', '', 'pattern')
    this.addWidget('combo', 'action', 'extract', null, { property: "action", 
      values: ['extract', 'replace', 'match', 'split', 'test'],
    })
    this.addWidget('string', 'replacement', '', 'replacement')
    this.addWidget('string', 'flags', 'g', 'flags')
  }

  onExecute(): void {
    // Apply regex operation
  }
}

/** 数组操作节点 - 数组过滤、映射、排序、去重 */
class ArrayNode extends WorkflowNode {
  static title = '数组操作'
  static type = 'array'

  constructor(title?: string) {
    super(title || '数组操作')
    this.color = COLOR_DATA
    this.addInput('data', 'array')
    this.addOutput('data', 'array')
    this.addWidget('combo', 'action', 'filter', null, { property: "action", 
      values: ['filter', 'map', 'sort', 'dedupe', 'slice', 'reverse', 'join', 'group'],
    })
    this.addWidget('string', 'expression', '', 'expression')
    this.addWidget('number', 'limit', 100, 'limit')
  }

  onExecute(): void {
    // Perform array operation
  }
}

/** 类型转换节点 - 字符串↔数字↔布尔↔JSON 等类型互转 */
class ConvertNode extends WorkflowNode {
  static title = '类型转换'
  static type = 'convert'

  constructor(title?: string) {
    super(title || '类型转换')
    this.color = COLOR_DATA
    this.addInput('data', 'string')
    this.addOutput('data', 'string')
    this.addWidget('combo', 'from', 'auto', null, { property: "from", 
      values: ['auto', 'string', 'number', 'boolean', 'json', 'base64', 'hex'],
    })
    this.addWidget('combo', 'to', 'string', null, { property: "to", 
      values: ['string', 'number', 'boolean', 'json', 'base64', 'hex'],
    })
  }

  onExecute(): void {
    // Convert data type
  }
}

/** 文本拼接节点 - 模板替换拼接文本 */
class TextTemplateNode extends WorkflowNode {
  static title = '文本拼接'
  static type = 'text_template'

  constructor(title?: string) {
    super(title || '文本拼接')
    this.color = COLOR_DATA
    this.addInput('data', 'string')
    this.addOutput('data', 'string')
    this.addWidget('string', 'template', '', 'template')
    this.addWidget('string', 'output_key', '', 'output_key')
  }

  onExecute(): void {
    // Apply template substitution
  }
}

/** 变量操作节点 - 变量设置/读取/合并/默认值 */
class DataNode extends WorkflowNode {
  static title = '变量操作'
  static type = 'data'

  constructor(title?: string) {
    super(title || '变量操作')
    this.color = COLOR_DATA
    this.addOutput('data', 'any')
    this.addWidget('combo', 'action', 'set', null, { property: "action", 
      values: ['set', 'get', 'merge', 'default', 'delete', 'length', 'keys'],
    })
    this.addWidget('string', 'key', '', 'key')
    this.addWidget('string', 'value', '', 'value')
  }

  onExecute(): void {
    // Read/write variable
  }
}

// ═══════════════════════════════════════════
// CONTROL FLOW nodes
// ═══════════════════════════════════════════

/** 延时节点 - 暂停执行指定时长（毫秒） */
class DelayNode extends WorkflowNode {
  static title = '延时'
  static type = 'delay'

  constructor(title?: string) {
    super(title || '延时')
    this.color = COLOR_CONTROL
    this.addInput('trigger', 'action')
    this.addOutput('trigger', 'action')
    this.addWidget('number', 'duration_ms', 1000, 'duration_ms', {
      min: 0,
      max: 300000,
      step2: 100,
    })
  }

  onExecute(): void {
    // Delay execution
  }
}

/** 循环节点 - 遍历数组，对每个元素执行子步骤 */
class LoopNode extends WorkflowNode {
  static title = '循环'
  static type = 'loop'

  constructor(title?: string) {
    super(title || '循环')
    this.color = COLOR_CONTROL
    this.addInput('list', 'array')
    this.addOutput('item', 'any')
    this.addOutput('done', 'action')
    this.addWidget('number', 'max_iterations', 1000, 'max_iterations', {
      min: 1,
      max: 100000,
      step2: 100,
    })
    this.addWidget('combo', 'on_error', 'fail', null, { property: "on_error", 
      values: ['fail', 'skip', 'retry'],
    })
  }

  onExecute(): void {
    // Iterate over array
  }
}

/** While 循环节点 - 条件循环，满足条件时重复执行 */
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
    this.addWidget('number', 'max_iterations', 1000, 'max_iterations', {
      min: 1,
      max: 100000,
      step2: 100,
    })
  }

  onExecute(): void {
    // While loop execution
  }
}

/** 条件判断节点 - 根据条件表达式选择执行分支 */
class ConditionNode extends WorkflowNode {
  static title = '条件判断'
  static type = 'condition'

  constructor(title?: string) {
    super(title || '条件判断')
    this.color = COLOR_CONTROL
    this.addInput('trigger', 'action')
    this.addOutput('true', 'action')
    this.addOutput('false', 'action')
    this.addWidget('combo', 'op', '==', null, { property: "op", 
      values: [
        '==', '!=', '>', '<', '>=', '<=',
        'contains', 'starts_with', 'ends_with',
        'regex', 'is_empty', 'is_truthy',
      ],
    })
    this.addWidget('string', 'left', '', 'left')
    this.addWidget('string', 'right', '', 'right')
  }

  onExecute(): void {
    // Evaluate condition
  }
}

/** 子流程节点 - 打包一组节点为可复用的子流程图 */
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

  onExecute(): void {
    // Execute sub-workflow
  }
}

/** 审批节点 - 等待人工审批 */
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
    this.addWidget('number', 'timeout', 300, 'timeout', {
      min: 0,
      max: 86400,
      step2: 60,
    })
  }

  onExecute(): void {
    // Wait for approval
  }
}

/** 并行节点 - 同时执行多个分支 */
class ParallelNode extends WorkflowNode {
  static title = '并行'
  static type = 'parallel'

  constructor(title?: string) {
    super(title || '并行')
    this.color = COLOR_CONTROL
    this.addInput('trigger', 'action')
    this.addOutput('done', 'action')
    this.addWidget('number', 'branch_count', 2, 'branch_count', {
      min: 2,
      max: 16,
      step2: 1,
    })
  }

  onExecute(): void {
    // Execute branches in parallel
  }
}

// ═══════════════════════════════════════════
// AI nodes
// ═══════════════════════════════════════════

/** AI 调用节点 - 调用 LLM 执行通用任务 */
class AiNode extends WorkflowNode {
  static title = 'AI 调用'
  static type = 'ai'

  constructor(title?: string) {
    super(title || 'AI 调用')
    this.color = COLOR_AI
    this.addInput('prompt', 'string')
    this.addOutput('result', 'string')
    this.addWidget('combo', 'model', 'gpt-3.5-turbo', null, { property: "model", 
      values: [
        'gpt-3.5-turbo', 'gpt-4', 'gpt-4-turbo', 'gpt-4o',
        'claude-3-opus', 'claude-3-sonnet', 'claude-3-haiku',
        'gemini-pro', 'llama-3-70b',
      ],
    })
    this.addWidget('number', 'temperature', 0.7, 'temperature', {
      min: 0,
      max: 2,
      step2: 0.1,
      precision: 1,
    })
    this.addWidget('number', 'max_tokens', 1024, 'max_tokens', {
      min: 1,
      max: 32768,
      step2: 256,
    })
  }

  onExecute(): void {
    // Call LLM
  }
}

/** AI 翻译节点 - 使用 AI 翻译文本 */
class AiTranslateNode extends WorkflowNode {
  static title = 'AI 翻译'
  static type = 'ai_translate'

  constructor(title?: string) {
    super(title || 'AI 翻译')
    this.color = COLOR_AI
    this.addInput('text', 'string')
    this.addOutput('result', 'string')
    this.addWidget('combo', 'source_lang', 'auto', null, { property: "source_lang", 
      values: [
        'auto', 'zh', 'en', 'ja', 'ko', 'fr', 'de', 'es',
        'pt', 'ru', 'ar', 'hi', 'th', 'vi', 'it', 'nl',
      ],
    })
    this.addWidget('combo', 'target_lang', 'en', null, { property: "target_lang", 
      values: [
        'en', 'zh', 'ja', 'ko', 'fr', 'de', 'es',
        'pt', 'ru', 'ar', 'hi', 'th', 'vi', 'it', 'nl',
      ],
    })
    this.addWidget('combo', 'model', 'gpt-3.5-turbo', null, { property: "model", 
      values: ['gpt-3.5-turbo', 'gpt-4', 'gpt-4o', 'claude-3-haiku'],
    })
  }

  onExecute(): void {
    // Translate text
  }
}

/** AI 摘要节点 - 使用 AI 生成文本摘要 */
class AiSummarizeNode extends WorkflowNode {
  static title = 'AI 摘要'
  static type = 'ai_summarize'

  constructor(title?: string) {
    super(title || 'AI 摘要')
    this.color = COLOR_AI
    this.addInput('text', 'string')
    this.addOutput('result', 'string')
    this.addWidget('combo', 'model', 'gpt-3.5-turbo', null, { property: "model", 
      values: ['gpt-3.5-turbo', 'gpt-4', 'gpt-4o', 'claude-3-haiku'],
    })
    this.addWidget('number', 'max_length', 200, 'max_length', {
      min: 10,
      max: 4096,
      step2: 10,
    })
  }

  onExecute(): void {
    // Summarize text
  }
}

/** AI 分类节点 - 使用 AI 对文本进行分类 */
class AiClassifyNode extends WorkflowNode {
  static title = 'AI 分类'
  static type = 'ai_classify'

  constructor(title?: string) {
    super(title || 'AI 分类')
    this.color = COLOR_AI
    this.addInput('text', 'string')
    this.addOutput('result', 'string')
    this.addWidget('string', 'labels', '', 'labels')
    this.addWidget('combo', 'model', 'gpt-3.5-turbo', null, { property: "model", 
      values: ['gpt-3.5-turbo', 'gpt-4', 'gpt-4o', 'claude-3-haiku'],
    })
  }

  onExecute(): void {
    // Classify text
  }
}

/** AI 情感分析节点 - 分析文本的情感倾向 */
class AiSentimentNode extends WorkflowNode {
  static title = 'AI 情感分析'
  static type = 'ai_sentiment'

  constructor(title?: string) {
    super(title || 'AI 情感分析')
    this.color = COLOR_AI
    this.addInput('text', 'string')
    this.addOutput('result', 'string')
    this.addWidget('combo', 'model', 'gpt-3.5-turbo', null, { property: "model", 
      values: ['gpt-3.5-turbo', 'gpt-4', 'gpt-4o', 'claude-3-haiku'],
    })
  }

  onExecute(): void {
    // Analyze sentiment
  }
}

/** AI 实体提取节点 - 使用 AI 提取文本中的命名实体 */
class AiEntitiesNode extends WorkflowNode {
  static title = 'AI 实体提取'
  static type = 'ai_entities'

  constructor(title?: string) {
    super(title || 'AI 实体提取')
    this.color = COLOR_AI
    this.addInput('text', 'string')
    this.addOutput('result', 'string')
    this.addWidget('string', 'entity_types', 'person,org,location', 'entity_types')
    this.addWidget('combo', 'model', 'gpt-3.5-turbo', null, { property: "model", 
      values: ['gpt-3.5-turbo', 'gpt-4', 'gpt-4o', 'claude-3-haiku'],
    })
  }

  onExecute(): void {
    // Extract entities
  }
}

// ═══════════════════════════════════════════
// OUTPUT nodes
// ═══════════════════════════════════════════

/** 保存文件节点 - 写入文件到本地 */
class FileSaveNode extends WorkflowNode {
  static title = '保存文件'
  static type = 'file_save'

  constructor(title?: string) {
    super(title || '保存文件')
    this.color = COLOR_OUTPUT
    this.addInput('data', 'string')
    this.addOutput('trigger', 'action')
    this.addWidget('string', 'path', './output.txt', 'path')
    this.addWidget('combo', 'format', 'json', null, { property: "format", 
      values: ['json', 'yaml', 'csv', 'txt', 'binary', 'auto'],
    })
    this.addWidget('combo', 'encoding', 'utf-8', null, { property: "encoding", 
      values: ['utf-8', 'ascii', 'latin1', 'utf-16', 'base64'],
    })
  }

  onExecute(): void {
    // Save file to disk
  }
}

/** 控制台打印节点 - 将数据输出到控制台日志 */
class PrintNode extends WorkflowNode {
  static title = '控制台打印'
  static type = 'print'

  constructor(title?: string) {
    super(title || '控制台打印')
    this.color = COLOR_OUTPUT
    this.addInput('data', 'any')
    this.addOutput('trigger', 'action')
    this.addWidget('string', 'prefix', '', 'prefix')
    this.addWidget('toggle', 'colorize', true, 'colorize')
  }

  onExecute(): void {
    // Print to console
  }
}

/** 通知节点 - 发送系统通知 */
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

  onExecute(): void {
    // Send notification
  }
}

// ═══════════════════════════════════════════
// AUTOMATION / DESKTOP nodes
// ═══════════════════════════════════════════

/** 鼠标/键盘节点 - 模拟鼠标键盘操作 */
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
    this.addWidget('combo', 'button', 'left', null, { property: "button", 
      values: ['left', 'right', 'middle'],
    })
  }

  onExecute(): void {
    // Simulate mouse/keyboard input
  }
}

/** 窗口管理节点 - 查找和操作窗口 */
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

  onExecute(): void {
    // Manage window
  }
}

/** 操作录制节点 - 录制/回放鼠标键盘操作 */
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

  onExecute(): void {
    // Record or playback automation
  }
}

/** OCR 识别节点 - 从图片中提取文字 */
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

  onExecute(): void {
    // Perform OCR
  }
}

/** 浏览器节点 - 控制浏览器执行操作 */
/** 浏览器通用节点（万能兜底，支持自定义 action） */
class BrowserNode extends WorkflowNode {
  static title = '浏览器（万能）'
  static type = 'browser'

  constructor(title?: string) {
    super(title || '浏览器')
    this.color = COLOR_DATA
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

// ═══════════════════════════════════════════════
// 专用浏览器节点 (v2) — 每个映射一个 Playwright 动作
// ═══════════════════════════════════════════════

/** 导航到 URL */
class BrowserNavigateNode extends WorkflowNode {
  static title = '浏览器导航'
  static type = 'browser_navigate'
  constructor(title?: string) {
    super(title || '导航')
    this.color = COLOR_DATA
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

/** 点击元素 */
class BrowserClickNode extends WorkflowNode {
  static title = '浏览器点击'
  static type = 'browser_click'
  constructor(title?: string) {
    super(title || '点击')
    this.color = COLOR_DATA
    this.addInput('selector', 'string')
    this.addOutput('data', 'object')
    this.addOutput('error', 'string')
    this.addWidget('string', 'selector', '', 'selector')
  }
  onExecute(): void {}
}

/** 填写表单 */
class BrowserFillNode extends WorkflowNode {
  static title = '浏览器填写'
  static type = 'browser_fill'
  constructor(title?: string) {
    super(title || '填写')
    this.color = COLOR_DATA
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

/** 提取页面数据 */
class BrowserExtractNode extends WorkflowNode {
  static title = '浏览器提取'
  static type = 'browser_extract'
  constructor(title?: string) {
    super(title || '提取')
    this.color = COLOR_DATA
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

/** 截图 */
class BrowserScreenshotNode extends WorkflowNode {
  static title = '浏览器截图'
  static type = 'browser_screenshot'
  constructor(title?: string) {
    super(title || '截图')
    this.color = COLOR_DATA
    this.addOutput('path', 'string')
    this.addOutput('error', 'string')
    this.addWidget('string', 'path', 'screenshot.png', 'path')
    this.addWidget('toggle', 'full_page', false, 'full_page')
  }
  onExecute(): void {}
}

/** 执行 JS */
class BrowserEvaluateNode extends WorkflowNode {
  static title = '浏览器执行JS'
  static type = 'browser_evaluate'
  constructor(title?: string) {
    super(title || '执行JS')
    this.color = COLOR_DATA
    this.addInput('script', 'string')
    this.addOutput('result', 'any')
    this.addOutput('error', 'string')
    this.addWidget('text', 'script', 'document.title', 'script')
  }
  onExecute(): void {}
}

/** 滚动页面 */
class BrowserScrollNode extends WorkflowNode {
  static title = '浏览器滚动'
  static type = 'browser_scroll'
  constructor(title?: string) {
    super(title || '滚动')
    this.color = COLOR_DATA
    this.addOutput('data', 'object')
    this.addWidget('combo', 'direction', 'bottom', null, { property: "direction",
      values: ['bottom', 'top'],
    })
    this.addWidget('number', 'times', 1, 'times', { min: 1, max: 100, step2: 1 })
    this.addWidget('number', 'delay_ms', 500, 'delay_ms', { min: 0, max: 10000, step2: 100 })
  }
  onExecute(): void {}
}

/** 等待元素 */
class BrowserWaitNode extends WorkflowNode {
  static title = '浏览器等待'
  static type = 'browser_wait'
  constructor(title?: string) {
    super(title || '等待')
    this.color = COLOR_DATA
    this.addInput('selector', 'string')
    this.addOutput('found', 'boolean')
    this.addWidget('string', 'selector', '', 'selector')
    this.addWidget('number', 'timeout_ms', 30000, 'timeout_ms', { min: 1000, max: 300000, step2: 1000 })
  }
  onExecute(): void {}
}

/** 生成 PDF */
class BrowserPdfNode extends WorkflowNode {
  static title = '浏览器PDF'
  static type = 'browser_pdf'
  constructor(title?: string) {
    super(title || '生成PDF')
    this.color = COLOR_DATA
    this.addOutput('path', 'string')
    this.addWidget('string', 'path', 'output.pdf', 'path')
  }
  onExecute(): void {}
}

/** 网页抓取节点 - 抓取网页内容 */
class WebScrapeNode extends WorkflowNode {
  static title = '网页抓取'
  static type = 'web_scrape'

  constructor(title?: string) {
    super(title || '网页抓取')
    this.color = COLOR_DATA
    this.addInput('url', 'string')
    this.addOutput('data', 'string')
    this.addWidget('string', 'url', 'https://example.com', 'url')
    this.addWidget('string', 'wait_for', 'body', 'wait_for')
    this.addWidget('number', 'delay_ms', 1000, 'delay_ms', {
      min: 0,
      max: 60000,
      step2: 100,
    })
    this.addWidget('toggle', 'scroll', false, 'scroll')
    this.addWidget('number', 'max_pages', 1, 'max_pages', {
      min: 1,
      max: 100,
      step2: 1,
    })
  }

  onExecute(): void {
    // Scrape web page
  }
}

/** 脚本节点 - 执行自定义脚本 */
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

  onExecute(): void {
    // Execute script
  }
}

/** 数据映射节点 - 对数组中每个元素执行操作 */
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

  onExecute(): void {
    // Map over items
  }
}

/** Excel 节点 - 读取/写入 Excel 文件 */
class ExcelNode extends WorkflowNode {
  static title = 'Excel'
  static type = 'excel'

  constructor(title?: string) {
    super(title || 'Excel')
    this.color = COLOR_DATA
    this.addInput('file', 'string')
    this.addOutput('data', 'object')
    this.addWidget('combo', 'action', 'read', null, { property: "action", 
      values: ['read', 'write', 'append', 'create', 'merge'],
    })
    this.addWidget('string', 'path', './input.xlsx', 'path')
    this.addWidget('string', 'sheet', 'Sheet1', 'sheet')
    this.addWidget('string', 'range', 'A1', 'range')
  }

  onExecute(): void {
    // Process Excel file
  }
}

/** Word 节点 - 读取/写入 Word 文档 */
class WordNode extends WorkflowNode {
  static title = 'Word'
  static type = 'word'

  constructor(title?: string) {
    super(title || 'Word')
    this.color = COLOR_DATA
    this.addInput('file', 'string')
    this.addOutput('data', 'string')
    this.addWidget('combo', 'action', 'read', null, { property: "action", 
      values: ['read', 'write', 'create', 'convert', 'merge'],
    })
    this.addWidget('string', 'path', './input.docx', 'path')
    this.addWidget('combo', 'format', 'text', null, { property: "format", 
      values: ['text', 'html', 'markdown', 'docx'],
    })
  }

  onExecute(): void {
    // Process Word document
  }
}

// ═══════════════════════════════════════════
// Registry: register all node types
// ═══════════════════════════════════════════

export function registerAllNodes(): void {
  // Source / Data nodes
  LiteGraph.registerNodeType('http', HttpNode)
  LiteGraph.registerNodeType('file', FileNode)
  LiteGraph.registerNodeType('clipboard', ClipboardNode)
  LiteGraph.registerNodeType('json_parse', JsonParseNode)
  LiteGraph.registerNodeType('regex', RegexNode)
  LiteGraph.registerNodeType('array', ArrayNode)
  LiteGraph.registerNodeType('convert', ConvertNode)
  LiteGraph.registerNodeType('text_template', TextTemplateNode)
  LiteGraph.registerNodeType('data', DataNode)

  // Control flow nodes
  LiteGraph.registerNodeType('delay', DelayNode)
  LiteGraph.registerNodeType('loop', LoopNode)
  LiteGraph.registerNodeType('while', WhileNode)
  LiteGraph.registerNodeType('condition', ConditionNode)
  LiteGraph.registerNodeType('sub_workflow', SubWorkflowNode)
  LiteGraph.registerNodeType('approval', ApprovalNode)
  LiteGraph.registerNodeType('parallel', ParallelNode)

  // AI nodes
  LiteGraph.registerNodeType('ai', AiNode)
  LiteGraph.registerNodeType('ai_translate', AiTranslateNode)
  LiteGraph.registerNodeType('ai_summarize', AiSummarizeNode)
  LiteGraph.registerNodeType('ai_classify', AiClassifyNode)
  LiteGraph.registerNodeType('ai_sentiment', AiSentimentNode)
  LiteGraph.registerNodeType('ai_entities', AiEntitiesNode)

  // Output nodes
  LiteGraph.registerNodeType('file_save', FileSaveNode)
  LiteGraph.registerNodeType('print', PrintNode)
  LiteGraph.registerNodeType('notify', NotifyNode)

  // Automation / Desktop nodes
  LiteGraph.registerNodeType('mouse_keyboard', MouseKeyboardNode)
  LiteGraph.registerNodeType('window', WindowNode)
  LiteGraph.registerNodeType('recording', RecordingNode)
  LiteGraph.registerNodeType('ocr', OcrNode)
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
  LiteGraph.registerNodeType('web_scrape', WebScrapeNode)
  LiteGraph.registerNodeType('script', ScriptNode)
  LiteGraph.registerNodeType('map', MapNode)
  LiteGraph.registerNodeType('excel', ExcelNode)
  LiteGraph.registerNodeType('word', WordNode)
}

// Export all node classes for direct usage
export {
  WorkflowNode,
  HttpNode,
  FileNode,
  ClipboardNode,
  JsonParseNode,
  RegexNode,
  ArrayNode,
  ConvertNode,
  TextTemplateNode,
  DataNode,
  DelayNode,
  LoopNode,
  WhileNode,
  ConditionNode,
  SubWorkflowNode,
  AiNode,
  AiTranslateNode,
  AiSummarizeNode,
  AiClassifyNode,
  AiSentimentNode,
  AiEntitiesNode,
  FileSaveNode,
  PrintNode,
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
  NotifyNode,
  ApprovalNode,
  ScriptNode,
  ParallelNode,
  MapNode,
  WebScrapeNode,
  ExcelNode,
  WordNode,
}
