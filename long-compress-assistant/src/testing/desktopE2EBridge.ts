import { appWindow } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'
import type { UpdateManifest } from '@tauri-apps/api/updater'
import { useTaskStore, type TaskStatus } from '@/stores/task'
import { useCompressionStore } from '@/stores/compression'
import { useUpdateStore } from '@/stores/update'
import type { WatchFolderDraftBatch, WatchFolderRegistration } from '@/types/profile'
import type { ResourcePreflightReport } from '@/types/resourcePreflight'
import type { CompressionAnalysisResult } from '@/composables/useTauriCommands'
import { attachResourcePreflight } from '@/utils/resourcePreflight'
import { runArchiveTasks } from '@/utils/taskConcurrency'
import type { TaskHistoryRecord } from '@/types/taskHistory'

const TEST_TASK_ID = 'desktop-e2e-lifecycle-task'

export interface DesktopE2EBridge {
  startCancellableTask: (outputPath: string) => Promise<string>
  seedActiveTask: () => Promise<string>
  cancelTask: (taskId: string) => Promise<boolean>
  clearTasks: () => Promise<void>
  clearTaskHistory: () => Promise<void>
  taskHistory: () => Promise<TaskHistoryRecord[]>
  seedVaultPassword: (name: string, password: string) => Promise<string>
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
  diagnoseArchive: (filePath: string) => Promise<{
    actualFormat: string
    status: string
    totalFiles: number
    integrityTested: boolean
    canRepair: boolean
  }>
  repairZip: (filePath: string, outputPath: string) => Promise<{
    outputPath: string
    recoveredFiles: number
    recoveredDirectories: number
    skippedEntries: string[]
    verified: boolean
  }>
  previewArchiveImage: (filePath: string, entryPath: string) => Promise<{
    entryPath: string
    mimeType: string
    dataUrl: string
    byteSize: number
    width: number
    height: number
  }>
  seedCompressionAnalysisWorkspace: (entries: Array<{
    name: string
    path: string
    size: number
    isDirectory: boolean
  }>) => string
  compressionAnalysisAuditState: (jobId: string) => {
    analysis: CompressionAnalysisResult | null
    status: string | null
    settings: { format: string; level: number; createSolidArchive: boolean } | null
  }
  archiveBrowserTaskState: () => Array<{
    status: TaskStatus
    error: string | null
    selectedEntries: string[]
    outputPath: string
    logs: string[]
  }>
  compressionVerificationTaskState: () => Array<{
    status: TaskStatus
    error: string | null
    outputPath: string
    deleteAfter: boolean
    verifyAfter: boolean
    logs: string[]
  }>
  startSevenZipCompression: (sourcePath: string, archivePath: string) => Promise<string>
  startArchiveCompressionBatch: (
    jobs: Array<{ sourcePath: string; archivePath: string }>,
  ) => Promise<string[]>
  startZipTelemetryCompression: (
    sourcePath: string | string[],
    archivePath: string,
    password?: string,
    format?: string,
  ) => Promise<string>
  startSharedOutputExtraction: (archivePaths: string[], outputPath: string) => Promise<string[]>
  archiveFlowAuditState: () => {
    compressionMaxActive: number
    compressionDone: boolean
    extractionMaxActive: number
    extractionDone: boolean
    telemetry: Array<{
      taskId: string
      speed?: string
      etaSeconds?: number
      processedBytes: number
      totalBytes: number
      outputBytes?: number
    }>
    zipDone: boolean
    zipTaskId: string | null
    zipTelemetry: Array<{
      taskId: string
      speed?: string
      etaSeconds?: number
      processedBytes: number
      totalBytes: number
      outputBytes?: number
    }>
    errors: string[]
  }
  runEncryptedSolidSevenZipRoundTrip: (
    sourcePath: string,
    archivePath: string,
    outputPath: string,
    password: string,
  ) => Promise<string>
  queueDesktopConfirmations: (choices: boolean[]) => void
  takeDesktopConfirmation: () => boolean | undefined
  seedPasswordFallbackWorkspace: (sourcePath: string, outputPath: string) => string
  compressionWorkspaceAuditState: () => {
    taskCount: number
    pendingGroupCount: number
    groupFormats: string[]
    outputPaths: string[]
  }
  showAvailableUpdate: () => void
  seedResponsiveWorkspace: (type: 'compression' | 'decompression') => string
  setCloseToTray: (enabled: boolean) => Promise<void>
  requestAppExit: () => void
  hideWindow: (markerPath: string) => Promise<void>
  isWindowVisible: () => Promise<boolean>
  desktopBehaviorState: () => Promise<{ close_to_tray: boolean; has_active_tasks: boolean }>
  requestExitConfirmation: () => Promise<boolean>
  createTaskTemplateAuditProfile: () => Promise<{ id: string; name: string }>
  taskTemplateAuditState: (
    sourceProfileId: string,
    profileName: string,
  ) => Promise<{
    importedProfiles: Array<{
      password: string | null
      deleteAfter: boolean
      autoApplyEnabled: boolean
      passwordStrategy: unknown
      extraParams: Record<string, string>
    }>
    draftGroups: Array<{
      fileCount: number
      password: string
      deleteAfter: boolean
      taskId: string | null
    }>
    taskCount: number
    activeTaskCount: number
    autoStartRequested: boolean
  }>
  clearCompressionWorkspace: () => void
  queueTaskTemplateDialogSelections: (selections: Array<string | string[] | null>) => void
  takeTaskTemplateDialogSelection: () => string | string[] | null | undefined
  taskTemplateDialogQueueLength: () => number
  queueDesktopDialogSelections: (selections: Array<string | string[] | null>) => void
  takeDesktopDialogSelection: () => string | string[] | null | undefined
  watchFolderAuditState: (profileId: string) => Promise<{
    registrations: WatchFolderRegistration[]
    pendingBatches: WatchFolderDraftBatch[]
    draftGroups: Array<{
      files: string[]
      password: string
      deleteAfter: boolean
      taskId: string | null
    }>
    taskCount: number
    activeTaskCount: number
    autoStartRequested: boolean
  }>
  resourcePreflightAuditState: (type: 'compression' | 'decompression') => {
    tasks: Array<{
      id: string
      status: TaskStatus
      outputPath: string
      report: ResourcePreflightReport | null
      logMessages: string[]
    }>
  }
  seedBlockedResourcePreflight: (
    archivePath: string,
    outputPath: string,
  ) => Promise<{ taskId: string; report: ResourcePreflightReport }>
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
  const taskTemplateDialogSelections: Array<string | string[] | null> = []
  const desktopConfirmations: boolean[] = []
  const archiveFlowAudit = {
    compressionMaxActive: 0,
    compressionDone: false,
    extractionMaxActive: 0,
    extractionDone: false,
    telemetry: [] as Array<{
      taskId: string
      speed?: string
      etaSeconds?: number
      processedBytes: number
      totalBytes: number
      outputBytes?: number
    }>,
    zipDone: false,
    zipTaskId: null as string | null,
    zipTelemetry: [] as Array<{
      taskId: string
      speed?: string
      etaSeconds?: number
      processedBytes: number
      totalBytes: number
      outputBytes?: number
    }>,
    errors: [] as string[],
  }

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
    async seedVaultPassword(name, password) {
      const masterPassword = await invoke<string>('get_or_create_master_key')
      const isUnlocked = await invoke<boolean>('is_encrypted_password_service_unlocked')
      if (!isUnlocked) {
        const unlocked = await invoke<boolean>('unlock_encrypted_password_service', { masterPassword })
        if (!unlocked) await invoke('init_encrypted_password_service', { masterPassword })
      }
      const entry = await invoke<{ id: string }>('add_encrypted_password', {
        entry: {
          name,
          username: null,
          password,
          url: null,
          notes: 'desktop-e2e local-day usage audit',
          category: 'Other',
          tags: ['desktop-e2e'],
          strength: 'Strong',
          custom_fields: [],
        },
      })
      return entry.id
    },

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

