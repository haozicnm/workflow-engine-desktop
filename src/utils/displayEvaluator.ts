/**
 * displayEvaluator.ts — 声明式条件显示引擎（参考 n8n displayOptions）
 *
 * 核心逻辑：
 *   - show: 所有条件必须同时满足（AND 逻辑）
 *   - hide: 任一条件满足即隐藏（OR 逻辑）
 *   - 12 种条件运算符（eq/not/gt/lt/gte/lte/between/startsWith/endsWith/includes/regex/exists）
 */

import type { DisplayOptions, ConditionValue } from '../types/types'

/**
 * 评估单个条件值是否匹配当前参数值
 */
function evaluateCondition(currentValue: unknown, condition: ConditionValue): boolean {
  // 简单值匹配：直接比较
  if (condition === null || condition === undefined) {
    return currentValue === condition
  }

  // 高级条件运算：{ _cnd: { op, value } }
  if (typeof condition === 'object' && '_cnd' in condition) {
    const cnd = condition._cnd as { op: string; value?: any }
    const val = currentValue as any

    switch (cnd.op) {
      case 'eq':
        return val === cnd.value
      case 'not':
        return val !== cnd.value
      case 'gte':
        return typeof val === 'number' && val >= cnd.value
      case 'lte':
        return typeof val === 'number' && val <= cnd.value
      case 'gt':
        return typeof val === 'number' && val > cnd.value
      case 'lt':
        return typeof val === 'number' && val < cnd.value
      case 'between':
        return typeof val === 'number' && val >= cnd.value.from && val <= cnd.value.to
      case 'startsWith':
        return typeof val === 'string' && val.startsWith(cnd.value)
      case 'endsWith':
        return typeof val === 'string' && val.endsWith(cnd.value)
      case 'includes':
        return typeof val === 'string' && val.includes(cnd.value)
      case 'regex':
        try {
          return typeof val === 'string' && new RegExp(cnd.value).test(val)
        } catch {
          return false
        }
      case 'exists':
        return val !== undefined && val !== null
      default:
        return true
    }
  }

  // 简单值匹配（数组中任一匹配即满足）
  if (Array.isArray(condition)) {
    return condition.some(c => {
      if (typeof c === 'object' && c !== null && '_cnd' in c) {
        return evaluateCondition(currentValue, c)
      }
      return currentValue === c
    })
  }

  // 单值直接比较
  return currentValue === condition
}

/**
 * 评估 displayOptions 是否应该显示该参数
 *
 * @param displayOptions 条件显示规则
 * @param siblingValues 同级参数的当前值（key-value 对）
 * @returns true=显示, false=隐藏
 */
export function shouldDisplay(
  displayOptions: DisplayOptions | undefined,
  siblingValues: Record<string, unknown>
): boolean {
  if (!displayOptions) return true

  const { show, hide } = displayOptions

  // show: 所有条件必须同时满足（AND 逻辑）
  if (show) {
    for (const [paramName, conditions] of Object.entries(show)) {
      const currentValue = siblingValues[paramName]
      const matched = conditions.some(c => evaluateCondition(currentValue, c))
      if (!matched) return false
    }
  }

  // hide: 任一条件满足即隐藏（OR 逻辑）
  if (hide) {
    for (const [paramName, conditions] of Object.entries(hide)) {
      const currentValue = siblingValues[paramName]
      const matched = conditions.some(c => evaluateCondition(currentValue, c))
      if (matched) return false
    }
  }

  return true
}
