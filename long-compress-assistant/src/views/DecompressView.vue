<script setup lang="ts">
import SavedLocationPicker from '@/components/ui/SavedLocationPicker.vue'
import { useSavedLocations } from '@/composables/useSavedLocations'
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import { useTaskStore, type Task } from '@/stores/task'
import { useAppStore } from '@/stores/app'
import { usePasswordStore } from '@/stores/password'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { extractErrorMessage, generateId, isPasswordRelatedError } from '@/utils'
import { open } from '@tauri-apps/api/dialog'
import AeroTable from '@/components/tasks/AeroTable.vue'
import ConflictResolutionModal from '@/components/tasks/ConflictResolutionModal.vue'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'
import { DECOMPRESS_ARCHIVE_ACCEPT, DECOMPRESS_ARCHIVE_HINT, isDecompressArchivePath, isPotentialSplitArchivePath, isPotentialSfxPath } from '@/utils/compressionFormat'
import { detectArchiveGroup } from '@/utils/archiveSource'
import { appendResourcePreflightFallback, attachResourcePreflight } from '@/utils/resourcePreflight'
import { runArchiveTasks } from '@/utils/taskConcurrency'
import { sortTasksByName } from '@/utils/taskOrdering'
import { archiveInspectionMessageForDisplay } from '@/utils/taskFailure'
import { useDecompressionBatchStore } from '@/stores/decompressionBatch'
import DecompressionBatchSummary from '@/components/tasks/DecompressionBatchSummary.vue'
import FailedExtractionRetry from '@/components/tasks/FailedExtractionRetry.vue'
import { inferExtractionSubfolder } from '@/utils/smartExtraction'
import BatchConfigBar from '@/components/tasks/BatchConfigBar.vue'
import type { BatchConfigPatch } from '@/types/batchConfig'
import { parentDirectory } from '@/utils/resultLocation'
import { storeToRefs } from 'pinia'
import { useDecompressionPreferences } from '@/stores/decompressionPreferences'
import { zipPasswordEncodingForSource } from '@/utils/zipPasswordSource'

const taskStore = useTaskStore()
const batchStore = useDecompressionBatchStore()
const appStore = useAppStore()
const passwordStore = usePasswordStore()
const tauriCommands = useTauriCommands()

const selectedConflictTaskId = ref<string | null>(null)
const retryTaskIds = ref<string[] | null>(null)
const retrySuppliedPassword = ref('')
const openBatchRetry = () => {
  retrySuppliedPassword.value = ''
  retryTaskIds.value = [...batchStore.visibleFailureIds]
}
const closeRetry = () => {
  retryTaskIds.value = null
  retrySuppliedPassword.value = ''
}
const selectRetryDrafts = (ids: string[]) => {
  batchStore.showingFailures = false
  selectedTaskIds.value = new Set(ids)
  appStore.setSuccess(`已创建 ${ids.length} 个新任务，请核对后点击开始；原失败记录保留。`)
}
const showConflictModal = ref(false)
const resolvingConflictTaskId = computed(() => taskStore.tasks.find(task => task.conflictResolutionPending)?.id || null)
let disposed = false
const selectedTaskIds = ref<Set<string>>(new Set())
const supportedArchiveAccept = DECOMPRESS_ARCHIVE_ACCEPT
const supportedArchiveHint = DECOMPRESS_ARCHIVE_HINT
const decompressionTasks = computed(() => taskStore.tasksFor('decompression'))
const selectedPendingTasks = computed(() => decompressionTasks.value.filter(task => task.status === 'pending' && selectedTaskIds.value.has(task.id)))
watch(() => decompressionTasks.value.filter(task => task.status === 'pending').map(task => task.id), ids => {
  const valid = new Set(ids)
  selectedTaskIds.value = new Set([...selectedTaskIds.value].filter(id => valid.has(id)))
})
async function applySelectedConfiguration(patch: BatchConfigPatch, complete?: (error?: string) => void) {
  const encodingChanged = patch.zipPasswordEncoding !== undefined
  if (patch.outputDirectory === undefined && patch.extractToSubfolder === undefined && !encodingChanged) return
  const selected = selectedPendingTasks.value.map(task => ({ task, source: task.sourceFiles[0] }))
  let skipped = 0, updated = 0
  for (const { task, source } of selected) {
    let changed = false
    if (encodingChanged) {
      // Read the actual input format, not its possibly renamed extension.
      try {
        const group = await detectArchiveGroup(source)
        if (disposed || !taskStore.tasks.includes(task) || task.status !== 'pending' || task.sourceFiles[0] !== source) continue
        if (group?.archive_format) task.format = group.archive_format
        const encoding = patch.zipPasswordEncoding === 'auto' ? undefined : patch.zipPasswordEncoding
        if (group?.archive_format && ['zip', 'zipx'].includes(group.archive_format.toLowerCase())) {
          task.zipPasswordEncoding = encoding
          changed = true
        } else skipped++
      } catch { skipped++ }
    }
    if (disposed || !taskStore.tasks.includes(task) || task.status !== 'pending' || task.sourceFiles[0] !== source) continue
    if (patch.outputDirectory !== undefined) {
      task.outputPath = patch.outputDirectory || parentDirectory(source || '')
      changed = true
    }
    if (patch.extractToSubfolder !== undefined) { task.extractToSubfolder = patch.extractToSubfolder; changed = true }
    if (changed) {
      // Encoding is task-local; changing only it must not detach output defaults.
      if (patch.outputDirectory !== undefined || patch.extractToSubfolder !== undefined) task.configurationMode = 'individual'
      updated++
    }
  }
  if (updated) appStore.setSuccess(`已更新 ${updated} 项配置，其他设置保持不变。${skipped ? ` ${skipped} 项非 ZIP 或格式未确认，未修改密码编码。` : ''}`)
  if (!updated && skipped) { complete?.('未找到已确认的 ZIP/ZIPX 来源，未修改密码编码。'); return }
  complete?.()
}

