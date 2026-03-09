import { computed } from 'vue'
import { useUIStore } from '@/stores'

/**
 * 主题切换组合式函�?
 * 提供主题切换的相关功�?
 */
export function useTheme() {
  const uiStore = useUIStore()

  // 当前主题状�?
  const isDark = computed(() => uiStore.darkMode)
  const theme = computed(() => isDark.value ? 'dark' : 'light')

  // 主题选项
  const themeOptions = [
    { value: 'light', label: '浅色', icon: 'pi pi-sun' },
    { value: 'dark', label: '深色', icon: 'pi pi-moon' },
    { value: 'auto', label: '自动', icon: 'pi pi-desktop' }
  ]

  // 切换主题
  const toggleTheme = () => {
    uiStore.toggleDarkMode()
  }

  // 设置特定主题
  const setTheme = (theme: 'light' | 'dark' | 'auto') => {
    switch (theme) {
      case 'light':
        uiStore.setDarkMode(false)
        localStorage.removeItem('dark-mode') // 清除手动设置，使用系统默�?
        break
      case 'dark':
        uiStore.setDarkMode(true)
        localStorage.removeItem('dark-mode') // 清除手动设置，使用系统默�?
        break
      case 'auto':
        // 移除手动设置，让系统自动检�?
        localStorage.removeItem('dark-mode')
        const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
        uiStore.setDarkMode(prefersDark)
        break
    }
  }

  // 获取当前主题标签
  const currentThemeLabel = computed(() => {
    const savedTheme = localStorage.getItem('dark-mode')
    if (savedTheme === null) {
      return '自动'
    }
    return isDark.value ? '深色' : '浅色'
  })

  // 获取当前主题图标
  const currentThemeIcon = computed(() => {
    const savedTheme = localStorage.getItem('dark-mode')
    if (savedTheme === null) {
      return 'pi pi-desktop'
    }
    return isDark.value ? 'pi pi-moon' : 'pi pi-sun'
  })

  // 监听系统主题变化
  const watchSystemTheme = () => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')

    const handleChange = (e: MediaQueryListEvent) => {
      const savedTheme = localStorage.getItem('dark-mode')
      if (savedTheme === null) {
        // 如果没有手动设置过主题，跟随系统
        uiStore.setDarkMode(e.matches)
      }
    }

    mediaQuery.addEventListener('change', handleChange)

    // 返回清理函数
    return () => {
      mediaQuery.removeEventListener('change', handleChange)
    }
  }

  // 初始化主�?
  const initializeTheme = () => {
    // UI store已经初始化了主题，这里可以添加额外的初始化逻辑
    watchSystemTheme()
  }

  // 主题相关样式�?
  const themeClasses = computed(() => ({
    'dark': isDark.value,
    'light': !isDark.value
  }))

  // 主题相关颜色
  const themeColors = computed(() => ({
    primary: isDark.value ? '#3b82f6' : '#2563eb',
    background: isDark.value ? '#1f2937' : '#f9fafb',
    text: isDark.value ? '#f9fafb' : '#111827',
    border: isDark.value ? '#374151' : '#e5e7eb'
  }))

  return {
    // 状�?
    isDark,
    theme,
    currentThemeLabel,
    currentThemeIcon,
    themeClasses,
    themeColors,

    // 选项
    themeOptions,

    // 方法
    toggleTheme,
    setTheme,
    watchSystemTheme,
    initializeTheme
  }
}

/**
 * 主题切换组件属性类�?
 */
export interface ThemeToggleProps {
  showLabel?: boolean
  showIcon?: boolean
  size?: 'sm' | 'md' | 'lg'
  variant?: 'icon' | 'button' | 'switch'
}

/**
 * 主题配置类型
 */
export interface ThemeConfig {
  colors: {
    primary: string
    secondary: string
    success: string
    warning: string
    danger: string
    info: string
    background: string
    foreground: string
    card: string
    border: string
  }
  fonts: {
    sans: string[]
    mono: string[]
  }
  spacing: Record<string, string>
  borderRadius: Record<string, string>
  shadows: Record<string, string>
}

/**
 * 默认主题配置
 */
export const defaultThemeConfig: ThemeConfig = {
  colors: {
    primary: '#3b82f6',
    secondary: '#6b7280',
    success: '#10b981',
    warning: '#f59e0b',
    danger: '#ef4444',
    info: '#06b6d4',
    background: 'var(--color-background)',
    foreground: 'var(--color-foreground)',
    card: 'var(--color-card)',
    border: 'var(--color-border)'
  },
  fonts: {
    sans: ['Inter', 'system-ui', '-apple-system', 'sans-serif'],
    mono: ['JetBrains Mono', 'monospace']
  },
  spacing: {
    xs: '0.25rem',
    sm: '0.5rem',
    md: '1rem',
    lg: '1.5rem',
    xl: '2rem',
    '2xl': '3rem',
    '3xl': '4rem'
  },
  borderRadius: {
    sm: '0.125rem',
    md: '0.375rem',
    lg: '0.5rem',
    xl: '0.75rem',
    '2xl': '1rem',
    full: '9999px'
  },
  shadows: {
    sm: '0 1px 2px 0 rgb(0 0 0 / 0.05)',
    md: '0 4px 6px -1px rgb(0 0 0 / 0.1)',
    lg: '0 10px 15px -3px rgb(0 0 0 / 0.1)',
    xl: '0 20px 25px -5px rgb(0 0 0 / 0.1)',
    '2xl': '0 25px 50px -12px rgb(0 0 0 / 0.25)'
  }
}

/**
 * 深色主题配置
 */
export const darkThemeConfig: ThemeConfig = {
  ...defaultThemeConfig,
  colors: {
    ...defaultThemeConfig.colors,
    background: '#1f2937',
    foreground: '#f9fafb',
    card: '#374151',
    border: '#4b5563'
  }
}

/**
 * 浅色主题配置
 */
export const lightThemeConfig: ThemeConfig = {
  ...defaultThemeConfig,
  colors: {
    ...defaultThemeConfig.colors,
    background: '#f9fafb',
    foreground: '#111827',
    card: '#ffffff',
    border: '#e5e7eb'
  }
}
