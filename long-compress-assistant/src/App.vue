<template>
  <div id="app">
    <MainLayout />
    <ToastContainer />
    <UpdateDialog />
    <Modal
      :visible="showExitConfirmation"
      :title="appStore.t('exit.confirm.title')"
      :description="appStore.t('exit.confirm.desc').replace('{0}', String(taskStore.activeTaskCount))"
      icon="pi pi-exclamation-triangle"
      size="sm"
      :show-close-button="false"
      :show-footer="true"
      :close-on-backdrop="false"
      :close-on-escape="false"
    >
      <p class="text-sm text-muted leading-relaxed">{{ appStore.t('exit.confirm.warning') }}</p>
      <template #footer>
        <button type="button" class="px-4 py-2.5 rounded-xl bg-input border border-subtle text-muted text-xs font-bold hover:text-content transition-all" @click="showExitConfirmation = false">
          {{ appStore.t('common.cancel') }}
        </button>
        <button type="button" class="px-4 py-2.5 rounded-xl bg-primary/10 border border-primary/30 text-primary text-xs font-bold hover:bg-primary/20 transition-all" @click="continueInBackground">
          {{ appStore.t('exit.confirm.background') }}
        </button>
        <button type="button" :disabled="exitInProgress" class="px-4 py-2.5 rounded-xl bg-red-500 text-white text-xs font-bold hover:bg-red-600 disabled:opacity-60 transition-all flex items-center gap-2" @click="cancelTasksAndExit">
          <i v-if="exitInProgress" class="pi pi-spin pi-spinner"></i>
          {{ appStore.t('exit.confirm.stop_and_exit') }}
        </button>
      </template>
    </Modal>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import MainLayout from '@/components/layouts/MainLayout.vue'
import ToastContainer from '@/components/ui/ToastContainer.vue'
import Modal from '@/components/ui/Modal.vue'
import UpdateDialog from '@/components/update/UpdateDialog.vue'
import { useTaskStore } from '@/stores/task'
import { useCompressionStore } from '@/stores/compression'
import { useAppStore } from '@/stores/app'
import { useUIStore } from '@/stores/ui'
import { useUpdateStore } from '@/stores/update'
import { useAccessibility } from '@/composables/useAccessibility'
import { useCompressionProfiles } from '@/composables/useCompressionProfiles'
import { installDesktopE2EBridge } from '@/testing/desktopE2EBridge'
import { appWindow, LogicalPosition, LogicalSize } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/tauri'
import { createContextCompressionEntry, createQuickPackCandidate, createQuickPackPlan, groupContextActions, type ContextAction } from '@/utils/contextActions'
import { buildSafeTemplateDraftSettings, taskTemplateCandidatesToFiles } from '@/utils/taskTemplateDraft'

const router = useRouter()
const taskStore = useTaskStore()
const compressionStore = useCompressionStore()
const appStore = useAppStore()
const uiStore = useUIStore()
const updateStore = useUpdateStore()
const taskTemplates = useCompressionProfiles()
const { initAccessibility, setupWatchers, watchSystemPreferences } = useAccessibility()

let saveWindowTimer: any = null
let unlistenResize: any = null
let cleanupSystemWatcher: (() => void) | null = null
let cleanupDesktopE2EBridge: (() => void) | null = null
let contextDrainTimer: ReturnType<typeof setTimeout> | null = null
let contextDrainPromise: Promise<void> | null = null
let contextDrainAgain = false
let watchBatchTimer: ReturnType<typeof setInterval> | null = null
let watchBatchDrainPromise: Promise<void> | null = null
const appUnlisteners: Array<() => void> = []
let isUnmounted = false
const showExitConfirmation = ref(false)
const exitInProgress = ref(false)

const continueInBackground = async () => {
  showExitConfirmation.value = false
  await appWindow.hide()
}

const cancelTasksAndExit = async () => {
  if (exitInProgress.value) return
  exitInProgress.value = true
  const unfinished = taskStore.tasks.filter(
    task => !['completed', 'failed', 'cancelled'].includes(task.status)
  )
  try {
    await invoke('cancel_tasks_and_wait', { taskIds: unfinished.map(task => task.id) })
    unfinished.forEach(task => taskStore.updateTaskStatus(task.id, 'cancelled'))
    await invoke('exit_app')
  } catch (error) {
    exitInProgress.value = false
    appStore.setError(String(error))
  }
}

watch(() => appStore.error, message => {
  if (message) uiStore.showToast('error', message, 5000)
})

watch(() => appStore.successMessage, message => {
  if (message) uiStore.showToast('success', message)
})