// 全局配置状态
const { outputPath: globalOutputPath, sameDirectory: isGlobalSameDir,
  extractToSubfolder: globalExtractToSubfolder, recycleSource: globalRecycleSourceAfterExtract } = storeToRefs(useDecompressionPreferences())
let subfolderChoiceRevision = 0
watch(() => appStore.settings.smartExtractionDirectory, () => { subfolderChoiceRevision++ }, { flush: 'sync' })

watch(
  () => appStore.settings.autoDeleteSource,
  enabled => { globalRecycleSourceAfterExtract.value = enabled }
)

let contextActionDrain: Promise<void> | null = null

const drainPendingContextActions = () => {
  if (contextActionDrain) return contextActionDrain
  contextActionDrain = (async () => {
    while (appStore.pendingContextActions.length > 0) {
      const requests = appStore.takeContextActions()
      for (const request of requests) {
        const files = request.files.filter(file => file && !file.startsWith('%'))
        if (files.length === 0) continue

        if (request.action === 'context-test-archive') {
          for (const path of files) {
            try {
              const result = await tauriCommands.testArchiveIntegrity(path)
              appStore.setSuccess(appStore.t('decompress.integrity_passed').replace('{0}', result))
            } catch (e: any) {
              appStore.setError(appStore.t('decompress.integrity_failed').replace('{0}', String(e)))
            }
          }
          continue
        }

        const createdTaskIds = await onFilesSelected(files.map(path => ({ path })) as any)
        if (request.action === 'context-open') continue

        const createdTasks = decompressionTasks.value.filter(
          task => createdTaskIds.includes(task.id) && task.status === 'pending'
        )
        if (request.outputPath) createdTasks.forEach(task => { task.outputPath = request.outputPath! })
        const extractToSubfolder = request.action !== 'context-extract-here'
        createdTasks.forEach(task => {
          task.extractToSubfolder = extractToSubfolder
          task.configurationMode = 'individual'
        })
        if (createdTasks.length > 0) {
          appStore.setSuccess(
            request.action === 'context-quick-extract'
              ? appStore.t('decompress.quick_extract_started').replace('{0}', String(createdTasks.length))
              : appStore.t('decompress.context_menu_added').replace('{0}', String(createdTasks.length))
          )
          await startDecompression(createdTaskIds)
        }
      }
    }
  })().finally(() => {
    contextActionDrain = null
    if (appStore.pendingContextActions.length > 0) void drainPendingContextActions()
  })
  return contextActionDrain
}

watch(
  () => appStore.pendingContextActions.length,
  count => { if (count > 0) void drainPendingContextActions() }
)

onMounted(() => {
  void drainPendingContextActions()
})

onUnmounted(() => {
  disposed = true
  unsubConflict()
})

