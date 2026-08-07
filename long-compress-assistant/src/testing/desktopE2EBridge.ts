import { appWindow } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'
import type { UpdateManifest } from '@tauri-apps/api/updater'
import { useTaskStore, type TaskStatus } from '@/stores/task'
import { useCompressionStore } from '@/stores/compression'
import { useUpdateStore } from '@/stores/update'

const TEST_TASK_ID = 'desktop-e2e-lifecycle-task'

export interface DesktopE2EBridge {
  startCancellableTask: (outputPath: string) => Promise<string>
  seedActiveTask: () => Promise<string>
  cancelTask: (taskId: string) => Promise<boolean>
  clearTasks: () => Promise<void>
  reset: () => Promise<void>
  taskStatus: (taskId: string) => TaskStatus | null
  taskProgress: (taskId: string) => number | null
  runSevenZipRoundTrip: (
    sourcePath: string,
    archivePath: string,
    outputPath: string,
  ) => Promise<{ compressionProgress: number[]; extractionProgress: number[]; extractedPath: string }>
  runArchiveRoundTrip: (
    sourcePath: string,
    archivePath: string,
    outputPath: string,
    format: string,
    password?: string | null,
  ) => Promise<string>
  extractArchive: (
    archivePath: string,
    outputPath: string,
    password?: string | null,
  ) => Promise<string>
  startSevenZipCompression: (sourcePath: string, archivePath: string) => Promise<string>
  showAvailableUpdate: () => void
  seedResponsiveWorkspace: (type: 'compression' | 'decompression') => string
  setCloseToTray: (enabled: boolean) => Promise<void>
  hideWindow: (markerPath: string) => Promise<void>
  isWindowVisible: () => Promise<boolean>
  desktopBehaviorState: () => Promise<{ close_to_tray: boolean; has_active_tasks: boolean }>
  requestExitConfirmation: () => Promise<boolean>
}

declare global {
  interface Window {
    __LONG_DECOMPRESS_DESKTOP_E2E__?: DesktopE2EBridge
  }
}