    diagnoseArchive: filePath => invoke('diagnose_archive', {
      diagnosticId: `desktop-e2e-diagnosis-${Date.now()}-${Math.random().toString(16).slice(2)}`,
      filePath,
      password: null,
    }),

    repairZip: (filePath, outputPath) => invoke('repair_zip', {
      repairId: `desktop-e2e-repair-${Date.now()}-${Math.random().toString(16).slice(2)}`,
      filePath,
      outputPath,
    }),

    previewArchiveImage: (filePath, entryPath) => invoke('preview_archive_image', {
      filePath,
      entryPath,
      password: null,
    }),

    seedCompressionAnalysisWorkspace(entries) {
      compressionStore.prepareQuickPacks()
      const draft = compressionStore.addTemplateDraft(
        entries.map(entry => ({
          ...entry,
          type: entry.isDirectory ? 'directory' : 'file',
        })),
        'Desktop E2E 智能分析',
        { ...compressionStore.globalSettings, format: 'zip', level: 6 },
      )
      if (!draft) throw new Error('Unable to seed the smart-compression workspace')
      const group = compressionStore.groups.find(item => item.id === draft.id)
      if (group) group.expanded = true
      return draft.id
    },

    compressionAnalysisAuditState(jobId) {
      const state = compressionStore.compressionAnalysis[jobId]
      const group = compressionStore.groups.find(item => item.id === jobId)
      const settings = group ? compressionStore.getEffectiveSettings(group.settings) : null
      return {
        analysis: state?.result ?? null,
        status: state?.status ?? null,
        settings: settings
          ? {
              format: settings.format,
              level: settings.level,
              createSolidArchive: settings.createSolidArchive,
            }
          : null,
      }
    },

