<template>
  <div id="app">
    <MainLayout />
    <ToastContainer />
    <UpdateDialog />
    <ArchiveConversionModal />
    <ZpaqMultipartDialog />
    <ArchiveOpenDialog />
    <ArchiveOpeningReminder />
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
import ArchiveOpeningReminder from '@/components/settings/ArchiveOpeningReminder.vue'
import ArchiveOpenDialog from '@/components/archive/ArchiveOpenDialog.vue'
import { open } from '@tauri-apps/api/dialog'
import { navigationPathForShortcut } from '@/utils/navigation'
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import MainLayout from '@/components/layouts/MainLayout.vue'
import ToastContainer from '@/components/ui/ToastContainer.vue'
import ZpaqMultipartDialog from '@/components/archive/ZpaqMultipartDialog.vue'
import ArchiveConversionModal from '@/components/compression/ArchiveConversionModal.vue'
import { useArchiveConversionStore } from '@/stores/archiveConversion'
import Modal from '@/components/ui/Modal.vue'
import UpdateDialog from '@/components/update/UpdateDialog.vue'
import { useTaskStore } from '@/stores/task'
import { useCompressionStore } from '@/stores/compression'
import { useWorkflowStore } from '@/stores/workflows'
import { useWorkflowAutomationStore } from '@/stores/workflowAutomation'
import { useZipEditStore } from '@/stores/zipEdit'
import { usePendingDraftStore } from '@/stores/pendingDrafts'
import { useAppStore } from '@/stores/app'
import { useUIStore } from '@/stores/ui'
import { useUpdateStore } from '@/stores/update'
import { useAccessibility } from '@/composables/useAccessibility'
import { useCompressionProfiles } from '@/composables/useCompressionProfiles'
import { installDesktopE2EBridge } from '@/testing/desktopE2EBridge'
import { appWindow, LogicalPosition, LogicalSize } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/tauri'
import { isChecksumManifest, createContextCompressionEntry, createQuickPackCandidate, createQuickPackPlan, groupContextActions, type ContextAction } from '@/utils/contextActions'
import { buildSafeTemplateDraftSettings, taskTemplateCandidatesToFiles } from '@/utils/taskTemplateDraft'

const router = useRouter()
const taskStore = useTaskStore()
const compressionStore = useCompressionStore()
const pendingDrafts = usePendingDraftStore()
pendingDrafts.initialize()
const workflows=useWorkflowStore()
const workflowAutomation=useWorkflowAutomationStore()
const zipEdit=useZipEditStore()
void zipEdit.initialize()
const appStore = useAppStore()
watch(() => appStore.pendingArchiveBrowserPath, path => {
  if (path) void router.push('/browser')
})
watch(() => appStore.pendingArchiveDiagnosis, request => {
  if (request) void router.push('/integrity')
})
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
  pendingDrafts.flush()
  if(!await zipEdit.flush()){exitInProgress.value=false;appStore.setError(zipEdit.saveError);return}
  const unfinished = taskStore.tasks.filter(
    task => !['completed', 'failed', 'cancelled'].includes(task.status)
  )
  try {
    workflowAutomation.prepareExit()
    await workflows.stop()
    await invoke('cancel_tasks_and_wait', { taskIds: unfinished.map(task => task.id) })
    unfinished.forEach(task => taskStore.updateTaskStatus(task.id, 'cancelled'))
    if(!await workflowAutomation.flush()){exitInProgress.value=false;workflowAutomation.resumeAfterExitFailure();appStore.setError(workflowAutomation.error);return}
    if(!await workflows.flush()){exitInProgress.value=false;workflowAutomation.resumeAfterExitFailure();appStore.setError(workflows.saveError);return}
    await invoke('exit_app')
  } catch (error) {
    exitInProgress.value = false
    workflowAutomation.resumeAfterExitFailure()
    appStore.setError(String(error))
  }
}