const onFilesSelected = async (files: any[]) => {
  batchStore.showingFailures = false
  const createdTaskIds: string[] = []
  const errors: string[] = []
  let groupedSplitCount = 0
  const normalizePath = (path: string) => path.replace(/\\/g, '/').toLowerCase()
  const uniqueFiles = new Map<string, any>()
  let unsupportedCount = 0
  for (const file of files) {
    if (typeof file?.path !== 'string' || !file.path.trim()) {
      unsupportedCount++
      continue
    }
    if (!isDecompressArchivePath(file.path) && !isPotentialSplitArchivePath(file.path) && !isPotentialSfxPath(file.path)) {
      unsupportedCount++
      continue
    }
    uniqueFiles.set(normalizePath(file.path), file)
  }

  const handledPaths = new Set<string>()
  const existingSources = new Set(
    decompressionTasks.value.flatMap(task => task.sourceFiles.map(normalizePath))
  )

  const createTask = (sourcePath: string, name: string, splitCount?: number, actualFormat?: string | null) => {
    const parentDir = sourcePath.substring(0, Math.max(sourcePath.lastIndexOf('/'), sourcePath.lastIndexOf('\\')))
    const taskId = taskStore.addTask({
      id: generateId(),
      name: splitCount ? `${name} (${splitCount} 个分卷)` : name,
      type: 'decompression',
      sourceFiles: [sourcePath],
      outputPath: isGlobalSameDir.value ? parentDir : globalOutputPath.value,
      extractToSubfolder: globalExtractToSubfolder.value,
      recycleSourceAfterExtract: globalRecycleSourceAfterExtract.value,
      configurationMode: 'global',
      format: actualFormat || undefined,
    })
    createdTaskIds.push(taskId)
    appStore.addRecentFile(sourcePath)
    if (appStore.settings.smartExtractionDirectory !== true) return
    const requestedRevision = subfolderChoiceRevision
    tauriCommands.listArchiveContents(sourcePath).then((contents: string[]) => {
      const task = taskStore.tasks.find(task => task.id === taskId)
      if (appStore.settings.smartExtractionDirectory !== true || !task || task.status !== 'pending' || task.configurationMode !== 'global'
        || task.sourceFiles[0] !== sourcePath || requestedRevision !== subfolderChoiceRevision) return
      const subfolder = inferExtractionSubfolder(contents)
      if (subfolder !== undefined) task.extractToSubfolder = subfolder
    }).catch(error => console.debug('Smart extract skipped (unable to list contents):', sourcePath, error))
  }

  for (const file of uniqueFiles.values()) {
    const sourcePath = file.path as string
    const normalizedSource = normalizePath(sourcePath)
    if (handledPaths.has(normalizedSource) || existingSources.has(normalizedSource)) continue
    handledPaths.add(normalizedSource)

    let detectedFormat: string | null | undefined
    try {
      const splitInfo = await detectArchiveGroup(sourcePath)
      detectedFormat = splitInfo?.archive_format
      if (isPotentialSfxPath(sourcePath) && !splitInfo?.archive_format) {
        throw new Error('不是可读取的 ZIP、7Z 或 RAR 自解压归档；不会执行此程序')
      }
      if (splitInfo && splitInfo.is_split) {
        const groupPaths = Array.isArray(splitInfo.parts) ? splitInfo.parts : []
        groupPaths.forEach((part: string) => handledPaths.add(normalizePath(part)))
        const firstPart = typeof splitInfo.first_part === 'string' ? splitInfo.first_part : ''
        if (!firstPart) {
          errors.push(appStore.t('decompress.split.first_missing').replace('{0}', file.name || sourcePath))
          continue
        }
        const missingParts = Array.isArray(splitInfo.missing_parts) ? splitInfo.missing_parts : []
        if (splitInfo.is_complete === false || missingParts.length > 0) {
          const missingNames = missingParts.slice(0, 5).map((part: string) => part.split(/[\\/]/).pop()).join('、')
          errors.push(appStore.t('decompress.split.incomplete')
            .replace('{0}', file.name || sourcePath)
            .replace('{1}', missingNames || appStore.t('decompress.split.required_part')))
          continue
        }
        const normalizedFirst = normalizePath(firstPart)
        if (existingSources.has(normalizedFirst)) continue
        const displayName = splitInfo.base_name || firstPart.split(/[\\/]/).pop() || file.name || 'Unknown'
        createTask(firstPart, displayName, Number(splitInfo.total_parts || groupPaths.length || 1), detectedFormat)
        groupedSplitCount++
        existingSources.add(normalizedFirst)
        continue
      }
    } catch (error) {
      if (isPotentialSfxPath(sourcePath)) {
        const detail = archiveInspectionMessageForDisplay(extractErrorMessage(error))
        errors.push(`${file.name || sourcePath}：${detail}`)
        continue
      }
      if (isPotentialSplitArchivePath(sourcePath)) {
        const detail = archiveInspectionMessageForDisplay(extractErrorMessage(error))
        errors.push(appStore.t('decompress.split.detection_failed')
          .replace('{0}', file.name || sourcePath)
          .replace('{1}', detail))
        continue
      }
      console.debug('Split archive detection skipped:', error)
    }
    if (isPotentialSplitArchivePath(sourcePath)) {
      errors.push(appStore.t('decompress.split.group_not_found').replace('{0}', file.name || sourcePath))
      continue
    }
    createTask(sourcePath, file.name || sourcePath.split(/[\\/]/).pop() || 'Unknown', undefined, detectedFormat)
  }

  if (unsupportedCount > 0) errors.push(appStore.t('decompress.unsupported_files').replace('{0}', String(unsupportedCount)))
  if (groupedSplitCount > 0) {
    appStore.setSuccess(appStore.t('decompress.split.grouped').replace('{0}', String(groupedSplitCount)))
  }
  if (errors.length > 0) appStore.setError(errors.slice(0, 3).join('；'))
  return createdTaskIds
}