export const installDesktopE2EBridge = () => {
  const taskStore = useTaskStore()
  const compressionStore = useCompressionStore()
  const updateStore = useUpdateStore()

  const addActiveTask = () => {
    taskStore.removeTask(TEST_TASK_ID)
    taskStore.addTask({
      id: TEST_TASK_ID,
      name: 'Desktop E2E lifecycle task',
      type: 'compression',
      sourceFiles: [],
      outputPath: '',
      format: 'zip',
    })
    taskStore.updateTaskStatus(TEST_TASK_ID, 'compressing')
    return TEST_TASK_ID
  }

  const syncActiveState = () =>
    invoke('set_has_active_tasks', { active: taskStore.activeTaskCount > 0 })

  const addArchiveTask = (
    taskId: string,
    type: 'compression' | 'decompression',
    sourcePath: string,
    outputPath: string,
  ) => {
    taskStore.addTask({
      id: taskId,
      name: sourcePath.split(/[\\/]/).pop() || taskId,
      type,
      sourceFiles: [sourcePath],
      outputPath,
      format: '7z',
    })
  }

  const sevenZipOptions = {
    format: '7z',
    level: 3,
    password: null,
    split_size: null,
    preserve_paths: true,
    delete_after: false,
    allow_insecure_password_cli: false,
  }

  const extractionOptions = {
    preserve_paths: true,
    overwrite_existing: false,
    delete_after: false,
    preserve_timestamps: true,
    skip_corrupted: false,
    extract_only_newer: false,
    create_subdirectory: false,
    file_filter: null,
    conflict_policy: 'rename',
    enable_bruteforce: false,
    bruteforce_wordlists: [],
  }

  const bridge: DesktopE2EBridge = {
    async startCancellableTask(outputPath) {
      const taskId = addActiveTask()
      const task = taskStore.tasks.find(item => item.id === taskId)
      if (task) task.outputPath = outputPath
      await syncActiveState()
      void invoke('desktop_e2e_run_cancellable_task', { taskId, outputPath })
        .then(() => {
          if (!['cancelled', 'cancelling'].includes(taskStore.tasks.find(item => item.id === taskId)?.status || '')) {
            taskStore.updateTaskStatus(taskId, 'completed')
          }
        })
        .catch(error => {
          const current = taskStore.tasks.find(item => item.id === taskId)
          if (current && !['cancelled', 'cancelling'].includes(current.status)) {
            current.error = String(error)
            taskStore.updateTaskStatus(taskId, 'failed')
          }
        })
      return taskId
    },

    async seedActiveTask() {
      const taskId = addActiveTask()
      await syncActiveState()
      return taskId
    },

    async cancelTask(taskId) {
      return taskStore.cancelTask(taskId)
    },

    async clearTasks() {
      taskStore.tasks.splice(0)
      await syncActiveState()
    },

    async reset() {
      taskStore.tasks.splice(0)
      await syncActiveState()
      updateStore.$patch({
        status: 'idle',
        manifest: null,
        errorMessage: '',
        dialogVisible: false,
      })
    },

    taskStatus(taskId) {
      return taskStore.tasks.find(item => item.id === taskId)?.status ?? null
    },

    taskProgress(taskId) {
      return taskStore.tasks.find(item => item.id === taskId)?.progress ?? null
    },

    async runSevenZipRoundTrip(sourcePath, archivePath, outputPath) {
      const compressionTaskId = `desktop-e2e-7z-compress-${Date.now()}`
      const extractionTaskId = `desktop-e2e-7z-extract-${Date.now()}`
      const compressionProgress: number[] = []
      const extractionProgress: number[] = []
      const unlisten = await listen<{ task_id: string; progress: number }>('task-progress', event => {
        const percentage = Math.round(event.payload.progress * 100)
        if (event.payload.task_id === compressionTaskId) compressionProgress.push(percentage)
        if (event.payload.task_id === extractionTaskId) extractionProgress.push(percentage)
      })

      try {
        addArchiveTask(compressionTaskId, 'compression', sourcePath, archivePath)
        taskStore.updateTaskStatus(compressionTaskId, 'compressing')
        await syncActiveState()
        await invoke('compress_files', {
          taskId: compressionTaskId,
          files: [sourcePath],
          outputPath: archivePath,
          options: sevenZipOptions,
        })
        taskStore.updateTaskStatus(compressionTaskId, 'completed')

        addArchiveTask(extractionTaskId, 'decompression', archivePath, outputPath)
        taskStore.updateTaskStatus(extractionTaskId, 'extracting')
        await syncActiveState()
        const extractedPath = await invoke<string>('extract_file', {
          taskId: extractionTaskId,
          filePath: archivePath,
          outputPath,
          password: null,
          options: extractionOptions,
        })
        taskStore.updateTaskStatus(extractionTaskId, 'completed')
        return { compressionProgress, extractionProgress, extractedPath }
      } finally {
        unlisten()
        await syncActiveState()
      }
    },

    async runArchiveRoundTrip(sourcePath, archivePath, outputPath, format, password = null) {
      const nonce = `${format.replaceAll('.', '-')}-${Date.now()}-${Math.random().toString(16).slice(2)}`
      const compressionTaskId = `desktop-e2e-matrix-compress-${nonce}`
      const extractionTaskId = `desktop-e2e-matrix-extract-${nonce}`
      addArchiveTask(compressionTaskId, 'compression', sourcePath, archivePath)
      const compressionTask = taskStore.tasks.find(item => item.id === compressionTaskId)
      if (compressionTask) compressionTask.format = format
      taskStore.updateTaskStatus(compressionTaskId, 'compressing')
      await syncActiveState()
      await invoke('compress_files', {
        taskId: compressionTaskId,
        files: [sourcePath],
        outputPath: archivePath,
        options: {
          ...sevenZipOptions,
          format,
          password,
        },
      })
      taskStore.updateTaskStatus(compressionTaskId, 'completed')

      addArchiveTask(extractionTaskId, 'decompression', archivePath, outputPath)
      const extractionTask = taskStore.tasks.find(item => item.id === extractionTaskId)
      if (extractionTask) extractionTask.format = format
      taskStore.updateTaskStatus(extractionTaskId, 'extracting')
      await syncActiveState()
      const extractedPath = await invoke<string>('extract_file', {
        taskId: extractionTaskId,
        filePath: archivePath,
        outputPath,
        password,
        options: extractionOptions,
      })
      taskStore.updateTaskStatus(extractionTaskId, 'completed')
      await syncActiveState()
      return extractedPath
    },

    async extractArchive(archivePath, outputPath, password = null) {
      const nonce = `${Date.now()}-${Math.random().toString(16).slice(2)}`
      const taskId = `desktop-e2e-extract-only-${nonce}`
      addArchiveTask(taskId, 'decompression', archivePath, outputPath)
      taskStore.updateTaskStatus(taskId, 'extracting')
      await syncActiveState()
      try {
        const extractedPath = await invoke<string>('extract_file', {
          taskId,
          filePath: archivePath,
          outputPath,
          password,
          options: extractionOptions,
        })
        taskStore.updateTaskStatus(taskId, 'completed')
        return extractedPath
      } catch (error) {
        const task = taskStore.tasks.find(item => item.id === taskId)
        if (task) task.error = String(error)
        taskStore.updateTaskStatus(taskId, 'failed')
        throw error
      } finally {
        await syncActiveState()
      }
    },

    async startSevenZipCompression(sourcePath, archivePath) {
      const taskId = `desktop-e2e-7z-cancel-${Date.now()}`
      addArchiveTask(taskId, 'compression', sourcePath, archivePath)
      taskStore.updateTaskStatus(taskId, 'compressing')
      await syncActiveState()
      void invoke('compress_files', {
        taskId,
        files: [sourcePath],
        outputPath: archivePath,
        options: { ...sevenZipOptions, level: 1 },
      })
        .then(() => {
          const task = taskStore.tasks.find(item => item.id === taskId)
          if (task && !['cancelled', 'cancelling'].includes(task.status)) {
            taskStore.updateTaskStatus(taskId, 'completed')
          }
        })
        .catch(error => {
          const task = taskStore.tasks.find(item => item.id === taskId)
          if (task && !['cancelled', 'cancelling'].includes(task.status)) {
            task.error = String(error)
            taskStore.updateTaskStatus(taskId, 'failed')
          }
        })
        .finally(() => void syncActiveState())
      return taskId
    },

    showAvailableUpdate() {
      updateStore.$patch({
        status: 'available',
        manifest: {
          version: '99.0.0-e2e',
          date: '2026-07-27',
          body: 'Desktop lifecycle update blocking fixture.',
        } as UpdateManifest,
        errorMessage: '',
        dialogVisible: true,
      })
    },

    seedResponsiveWorkspace(type) {
      const taskId = `responsive-${type}`
      const longSourcePath = `C:\\Users\\ResponsiveFixture\\Documents\\${'deep-folder\\'.repeat(8)}source-file-with-a-very-long-name.bin`
      const longOutputPath = `C:\\Users\\ResponsiveFixture\\Archives\\${'nested-output\\'.repeat(6)}responsive-output.${type === 'compression' ? 'zip' : 'folder'}`

      taskStore.tasks = []
      compressionStore.prepareQuickPacks()
      taskStore.addTask({
        id: taskId,
        name: type === 'compression' ? 'responsive-output.zip' : 'responsive-input.7z',
        type,
        sourceFiles: [longSourcePath],
        outputPath: longOutputPath,
        format: type === 'compression' ? 'zip' : '7z',
        compressionOptions: type === 'compression'
          ? { format: 'zip', level: 6, preserve_paths: true }
          : undefined,
      })
      const task = taskStore.tasks.find(item => item.id === taskId)!
      taskStore.updateTaskStatus(taskId, 'completed')
      task.progress = 100
      task.stage = 'Finalizing'
      task.currentFile = longSourcePath
      task.logs = Array.from({ length: 18 }, (_, index) => ({
        task_id: taskId,
        timestamp: new Date(Date.now() + index * 1000).toISOString(),
        message: `${index + 1}: ${longSourcePath} -> ${longOutputPath}`,
        severity: index === 17 ? 'success' as const : 'info' as const,
      }))

      if (type === 'compression') {
        compressionStore.addFile({
          name: 'source-file-with-a-very-long-name.bin',
          path: longSourcePath,
          size: 4096,
          type: 'file',
          isDirectory: false,
        })
        const file = compressionStore.selectedFiles[0]
        file.expanded = true
        compressionStore.bindJobTask(
          longSourcePath,
          taskId,
          { ...compressionStore.globalSettings, filename: 'responsive-output' },
          longOutputPath,
        )
      }

      return taskId
    },

    async setCloseToTray(enabled) {
      await invoke('set_close_to_tray', { enabled })
    },

    hideWindow: (markerPath) => invoke('desktop_e2e_hide_window', { markerPath }),
    isWindowVisible: () => appWindow.isVisible(),
    desktopBehaviorState: () => invoke('desktop_e2e_get_behavior_state'),
    requestExitConfirmation: () => invoke('desktop_e2e_request_exit_confirmation'),
  }

  window.__LONG_DECOMPRESS_DESKTOP_E2E__ = bridge
  return () => {
    delete window.__LONG_DECOMPRESS_DESKTOP_E2E__
  }
}