let currentErrorToast: string | undefined
watch(() => appStore.error, message => {
  if (message) currentErrorToast = uiStore.showToast('error', message, 5000)
  else if (currentErrorToast) {
    uiStore.removeToast(currentErrorToast)
    currentErrorToast = undefined
  }
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

const handleKeydown = (event: KeyboardEvent) => {
  const path = navigationPathForShortcut(event)
  if (!path) return
  event.preventDefault()
  void router.push(path)
}

onMounted(async () => {
  keepAppListener(await listen('exit-drafts-requested',async()=>{
    pendingDrafts.flush()
    workflowAutomation.prepareExit()
    await workflows.stop()
    if(!await workflowAutomation.flush()){workflowAutomation.resumeAfterExitFailure();appStore.setError(workflowAutomation.error);return}
    if(!await workflows.flush()){workflowAutomation.resumeAfterExitFailure();appStore.setError(workflows.saveError);return}
    if(await zipEdit.flush())await invoke('exit_app')
    else {workflowAutomation.resumeAfterExitFailure();appStore.setError(zipEdit.saveError)}
  }))
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
    if (request.action === 'context-system-open') { appStore.enqueueArchiveOpen(files); return }
    if (request.action === 'context-extract-choose') {
      const directory = await open({ directory: true, multiple: false, title: '选择解压文件夹' })
      if (typeof directory === 'string') { appStore.enqueueContextAction({ action: 'context-extract-to', files, outputPath: directory, extractToSubfolder: files.length > 1, openResultFolder: appStore.settings.archiveAfterExtract === 'open-folder' }); void router.push('/decompress') }
      return
    }

    if (request.action === 'context-checksum-create' || request.action === 'context-checksum-verify') {
      const kind = request.action === 'context-checksum-create' ? 'calculate' : 'verify'
      const metadata = await Promise.all(files.map(path => invoke<{ is_dir: boolean }>('get_file_info', { path }).catch(() => null)))
      const accepted = files.filter((path, index) => !metadata[index]?.is_dir && (kind === 'calculate' || isChecksumManifest(path)))
      if (accepted.length) {
        appStore.enqueueExplorerChecksums({ kind, paths: accepted })
        void router.push('/integrity')
      }
      if (accepted.length !== files.length) appStore.setError(kind === 'verify' ? '请选择 SFV、MD5 或 SHA256 清单文件；目录不会递归读取。' : '只带入所选文件；目录不会递归读取，请选择目录内的文件。')
      return
    }

    if (request.action === 'context-repair-archive') {
      appStore.openArchiveForDiagnosis({ path: files[0], paths: files })
      void router.push('/integrity')
      return
    }

    if (request.action === 'context-browse-archive') {
      appStore.openArchiveInBrowser(files[0])
      void router.push('/browser')
      return
    }

    if (request.action === 'context-convert-archive' || request.action === 'context-reconfigure-archive') {
      useArchiveConversionStore().openFromExplorer(files, request.action === 'context-reconfigure-archive' ? 'reconfigure' : 'convert')
      void router.push('/compress')
      return
    }

    if (request.action.startsWith('context-image-')) {
      const metadata = await Promise.all(files.map(path => invoke<{size:number;is_dir:boolean}>('get_file_info', {path}).catch(() => null)))
      const candidates = files.flatMap((path,index) => metadata[index] ? [createContextCompressionEntry(path,metadata[index])] : [])
      const result = compressionStore.addImageCandidates(candidates)
      if (request.action === 'context-image-webp') result.accepted.forEach(item => { item.settings = { ...compressionStore.imageGlobalSettings, outputFormat: 'webp' } })
      if (request.action === 'context-image-resize') result.accepted.forEach(item => { item.settings = { ...compressionStore.imageGlobalSettings, resizeMode: 'limit' } })
      void router.push({path:'/special-compression',query:{mode:'image'}})
      if (result.rejected.length || candidates.length !== files.length) appStore.setError('部分项目不是可读取的 JPG、PNG 或 WebP 图片，已跳过')
      else appStore.setSuccess('已带入图片，确认压缩选项后开始')
      return
    }
    if (request.action === 'context-quick-pack' || request.action.startsWith('context-compress-')) {
      const metadata = await Promise.all(files.map(path =>
        invoke<{ size: number; is_dir: boolean }>('get_file_info', { path }).catch(() => null)
      ))
      const entries = files.flatMap((path,index) => metadata[index] ? [createContextCompressionEntry(path,metadata[index])] : [])
      const acceptedCount = entries.length
      if (!acceptedCount) { appStore.setError('所选文件无法读取，请刷新文件夹后重试'); return }
      const acceptedPaths = entries.map(entry=>entry.path)
      if (acceptedCount !== files.length) appStore.setError('部分所选文件无法读取，已跳过')
      if (['context-quick-pack','context-compress-zip','context-compress-7z'].includes(request.action)) {
        const plan = createQuickPackPlan(acceptedPaths)
        const format = request.action === 'context-compress-7z' ? '7z' : 'zip'
        let candidate = createQuickPackCandidate(plan, 0, format)
        let available = false
        for (let collisionIndex = 0; collisionIndex < 1000; collisionIndex++) {
          candidate = createQuickPackCandidate(plan, collisionIndex, format)
          if (!await invoke<boolean>('path_exists', { path: candidate.outputPath })) {
            available = true
            break
          }
        }
        if (!available) candidate = createQuickPackCandidate(plan, Date.now(), format)
        compressionStore.addQuickPack(entries, candidate.archiveName, plan.outputDirectory, request.action === 'context-compress-7z' ? '7z' : 'zip')
      } else {
        const plan = createQuickPackPlan(acceptedPaths)
        compressionStore.addExplorerDraft(entries, plan.archiveName, plan.outputDirectory,
          request.action === 'context-compress-encrypted' ? 'encrypted' : request.action === 'context-compress-split' ? 'split' : 'custom')
      }
      void router.push('/compress')
      appStore.setSuccess(['context-quick-pack','context-compress-zip','context-compress-7z'].includes(request.action)
        ? `已加入 ${request.action === 'context-compress-7z' ? '7Z' : 'ZIP'} 压缩队列 · ${acceptedCount} 项`
        : request.action === 'context-compress-encrypted' ? `已添加 ${acceptedCount} 项，请填写密码后开始`
        : request.action === 'context-compress-split' ? `已添加 ${acceptedCount} 项，请确认分卷大小后开始`
        : appStore.t('compress.context_menu_custom').replace('{0}', String(acceptedCount)))
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

  workflowAutomation.begin()
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
  pendingDrafts.dispose()
  workflowAutomation.dispose()
  zipEdit.dispose()
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
