<template>
  <div id="app" @mousemove="resetIdleTimer" @keydown="resetIdleTimer">
    <MainLayout />
    <ToastContainer />
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import MainLayout from '@/components/layouts/MainLayout.vue'
import ToastContainer from '@/components/ui/ToastContainer.vue'
import { useConfigStore } from '@/stores/config'
import { usePasswordStore } from '@/stores/password'
import { appWindow } from '@tauri-apps/api/window'

const configStore = useConfigStore()
const passwordStore = usePasswordStore()

let idleTimer: any = null
let saveWindowTimer: any = null

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
    // 仅在坐标有意义时恢复（避免多显示器场景下窗口跑到屏幕外）
    if (x > -1000 && y > -1000 && width >= 720 && height >= 540) {
      await appWindow.setPosition({ x, y })
      await appWindow.setSize({ width, height })
    }
  } catch { /* ignore on first launch */ }
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
  resetIdleTimer()
  await restoreWindowState()
  // 请求浏览器通知权限（后台任务完成时通知用户）
  if ('Notification' in window && Notification.permission === 'default') {
    try { await Notification.requestPermission() } catch { /* ignore */ }
  }
  // 延迟保存窗口状态（debounce）
  window.addEventListener('resize', () => {
    if (saveWindowTimer) clearTimeout(saveWindowTimer)
    saveWindowTimer = setTimeout(saveWindowState, 1000)
  })
})

onUnmounted(() => {
  if (idleTimer) clearTimeout(idleTimer)
  if (saveWindowTimer) clearTimeout(saveWindowTimer)
  saveWindowState()
})
</script>

<style>
#app {
  font-family: 'Plus Jakarta Sans', system-ui, sans-serif;
  -webkit-font-smoothing: antialiased;
}
</style>