const setGlobalOutput = (selected: string) => {
  globalOutputPath.value = selected
  isGlobalSameDir.value = false
  decompressionTasks.value.forEach(t => {
    if (t.status === 'pending' && t.configurationMode === 'global') t.outputPath = selected
  })
}
const handleGlobalSelectDir = async () => {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: appStore.t('decompress.config.output_select')
    })
    if (selected && typeof selected === 'string') {
      setGlobalOutput(selected)
      useSavedLocations().remember(selected)
    }
  } catch (err) {
    console.error('Failed to select global dir:', err)
  }
}

const handleGlobalSetSameDir = () => {
  isGlobalSameDir.value = true
  globalOutputPath.value = ''
  // 同步到所有待处理任务：设置各自的父目录
  decompressionTasks.value.forEach(t => {
    if (t.status === 'pending' && t.configurationMode === 'global' && t.sourceFiles.length > 0) {
      const sp = t.sourceFiles[0]
      t.outputPath = sp.substring(0, Math.max(sp.lastIndexOf('/'), sp.lastIndexOf('\\')))
    }
  })
}

const toggleGlobalSubfolder = () => {
  subfolderChoiceRevision++
  globalExtractToSubfolder.value = !globalExtractToSubfolder.value
  decompressionTasks.value.forEach(t => {
    if (t.status === 'pending' && t.configurationMode === 'global') t.extractToSubfolder = globalExtractToSubfolder.value
  })
}

const toggleGlobalRecycleSource = () => {
  globalRecycleSourceAfterExtract.value = !globalRecycleSourceAfterExtract.value
  appStore.updateSettings({ autoDeleteSource: globalRecycleSourceAfterExtract.value })
  decompressionTasks.value.forEach(t => {
    if (t.status === 'pending' && t.configurationMode === 'global') t.recycleSourceAfterExtract = globalRecycleSourceAfterExtract.value
  })
}

const applyGlobalConfiguration = (task: Task) => {
  const sourcePath = task.sourceFiles[0] || ''
  const separatorIndex = Math.max(sourcePath.lastIndexOf('/'), sourcePath.lastIndexOf('\\'))
  task.outputPath = isGlobalSameDir.value
    ? sourcePath.substring(0, separatorIndex)
    : globalOutputPath.value
  task.recycleSourceAfterExtract = globalRecycleSourceAfterExtract.value
}

const toggleTaskSelection = (taskId: string) => {
  if (selectedTaskIds.value.has(taskId)) {
    selectedTaskIds.value.delete(taskId)
  } else {
    selectedTaskIds.value.add(taskId)
  }
  // trigger reactivity
  selectedTaskIds.value = new Set(selectedTaskIds.value)
}

const selectAllPending = () => {
  const ids = decompressionTasks.value.filter(t => t.status === 'pending').map(t => t.id)
  selectedTaskIds.value = new Set(ids)
}

const deselectAll = () => {
  selectedTaskIds.value = new Set()
}

const isProcessing = ref(false)

const runDecompressionResourcePreflight = async (task: Task) => {
  taskStore.updateTaskStatus(task.id, 'preparing')
  const stopped = () => task.status === 'cancelled' || task.status === 'cancelling'
  // Old/reopened drafts may have an explicit ZIP encoding but no detected
  // format. A renamed ZIP must not lose that choice because of its suffix.
  if (task.zipPasswordEncoding !== undefined && !task.format) {
    try { task.format = (await detectArchiveGroup(task.sourceFiles[0]))?.archive_format || undefined }
    catch { /* The actual extraction owns source errors; do not change the chosen encoding. */ }
  }
  if (stopped()) return false
  try {
    const report = await tauriCommands.preflightOperationResources({
      operation: 'decompression',
      outputPath: task.outputPath,
      sourcePaths: task.sourceFiles,
      password: task.password,
    })
    if (stopped()) return false
    attachResourcePreflight(task, report)
    if (!report.canStart) {
      taskStore.failTask(task.id, report.summary)
      appStore.setError(`${appStore.t('common.error')}: ${report.summary}`)
      return false
    }
  } catch (error) {
    appendResourcePreflightFallback(task, error)
  }
  return !stopped()
}

