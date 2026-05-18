import { createI18n } from 'vue-i18n'
import zhCN from './locales/zh-CN'
import enUS from './locales/en-US'

export type Locale = 'zh-CN' | 'en-US'

const STORAGE_KEY = 'wf-engine-locale'

function detectLocale(): Locale {
  const stored = localStorage.getItem(STORAGE_KEY)
  if (stored === 'en-US' || stored === 'zh-CN') return stored
  // Detect from browser
  const nav = navigator.language
  if (nav.startsWith('zh')) return 'zh-CN'
  return 'en-US'
}

export function getStoredLocale(): Locale {
  return detectLocale()
}

export function setStoredLocale(locale: Locale) {
  localStorage.setItem(STORAGE_KEY, locale)
}

export const i18n = createI18n({
  legacy: false,
  locale: detectLocale(),
  fallbackLocale: 'zh-CN',
  messages: {
    'zh-CN': zhCN,
    'en-US': enUS,
  },
})

export default i18n
