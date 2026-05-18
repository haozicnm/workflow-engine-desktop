import { type ClassValue, clsx } from 'clsx'
import { twMerge } from 'tailwind-merge'

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/** Deep merge for nested params — recursively merges source into target */
export function deepMerge(target: Record<string, unknown>, source: Record<string, unknown>): Record<string, unknown> {
  const result = { ...target }
  for (const key of Object.keys(source)) {
    if (
      result[key] && typeof result[key] === 'object' && !Array.isArray(result[key]) &&
      source[key] && typeof source[key] === 'object' && !Array.isArray(source[key])
    ) {
      result[key] = deepMerge(result[key] as Record<string, unknown>, source[key] as Record<string, unknown>)
    } else {
      result[key] = source[key]
    }
  }
  return result
}
