import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { checkUpdate, installUpdate, onUpdaterEvent, type UpdateManifest } from '@tauri-apps/api/updater'

export type UpdateStatus = 'idle' | 'checking' | 'available' | 'up-to-date' | 'installing' | 'error'

const LAST_CHECK_KEY = 'updater-last-check-at'
const SKIPPED_VERSION_KEY = 'updater-skipped-version'
const AUTO_CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000

const isTauriRuntime = () => typeof window !== 'undefined' && '__TAURI_IPC__' in window

export const useUpdateStore = defineStore('update', () => {
  const status = ref<UpdateStatus>('idle')
  const manifest = ref<UpdateManifest | null>(null)
  const errorMessage = ref('')
  const dialogVisible = ref(false)
  let autoCheckTimer: ReturnType<typeof setTimeout> | null = null
  let unlistenUpdater: (() => void) | null = null

  const busy = computed(() => status.value === 'checking' || status.value === 'installing')
  const availableVersion = computed(() => manifest.value?.version || '')

  const initialize = async () => {
    if (!isTauriRuntime() || unlistenUpdater) return
    try {
      unlistenUpdater = await onUpdaterEvent(({ status: eventStatus, error }) => {
        if (eventStatus === 'ERROR') {
          status.value = 'error'
          errorMessage.value = error || '更新安装失败，请稍后重试。'
        }
      })
    } catch (error) {
      console.warn('Updater event listener is unavailable:', error)
    }
  }

  const checkForUpdates = async (manual = false) => {
    if (busy.value) return
    errorMessage.value = ''
    manifest.value = null
    status.value = 'checking'
    if (manual) dialogVisible.value = true

    if (!isTauriRuntime()) {
      status.value = 'error'
      errorMessage.value = '只能在已安装的桌面应用中检查更新。'
      return
    }

    try {
      const result = await checkUpdate()
      if (result.shouldUpdate && result.manifest) {
        manifest.value = result.manifest
        status.value = 'available'
        const skippedVersion = localStorage.getItem(SKIPPED_VERSION_KEY)
        if (manual || skippedVersion !== result.manifest.version) dialogVisible.value = true
      } else {
        status.value = 'up-to-date'
      }
    } catch (error) {
      status.value = 'error'
      errorMessage.value = error instanceof Error ? error.message : String(error)
      if (!manual) console.warn('Automatic update check failed:', error)
    }
  }

  const scheduleAutoCheck = (enabled: boolean) => {
    if (autoCheckTimer) clearTimeout(autoCheckTimer)
    autoCheckTimer = null
    if (!enabled || !isTauriRuntime()) return

    const lastCheck = Number(localStorage.getItem(LAST_CHECK_KEY) || 0)
    if (Number.isFinite(lastCheck) && Date.now() - lastCheck < AUTO_CHECK_INTERVAL_MS) return

    autoCheckTimer = setTimeout(() => {
      localStorage.setItem(LAST_CHECK_KEY, String(Date.now()))
      void checkForUpdates(false)
    }, 2500)
  }

  const install = async (activeTaskCount: number) => {
    if (activeTaskCount > 0) {
      status.value = 'error'
      errorMessage.value = `还有 ${activeTaskCount} 个任务正在运行，请等待任务结束后再更新。`
      dialogVisible.value = true
      return
    }
    if (!manifest.value || status.value === 'installing') return

    status.value = 'installing'
    errorMessage.value = ''
    try {
      await installUpdate()
    } catch (error) {
      status.value = 'error'
      errorMessage.value = error instanceof Error ? error.message : String(error)
    }
  }

  const remindLater = () => {
    dialogVisible.value = false
    if (status.value !== 'installing') status.value = 'idle'
  }

  const skipCurrentVersion = () => {
    if (manifest.value?.version) localStorage.setItem(SKIPPED_VERSION_KEY, manifest.value.version)
    remindLater()
  }

  const cleanup = () => {
    if (autoCheckTimer) clearTimeout(autoCheckTimer)
    autoCheckTimer = null
    unlistenUpdater?.()
    unlistenUpdater = null
  }

  return {
    status, manifest, errorMessage, dialogVisible, busy, availableVersion,
    initialize, checkForUpdates, scheduleAutoCheck, install,
    remindLater, skipCurrentVersion, cleanup,
  }
})