watch(
  () => taskStore.activeTaskCount,
  count => { void invoke('set_has_active_tasks', { active: count > 0 }).catch(() => {}) },
  { immediate: true }
)

watch(() => appStore.settings.autoCheckUpdates, enabled => {
  updateStore.scheduleAutoCheck(enabled)
})

const keepAppListener = (unlisten: () => void) => {
  if (isUnmounted) unlisten()
  else appUnlisteners.push(unlisten)
}

const drainWatchFolderBatches = () => {
  if (watchBatchDrainPromise) return watchBatchDrainPromise
  watchBatchDrainPromise = (async () => {
    try {
      const batches = await taskTemplates.listPendingTaskTemplateWatchBatches()
      if (!Array.isArray(batches) || batches.length === 0) return
      let addedFiles = 0
      for (const batch of batches) {
        const profile = await taskTemplates.getProfileById(batch.profileId)
        if (!profile) {
          await taskTemplates.acknowledgeTaskTemplateWatchBatch(batch.id)
          continue
        }
        const result = compressionStore.addTemplateDraft(
          taskTemplateCandidatesToFiles(batch.candidates),
          `${profile.name} · 监控草稿`,
          buildSafeTemplateDraftSettings(profile, batch.candidates),
        )
        await taskTemplates.acknowledgeTaskTemplateWatchBatch(batch.id)
        addedFiles += result?.addedCount || 0
      }
      if (addedFiles > 0) {
        appStore.setSuccess(`监控目录发现 ${addedFiles} 个稳定文件，已加入压缩中心待确认草稿`)
      }
    } catch (error) {
      console.warn('Unable to drain watch-folder draft batches:', error)
    }
  })().finally(() => {
    watchBatchDrainPromise = null
  })
  return watchBatchDrainPromise
}