const startDecompression = async (onlyTaskIds?: string[]) => {
  // 防止重复点击
  if (isProcessing.value) return
  // 如果有选中的任务，优先处理选中的；否则处理所有 pending 任务
  const pendingTasks = sortTasksByName(onlyTaskIds
    ? decompressionTasks.value.filter(t => onlyTaskIds.includes(t.id) && t.status === 'pending')
    : selectedTaskIds.value.size > 0
    ? decompressionTasks.value.filter(t => selectedTaskIds.value.has(t.id) && t.status === 'pending')
    : decompressionTasks.value.filter(t => t.status === 'pending')).filter(task => !task.conflicts.length && !task.conflictResolutionPending)
  if (pendingTasks.length === 0) return

  isProcessing.value = true
  batchStore.begin(pendingTasks.map(task => task.id))

  // 启动后清除选择
  selectedTaskIds.value = new Set()

  try {
    await runArchiveTasks(
      pendingTasks,
      appStore.settings.maxConcurrentTasks,
      async task => {
    // The batch keeps a snapshot of queued tasks. A task cancelled while it is
    // waiting for a concurrency slot must never start afterwards.
    if (task.status === 'cancelled') return
    if (task.sourceIssue || task.draftPasswordRequired) {
      appStore.setError(task.sourceIssue || '此任务的密码未保存在草稿中，请展开条目重新填写。')
      return
    }
    if (task.configurationMode === 'global') applyGlobalConfiguration(task)
    // 不预先添加密码，先尝试解压，只有明确要求密码时才使用保险箱
    const options = {
      outputPath: task.outputPath,
      keepStructure: true,
      overwrite: false,
      deleteAfter: task.recycleSourceAfterExtract ?? false,
      createSubdirectory: task.extractToSubfolder ?? false,
      password: task.password || undefined, // 只使用用户手动输入的密码
      zipPasswordEncoding: zipPasswordEncodingForSource(task.sourceFiles[0], task.zipPasswordEncoding, task.format),
      fileFilter: task.fileFilter || null
      ,conflictPolicy: appStore.settings.conflictPolicy
    }

    try {
      if (!await runDecompressionResourcePreflight(task)) return
      options.zipPasswordEncoding = task.format
        ? zipPasswordEncodingForSource(task.sourceFiles[0], task.zipPasswordEncoding, task.format)
        : task.zipPasswordEncoding
      task.passwordRequired = false
      await tauriCommands.decompressFile(task.sourceFiles[0], options, task.id)

      // 解压成功，递增保险箱中匹配密码的 use_count
      if (task.password) {
        const matchedEntry = passwordStore.entries.find(e => e.password === task.password)
        if (matchedEntry) {
          try {
            await passwordStore.usePassword(matchedEntry.id)
          } catch { /* 非关键操作 */ }
        }
      }
      task.password = undefined
      task.currentPassword = undefined
    } catch (error) {
      // 只在后端明确返回密码相关错误时才尝试密码破解
      const errorMsg = extractErrorMessage(error) || String(error)
      const isPasswordError = isPasswordRelatedError(error)
      const isConflictResolutionRequired = /File conflict requires resolution/i.test(errorMsg)

      if (isConflictResolutionRequired) {
        // The backend has staged the extraction and emitted file-conflict.
        // Keep the task resumable while the modal collects the user's policy.
        taskStore.updateTaskStatus(task.id, 'pending')
        task.error = undefined
        task.logs.push({
          task_id: task.id,
          message: appStore.t('decompress.conflict_waiting'),
          severity: 'warning',
          timestamp: new Date().toISOString()
        })
      } else if (isPasswordError) {
        // Password discovery and optional dictionary attempts are owned by the
        // backend state machine. The frontend only exposes the manual retry
        // state, preventing duplicate vault/dictionary attempts and logs.
        task.passwordRequired = true
        task.currentPassword = undefined
        // The command owner has already persisted its final error. Expose the
        // retry UI without rewriting that record with a generic localized hint.
        if (task.status !== 'failed' || !task.error) {
          const finalReason = task.password
            ? appStore.t('tasks.password.wrong')
            : appStore.t('tasks.password.required')
          taskStore.failTask(task.id, finalReason)
        }
      } else {
        // 非密码错误，直接失败
        if (task.status !== 'failed' || !task.error) taskStore.failTask(task.id, errorMsg)
        appStore.setError(`${appStore.t('common.error')}: ${archiveInspectionMessageForDisplay(errorMsg)}`)
      }
    }
      },
      task => task.outputPath.replace(/\\/g, '/').replace(/\/$/, '').toLocaleLowerCase(),
    )
  } finally {
    isProcessing.value = false
  }
}

const hasPendingTasks = computed(() => decompressionTasks.value.some(t => t.status === 'pending'))
const showBatchFailures = () => {
  selectedTaskIds.value = new Set()
  batchStore.showingFailures = true
}
const pausableTasks = computed(() => decompressionTasks.value.filter(t => ['running', 'extracting', 'preparing', 'finalizing'].includes(t.status)))
const pausedTasks = computed(() => decompressionTasks.value.filter(t => t.status === 'paused'))
const isRunning = computed(() => decompressionTasks.value.some(t => ['running', 'extracting', 'preparing', 'finalizing', 'paused', 'cancelling'].includes(t.status)))

