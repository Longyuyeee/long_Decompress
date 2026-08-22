<script setup lang="ts">
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
import { DECOMPRESS_ARCHIVE_ACCEPT, DECOMPRESS_ARCHIVE_HINT, isDecompressArchivePath, isPotentialSplitArchivePath } from '@/utils/compressionFormat'
import { appendResourcePreflightFallback, attachResourcePreflight } from '@/utils/resourcePreflight'
import { runArchiveTasks } from '@/utils/taskConcurrency'

const taskStore = useTaskStore()
const appStore = useAppStore()
const passwordStore = usePasswordStore()
const tauriCommands = useTauriCommands()

const selectedConflictTaskId = ref<string | null>(null)
const showConflictModal = ref(false)
const conflictSelections = new Map<string, Array<{
  destPath: string
  action: 'overwrite' | 'skip' | 'rename'
}>>()
const selectedTaskIds = ref<Set<string>>(new Set())
const supportedArchiveAccept = DECOMPRESS_ARCHIVE_ACCEPT
const supportedArchiveHint = DECOMPRESS_ARCHIVE_HINT
const decompressionTasks = computed(() => taskStore.tasksFor('decompression'))

// 全局配置状态
const globalOutputPath = ref('')
const isGlobalSameDir = ref(true) // 默认同目录，用户可通过按钮手动选择
const globalExtractToSubfolder = ref(false)
const globalRecycleSourceAfterExtract = ref(appStore.settings.autoDeleteSource)

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
        const extractToSubfolder = request.action !== 'context-extract-here'
        createdTasks.forEach(task => { task.extractToSubfolder = extractToSubfolder })
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
  unsubConflict()
})

const onFilesSelected = async (files: any[]) => {
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
    if (!isDecompressArchivePath(file.path) && !isPotentialSplitArchivePath(file.path)) {
      unsupportedCount++
      continue
    }
    uniqueFiles.set(normalizePath(file.path), file)
  }

  const handledPaths = new Set<string>()
  const existingSources = new Set(
    decompressionTasks.value.flatMap(task => task.sourceFiles.map(normalizePath))
  )

  const createTask = (sourcePath: string, name: string, splitCount?: number) => {
    const parentDir = sourcePath.substring(0, Math.max(sourcePath.lastIndexOf('/'), sourcePath.lastIndexOf('\\')))
    const taskId = taskStore.addTask({
      id: generateId(),
      name: splitCount ? `${name} (${splitCount} 个分卷)` : name,
      type: 'decompression',
      sourceFiles: [sourcePath],
      outputPath: isGlobalSameDir.value ? parentDir : globalOutputPath.value,
      extractToSubfolder: globalExtractToSubfolder.value,
      recycleSourceAfterExtract: globalRecycleSourceAfterExtract.value
    })
    createdTaskIds.push(taskId)
    appStore.addRecentFile(sourcePath)
    tauriCommands.listArchiveContents(sourcePath).then((contents: string[]) => {
      const task = taskStore.tasks.find(task => task.id === taskId)
      if (!task || task.status !== 'pending') return
      const rootEntries = contents.filter(item => !item.includes('/')).length
      if (rootEntries > 1) task.extractToSubfolder = true
      else if (rootEntries === 1) task.extractToSubfolder = false
    }).catch(error => console.debug('Smart extract skipped (unable to list contents):', sourcePath, error))
  }

  for (const file of uniqueFiles.values()) {
    const sourcePath = file.path as string
    const normalizedSource = normalizePath(sourcePath)
    if (handledPaths.has(normalizedSource) || existingSources.has(normalizedSource)) continue
    handledPaths.add(normalizedSource)

    try {
      const splitInfo = await tauriCommands.invoke<any>('detect_split_archive', { path: sourcePath })
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
        createTask(firstPart, displayName, Number(splitInfo.total_parts || groupPaths.length || 1))
        groupedSplitCount++
        existingSources.add(normalizedFirst)
        continue
      }
    } catch (error) {
      if (isPotentialSplitArchivePath(sourcePath) && !isDecompressArchivePath(sourcePath)) {
        errors.push(appStore.t('decompress.split.detection_failed')
          .replace('{0}', file.name || sourcePath)
          .replace('{1}', String(error)))
        continue
      }
      console.debug('Split archive detection skipped:', error)
    }
    if (isPotentialSplitArchivePath(sourcePath) && !isDecompressArchivePath(sourcePath)) {
      errors.push(appStore.t('decompress.split.group_not_found').replace('{0}', file.name || sourcePath))
      continue
    }
    createTask(sourcePath, file.name || sourcePath.split(/[\\/]/).pop() || 'Unknown')
  }

  if (unsupportedCount > 0) errors.push(appStore.t('decompress.unsupported_files').replace('{0}', String(unsupportedCount)))
  if (groupedSplitCount > 0) {
    appStore.setSuccess(appStore.t('decompress.split.grouped').replace('{0}', String(groupedSplitCount)))
  }
  if (errors.length > 0) appStore.setError(errors.slice(0, 3).join('；'))
  return createdTaskIds
}

