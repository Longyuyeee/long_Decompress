/**
 * 可访问性增强组合式函数
 * 提供字体大小、高对比度、色盲模式等功能
 */

import { watch, onMounted, onUnmounted } from 'vue'
import { useAppStore } from '@/stores/app'

export function useAccessibility() {
  const appStore = useAppStore()

  // 应用字体大小
  const applyFontSize = (size: 'normal' | 'large' | 'x-large') => {
    const root = document.documentElement

    switch (size) {
      case 'large':
        root.style.fontSize = '112.5%' // 18px base (was 16px)
        break
      case 'x-large':
        root.style.fontSize = '125%' // 20px base (was 16px)
        break
      default:
        root.style.fontSize = '100%' // 16px base
    }
  }

  // 应用高对比度模式
  const applyHighContrast = (enabled: boolean) => {
    const root = document.documentElement

    if (enabled) {
      root.classList.add('high-contrast')
    } else {
      root.classList.remove('high-contrast')
    }
  }

  // 应用色盲模式
  const applyColorBlindMode = (mode: 'none' | 'protanopia' | 'deuteranopia' | 'tritanopia') => {
    const root = document.documentElement

    // 移除所有色盲模式类
    root.classList.remove('protanopia', 'deuteranopia', 'tritanopia')

    if (mode !== 'none') {
      root.classList.add(mode)
    }
  }

  // 应用减少动画
  const applyReduceMotion = (enabled: boolean) => {
    const root = document.documentElement

    if (enabled) {
      root.classList.add('reduce-motion')
    } else {
      root.classList.remove('reduce-motion')
    }
  }

  // 应用焦点指示器
  const applyFocusIndicator = (enabled: boolean) => {
    const root = document.documentElement

    if (enabled) {
      root.classList.add('enhanced-focus')
    } else {
      root.classList.remove('enhanced-focus')
    }
  }

  // 初始化所有可访问性设置
  const initAccessibility = () => {
    const accessibility = appStore.settings.accessibility || {
      fontSize: 'normal',
      highContrast: false,
      colorBlindMode: 'none',
      reduceMotion: false,
      focusIndicator: true,
    }

    applyFontSize(accessibility.fontSize)
    applyHighContrast(accessibility.highContrast)
    applyColorBlindMode(accessibility.colorBlindMode)
    applyReduceMotion(accessibility.reduceMotion)
    applyFocusIndicator(accessibility.focusIndicator)
  }

  // 监听系统偏好设置
  const watchSystemPreferences = () => {
    // 检测系统减少动画偏好
    const motionQuery = window.matchMedia('(prefers-reduced-motion: reduce)')
    const handleMotionChange = (e: MediaQueryListEvent | MediaQueryList) => {
      if (e.matches && !appStore.settings.accessibility?.reduceMotion) {
        // 系统启用了减少动画，建议用户启用
        console.log('System prefers reduced motion')
      }
    }

    motionQuery.addEventListener('change', handleMotionChange)
    handleMotionChange(motionQuery)

    // 检测系统高对比度偏好
    const contrastQuery = window.matchMedia('(prefers-contrast: high)')
    const handleContrastChange = (e: MediaQueryListEvent | MediaQueryList) => {
      if (e.matches && !appStore.settings.accessibility?.highContrast) {
        console.log('System prefers high contrast')
      }
    }

    contrastQuery.addEventListener('change', handleContrastChange)
    handleContrastChange(contrastQuery)

    return () => {
      motionQuery.removeEventListener('change', handleMotionChange)
      contrastQuery.removeEventListener('change', handleContrastChange)
    }
  }

  // 监听设置变化
  const setupWatchers = () => {
    watch(() => appStore.settings.accessibility?.fontSize, (val) => {
      if (val) applyFontSize(val)
    })
    watch(() => appStore.settings.accessibility?.highContrast, (val) => {
      if (val !== undefined) applyHighContrast(val)
    })
    watch(() => appStore.settings.accessibility?.colorBlindMode, (val) => {
      if (val) applyColorBlindMode(val)
    })
    watch(() => appStore.settings.accessibility?.reduceMotion, (val) => {
      if (val !== undefined) applyReduceMotion(val)
    })
    watch(() => appStore.settings.accessibility?.focusIndicator, (val) => {
      if (val !== undefined) applyFocusIndicator(val)
    })
  }

  return {
    initAccessibility,
    watchSystemPreferences,
    setupWatchers,
    applyFontSize,
    applyHighContrast,
    applyColorBlindMode,
    applyReduceMotion,
    applyFocusIndicator,
  }
}