const pauseAllTasks = async () => {
  await Promise.all(pausableTasks.value.map(task => taskStore.pauseTask(task.id)))
}

const resumeAllTasks = async () => {
  await Promise.all(pausedTasks.value.map(task => taskStore.resumeTask(task.id)))
}

const retryWithPassword = (taskId: string) => {
  const task = taskStore.tasks.find(item => item.id === taskId)
  if (!task?.password) {
    appStore.setError(appStore.t('tasks.password.required'))
    return
  }
  retrySuppliedPassword.value = task.password
  retryTaskIds.value = [task.id]
}

const cancelAllTasks = async () => {
  const cancellable = decompressionTasks.value.filter(t =>
    ['pending', 'running', 'extracting', 'preparing', 'finalizing', 'paused'].includes(t.status)
  )
  // Signal every backend process at once. Waiting for each cleanup in sequence
  // made the global stop appear unresponsive and left later tasks running.
  const results = await Promise.all(cancellable.map(task => taskStore.cancelTask(task.id)))
  const cancelled = results.filter(Boolean).length
  const failed = results.length - cancelled
  if (failed > 0) {
    appStore.setError(appStore.t('decompress.cancel_status').replace('{0}', String(cancelled)).replace('{1}', String(failed)))
  }
}

const handleConflict = (taskId: string) => {
  selectedConflictTaskId.value = taskId
  showConflictModal.value = true
}

// 处理冲突解决：收集逐项策略，并直接提交后端保留的暂存结果。
const handleConflictResolve = async (action: 'overwrite' | 'skip' | 'rename', applyToAll: boolean) => {
  if (resolvingConflictTaskId.value) return
  const taskId = selectedConflictTaskId.value
  if (!taskId) return
  const task = taskStore.tasks.find(t => t.id === taskId)
  if (!task) return

  if (['failed', 'cancelled', 'cancelling', 'completed', 'finalizing'].includes(task.status)) return

  const currentConflict = task.conflicts[0]
  if (!currentConflict) return
  const selections = task.conflictSelections || []
  selections.push({ destPath: currentConflict.destPath, action })
  task.conflictSelections = selections

  // 未选择“应用全部”时，让弹窗继续处理剩余冲突，最后一次再提交。
  if (!applyToAll && task.conflicts.length > 1) {
    task.conflicts.shift()
    return
  }

  // This choice belongs only to this task; never rewrite global defaults.
  task.conflictResolutionPending = true

  try {
    task.error = undefined
    await tauriCommands.resolveExtractionConflict(
      taskId,
      selections,
      applyToAll ? action : undefined,
    )
    task.conflicts = []
    task.conflictSelections = undefined
    task.password = undefined
    task.currentPassword = undefined
  } catch (e: any) {
    task.conflictSelections = undefined
    const reason = extractErrorMessage(e) || String(e)
    if (/File conflict requires resolution/i.test(reason)) {
      task.error = undefined
      appStore.setError(reason.replace(/.*File conflict requires resolution[:：]?\s*/i, '') || '目标文件发生变化，请重新确认冲突。')
    } else {
      task.error = reason
      appStore.setError(appStore.t('decompress.extract_failed').replace('{0}', e))
    }
  } finally {
    task.conflictResolutionPending = false
    showConflictModal.value = false
    selectedConflictTaskId.value = null
    if (!disposed) openNextConflict()
  }
}

// 监听冲突事件（仅当无弹窗显示时才打开，避免重复）
const openNextConflict = () => {
  if (resolvingConflictTaskId.value) return
  const taskWithConflict = taskStore.tasks.find(t => t.type === 'decompression' && t.conflicts.length > 0
    && !['failed', 'cancelled', 'cancelling', 'completed', 'finalizing'].includes(t.status))
  if (taskWithConflict && !showConflictModal.value) {
    handleConflict(taskWithConflict.id)
  }
}
const unsubConflict = taskStore.$subscribe(openNextConflict)
onMounted(openNextConflict)
</script>

