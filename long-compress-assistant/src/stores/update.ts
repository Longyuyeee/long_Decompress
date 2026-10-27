import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { onUpdaterEvent, type UpdateManifest } from '@tauri-apps/api/updater'
import { invoke } from '@tauri-apps/api/tauri'
import { listen, TauriEvent } from '@tauri-apps/api/event'

export type UpdateStatus = 'idle' | 'checking' | 'available' | 'up-to-date' | 'installing' | 'error'

const LEGACY_LAST_CHECK_KEY = 'updater-last-check-at'
const LAST_ATTEMPT_KEY = 'updater-last-attempt-at'
const LAST_SUCCESS_KEY = 'updater-last-success-at'
const SKIPPED_VERSION_KEY = 'updater-skipped-version'
const AUTO_CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000

const isTauriRuntime = () => typeof window !== 'undefined' && '__TAURI_IPC__' in window
const readStoredTimestamp = (...keys: string[]) => {
  for (const key of keys) {
    const value = Number(localStorage.getItem(key) || 0)
    if (Number.isFinite(value) && value > 0) return value
  }
  return null
}

export const useUpdateStore = defineStore('update', () => {
  const status = ref<UpdateStatus>('idle')
  const manifest = ref<UpdateManifest | null>(null)
  const errorMessage = ref('')
  const dialogVisible = ref(false)
  const installPhase = ref<'preparing' | 'downloading' | 'installing' | 'restarting'>('preparing')
  const downloadedBytes = ref(0)
  const totalBytes = ref<number | null>(null)
  const lastAttemptAt = ref<number | null>(readStoredTimestamp(LAST_ATTEMPT_KEY, LEGACY_LAST_CHECK_KEY))
  const lastSuccessAt = ref<number | null>(readStoredTimestamp(LAST_SUCCESS_KEY))
  let autoCheckTimer: ReturnType<typeof setTimeout> | null = null
  let unlistenUpdater: (() => void) | null = null
  let unlistenProgress: (() => void) | null = null
  let initializing: Promise<void> | null = null
  let listenerGeneration = 0
  let unreliableLength = false

  const busy = computed(() => status.value === 'checking' || status.value === 'installing')
  const availableVersion = computed(() => manifest.value?.version || '')
  const downloadPercent = computed(() => totalBytes.value
    ? Math.min(100, Math.floor(downloadedBytes.value / totalBytes.value * 100)) : null)
  const resetProgress = () => {
    installPhase.value = 'preparing'
    downloadedBytes.value = 0
    totalBytes.value = null
    unreliableLength = false
  }

  const initialize = async () => {
    if (!isTauriRuntime() || unlistenUpdater) return
    if (initializing) return initializing
    const generation = listenerGeneration
    initializing = (async () => {
      const registered: Array<() => void> = []
      try {
        const stopStatus = await onUpdaterEvent(({ status: eventStatus, error }) => {
          if (generation !== listenerGeneration) return
          const phase: string = eventStatus // Tauri 1.8 also emits DOWNLOADED, absent in v1 JS typings.
          if (phase === 'ERROR' && busy.value) {
            status.value = 'error'
            errorMessage.value = error || '更新安装失败，请稍后重试。'
          } else if (status.value === 'installing' && phase === 'DOWNLOADED') {
            installPhase.value = 'installing'
          } else if (status.value === 'installing' && phase === 'DONE') {
            installPhase.value = 'restarting'
          }
        })
        registered.push(stopStatus)
        const stopProgress = await listen<{ chunkLength: number; contentLength?: number | null }>(TauriEvent.DOWNLOAD_PROGRESS, ({ payload }) => {
          if (generation !== listenerGeneration) return
          if (status.value !== 'installing' || !['preparing', 'downloading'].includes(installPhase.value)) return
          installPhase.value = 'downloading'
          if (!unreliableLength && Number.isSafeInteger(payload.contentLength) && payload.contentLength! > 0) totalBytes.value = payload.contentLength!
          if (Number.isSafeInteger(payload.chunkLength) && payload.chunkLength > 0 && Number.isSafeInteger(downloadedBytes.value + payload.chunkLength)) downloadedBytes.value += payload.chunkLength
          if (totalBytes.value !== null && downloadedBytes.value > totalBytes.value) {
            // An inconsistent server length must not display an impossible fraction
            // or a false 100%; retain bytes received and use indeterminate progress.
            unreliableLength = true
            totalBytes.value = null
          }
        })
        registered.push(stopProgress)
        if (generation !== listenerGeneration) { registered.forEach(stop => stop()); return }
        unlistenUpdater = stopStatus
        unlistenProgress = stopProgress
      } catch (error) {
        registered.forEach(stop => stop())
        console.warn('Updater event listener is unavailable:', error)
      }
    })().finally(() => { initializing = null })
    return initializing
  }

  const checkForUpdates = async (manual = false) => {
    if (busy.value) return
    lastAttemptAt.value = Date.now()
    localStorage.setItem(LAST_ATTEMPT_KEY, String(lastAttemptAt.value))
    errorMessage.value = ''
    resetProgress()
    manifest.value = null
    status.value = 'checking'
    if (manual) dialogVisible.value = true

    if (!isTauriRuntime()) {
      status.value = 'error'
      errorMessage.value = '只能在已安装的桌面应用中检查更新。'
      return
    }

    try {
      const result = await invoke<{shouldUpdate:boolean;manifest?:UpdateManifest}>('check_app_update')
      lastSuccessAt.value = Date.now()
      localStorage.setItem(LAST_SUCCESS_KEY, String(lastSuccessAt.value))
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

    if (lastSuccessAt.value && Date.now() - lastSuccessAt.value < AUTO_CHECK_INTERVAL_MS) return

    autoCheckTimer = setTimeout(() => {
      void checkForUpdates(false)
    }, 2500)
  }

  const install = async (activeTaskCount: number) => {
    if (!manifest.value || status.value === 'installing') return
    if (activeTaskCount > 0) {
      status.value = 'error'
      errorMessage.value = `还有 ${activeTaskCount} 个任务正在运行，请等待任务结束后再更新。`
      dialogVisible.value = true
      return
    }

    status.value = 'installing'
    errorMessage.value = ''
    dialogVisible.value = true
    resetProgress()
    try {
      // Register before triggering the native download, including manual updates.
      await initialize()
      await invoke('install_app_update', {expectedVersion:manifest.value.version})
      if (status.value === 'installing') installPhase.value = 'restarting'
    } catch (error) {
      status.value = 'error'
      errorMessage.value = error instanceof Error ? error.message : String(error)
    }
  }

  const remindLater = () => {
    if (status.value === 'installing') return
    dialogVisible.value = false
    status.value = 'idle'
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
    unlistenProgress?.()
    unlistenProgress = null
    listenerGeneration++
  }

  return {
    status, manifest, errorMessage, dialogVisible, busy, availableVersion,
    lastAttemptAt, lastSuccessAt,
    installPhase, downloadedBytes, totalBytes, downloadPercent,
    initialize, checkForUpdates, scheduleAutoCheck, install,
    remindLater, skipCurrentVersion, cleanup,
  }
})