// v2 intentionally resets the oversized window state saved by earlier releases.
const WINDOW_STATE_KEY = 'window-state-v2'

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
      width >= 760 && width <= screenWidth + 100 &&
      height >= 520 && height <= screenHeight + 100

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
  const target = e.target
  const isEditable = target instanceof Element &&
    target.matches('input, textarea, select, [contenteditable="true"]')
  if (isEditable) return
  // 全局快捷键
  if (e.ctrlKey || e.metaKey) {
    switch (e.key.toLowerCase()) {
      case 'o': e.preventDefault(); router.push('/decompress'); break  // Ctrl+O → 解压
      case 'n': e.preventDefault(); router.push('/compress'); break   // Ctrl+N → 压缩
      case 'b': e.preventDefault(); router.push('/browser'); break   // Ctrl+B → 浏览压缩包
      case 'v': if (e.shiftKey) { e.preventDefault(); router.push('/vault') }; break
      case 'i': e.preventDefault(); router.push('/integrity'); break
      case ',': e.preventDefault(); router.push('/settings'); break  // Ctrl+, → 设置
    }
  }
  if (e.key === 'Escape') {
    // Esc 关闭当前聚焦的弹窗由各组件自行处理
  }
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

  const handleContextAction = async (request: ContextAction) => {
    const files = request.files.filter(file => file && !file.startsWith('%'))
    if (files.length === 0) return

    if (request.action === 'context-browse-archive') {
      appStore.openArchiveInBrowser(files[0])
      void router.push('/browser')
      return
    }

    if (request.action === 'context-quick-pack' || request.action.startsWith('context-compress-')) {
      const metadata = await Promise.all(files.map(path =>
        invoke<{ size: number; is_dir: boolean }>('get_file_info', { path }).catch(() => null)
      ))
      const entries = files.map((path, index) => createContextCompressionEntry(path, metadata[index]))
      let acceptedCount = entries.length
      if (request.action === 'context-quick-pack') {
        const plan = createQuickPackPlan(files)
        let candidate = createQuickPackCandidate(plan)
        let available = false
        for (let collisionIndex = 0; collisionIndex < 1000; collisionIndex++) {
          candidate = createQuickPackCandidate(plan, collisionIndex)
          if (!await invoke<boolean>('path_exists', { path: candidate.outputPath })) {
            available = true
            break
          }
        }
        if (!available) candidate = createQuickPackCandidate(plan, Date.now())
        compressionStore.addQuickPack(entries, candidate.archiveName, plan.outputDirectory)
      } else {
        const requestedPaths = new Set(files.map(path => path.replace(/\//g, '\\').toLowerCase()))
        const finishedTaskIds = new Set(
          taskStore.tasks
            .filter(task =>
              task.type === 'compression' &&
              ['completed', 'failed', 'cancelled'].includes(task.status)
            )
            .map(task => task.id)
        )
        const replaceableTaskIds = new Set<string>()
        compressionStore.selectedFiles.forEach(file => {
          if (
            file.taskId &&
            finishedTaskIds.has(file.taskId) &&
            requestedPaths.has(file.path.replace(/\//g, '\\').toLowerCase())
          ) {
            replaceableTaskIds.add(file.taskId)
          }
        })
        compressionStore.groups.forEach(group => {
          if (
            group.taskId &&
            finishedTaskIds.has(group.taskId) &&
            group.files.some(file => requestedPaths.has(file.path.replace(/\//g, '\\').toLowerCase()))
          ) {
            replaceableTaskIds.add(group.taskId)
          }
        })
        if (replaceableTaskIds.size > 0) {
          const taskIds = [...replaceableTaskIds]
          compressionStore.removeJobsByTaskIds(taskIds)
          taskIds.forEach(taskId => taskStore.removeTask(taskId))
        }

        acceptedCount = entries.reduce(
          (count, entry) => count + (compressionStore.addFile(entry) ? 1 : 0),
          0
        )
        if (acceptedCount === 0) {
          void router.push('/compress')
          appStore.setError(appStore.t('compress.context_menu_duplicate'))
          return
        }
      }
      if (request.action === 'context-compress-zip') {
        compressionStore.globalSettings.format = 'zip'
        compressionStore.requestAutoStart()
      } else if (request.action === 'context-compress-7z') {
        compressionStore.globalSettings.format = '7z'
        compressionStore.requestAutoStart()
      }
      void router.push('/compress')
      appStore.setSuccess(appStore.t('compress.context_menu_custom').replace('{0}', String(acceptedCount)))
      return
    }

    appStore.enqueueContextAction({ ...request, files })
    void router.push('/decompress')
  }
  await updateStore.initialize()
  updateStore.scheduleAutoCheck(appStore.settings.autoCheckUpdates)

  try {
    const unlisten = await listen('exit-confirmation-requested', () => {
      showExitConfirmation.value = true
    })
    keepAppListener(unlisten)
  } catch (error) {
    console.warn('Exit confirmation listener is unavailable:', error)
  }

  try {
    const unlisten = await listen<string>('tray-navigate', event => {
      if (event.payload === '/decompress' || event.payload === '/compress') {
        void router.push(event.payload)
      }
    })
    keepAppListener(unlisten)
  } catch (error) {
    console.warn('Tray navigation listener is unavailable:', error)
  }

  const drainContextActions = () => {
    if (contextDrainPromise) return contextDrainPromise
    contextDrainPromise = (async () => {
      try {
        const actions = groupContextActions(await invoke<ContextAction[]>('take_pending_context_actions'))
        if (actions.some(action => action.action === 'context-quick-pack')) {
          compressionStore.prepareQuickPacks()
        }
        for (const action of actions) {
          await handleContextAction(action)
        }
      } catch (error) {
        console.warn('Unable to read context menu actions:', error)
      }
    })().finally(() => {
      contextDrainPromise = null
      if (contextDrainAgain) {
        contextDrainAgain = false
        scheduleContextDrain()
      }
    })
    return contextDrainPromise
  }

  const scheduleContextDrain = () => {
    if (contextDrainPromise) {
      contextDrainAgain = true
      return
    }
    if (contextDrainTimer) clearTimeout(contextDrainTimer)
    contextDrainTimer = setTimeout(() => {
      contextDrainTimer = null
      void drainContextActions()
    }, 150)
  }

  try {
    const unlisten = await listen('context-actions-available', scheduleContextDrain)
    keepAppListener(unlisten)
    await drainContextActions()
  } catch (error) {
    console.warn('Context menu listener is unavailable:', error)
  }

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

  void drainWatchFolderBatches()
  watchBatchTimer = setInterval(() => {
    void drainWatchFolderBatches()
  }, 1_500)

  // Expose the desktop test bridge only after native event listeners are ready.
  // This is the application-readiness boundary for desktop lifecycle assertions.
  if (import.meta.env.VITE_DESKTOP_E2E === '1') {
    cleanupDesktopE2EBridge = installDesktopE2EBridge()
  }
})

onUnmounted(() => {
  isUnmounted = true
  window.removeEventListener('keydown', handleKeydown)
  if (saveWindowTimer) clearTimeout(saveWindowTimer)
  if (contextDrainTimer) clearTimeout(contextDrainTimer)
  if (watchBatchTimer) clearInterval(watchBatchTimer)
  if (unlistenResize) unlistenResize()
  if (cleanupSystemWatcher) cleanupSystemWatcher()
  cleanupDesktopE2EBridge?.()
  appUnlisteners.forEach(unlisten => unlisten())
  updateStore.cleanup()
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