    archiveBrowserTaskState() {
      return taskStore.tasksFor('decompression').map(task => ({
        status: task.status,
        error: task.error ?? null,
        selectedEntries: task.selectedEntries ? [...task.selectedEntries] : [],
        outputPath: task.outputPath,
        logs: task.logs.map(log => log.message),
      }))
    },

    compressionVerificationTaskState() {
      return taskStore.tasksFor('compression').map(task => ({
        status: task.status,
        error: task.error ?? null,
        outputPath: task.outputPath,
        deleteAfter: task.compressionOptions?.delete_after ?? false,
        verifyAfter: task.compressionOptions?.verify_after ?? true,
        logs: task.logs.map(log => log.message),
      }))
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

    async startArchiveCompressionBatch(jobs) {
      archiveFlowAudit.compressionMaxActive = 0
      archiveFlowAudit.compressionDone = false
      archiveFlowAudit.telemetry = []
      archiveFlowAudit.errors = []
      let active = 0
      const taskIds = jobs.map((job, index) => {
        const taskId = `desktop-e2e-concurrent-compress-${Date.now()}-${index}`
        addArchiveTask(taskId, 'compression', job.sourcePath, job.archivePath)
        return taskId
      })
      const unlisten = await listen<{
        task_id: string
        speed?: string
        eta_seconds?: number
        processed_bytes: number
        total_bytes: number
        output_bytes?: number
      }>('task-progress', event => {
        if (!taskIds.includes(event.payload.task_id)) return
        archiveFlowAudit.telemetry.push({
          taskId: event.payload.task_id,
          speed: event.payload.speed,
          etaSeconds: event.payload.eta_seconds,
          processedBytes: event.payload.processed_bytes,
          totalBytes: event.payload.total_bytes,
          outputBytes: event.payload.output_bytes,
        })
      })
      await syncActiveState()
      void runArchiveTasks(jobs, 2, async (job, index) => {
        const taskId = taskIds[index]
        active++
        archiveFlowAudit.compressionMaxActive = Math.max(
          archiveFlowAudit.compressionMaxActive,
          active,
        )
        taskStore.updateTaskStatus(taskId, 'compressing')
        try {
          await invoke('compress_files', {
            taskId,
            files: [job.sourcePath],
            outputPath: job.archivePath,
            options: { ...sevenZipOptions, level: 1 },
          })
          taskStore.updateTaskStatus(taskId, 'completed')
        } catch (error) {
          archiveFlowAudit.errors.push(String(error))
          taskStore.updateTaskStatus(taskId, 'failed')
        } finally {
          active--
        }
      }).finally(() => {
        archiveFlowAudit.compressionDone = true
        unlisten()
        void syncActiveState()
      })
      return taskIds
    },

    async clearTaskHistory() {
      await invoke('clear_task_history')
    },

    async taskHistory() {
      return invoke<TaskHistoryRecord[]>('list_task_history', { limit: 500 })
    },

    async startZipTelemetryCompression(sourcePath, archivePath, password, format = 'zip') {
      archiveFlowAudit.zipDone = false
      archiveFlowAudit.zipTelemetry = []
      const taskId = `desktop-e2e-${format.replaceAll('.', '-')}-telemetry-${Date.now()}`
      archiveFlowAudit.zipTaskId = taskId
      const sourcePaths = Array.isArray(sourcePath) ? sourcePath : [sourcePath]
      addArchiveTask(taskId, 'compression', sourcePaths[0], archivePath)
      const task = taskStore.tasks.find(item => item.id === taskId)
      if (task) task.format = format
      const unlisten = await listen<{
        task_id: string
        speed?: string
        eta_seconds?: number
        processed_bytes: number
        total_bytes: number
        output_bytes?: number
      }>('task-progress', event => {
        if (event.payload.task_id !== taskId) return
        archiveFlowAudit.zipTelemetry.push({
          taskId,
          speed: event.payload.speed,
          etaSeconds: event.payload.eta_seconds,
          processedBytes: event.payload.processed_bytes,
          totalBytes: event.payload.total_bytes,
          outputBytes: event.payload.output_bytes,
        })
      })
      taskStore.updateTaskStatus(taskId, 'compressing')
      await syncActiveState()
      void invoke('compress_files', {
        taskId,
        files: sourcePaths,
        outputPath: archivePath,
        options: {
          ...sevenZipOptions,
          format,
          level: format === 'zip' ? 9 : 3,
          password: password || null,
        },
      })
        .then(() => taskStore.updateTaskStatus(taskId, 'completed'))
        .catch(error => {
          archiveFlowAudit.errors.push(String(error))
          taskStore.updateTaskStatus(taskId, 'failed')
        })
        .finally(() => {
          archiveFlowAudit.zipDone = true
          unlisten()
          void syncActiveState()
        })
      return taskId
    },

    async startSharedOutputExtraction(archivePaths, outputPath) {
      archiveFlowAudit.extractionMaxActive = 0
      archiveFlowAudit.extractionDone = false
      let active = 0
      const taskIds = archivePaths.map((archivePath, index) => {
        const taskId = `desktop-e2e-serial-extract-${Date.now()}-${index}`
        addArchiveTask(taskId, 'decompression', archivePath, outputPath)
        return taskId
      })
      await syncActiveState()
      void runArchiveTasks(archivePaths, 2, async (archivePath, index) => {
        const taskId = taskIds[index]
        active++
        archiveFlowAudit.extractionMaxActive = Math.max(
          archiveFlowAudit.extractionMaxActive,
          active,
        )
        taskStore.updateTaskStatus(taskId, 'extracting')
        try {
          await invoke('extract_file', {
            taskId,
            filePath: archivePath,
            outputPath,
            password: null,
            options: extractionOptions,
          })
          taskStore.updateTaskStatus(taskId, 'completed')
        } catch (error) {
          archiveFlowAudit.errors.push(String(error))
          taskStore.updateTaskStatus(taskId, 'failed')
        } finally {
          active--
        }
      }, () => outputPath.replace(/\\/g, '/').replace(/\/$/, '').toLocaleLowerCase())
        .finally(() => {
          archiveFlowAudit.extractionDone = true
          void syncActiveState()
        })
      return taskIds
    },

    archiveFlowAuditState() {
      return {
        ...archiveFlowAudit,
        telemetry: archiveFlowAudit.telemetry.map(item => ({ ...item })),
        zipTelemetry: archiveFlowAudit.zipTelemetry.map(item => ({ ...item })),
        errors: [...archiveFlowAudit.errors],
      }
    },

    async runEncryptedSolidSevenZipRoundTrip(sourcePath, archivePath, outputPath, password) {
      const nonce = `${Date.now()}-${Math.random().toString(16).slice(2)}`
      const compressionTaskId = `desktop-e2e-solid-compress-${nonce}`
      const extractionTaskId = `desktop-e2e-solid-extract-${nonce}`
      addArchiveTask(compressionTaskId, 'compression', sourcePath, archivePath)
      taskStore.updateTaskStatus(compressionTaskId, 'compressing')
      await syncActiveState()
      await invoke('compress_files', {
        taskId: compressionTaskId,
        files: [sourcePath],
        outputPath: archivePath,
        options: {
          ...sevenZipOptions,
          level: 5,
          password,
          create_solid_archive: true,
        },
      })
      taskStore.updateTaskStatus(compressionTaskId, 'completed')

      addArchiveTask(extractionTaskId, 'decompression', archivePath, outputPath)
      taskStore.updateTaskStatus(extractionTaskId, 'extracting')
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

    queueDesktopConfirmations(choices) {
      desktopConfirmations.splice(0, desktopConfirmations.length, ...choices)
    },

    takeDesktopConfirmation() {
      return desktopConfirmations.shift()
    },

    seedPasswordFallbackWorkspace(sourcePath, outputPath) {
      compressionStore.prepareQuickPacks()
      taskStore.tasks.splice(0)
      const draft = compressionStore.addTemplateDraft(
        [{
          name: sourcePath.split(/[\\/]/).pop() || 'password-fallback.txt',
          path: sourcePath,
          size: 1,
          type: 'file',
          isDirectory: false,
        }],
        'Desktop E2E 密码格式回退',
        {
          ...compressionStore.globalSettings,
          format: 'tar',
          password: 'desktop-e2e-password',
          filename: 'password-fallback',
        },
      )
      if (!draft) throw new Error('Unable to seed the password fallback workspace')
      const group = compressionStore.groups.find(item => item.id === draft.id)
      if (!group) throw new Error('Password fallback draft group was not created')
      group.outputPath = outputPath
      return draft.id
    },

    compressionWorkspaceAuditState() {
      const pendingGroups = compressionStore.groups.filter(group => !group.taskId)
      return {
        taskCount: taskStore.tasks.length,
        pendingGroupCount: pendingGroups.length,
        groupFormats: pendingGroups.map(group =>
          compressionStore.getEffectiveSettings(group.settings).format,
        ),
        outputPaths: pendingGroups.map(group => group.outputPath || ''),
      }
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
      task.resourcePreflight = {
        operation: type,
        outputPath: longOutputPath,
        probePath: 'C:\\',
        mountPoint: 'C:\\',
        fileSystem: 'NTFS',
        location: 'local',
        medium: 'ssd',
        totalBytes: 512 * 1024 * 1024 * 1024,
        availableBytes: 106 * 1024 * 1024 * 1024,
        estimatedOutputBytes: 780 * 1024 * 1024,
        requiredBytes: 908 * 1024 * 1024,
        reserveBytes: 128 * 1024 * 1024,
        estimateSource: type === 'compression' ? 'provided_estimate' : 'archive_metadata',
        estimateReliable: true,
        status: 'ready',
        canStart: true,
        summary: '目标盘空间满足当前估算；运行时仍会持续执行事务与容量检查。',
        warnings: [],
      }
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

    requestAppExit() {
      void invoke('exit_app')
    },

    hideWindow: (markerPath) => invoke('desktop_e2e_hide_window', { markerPath }),
    isWindowVisible: () => appWindow.isVisible(),
    desktopBehaviorState: () => invoke('desktop_e2e_get_behavior_state'),
    requestExitConfirmation: () => invoke('desktop_e2e_request_exit_confirmation'),

    async createTaskTemplateAuditProfile() {
      compressionStore.prepareQuickPacks()
      taskStore.tasks.splice(0)
      await syncActiveState()
      const name = `Desktop E2E 任务模板 ${Date.now()}`
      const id = await invoke<string>('create_compression_profile', {
        profile: {
          name,
          icon: '📦',
          description: '真实桌面任务模板安全门禁',
          config: {
            format: '7z',
            level: 7,
            password: 'desktop-e2e-secret-must-not-export',
            splitArchive: false,
            splitSize: null,
            keepStructure: true,
            deleteAfter: true,
            verifyAfter: true,
            createSolidArchive: true,
            filenameTemplate: '{name}-{date}',
            extraParams: { unsafe: 'must-not-export' },
          },
          autoApply: {
            enabled: false,
            mode: 'pattern',
            file_patterns: ['*.log'],
            exclude_patterns: ['*.tmp'],
            size_range: null,
          },
        },
      })
      return { id, name }
    },

    async taskTemplateAuditState(sourceProfileId, profileName) {
      const profiles = await invoke<any[]>('get_compression_profiles')
      const importedProfiles = profiles
        .filter(profile => profile.id !== sourceProfileId && profile.name === profileName)
        .map(profile => ({
          password: profile.config.password ?? null,
          deleteAfter: profile.config.delete_after ?? false,
          autoApplyEnabled: profile.auto_apply?.enabled ?? false,
          passwordStrategy: profile.password_strategy,
          extraParams: profile.config.extra_params ?? {},
        }))
      const draftGroups = compressionStore.groups
        .filter(group => group.id.startsWith('template-draft-'))
        .map(group => ({
          fileCount: group.files.length,
          password: group.settings?.password ?? '',
          deleteAfter: group.settings?.deleteAfter ?? false,
          taskId: group.taskId ?? null,
        }))
      return {
        importedProfiles,
        draftGroups,
        taskCount: taskStore.tasks.length,
        activeTaskCount: taskStore.activeTaskCount,
        autoStartRequested: compressionStore.autoStartRequested,
      }
    },

    clearCompressionWorkspace() {
      compressionStore.prepareQuickPacks()
    },

    queueTaskTemplateDialogSelections(selections) {
      taskTemplateDialogSelections.splice(0, taskTemplateDialogSelections.length, ...selections)
    },

    takeTaskTemplateDialogSelection() {
      return taskTemplateDialogSelections.shift()
    },

    taskTemplateDialogQueueLength() {
      return taskTemplateDialogSelections.length
    },

    queueDesktopDialogSelections(selections) {
      taskTemplateDialogSelections.splice(0, taskTemplateDialogSelections.length, ...selections)
    },

    takeDesktopDialogSelection() {
      return taskTemplateDialogSelections.shift()
    },

    async watchFolderAuditState(profileId) {
      const [registrations, pendingBatches] = await Promise.all([
        invoke<WatchFolderRegistration[]>('list_task_template_watch_folders'),
        invoke<WatchFolderDraftBatch[]>('list_pending_task_template_watch_batches'),
      ])
      const draftGroups = compressionStore.groups
        .filter(group => group.name.endsWith('· 监控草稿'))
        .map(group => ({
          files: group.files.map(file => file.path),
          password: group.settings?.password ?? '',
          deleteAfter: group.settings?.deleteAfter ?? false,
          taskId: group.taskId ?? null,
        }))
      return {
        registrations: registrations.filter(item => item.profileId === profileId),
        pendingBatches: pendingBatches.filter(item => item.profileId === profileId),
        draftGroups,
        taskCount: taskStore.tasks.length,
        activeTaskCount: taskStore.activeTaskCount,
        autoStartRequested: compressionStore.autoStartRequested,
      }
    },

    resourcePreflightAuditState(type) {
      return {
        tasks: taskStore.tasksFor(type).map(task => ({
          id: task.id,
          status: task.status,
          outputPath: task.outputPath,
          report: task.resourcePreflight
            ? { ...task.resourcePreflight, warnings: [...task.resourcePreflight.warnings] }
            : null,
          logMessages: task.logs.map(log => log.message),
        })),
      }
    },

    async seedBlockedResourcePreflight(archivePath, outputPath) {
      const report = await invoke<ResourcePreflightReport>('preflight_operation_resources', {
        operation: 'decompression',
        outputPath,
        sourcePaths: [archivePath],
        password: null,
        estimatedOutputBytes: Number.MAX_SAFE_INTEGER,
        estimateReliable: true,
      })
      if (report.canStart || report.status !== 'blocked') {
        throw new Error(`Reliable oversized estimate was not blocked: ${JSON.stringify(report)}`)
      }

      const taskId = `desktop-e2e-resource-blocked-${Date.now()}`
      addArchiveTask(taskId, 'decompression', archivePath, outputPath)
      const task = taskStore.tasks.find(item => item.id === taskId)!
      attachResourcePreflight(task, report)
      task.error = report.summary
      taskStore.updateTaskStatus(taskId, 'failed')
      await syncActiveState()
      return { taskId, report }
    },
  }

  window.__LONG_DECOMPRESS_DESKTOP_E2E__ = bridge
  return () => {
    delete window.__LONG_DECOMPRESS_DESKTOP_E2E__
  }
}