<template>
  <div class="decompress-view p-3 sm:p-4 h-full flex flex-col gap-2.5 transition-colors duration-700 relative overflow-hidden">
    <header data-testid="decompression-header" class="compact-workspace-header flex justify-between items-center shrink-0">
      <div>
        <h1 class="text-xl md:text-2xl font-black text-content tracking-tight">{{ appStore.t('nav.decompress') }}</h1>
        <p class="text-xs text-muted font-semibold mt-0.5">{{ appStore.t('decompress.subtitle') }}</p>
      </div>
      <div class="flex gap-3">
        <button
          v-if="!batchStore.showingFailures && !isRunning && decompressionTasks.some(t => ['completed', 'failed', 'cancelled'].includes(t.status))"
          @click="taskStore.clearFinishedTasks('decompression')"
          class="h-9 px-5 rounded-lg bg-input border border-subtle text-muted text-xs font-bold uppercase tracking-wider hover:text-red-500 hover:border-red-500/30 transition-all flex items-center gap-2"
        >
          <i class="pi pi-trash text-xs"></i>
          {{ appStore.t('decompress.clear_finished') }}
        </button>
        <button
          v-if="pausableTasks.length > 0 && !batchStore.showingFailures"
          @click="pauseAllTasks"
          data-testid="pause-all-archive-tasks"
          class="h-9 px-4 rounded-lg bg-amber-500/10 text-amber-500 border border-amber-500/30 text-xs font-bold uppercase tracking-wider hover:bg-amber-500 hover:text-white transition-all flex items-center gap-2"
        >
          <i class="pi pi-pause text-xs"></i>
          {{ appStore.t('tasks.pause_all') }}
        </button>
        <button
          v-if="pausedTasks.length > 0 && !batchStore.showingFailures"
          @click="resumeAllTasks"
          data-testid="resume-all-archive-tasks"
          class="h-9 px-4 rounded-lg bg-green-500/10 text-green-500 border border-green-500/30 text-xs font-bold uppercase tracking-wider hover:bg-green-500 hover:text-white transition-all flex items-center gap-2"
        >
          <i class="pi pi-play text-xs"></i>
          {{ appStore.t('tasks.resume_all') }}
        </button>
        <button
          v-if="isRunning && !batchStore.showingFailures"
          @click="cancelAllTasks"
          class="h-9 px-5 rounded-lg bg-red-500/10 text-red-500 border border-red-500/30 text-xs font-bold uppercase tracking-wider hover:bg-red-500 hover:text-white transition-all flex items-center gap-2"
        >
          <i class="pi pi-stop-circle text-xs"></i>
          {{ appStore.t('decompress.stop_all') }}
        </button>
        <button
          v-if="hasPendingTasks && !isRunning && !batchStore.showingFailures"
          data-testid="start-decompression-queue"
          @click="startDecompression()"
          class="h-9 px-6 rounded-lg bg-primary text-white text-xs font-bold uppercase tracking-wider hover:brightness-110 active:scale-[0.98] transition-all shadow-lg shadow-primary/25 flex items-center gap-2"
        >
          <i class="pi pi-play-circle text-xs"></i>
          {{ appStore.t('decompress.start_queue') }}
        </button>
      </div>
    </header>

    <div data-testid="decompression-workspace-shell" class="flex-1 min-h-0 aero-card overflow-hidden flex flex-col relative border border-subtle bg-card/40 shadow-2xl">
      <DecompressionBatchSummary @show-failures="showBatchFailures" @show-all="batchStore.showingFailures = false"
        @retry-failures="openBatchRetry" />
      <div class="flex-1 overflow-hidden flex flex-col relative">
        <div v-if="batchStore.showingFailures && !batchStore.visibleFailureIds.length" class="flex-1 flex flex-col items-center justify-center gap-3 p-6 text-sm text-muted" data-testid="batch-failures-cleared">
          <p>{{ appStore.t('decompress.batch.no_visible_failures') }}</p>
          <RouterLink to="/history" class="text-primary underline">{{ appStore.t('nav.history') }}</RouterLink>
        </div>
        <!-- 显示所有任务，不再过滤只显示 pending -->
        <div v-else-if="decompressionTasks.length > 0" class="flex-1 min-h-0">
          <AeroTable
          :selectedTaskIds="selectedTaskIds"
          statusFilter="all"
          taskType="decompression"
          :taskIds="batchStore.showingFailures ? batchStore.visibleFailureIds : undefined"
          @toggle-task="toggleTaskSelection"
          @select-all-pending="selectAllPending"
          @deselect-all="deselectAll"
          @retry-with-password="retryWithPassword"
          @retry-created="selectRetryDrafts"
          @cancel-task="taskStore.cancelTask"
          @pause-task="taskStore.pauseTask"
          @resume-task="taskStore.resumeTask"
          >
            <template #before-tasks>
              <BatchConfigBar v-if="selectedPendingTasks.length && !batchStore.showingFailures" class="mb-3" :count="selectedPendingTasks.length" mode="decompression" @apply="applySelectedConfiguration" @clear="deselectAll" />
            </template>
          </AeroTable>
        </div>

        <!-- 空状态 -->
        <div v-else class="flex-1 flex flex-col items-center justify-center p-8">
          <EnhancedFileDropzone
            @files-selected="onFilesSelected"
            :accept="supportedArchiveAccept"
            :hint="appStore.t('decompress.drop_hint')"
            :sub-hint="supportedArchiveHint"
            picker-title="选择压缩包"
            class="w-full max-w-lg shadow-sm"
          />
        </div>
      </div>

      <!-- 底部操作区 -->
      <div v-if="decompressionTasks.length > 0 && !batchStore.showingFailures" class="border-t border-subtle bg-input/10 px-3 py-2 flex items-center gap-3 flex-wrap shrink-0">
          <div class="global-config-controls" aria-label="全局解压设置">
            <SavedLocationPicker :current="globalOutputPath" @select="setGlobalOutput" />
            <span class="text-xs font-black text-primary uppercase tracking-widest opacity-80 shrink-0">{{ appStore.t('decompress.config.output') }}</span>

            <button @click="handleGlobalSelectDir"
                    class="h-6 px-2.5 rounded-lg bg-primary text-white hover:brightness-110 active:scale-95 transition-all text-xs font-black flex items-center gap-1 shadow-sm shadow-primary/20 shrink-0">
              <i class="pi pi-folder-open text-xs"></i>
              <span class="hidden sm:inline">{{ appStore.t('decompress.config.output_select') }}</span>
              <span class="sm:hidden">选择</span>
            </button>

            <button @click="handleGlobalSetSameDir"
                    :class="isGlobalSameDir ? 'bg-primary/10 text-primary border-primary/20 shadow-inner' : 'bg-input/30 text-muted border-subtle/50'"
                    class="h-6 px-2.5 rounded-lg border text-xs font-bold transition-all hover:bg-primary/5 shrink-0">
              <span class="hidden sm:inline">{{ appStore.t('decompress.config.output_same') }}</span>
              <span class="sm:hidden">同目录</span>
            </button>

            <span class="text-xs font-mono text-content font-bold truncate flex-1 min-w-0" :title="isGlobalSameDir ? appStore.t('decompress.config.output_auto') : (globalOutputPath || appStore.t('decompress.config.output_auto'))">
              {{ isGlobalSameDir ? appStore.t('decompress.config.output_auto') : (globalOutputPath || appStore.t('decompress.config.output_auto')) }}
            </span>

            <button type="button" data-testid="global-subfolder" class="flex items-center gap-2 shrink-0" @click="toggleGlobalSubfolder">
              <span class="w-3 h-3 rounded border border-primary/30 flex items-center justify-center"
                   :class="globalExtractToSubfolder ? 'bg-primary border-primary' : 'bg-transparent'">
                <i v-if="globalExtractToSubfolder" class="pi pi-check text-[0.375rem] text-white"></i>
              </span>
              <span class="text-xs font-black text-muted uppercase tracking-widest whitespace-nowrap">{{ appStore.t('decompress.config.output_sub') }}</span>
            </button>

            <button
              type="button"
              role="switch"
              data-testid="global-recycle-source-switch"
              :aria-checked="globalRecycleSourceAfterExtract"
              :title="appStore.t('decompress.config.recycle_source.desc')"
              class="flex items-center gap-2 shrink-0 text-left"
              @click="toggleGlobalRecycleSource"
            >
              <span class="w-3 h-3 rounded border border-primary/30 flex items-center justify-center"
                    :class="globalRecycleSourceAfterExtract ? 'bg-primary border-primary' : 'bg-transparent'">
                <i v-if="globalRecycleSourceAfterExtract" class="pi pi-check text-[0.375rem] text-white"></i>
              </span>
              <span class="text-xs font-black text-muted uppercase tracking-widest whitespace-nowrap">{{ appStore.t('decompress.config.recycle_source') }}</span>
            </button>
          </div>
        <div class="w-px h-5 bg-subtle/20 mx-1 hidden md:block"></div>

        <EnhancedFileDropzone
          @files-selected="onFilesSelected"
          :compact="true"
          :accept="supportedArchiveAccept"
          :hint="appStore.t('compress.add_files')"
          :sub-hint="supportedArchiveHint"
          picker-title="选择压缩包"
          class="flex-1 min-w-[8rem] h-9"
        />
      </div>
    </div>

    <ConflictResolutionModal
      v-if="selectedConflictTaskId"
      v-model:visible="showConflictModal"
      :taskId="selectedConflictTaskId"
      :busy="resolvingConflictTaskId !== null"
      @resolve="handleConflictResolve"
    />
  </div>
    <FailedExtractionRetry v-if="retryTaskIds" :task-ids="retryTaskIds" :supplied-password="retrySuppliedPassword" @created="selectRetryDrafts" @close="closeRetry" />
</template>

<style scoped>
.global-config-controls {
  display: flex;
  min-width: 0;
  flex: 1 1 36rem;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.75rem;
}

.decompress-view {
  min-width: 0;
  overflow-x: hidden;
  background: radial-gradient(circle at 0% 0%, color-mix(in srgb, var(--dynamic-accent) 4%, transparent) 0%, transparent 40%);
}

.fade-morph-enter-active, .fade-morph-leave-active { transition: all 0.6s cubic-bezier(0.34, 1.56, 0.64, 1); }
.fade-morph-enter-from { opacity: 0; transform: scale(0.98); }
.fade-morph-leave-to { opacity: 0; transform: scale(1.02); }
</style>