const handleGlobalSelectDir = async () => {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: appStore.t('decompress.config.output_select')
    })
    if (selected && typeof selected === 'string') {
      globalOutputPath.value = selected
      isGlobalSameDir.value = false
      // 同步到所有待处理任务
      decompressionTasks.value.forEach(t => {
        if (t.status === 'pending') t.outputPath = selected
      })
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
    if (t.status === 'pending' && t.sourceFiles.length > 0) {
      const sp = t.sourceFiles[0]
      t.outputPath = sp.substring(0, Math.max(sp.lastIndexOf('/'), sp.lastIndexOf('\\')))
    }
  })
}

const toggleGlobalSubfolder = () => {
  globalExtractToSubfolder.value = !globalExtractToSubfolder.value
  decompressionTasks.value.forEach(t => {
    if (t.status === 'pending') t.extractToSubfolder = globalExtractToSubfolder.value
  })
}

const toggleGlobalRecycleSource = () => {
  globalRecycleSourceAfterExtract.value = !globalRecycleSourceAfterExtract.value
  appStore.updateSettings({ autoDeleteSource: globalRecycleSourceAfterExtract.value })
  decompressionTasks.value.forEach(t => {
    if (t.status === 'pending') t.recycleSourceAfterExtract = globalRecycleSourceAfterExtract.value
  })
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
  try {
    const report = await tauriCommands.preflightOperationResources({
      operation: 'decompression',
      outputPath: task.outputPath,
      sourcePaths: task.sourceFiles,
      password: task.password,
    })
    attachResourcePreflight(task, report)
    if (!report.canStart) {
      task.error = report.summary
      taskStore.updateTaskStatus(task.id, 'failed')
      appStore.setError(`${appStore.t('common.error')}: ${report.summary}`)
      return false
    }
  } catch (error) {
    appendResourcePreflightFallback(task, error)
  }
  return true
}

