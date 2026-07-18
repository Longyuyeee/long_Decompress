<template>
  <div id="app" @mousemove="resetIdleTimer">
    <MainLayout />
    <ToastContainer />
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import MainLayout from '@/components/layouts/MainLayout.vue'
import ToastContainer from '@/components/ui/ToastContainer.vue'
import { useConfigStore } from '@/stores/config'
import { usePasswordStore } from '@/stores/password'
import { useTaskStore } from '@/stores/task'
import { useCompressionStore } from '@/stores/compression'
import { useAppStore } from '@/stores/app'
import { useUIStore } from '@/stores/ui'
import { useAccessibility } from '@/composables/useAccessibility'
import { appWindow, LogicalPosition, LogicalSize } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'

const router = useRouter()
const configStore = useConfigStore()
const passwordStore = usePasswordStore()
const taskStore = useTaskStore()
const compressionStore = useCompressionStore()
const appStore = useAppStore()
const uiStore = useUIStore()
const { initAccessibility, setupWatchers, watchSystemPreferences } = useAccessibility()

let idleTimer: any = null
let saveWindowTimer: any = null
let unlistenResize: any = null
let cleanupSystemWatcher: (() => void) | null = null
const appUnlisteners: Array<() => void> = []
let isUnmounted = false

watch(() => appStore.error, message => {
  if (message) uiStore.showToast('error', message, 5000)
})

watch(() => appStore.successMessage, message => {
  if (message) uiStore.showToast('success', message)
})

const keepAppListener = (unlisten: () => void) => {
  if (isUnmounted) unlisten()
  else appUnlisteners.push(unlisten)
}

const WINDOW_STATE_KEY = 'window-state'

const saveWindowState = async () => {
  try {
    const pos = await appWindow.outerPosition()
    const size = await appWindow.outerSize()
    localStorage.setItem(WINDOW_STATE_KEY, JSON.stringify({
      x: pos.x, y: pos.y,
      width: size.width, height: size.height
    }))
  } catch { /* ignore if window not available */ }
}

const restoreWindowState = async () => {
  try {
    const saved = localStorage.getItem(WINDOW_STATE_KEY)
    if (!saved) return
    const { x, y, width, height } = JSON.parse(saved)

    // 严格的屏幕边界验证：确保窗口至少有 100px 可见区域在主屏幕内
    // 获取主屏幕尺寸（简化假设：1920x1080，实际可用 screen API）
    const screenWidth = window.screen.availWidth
    const screenHeight = window.screen.availHeight

    const isValid =
      x >= -100 && y >= -100 &&  // 左上角不能完全超出屏幕
      x < screenWidth && y < screenHeight &&  // 至少有部分可见
      width >= 720 && width <= screenWidth + 100 &&
      height >= 540 && height <= screenHeight + 100

    if (isValid) {
      await appWindow.setPosition(new LogicalPosition(x, y))
      await appWindow.setSize(new LogicalSize(width, height))
    } else {
      // 无效坐标时清除保存的状态，使用默认居中
      localStorage.removeItem(WINDOW_STATE_KEY)
    }
  } catch {
    // 出错时清除可能损坏的状态
    localStorage.removeItem(WINDOW_STATE_KEY)
  }
}

const handleKeydown = (e: KeyboardEvent) => {
  resetIdleTimer()
  const target = e.target as HTMLElement | null
  const isEditable = target?.matches('input, textarea, select, [contenteditable="true"]') ?? false
  if (isEditable) return
  // 全局快捷键
  if (e.ctrlKey || e.metaKey) {
    switch (e.key.toLowerCase()) {
      case 'o': e.preventDefault(); router.push('/decompress'); break  // Ctrl+O → 解压
      case 'n': e.preventDefault(); router.push('/compress'); break   // Ctrl+N → 压缩
      case 'v': if (e.shiftKey) { e.preventDefault(); router.push('/vault') }; break
      case 'i': e.preventDefault(); router.push('/integrity'); break
      case ',': e.preventDefault(); router.push('/settings'); break  // Ctrl+, → 设置
    }
  }
  if (e.key === 'Escape') {
    // Esc 关闭当前聚焦的弹窗由各组件自行处理
  }
}

const resetIdleTimer = () => {
  if (idleTimer) clearTimeout(idleTimer)
  const lockTimeStr = configStore.configs['security.auto_lock']
  if (!lockTimeStr || lockTimeStr === '0') return
  const lockTimeMs = parseInt(lockTimeStr) * 60 * 1000
  if (isNaN(lockTimeMs) || lockTimeMs <= 0) return

  idleTimer = window.setTimeout(() => {
    if (passwordStore.isUnlocked) {
      passwordStore.lock()
    }
  }, lockTimeMs)
}

onMounted(async () => {
  window.addEventListener('keydown', handleKeydown)
  // 初始化可访问性设置
  initAccessibility()
  setupWatchers()
  cleanupSystemWatcher = watchSystemPreferences()
  try {
    await taskStore.initListeners()
  } catch (error) {
    console.warn('Task listeners are unavailable:', error)
  }

  const registerCompressionAction = async (
    eventName: string,
    format?: 'zip' | '7z',
    autoStart = false
  ) => {
    try {
      const unlisten = await listen<string[]>(eventName, (event) => {
        const files = event.payload.filter(file => file && !file.startsWith('%'))
        if (files.length === 0) return

        files.forEach(path => {
          const name = path.split(/[\\/]/).pop() || path
          compressionStore.addFile({ name, path, size: 0, type: 'file', isDirectory: false })
        })
        if (format) compressionStore.globalSettings.format = format
        if (autoStart) compressionStore.requestAutoStart()
        void router.push('/compress')
        appStore.setSuccess(appStore.t('compress.context_menu_custom').replace('{0}', String(files.length)))
      })
      keepAppListener(unlisten)
    } catch (error) {
      console.warn(`Listener ${eventName} is unavailable:`, error)
    }
  }

  await registerCompressionAction('context-compress-custom')
  await registerCompressionAction('context-compress-zip', 'zip', true)
  await registerCompressionAction('context-compress-7z', '7z', true)

  resetIdleTimer()
  await restoreWindowState()
  // 请求浏览器通知权限（后台任务完成时通知用户）
  // 使用 Tauri 原生窗口 resize 事件（避免 zoom 触发的 resize 循环）
  try {
    const unlisten = await appWindow.onResized(() => {
      if (saveWindowTimer) clearTimeout(saveWindowTimer)
      saveWindowTimer = setTimeout(saveWindowState, 1000)
    })
    if (isUnmounted) unlisten()
    else unlistenResize = unlisten
  } catch (error) {
    console.warn('Window resize listener is unavailable:', error)
  }
})

onUnmounted(() => {
  isUnmounted = true
  window.removeEventListener('keydown', handleKeydown)
  if (idleTimer) clearTimeout(idleTimer)
  if (saveWindowTimer) clearTimeout(saveWindowTimer)
  if (unlistenResize) unlistenResize()
  if (cleanupSystemWatcher) cleanupSystemWatcher()
  appUnlisteners.forEach(unlisten => unlisten())
  saveWindowState()
})
</script>

<style>
#app {
  font-family: 'Plus Jakarta Sans', system-ui, sans-serif;
  -webkit-font-smoothing: antialiased;
  transform-origin: top left;
}
</style>
