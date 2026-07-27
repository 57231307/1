/**
 * V15 P1-20-16 暗黑模式切换 composable
 *
 * 使用方式：
 * import { useTheme } from '@/composables/useTheme'
 * const { isDark, toggleTheme, setTheme } = useTheme()
 */
import { ref, computed, onMounted } from 'vue'

type ThemeMode = 'light' | 'dark'

const STORAGE_KEY = 'bx-erp-theme'
const DARK_CLASS = 'dark'

const currentMode = ref<ThemeMode>('light')

function applyTheme(mode: ThemeMode) {
  const html = document.documentElement
  if (mode === 'dark') {
    html.classList.add(DARK_CLASS)
    html.setAttribute('data-theme', 'dark')
  } else {
    html.classList.remove(DARK_CLASS)
    html.setAttribute('data-theme', 'light')
  }
}

function loadStoredTheme(): ThemeMode {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored === 'dark' || stored === 'light') {
      return stored
    }
  } catch {
    // localStorage 不可用时降级
  }
  // 默认跟随系统偏好
  if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
    return 'dark'
  }
  return 'light'
}

export function useTheme() {
  const isDark = computed(() => currentMode.value === 'dark')

  const setTheme = (mode: ThemeMode) => {
    currentMode.value = mode
    applyTheme(mode)
    try {
      localStorage.setItem(STORAGE_KEY, mode)
    } catch {
      // 忽略存储错误
    }
  }

  const toggleTheme = () => {
    setTheme(currentMode.value === 'dark' ? 'light' : 'dark')
  }

  // 监听系统主题变化（用户未手动设置时跟随）
  const watchSystemTheme = () => {
    if (!window.matchMedia) return
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
    const handler = (e: MediaQueryListEvent) => {
      // 仅在用户未手动设置时跟随系统
      const stored = localStorage.getItem(STORAGE_KEY)
      if (!stored) {
        setTheme(e.matches ? 'dark' : 'light')
      }
    }
    mediaQuery.addEventListener('change', handler)
  }

  onMounted(() => {
    currentMode.value = loadStoredTheme()
    applyTheme(currentMode.value)
    watchSystemTheme()
  })

  return {
    isDark,
    currentMode: computed(() => currentMode.value),
    setTheme,
    toggleTheme,
  }
}