const startDecompression = async (onlyTaskIds?: string[]) => {
  // 防止重复点击
  if (isProcessing.value) return
  // 如果有选中的任务，优先处理选中的；否则处理所有 pending 任务
  const pendingTasks = onlyTaskIds
    ? decompressionTasks.value.filter(t => onlyTaskIds.includes(t.id) && t.status === 'pending')
    : selectedTaskIds.value.size > 0
    ? decompressionTasks.value.filter(t => selectedTaskIds.value.has(t.id) && t.status === 'pending')
    : decompressionTasks.value.filter(t => t.status === 'pending')
  if (pendingTasks.length === 0) return

  isProcessing.value = true

  // 启动后清除选择
  selectedTaskIds.value = new Set()

  try {
    await runArchiveTasks(
      pendingTasks,
      appStore.settings.maxConcurrentTasks,
      async task => {
    // 不预先添加密码，先尝试解压，只有明确要求密码时才使用保险箱
    const options = {
      outputPath: task.outputPath,
      keepStructure: true,
      overwrite: false,
      deleteAfter: task.recycleSourceAfterExtract ?? false,
      createSubdirectory: task.extractToSubfolder ?? false,
      password: task.password || undefined, // 只使用用户手动输入的密码
      fileFilter: task.fileFilter || null
      ,conflictPolicy: appStore.settings.conflictPolicy
    }

    try {
      if (!await runDecompressionResourcePreflight(task)) return
      task.passwordRequired = false
      await tauriCommands.decompressFile(task.sourceFiles[0], options, task.id)

      // 解压成功，递增保险箱中匹配密码的 use_count
      if (task.password) {
        const matchedEntry = passwordStore.entries.find(e => e.password === task.password)
        if (matchedEntry) {
          try {
            await passwordStore.updateEntry(matchedEntry.id, { use_count: (matchedEntry.use_count || 0) + 1 })
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
        taskStore.updateTaskStatus(task.id, 'failed')
        task.passwordRequired = true
        task.currentPassword = undefined
        task.error = task.password
          ? appStore.t('tasks.password.wrong')
          : appStore.t('tasks.password.required')
        if (!task.logs.some(log => log.message === task.error)) {
          task.logs.push({
            task_id: task.id,
            message: task.error,
            severity: 'warning',
            timestamp: new Date().toISOString()
          })
        }
      } else {
        // 非密码错误，直接失败
        taskStore.updateTaskStatus(task.id, 'failed')
        task.error = errorMsg
        appStore.setError(`${appStore.t('common.error')}: ${task.error}`)
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
const isRunning = computed(() => decompressionTasks.value.some(t => ['running', 'extracting', 'preparing', 'finalizing', 'cancelling'].includes(t.status)))

const retryWithPassword = async (taskId: string) => {
  const task = taskStore.tasks.find(item => item.id === taskId)
  if (!task?.password) {
    appStore.setError(appStore.t('tasks.password.required'))
    return
  }
  task.error = undefined
  task.passwordRequired = false
  taskStore.updateTaskStatus(task.id, 'pending')
  selectedTaskIds.value = new Set([task.id])
  await startDecompression()
}

const cancelAllTasks = async () => {
  let cancelled = 0
  let failed = 0
  for (const t of decompressionTasks.value) {
    if (['running', 'extracting', 'preparing', 'finalizing'].includes(t.status)) {
      const ok = await taskStore.cancelTask(t.id)
      if (ok) cancelled++; else failed++
    }
  }
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
  const taskId = selectedConflictTaskId.value
  if (!taskId) return
  const task = taskStore.tasks.find(t => t.id === taskId)
  if (!task) return

  const currentConflict = task.conflicts[0]
  if (!currentConflict) return
  const selections = conflictSelections.get(taskId) || []
  selections.push({ destPath: currentConflict.destPath, action })
  conflictSelections.set(taskId, selections)

  // 未选择“应用全部”时，让弹窗继续处理剩余冲突，最后一次再提交。
  if (!applyToAll && task.conflicts.length > 1) return

  // “应用全部”同时保存为之后任务的默认策略。
  if (applyToAll) {
    appStore.updateSettings({ conflictPolicy: action })
  }
  showConflictModal.value = false
  selectedConflictTaskId.value = null

  try {
    task.error = undefined
    await tauriCommands.resolveExtractionConflict(
      taskId,
      selections,
      applyToAll ? action : undefined,
    )
    task.conflicts = []
    conflictSelections.delete(taskId)
    task.password = undefined
    task.currentPassword = undefined
  } catch (e: any) {
    conflictSelections.delete(taskId)
    task.error = extractErrorMessage(e) || String(e)
    appStore.setError(appStore.t('decompress.extract_failed').replace('{0}', e))
  }
}

// 监听冲突事件（仅当无弹窗显示时才打开，避免重复）
const unsubConflict = taskStore.$subscribe((_mutation, state) => {
  const taskWithConflict = state.tasks.find(t => t.type === 'decompression' && t.conflicts.length > 0)
  if (taskWithConflict && !showConflictModal.value) {
    handleConflict(taskWithConflict.id)
  }
})
</script>

<template>
  <div class="decompress-view p-6 h-full flex flex-col gap-4 transition-colors duration-700 relative overflow-hidden">
    <header class="flex justify-between items-center shrink-0">
      <div>
        <h1 class="text-2xl md:text-3xl font-black text-content tracking-tight">{{ appStore.t('nav.decompress') }}</h1>
        <p class="text-xs md:text-sm text-muted font-semibold mt-1">{{ appStore.t('decompress.subtitle') }}</p>
      </div>
      <div class="flex gap-3">
        <button
          v-if="!isRunning && decompressionTasks.some(t => ['completed', 'failed', 'cancelled'].includes(t.status))"
          @click="taskStore.clearFinishedTasks('decompression')"
          class="h-9 px-5 rounded-lg bg-input border border-subtle text-muted text-xs font-bold uppercase tracking-wider hover:text-red-500 hover:border-red-500/30 transition-all flex items-center gap-2"
        >
          <i class="pi pi-trash text-xs"></i>
          {{ appStore.t('decompress.clear_finished') }}
        </button>
        <button
          v-if="isRunning"
          @click="cancelAllTasks"
          class="h-9 px-5 rounded-lg bg-red-500/10 text-red-500 border border-red-500/30 text-xs font-bold uppercase tracking-wider hover:bg-red-500 hover:text-white transition-all flex items-center gap-2"
        >
          <i class="pi pi-stop-circle text-xs"></i>
          {{ appStore.t('common.cancel') }}
        </button>
        <button
          v-if="hasPendingTasks && !isRunning"
          @click="startDecompression()"
          class="h-9 px-6 rounded-lg bg-primary text-white text-xs font-bold uppercase tracking-wider hover:brightness-110 active:scale-[0.98] transition-all shadow-lg shadow-primary/25 flex items-center gap-2"
        >
          <i class="pi pi-play-circle text-xs"></i>
          {{ appStore.t('decompress.start_queue') }}
        </button>
      </div>
    </header>

    <div class="flex-1 min-h-0 aero-card overflow-hidden flex flex-col relative border border-subtle bg-card/40 shadow-2xl">
      <div class="flex-1 overflow-hidden flex flex-col relative">
        <!-- 显示所有任务，不再过滤只显示 pending -->
        <div v-if="decompressionTasks.length > 0" class="flex-1 min-h-0">
          <AeroTable
          :selectedTaskIds="selectedTaskIds"
          statusFilter="all"
          taskType="decompression"
          @toggle-task="toggleTaskSelection"
          @select-all-pending="selectAllPending"
          @deselect-all="deselectAll"
          @retry-with-password="retryWithPassword"
        />
        </div>

        <!-- 空状态 -->
        <div v-else class="flex-1 flex flex-col items-center justify-center p-8">
          <EnhancedFileDropzone
            @files-selected="onFilesSelected"
            :accept="supportedArchiveAccept"
            :unfiltered-picker="true"
            class="w-full max-w-lg shadow-sm"
          />
        </div>
      </div>

      <!-- 底部操作区 -->
      <div v-if="decompressionTasks.length > 0" class="border-t border-subtle bg-input/10 px-3 py-3 flex items-center gap-3 flex-wrap shrink-0">
        <span class="text-xs font-black text-primary uppercase tracking-widest opacity-80 shrink-0 w-12">{{ appStore.t('decompress.config.output') }}</span>

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

        <span class="text-xs font-mono text-content font-bold truncate flex-1 min-w-0">
          {{ isGlobalSameDir ? appStore.t('decompress.config.output_auto') : (globalOutputPath || appStore.t('decompress.config.output_auto')) }}
        </span>

        <div class="flex items-center gap-2 cursor-pointer shrink-0" @click="toggleGlobalSubfolder">
          <div class="w-3 h-3 rounded border border-primary/30 flex items-center justify-center"
               :class="globalExtractToSubfolder ? 'bg-primary border-primary' : 'bg-transparent'">
            <i v-if="globalExtractToSubfolder" class="pi pi-check text-[0.375rem] text-white"></i>
          </div>
          <span class="text-xs font-black text-muted uppercase tracking-widest">{{ appStore.t('decompress.config.output_sub') }}</span>
        </div>

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

        <div class="w-px h-5 bg-subtle/20 mx-1 hidden md:block"></div>

        <EnhancedFileDropzone
          @files-selected="onFilesSelected"
          :compact="true"
          :accept="supportedArchiveAccept"
          :unfiltered-picker="true"
          class="flex-1 min-w-[8rem] h-9"
        />
      </div>
    </div>

    <ConflictResolutionModal
      v-if="selectedConflictTaskId"
      v-model:visible="showConflictModal"
      :taskId="selectedConflictTaskId"
      @resolve="handleConflictResolve"
    />
  </div>
</template>

<style scoped>
.decompress-view {
  min-width: 0;
  overflow-x: hidden;
  background: radial-gradient(circle at 0% 0%, color-mix(in srgb, var(--dynamic-accent) 4%, transparent) 0%, transparent 40%);
}

.fade-morph-enter-active, .fade-morph-leave-active { transition: all 0.6s cubic-bezier(0.34, 1.56, 0.64, 1); }
.fade-morph-enter-from { opacity: 0; transform: scale(0.98); }
.fade-morph-leave-to { opacity: 0; transform: scale(1.02); }
</style>
