import { ref } from 'vue'

/**
 * TRAE Work: Light-mode only.
 * Theme toggle is retained for API compatibility but always resolves to 'light'.
 */
export type Theme = 'light'

const theme = ref<Theme>('light')
const resolvedTheme = ref<'light'>('light')

export function useTheme() {
  function setTheme(_t: Theme) {
    theme.value = 'light'
    resolvedTheme.value = 'light'
    localStorage.setItem('theme', 'light')
  }

  return {
    theme,
    resolvedTheme,
    setTheme,
  }
}
