import { ref, watch } from 'vue'

export type Theme = 'light' | 'dark' | 'system'

const theme = ref<Theme>('system')
const resolvedTheme = ref<'light' | 'dark'>('dark')

function getSystemTheme(): 'light' | 'dark' {
  if (typeof window === 'undefined') return 'dark'
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

function applyTheme(t: Theme) {
  const resolved = t === 'system' ? getSystemTheme() : t
  resolvedTheme.value = resolved

  const root = document.documentElement
  if (resolved === 'dark') {
    root.classList.add('dark')
  } else {
    root.classList.remove('dark')
  }
}

export function useTheme() {
  // Load saved theme
  const saved = localStorage.getItem('theme') as Theme | null
  if (saved && ['light', 'dark', 'system'].includes(saved)) {
    theme.value = saved
  }
  applyTheme(theme.value)

  // Listen to system theme changes (保存引用以便清理)
  const mediaQuery = typeof window !== 'undefined'
    ? window.matchMedia('(prefers-color-scheme: dark)')
    : null
  const onSystemThemeChange = () => {
    if (theme.value === 'system') {
      applyTheme('system')
    }
  }
  mediaQuery?.addEventListener('change', onSystemThemeChange)

  watch(theme, (newTheme) => {
    localStorage.setItem('theme', newTheme)
    applyTheme(newTheme)
  })

  function setTheme(t: Theme) {
    theme.value = t
  }

  return {
    theme,
    resolvedTheme,
    setTheme,
  }
}
