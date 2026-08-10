<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useAppStore } from '@/stores/app'
import { useCompressionStore, type CompressionGroup, type FileObject } from '@/stores/compression'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { useTaskStore } from '@/stores/task'
import { extractErrorMessage, generateId } from '@/utils'
import { appendResourcePreflightFallback, attachResourcePreflight } from '@/utils/resourcePreflight'
import { effectiveFormatForPassword, extensionForFormat, isPasswordSupportedFormat, isSingleFileStreamFormat } from '@/utils/compressionFormat'
import {
  compressionStatusClass,
  compressionStatusIcon,
  isActiveCompressionStatus,
  isFinishedCompressionStatus,
} from '@/utils/compressionTaskPresentation'
import CompressionExecutionPanel from '@/components/compression/CompressionExecutionPanel.vue'
import CompressionAnalysisCard from '@/components/compression/CompressionAnalysisCard.vue'
import CompressionSettingsPanel from '@/components/compression/CompressionSettingsPanel.vue'
import CompressionStatusCell from '@/components/compression/CompressionStatusCell.vue'
import CompressionToolbar from '@/components/compression/CompressionToolbar.vue'
import GlobalSettingsModal from '@/components/compression/GlobalSettingsModal.vue'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'
import Modal from '@/components/ui/Modal.vue'
import { ask } from '@tauri-apps/api/dialog'

const appStore = useAppStore()
const compressionStore = useCompressionStore()
const tauriCommands = useTauriCommands()
const taskStore = useTaskStore()

const selectedRows = ref<Set<string>>(new Set())
const showGlobalSettingsModal = ref(false)
const rarSupport = ref<{ available: boolean; encoder_path?: string | null; message: string } | null>(null)
const checkingRarSupport = ref(false)
const isCompressing = ref(false)
const showRarResolution = ref(false)
const installingWinRar = ref(false)
const rarResolutionMessage = ref('')
type RarResolution = 'retry' | 'use-7z' | 'cancel'
let resolveRarResolution: ((choice: RarResolution) => void) | null = null

const compressionTasks = computed(() => taskStore.tasksFor('compression'))
const activeCompressionTasks = computed(() =>
  compressionTasks.value.filter(task => !['completed', 'failed', 'cancelled'].includes(task.status))
)
const hasFinishedCompressionTasks = computed(() =>
  compressionTasks.value.some(task => ['completed', 'failed', 'cancelled'].includes(task.status))
)
const compressionTaskById = computed(() =>
  new Map(compressionTasks.value.map(task => [task.id, task]))
)
const taskForJob = (taskId?: string) => taskId ? compressionTaskById.value.get(taskId) : undefined

const onFilesSelected = (files: any[]) => {
  files.forEach(f => {
    // 检查是否已经存在
    compressionStore.addFile({
      name: f.name,
      path: f.path,
      size: f.size || 0,
      type: f.type || 'file',
      isDirectory: f.isDirectory || false
    })
  })
}

const toggleSelection = (path: string) => {
  if (selectedRows.value.has(path)) selectedRows.value.delete(path)
  else selectedRows.value.add(path)
}

const handleCreateGroup = () => {
  if (selectedRows.value.size > 0) {
    compressionStore.createGroup(Array.from(selectedRows.value))
    selectedRows.value.clear()
  }
}

const getParentDir = (path: string) => {
  const index = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'))
  return index >= 0 ? path.substring(0, index) : '.'
}

const getBaseName = (path: string) => {
  const fileName = path.split(/[\\/]/).pop() || 'archive'
  return fileName.replace(/\.[^/.]+$/, '') || fileName
}

const joinPath = (dir: string, fileName: string) => {
  const separator = dir.includes('\\') ? '\\' : '/'
  return dir.endsWith('/') || dir.endsWith('\\') ? `${dir}${fileName}` : `${dir}${separator}${fileName}`
}

const normalizeOutputPath = (path: string) => path.replace(/\//g, '\\').toLocaleLowerCase()

const canUseSingleFileFormats = (files: Array<{ isDirectory: boolean }>) => {
  return files.length === 1 && !files[0]?.isDirectory
}

const canGlobalUseSingleFileFormats = computed(() => {
  return compressionStore.groups.every(group => canUseSingleFileFormats(group.files)) &&
    compressionStore.selectedFiles.every(file => canUseSingleFileFormats([file]))
})

const usesRarFormat = computed(() => {
  return compressionStore.globalSettings.format === 'rar' ||
    compressionStore.groups.some(group => compressionStore.getEffectiveSettings(group.settings).format === 'rar') ||
    compressionStore.selectedFiles.some(file => compressionStore.getEffectiveSettings(file.settings).format === 'rar')
})

const ensureRarSupport = async () => {
  if (rarSupport.value || checkingRarSupport.value) return rarSupport.value
  checkingRarSupport.value = true
  try {
    rarSupport.value = await tauriCommands.checkRarCompressionSupport()
  } catch (error) {
    rarSupport.value = {
      available: false,
      message: `Unable to check RAR support: ${error}`
    }
  } finally {
    checkingRarSupport.value = false
  }
  return rarSupport.value
}

const refreshRarSupport = async () => {
  rarSupport.value = null
  await ensureRarSupport()
}

const openRarDownloadPage = async () => {
  try {
    await tauriCommands.openRarDownloadPage()
  } catch (error) {
    appStore.setError(`Unable to open RAR download page: ${error}`)
  }
}

watch(usesRarFormat, (usesRar) => {
  if (usesRar) {
    void ensureRarSupport()
  }
}, { immediate: true })

const buildOutputPath = (baseOutputPath: string, fallbackSourcePath: string, archiveName: string, format: string, password?: string) => {
  const outputDir = baseOutputPath || appStore.settings.defaultOutputPath || getParentDir(fallbackSourcePath)
  const extension = extensionForFormat(format, password)
  const cleanName = archiveName.trim() || getBaseName(fallbackSourcePath)
  return joinPath(outputDir, cleanName.endsWith(`.${extension}`) ? cleanName : `${cleanName}.${extension}`)
}

const getFileNameFromPath = (path: string) => path.split(/[\\/]/).pop() || path

const getGroupArchivePath = (group: CompressionGroup) => {
  const task = taskForJob(group.taskId)
  if (task?.outputPath) return task.outputPath
  if (group.taskId && group.outputPath) return group.outputPath
  const settings = compressionStore.getEffectiveSettings(group.settings)
  return buildOutputPath(
    compressionStore.getEffectiveOutputPath(group.outputPath),
    group.files[0]?.path || group.name,
    settings.filename || group.name,
    settings.format,
    settings.password
  )
}

const getFileArchivePath = (file: FileObject) => {
  const task = taskForJob(file.taskId)
  if (task?.outputPath) return task.outputPath
  if (file.taskId && file.outputPath) return file.outputPath
  const settings = compressionStore.getEffectiveSettings(file.settings)
  return buildOutputPath(
    compressionStore.getEffectiveOutputPath(file.outputPath),
    file.path,
    settings.filename || getBaseName(file.path),
    settings.format,
    settings.password
  )
}

const getCompressionEstimate = (
  jobId: string,
  files: FileObject[],
  format: string,
  level: number,
) => {
  const analysis = compressionStore.compressionAnalysis[jobId]
  if (
    analysis?.status === 'completed' &&
    analysis.result &&
    analysis.format === format &&
    analysis.level === level
  ) {
    return { estimatedOutputBytes: analysis.result.estimatedSize, estimateReliable: false }
  }

  if (files.every(file => !file.isDirectory)) {
    const sourceBytes = files.reduce((total, file) => total + Math.max(0, file.size || 0), 0)
    return { estimatedOutputBytes: Math.ceil(sourceBytes * 1.05), estimateReliable: true }
  }
  return { estimatedOutputBytes: undefined, estimateReliable: false }
}

const runCompressionResourcePreflight = async (
  taskId: string,
  job: { id: string, files: FileObject[], outputPath: string, settings: { format: string, level: number } },
) => {
  const task = taskStore.tasks.find(item => item.id === taskId)
  if (!task) return false

  taskStore.updateTaskStatus(taskId, 'preparing')
  try {
    const estimate = getCompressionEstimate(job.id, job.files, job.settings.format, job.settings.level)
    const report = await tauriCommands.preflightOperationResources({
      operation: 'compression',
      outputPath: job.outputPath,
      sourcePaths: job.files.map(file => file.path),
      ...estimate,
    })
    attachResourcePreflight(task, report)
    if (!report.canStart) {
      task.error = report.summary
      taskStore.updateTaskStatus(taskId, 'failed')
      appStore.setError(`${appStore.t('common.error')}: ${report.summary}`)
      return false
    }
  } catch (error) {
    appendResourcePreflightFallback(task, error)
  }
  return true
}

const runCompression = async () => {
  if (compressionStore.groups.length === 0 && compressionStore.selectedFiles.length === 0) return

  let jobs = [
    ...compressionStore.groups.filter(group => !group.taskId).map(group => {
      const settings = compressionStore.getEffectiveSettings(group.settings)
      return {
        id: group.id,
        name: group.name,
        files: group.files,
        settings,
        outputPath: buildOutputPath(
          compressionStore.getEffectiveOutputPath(group.outputPath),
          group.files[0]?.path || group.name,
          settings.filename || group.name,
          settings.format,
          settings.password
        )
      }
    }),
    ...compressionStore.selectedFiles.filter(file => !file.taskId).map(file => {
      const settings = compressionStore.getEffectiveSettings(file.settings)
      return {
        id: file.path,
        name: file.name,
        files: [file],
        settings,
        outputPath: buildOutputPath(
          compressionStore.getEffectiveOutputPath(file.outputPath),
          file.path,
          settings.filename || getBaseName(file.path),
          settings.format,
          settings.password
        )
      }
    })
  ]

  if (jobs.length === 0) return

  if (jobs.some(job => job.settings.format === 'rar')) {
    const support = await ensureRarSupport()
    if (!support?.available) {
      const resolution = await requestRarResolution(support?.message || appStore.t('compress.error.rar_requires'))
      if (resolution === 'cancel') return
      if (resolution === 'use-7z') {
        jobs = jobs.map(job => job.settings.format === 'rar' ? {
          ...job,
          settings: { ...job.settings, format: '7z' },
          outputPath: job.outputPath.replace(/\.rar$/i, '.7z'),
        } : job)
      }
    }
  }

  let allowRarPasswordCli = false
  if (jobs.some(job => job.settings.format === 'rar' && Boolean(job.settings.password))) {
    allowRarPasswordCli = await ask(
      'WinRAR 的命令行编码器没有安全的密码输入通道。继续创建加密 RAR 时，密码会在本机进程参数中短暂可见。建议改用加密 ZIP 或 7Z。是否仍要继续？',
      { title: '加密 RAR 安全提示', type: 'warning' }
    )
    if (!allowRarPasswordCli) return
  }

  // 第一阶段：预校验并添加所有任务到列表（status: pending）
  const validJobs: Array<{ job: typeof jobs[0], taskId: string, effectiveFormat: string }> = []
  let failed = 0
  const outputPaths = new Set<string>()
  const activeOutputPaths = new Set(
    taskStore.tasks
      .filter(task =>
        task.type === 'compression' &&
        !['completed', 'failed', 'cancelled'].includes(task.status)
      )
      .map(task => normalizeOutputPath(task.outputPath))
  )

  for (const job of jobs) {
    const effectiveFormat = effectiveFormatForPassword(job.settings.format, job.settings.password)
    const normalizedOutputPath = normalizeOutputPath(job.outputPath)
    if (outputPaths.has(normalizedOutputPath)) {
      appStore.setError(`${appStore.t('common.error')}: Duplicate output path: ${job.outputPath}`)
      failed++
      continue
    }
    if (activeOutputPaths.has(normalizedOutputPath)) {
      appStore.setError(`${appStore.t('common.error')}: Another compression task is already writing this output: ${job.outputPath}`)
      failed++
      continue
    }
    outputPaths.add(normalizedOutputPath)
    activeOutputPaths.add(normalizedOutputPath)

    // 校验失败：跳过当前任务
    if (isSingleFileStreamFormat(job.settings.format) && !canUseSingleFileFormats(job.files)) {
      appStore.setError(appStore.t('compress.error.single_file').replace('{0}', job.settings.format.toUpperCase()))
      failed++
      continue
    }

    if (job.settings.password && !isPasswordSupportedFormat(effectiveFormat)) {
      appStore.setError(appStore.t('compress.error.no_password').replace('{0}', job.settings.format.toUpperCase()))
      failed++
      continue
    }

    // 添加任务到列表（pending 状态）
    const taskId = taskStore.addTask({
      id: generateId(),
      name: job.name,
      type: 'compression',
      sourceFiles: job.files.map(file => file.path),
      outputPath: job.outputPath,
      format: effectiveFormat,
      password: job.settings.password || undefined,
      compressionOptions: {
        format: effectiveFormat,
        level: job.settings.level,
        password: job.settings.password || undefined,
        split_size: job.settings.splitArchive ? Number(job.settings.splitSize) : null,
        preserve_paths: job.settings.keepStructure,
        delete_after: job.settings.deleteAfter,
        verify_after: job.settings.verifyAfter,
        allow_insecure_password_cli: effectiveFormat === 'rar' && allowRarPasswordCli
      }
    })

    compressionStore.bindJobTask(job.id, taskId, job.settings, job.outputPath)
    validJobs.push({ job, taskId, effectiveFormat })
  }

  // A validated draft becomes a task exactly once while remaining in the same
  // visible row. The taskId binding prevents a second submission and drives the
  // row's real status, progress and logs for its entire lifecycle.
  validJobs.forEach(({ job }) => selectedRows.value.delete(job.id))

  // 第二阶段：依次执行所有任务
  let succeeded = 0

  for (const { job, taskId, effectiveFormat } of validJobs) {
    const queuedTask = taskStore.tasks.find(task => task.id === taskId)
    if (!queuedTask || queuedTask.status === 'cancelled') {
      continue
    }
    try {
      if (!await runCompressionResourcePreflight(taskId, job)) {
        failed++
        continue
      }
      taskStore.updateTaskStatus(taskId, 'compressing')
      await tauriCommands.compressFiles(
        taskId,
        job.files.map(file => file.path),
        job.outputPath,
        {
          format: effectiveFormat,
          level: job.settings.level,
          password: job.settings.password || undefined,
          split_size: job.settings.splitArchive ? Number(job.settings.splitSize) : null,
          preserve_paths: job.settings.keepStructure,
          delete_after: job.settings.deleteAfter,
          verify_after: job.settings.verifyAfter,
          allow_insecure_password_cli: effectiveFormat === 'rar' && allowRarPasswordCli
        }
      )
      const finishedTask = taskStore.tasks.find(task => task.id === taskId)
      if (finishedTask && !['cancelled', 'cancelling'].includes(finishedTask.status)) {
        taskStore.updateTaskStatus(taskId, 'completed')
        if (!job.settings.splitArchive && compressionStore.compressionAnalysis[job.id]?.result) {
          const outputInfo = await tauriCommands.getFileInfo(job.outputPath)
          if (outputInfo) compressionStore.recordActualSize(job.id, outputInfo.size)
        }
        succeeded++
      }
    } catch (error) {
      const task = taskStore.tasks.find(t => t.id === taskId)
      if (task && !['cancelled', 'cancelling'].includes(task.status)) {
        taskStore.updateTaskStatus(taskId, 'failed')
      }
      if (task && !['cancelled', 'cancelling'].includes(task.status)) {
        task.error = extractErrorMessage(error)
        appStore.setError(`${appStore.t('common.error')}: ${extractErrorMessage(error)}`)
        failed++
      }
      // 继续处理下一个任务，不中断整个批次
    }
  }

  if (succeeded > 0 && failed === 0) {
    appStore.setSuccess(appStore.t('compress.status_success').replace('{0}', String(succeeded)).replace('{1}', succeeded === 1 ? '' : 's'))
  } else if (succeeded > 0) {
    appStore.setSuccess(appStore.t('compress.status_result').replace('{0}', String(succeeded)).replace('{1}', String(failed)))
  }
}

const handleCompress = async () => {
  if (isCompressing.value) return
  isCompressing.value = true
  try {
    await runCompression()
  } finally {
    isCompressing.value = false
  }
}

const finishRarResolution = (choice: RarResolution) => {
  showRarResolution.value = false
  resolveRarResolution?.(choice)
  resolveRarResolution = null
}

const requestRarResolution = (message: string) => {
  rarResolutionMessage.value = message
  showRarResolution.value = true
  return new Promise<RarResolution>(resolve => {
    resolveRarResolution = resolve
  })
}

const installWinRar = async () => {
  if (installingWinRar.value) return
  installingWinRar.value = true
  try {
    rarSupport.value = await tauriCommands.installWinRarWithWinget()
    finishRarResolution('retry')
  } catch (error) {
    rarResolutionMessage.value = extractErrorMessage(error)
  } finally {
    installingWinRar.value = false
  }
}

const retryRarDetection = async () => {
  await refreshRarSupport()
  if (rarSupport.value?.available) finishRarResolution('retry')
  else rarResolutionMessage.value = rarSupport.value?.message || appStore.t('compress.error.rar_requires')
}

const consumePendingAutoStart = () => {
  if (!isCompressing.value && compressionStore.consumeAutoStart()) {
    setTimeout(() => void handleCompress(), 100)
  }
}

onMounted(consumePendingAutoStart)

watch(
  [() => compressionStore.autoStartRequested, isCompressing],
  ([requested, compressing]) => {
    if (requested && !compressing) consumePendingAutoStart()
  }
)

const totalPayload = computed(() => {
  return compressionStore.selectedFiles.length + compressionStore.groups.reduce((acc, g) => acc + g.files.length, 0)
})

const pendingPayload = computed(() => {
  const files = compressionStore.selectedFiles.filter(file => !file.taskId).length
  const groupedFiles = compressionStore.groups
    .filter(group => !group.taskId)
    .reduce((total, group) => total + group.files.length, 0)
  return files + groupedFiles
})

const cancelCompressionTask = async (taskId: string) => {
  const task = taskStore.tasks.find(item => item.id === taskId)
  if (task?.status === 'pending') {
    taskStore.updateTaskStatus(taskId, 'cancelled')
    return
  }
  const cancelled = await taskStore.cancelTask(taskId)
  if (!cancelled) {
    appStore.setError('取消压缩失败，请查看任务日志。')
  }
}

const cancelAllCompressionTasks = async () => {
  for (const task of activeCompressionTasks.value) {
    await cancelCompressionTask(task.id)
  }
}

const clearFinishedCompressionTasks = () => {
  const finishedTaskIds = compressionTasks.value
    .filter(task => isFinishedCompressionStatus(task.status))
    .map(task => task.id)
  compressionStore.removeJobsByTaskIds(finishedTaskIds)
  taskStore.clearFinishedTasks('compression')
}

const removeFinishedCompressionJob = (taskId: string) => {
  const task = compressionTaskById.value.get(taskId)
  if (!task || !isFinishedCompressionStatus(task.status)) return
  compressionStore.removeJobsByTaskIds([taskId])
  taskStore.removeTask(taskId)
}

const onBeforeDetailEnter = (element: Element) => {
  const el = element as HTMLElement
  el.style.height = '0'
  el.style.opacity = '0'
  el.style.marginTop = '0'
  el.style.marginBottom = '0'
}

const onDetailEnter = (element: Element) => {
  const el = element as HTMLElement
  el.style.height = `${el.scrollHeight}px`
  el.style.opacity = '1'
  el.style.marginTop = '0.25rem'
  el.style.marginBottom = '0.5rem'
}

const onAfterDetailEnter = (element: Element) => {
  const el = element as HTMLElement
  el.style.height = 'auto'
}

const onBeforeDetailLeave = (element: Element) => {
  const el = element as HTMLElement
  el.style.height = `${el.scrollHeight}px`
  el.style.opacity = '1'
  el.style.marginTop = '0.25rem'
  el.style.marginBottom = '0.5rem'
}

const onDetailLeave = (element: Element) => {
  const el = element as HTMLElement
  void el.offsetHeight
  el.style.height = '0'
  el.style.opacity = '0'
  el.style.marginTop = '0'
  el.style.marginBottom = '0'
}
</script>

<template>
  <div class="compression-view p-4 md:p-6 h-full flex flex-col gap-4 transition-colors duration-700 overflow-hidden relative">
    <header class="flex justify-between items-center gap-3 shrink-0">
      <div class="min-w-0">
        <h1 class="text-2xl md:text-3xl font-black text-content tracking-tight">{{ appStore.t('nav.compress') }}</h1>
        <p class="text-xs md:text-sm text-muted font-semibold mt-1">{{ appStore.t('compress.subtitle') }}</p>
      </div>
      <CompressionToolbar
        :has-finished="hasFinishedCompressionTasks"
        :active-count="activeCompressionTasks.length"
        :pending-count="pendingPayload"
        :busy="isCompressing"
        @clear-finished="clearFinishedCompressionTasks"
        @cancel-active="cancelAllCompressionTasks"
        @open-settings="showGlobalSettingsModal = true"
        @start="handleCompress"
      />
    </header>

    <!-- 主工作区 -->
    <div class="flex-1 min-h-0 aero-card overflow-hidden flex flex-col relative border border-subtle bg-card/40 shadow-2xl">
      <div v-if="totalPayload > 0" class="compression-task-list flex-1 min-w-0 overflow-y-auto overflow-x-hidden custom-scrollbar p-3 space-y-3">
        <div
          data-testid="compression-table-header"
          class="compression-table-header sticky top-0 z-20 flex items-center px-4 py-2.5 border-b border-subtle bg-card/95 backdrop-blur-xl text-dim text-xs font-bold tracking-[0.1em] uppercase"
        >
          <div class="compression-leading-cell w-8 shrink-0"></div>
          <div data-testid="compression-name-header" class="flex-[1.25] min-w-0">压缩包名称</div>
          <div data-testid="compression-source-header" class="flex-1 min-w-0 px-4 hidden md:block">源文件路径</div>
          <div data-testid="compression-status-header" class="flex-1 min-w-0">压缩状态与进度</div>
          <div class="compression-row-actions w-20 shrink-0"></div>
        </div>

        <!-- 1. 压缩组列表 -->
        <div v-for="group in compressionStore.groups" :key="group.id" 
             data-testid="compression-group-row"
             class="compression-job-card group-container rounded-lg border transition-all duration-200 overflow-hidden"
             :class="group.expanded ? 'bg-card/60 border-primary/30 shadow-lg' : 'bg-card/40 border-subtle/40 hover:border-primary/30 hover:bg-card/60'"
             :style="{ borderColor: group.expanded ? group.themeColor : '' }">
          
          <!-- 组头部 -->
          <div class="compression-job-row flex items-center px-4 py-2.5 cursor-pointer group/header relative"
               role="button" tabindex="0" :aria-expanded="group.expanded"
               @click="group.expanded = !group.expanded"
               @keydown.enter="group.expanded = !group.expanded"
               @keydown.space.prevent="group.expanded = !group.expanded">
            <div class="absolute left-0 top-0 bottom-0 w-1 bg-primary opacity-0 group-hover/header:opacity-100 transition-opacity duration-200"></div>

            <div class="compression-leading-cell w-8 shrink-0 flex items-center justify-center">
              <div
                class="w-7 h-7 rounded-lg flex items-center justify-center shadow-sm transition-transform group-hover/header:rotate-6"
                :style="{ backgroundColor: `${group.themeColor}20`, color: group.themeColor, border: `1px solid ${group.themeColor}40` }"
              >
                <i class="pi pi-briefcase text-xs"></i>
              </div>
            </div>

            <div data-testid="compression-archive-name" class="flex-[1.25] min-w-0 overflow-hidden flex items-center gap-3">
              <div class="min-w-0">
                <div
                  class="text-sm font-black text-content tracking-tight truncate group-hover/header:text-primary transition-colors"
                  :title="getGroupArchivePath(group)"
                >
                  {{ getFileNameFromPath(getGroupArchivePath(group)) }}
                </div>
                <div class="flex items-center gap-2 mt-0.5">
                  <span class="text-xs font-bold text-muted">{{ group.files.length }} {{ appStore.t('compress.group_count') }}</span>
                  <span class="text-xs font-mono text-primary font-black uppercase">{{ compressionStore.getEffectiveSettings(group.settings).format }}</span>
                </div>
              </div>
            </div>

            <div
              data-testid="compression-source-path"
              class="flex-1 min-w-0 px-4 hidden md:flex items-center gap-2 text-muted text-xs font-mono font-light opacity-75"
              :title="group.files.map(file => file.path).join('\n')"
            >
              <span class="truncate">{{ group.files[0]?.path }}</span>
              <span v-if="group.files.length > 1" class="shrink-0 text-primary font-bold">+{{ group.files.length - 1 }}</span>
            </div>

            <CompressionStatusCell :task="taskForJob(group.taskId)" />

            <div class="compression-row-actions w-20 shrink-0 flex items-center justify-end gap-3">
              <button
                v-if="!group.taskId"
                @click.stop="compressionStore.dissolveGroup(group.id)"
                class="text-muted hover:text-red-500 transition-colors"
                title="解散分组"
              >
                <i class="pi pi-trash text-xs"></i>
              </button>
              <button
                v-else-if="isActiveCompressionStatus(taskForJob(group.taskId)?.status)"
                data-testid="compression-job-cancel"
                @click.stop="cancelCompressionTask(group.taskId)"
                class="text-red-400 hover:text-red-500 transition-colors"
                title="取消压缩"
              >
                <i class="pi pi-stop-circle text-xs"></i>
              </button>
              <button
                v-else-if="isFinishedCompressionStatus(taskForJob(group.taskId)?.status)"
                @click.stop="removeFinishedCompressionJob(group.taskId)"
                class="text-muted hover:text-red-500 transition-colors"
                title="清除任务"
              >
                <i class="pi pi-trash text-xs"></i>
              </button>
              <i class="pi transition-transform duration-500 text-muted text-sm" :class="group.expanded ? 'pi-chevron-up' : 'pi-chevron-down'"></i>
            </div>
          </div>

          <!-- 组展开：独立配置面板 -->
          <Transition
            name="aero-drawer"
            @before-enter="onBeforeDetailEnter"
            @enter="onDetailEnter"
            @after-enter="onAfterDetailEnter"
            @before-leave="onBeforeDetailLeave"
            @leave="onDetailLeave"
          >
            <div v-if="group.expanded" class="details-drawer px-3 md:px-6 pb-5 pt-2">
              <div data-testid="compression-draft-details" class="compression-detail-card compression-detail-grid">
                <div
                  data-testid="compression-draft-config"
                  class="compression-config-panel min-w-0 space-y-5"
                  :class="{ 'is-submitted opacity-80': Boolean(group.taskId) }"
                >
                  <div>
                    <h4 class="detail-heading justify-between">
                      <span class="flex items-center gap-2">
                        <i class="pi pi-cog text-sm"></i>
                        {{ appStore.t('decompress.column.config') }}
                      </span>
                      <span v-if="group.taskId" class="text-xs font-bold text-muted tracking-normal">
                        {{ appStore.t('compress.config_submitted') }}
                      </span>
                    </h4>
                    <CompressionAnalysisCard
                      class="mb-4"
                      :job-id="group.id"
                      :paths="group.files.map(file => file.path)"
                      :model-value="compressionStore.getEffectiveSettings(group.settings)"
                      :disabled="Boolean(group.taskId)"
                      @update:model-value="compressionStore.updateGroupSettings(group.id, $event)"
                    />
                    <CompressionSettingsPanel
                      :modelValue="compressionStore.getEffectiveSettings(group.settings)"
                      :outputPath="compressionStore.getEffectiveOutputPath(group.outputPath)"
                      :allow-single-file-formats="canUseSingleFileFormats(group.files)"
                      :suggested-filename="group.name"
                      @update:modelValue="compressionStore.updateGroupSettings(group.id, $event)"
                      @update:outputPath="compressionStore.updateGroupOutputPath(group.id, $event)"
                    />
                  </div>

                  <div class="space-y-2">
                    <h4 class="text-xs font-black text-muted uppercase tracking-widest mb-2">{{ appStore.t('compress.group_files') }}</h4>
                    <div v-for="file in group.files" :key="file.path" class="text-sm text-muted font-mono py-1 px-3 bg-card/40 rounded-lg border border-subtle/50 flex min-w-0 items-center justify-between gap-2 group/file">
                      <div class="flex items-center gap-2 overflow-hidden min-w-0">
                        <i :class="file.isDirectory ? 'pi pi-folder text-primary/60' : 'pi pi-file text-muted/60'" class="text-xs shrink-0"></i>
                        <span class="truncate">{{ file.name }}</span>
                      </div>
                      <div class="flex max-w-[55%] items-center gap-2 min-w-0">
                        <span class="opacity-75 italic ml-2 truncate" :title="file.path">{{ file.path }}</span>
                        <button
                          @click.stop="compressionStore.removeFileFromGroup(group.id, file.path)"
                          class="w-5 h-5 rounded-md flex items-center justify-center text-dim hover:text-red-400 hover:bg-red-500/10 opacity-0 group-hover/file:opacity-100 transition-all shrink-0"
                          :title="appStore.t('compress.remove_from_group')"
                        >
                          <i class="pi pi-times text-xs"></i>
                        </button>
                      </div>
                    </div>
                  </div>
                </div>

                <CompressionExecutionPanel :task="taskForJob(group.taskId)" />
              </div>
            </div>
          </Transition>
        </div>

        <!-- 2. 未分组文件列表 (待分配) -->
        <div v-if="compressionStore.selectedFiles.length > 0" class="space-y-3">
          <div data-testid="compression-grouping-actions" class="flex items-center justify-between gap-3 px-4">
            <h3 class="text-xs font-black text-muted uppercase tracking-[0.3em]">{{ appStore.t('compress.add_files') }}</h3>
            <transition name="pop">
              <button
                v-if="selectedRows.size > 0"
                @click="handleCreateGroup"
                class="h-8 px-3 rounded-lg bg-primary text-white text-xs font-bold tracking-wider shadow-md shadow-primary/20 hover:brightness-110 active:scale-[0.98] transition-all flex items-center gap-2 shrink-0"
              >
                <i class="pi pi-box text-xs"></i>
                <span>{{ appStore.t('compress.create_group') }}（{{ selectedRows.size }}）</span>
              </button>
            </transition>
          </div>
          <div v-for="file in compressionStore.selectedFiles" :key="file.path" 
               data-testid="compression-draft-row"
               :data-task-id="file.taskId || ''"
               @click="file.expanded = !file.expanded"
               class="compression-job-card compression-job-row flex flex-wrap items-center justify-between px-4 py-2.5 rounded-lg bg-card/40 border border-subtle/40 group/row hover:border-primary/30 hover:bg-card/60 transition-all duration-200 cursor-pointer relative overflow-hidden"
               :class="{ 'border-primary/30 bg-card/60 shadow-lg': file.expanded }">
            <div class="absolute left-0 top-0 bottom-0 w-1 bg-primary opacity-0 group-hover/row:opacity-100 transition-opacity duration-200"></div>
            
            <button
              v-if="!file.taskId"
              type="button"
              data-testid="compression-group-checkbox"
              class="compression-leading-cell w-8 flex shrink-0 items-center justify-center"
              :aria-label="`选择 ${file.name} 用于打组`"
              @click.stop="toggleSelection(file.path)"
            >
              <div class="w-4 h-4 rounded border border-subtle flex items-center justify-center transition-all"
                   :class="selectedRows.has(file.path) ? 'bg-primary border-primary' : 'bg-card'">
                <i v-if="selectedRows.has(file.path)" class="pi pi-check text-xs text-white"></i>
              </div>
            </button>
            <div v-else class="compression-leading-cell w-8 flex shrink-0 items-center justify-center">
              <i
                class="pi text-sm"
                :class="[compressionStatusIcon(taskForJob(file.taskId)?.status), compressionStatusClass(taskForJob(file.taskId)?.status)]"
              ></i>
            </div>

            <div data-testid="compression-archive-name" class="flex-[1.25] min-w-0 overflow-hidden flex items-center gap-3">
              <div class="w-7 h-7 rounded-lg bg-input/60 border border-subtle/50 flex items-center justify-center shrink-0">
                <i class="pi pi-box text-primary text-xs"></i>
              </div>
              <div class="min-w-0">
                <div
                  class="text-content font-bold truncate text-sm tracking-tight group-hover/row:text-primary transition-colors"
                  :title="getFileArchivePath(file)"
                >
                  {{ getFileNameFromPath(getFileArchivePath(file)) }}
                </div>
                <div class="flex items-center gap-2 mt-0.5">
                  <span class="text-xs text-muted truncate">{{ file.isDirectory ? '文件夹' : '单个文件' }}</span>
                  <span class="text-xs font-mono text-primary font-black uppercase">
                    {{ compressionStore.getEffectiveSettings(file.settings).format }}
                  </span>
                </div>
              </div>
            </div>

            <div
              data-testid="compression-source-path"
              class="flex-1 min-w-0 px-4 hidden md:block text-muted text-xs truncate font-mono font-light opacity-75"
              :title="file.path"
            >
              {{ file.path }}
            </div>

            <CompressionStatusCell :task="taskForJob(file.taskId)" />

            <div class="compression-row-actions w-20 shrink-0 flex items-center justify-end">
              <button
                v-if="!file.taskId"
                @click.stop="compressionStore.removeFile(file.path)"
                class="w-8 h-8 rounded-lg flex items-center justify-center text-dim hover:text-red-500 transition-all"
                title="移除文件"
              >
                <i class="pi pi-times text-sm"></i>
              </button>
              <button
                v-else-if="isActiveCompressionStatus(taskForJob(file.taskId)?.status)"
                data-testid="compression-job-cancel"
                @click.stop="cancelCompressionTask(file.taskId)"
                class="w-8 h-8 rounded-lg flex items-center justify-center text-red-400 hover:text-red-500 transition-all"
                title="取消压缩"
              >
                <i class="pi pi-stop-circle text-sm"></i>
              </button>
              <button
                v-else-if="isFinishedCompressionStatus(taskForJob(file.taskId)?.status)"
                @click.stop="removeFinishedCompressionJob(file.taskId)"
                class="w-8 h-8 rounded-lg flex items-center justify-center text-dim hover:text-red-500 transition-all"
                title="清除任务"
              >
                <i class="pi pi-trash text-sm"></i>
              </button>
              <button
                @click.stop="file.expanded = !file.expanded"
                class="w-8 h-8 rounded-lg flex items-center justify-center text-dim hover:text-primary transition-all"
                :aria-label="file.expanded ? '收起压缩详情' : '展开压缩详情'"
              >
                <i class="pi text-sm transition-transform" :class="file.expanded ? 'pi-chevron-up' : 'pi-chevron-down'"></i>
              </button>
            </div>

            <Transition
              name="aero-drawer"
              @before-enter="onBeforeDetailEnter"
              @enter="onDetailEnter"
              @after-enter="onAfterDetailEnter"
              @before-leave="onBeforeDetailLeave"
              @leave="onDetailLeave"
            >
              <div v-if="file.expanded" class="details-drawer w-full px-2 md:px-5 pb-4 pt-2" @click.stop>
                <div data-testid="compression-draft-details" class="compression-detail-card compression-detail-grid">
                  <div
                    data-testid="compression-draft-config"
                    class="compression-config-panel min-w-0"
                    :class="{ 'is-submitted opacity-80': Boolean(file.taskId) }"
                  >
                    <h4 class="detail-heading justify-between">
                      <span class="flex items-center gap-2">
                        <i class="pi pi-cog text-sm"></i>
                        {{ appStore.t('decompress.column.config') }}
                      </span>
                      <span v-if="file.taskId" class="text-xs font-bold text-muted tracking-normal">
                        {{ appStore.t('compress.config_submitted') }}
                      </span>
                    </h4>
                    <CompressionAnalysisCard
                      class="mb-4"
                      :job-id="file.path"
                      :paths="[file.path]"
                      :model-value="compressionStore.getEffectiveSettings(file.settings)"
                      :disabled="Boolean(file.taskId)"
                      @update:model-value="compressionStore.updateFileSettings(file.path, $event)"
                    />
                    <CompressionSettingsPanel
                      :modelValue="compressionStore.getEffectiveSettings(file.settings)"
                      :outputPath="compressionStore.getEffectiveOutputPath(file.outputPath)"
                      :allow-single-file-formats="canUseSingleFileFormats([file])"
                      :suggested-filename="getBaseName(file.path)"
                      @update:modelValue="compressionStore.updateFileSettings(file.path, $event)"
                      @update:outputPath="compressionStore.updateFileOutputPath(file.path, $event)"
                    />
                  </div>

                  <CompressionExecutionPanel :task="taskForJob(file.taskId)" />
                </div>
              </div>
            </Transition>
          </div>
        </div>

      </div>

      <!-- 3. 空状态 (引导式双列布局) -->
      <div v-else class="flex-1 min-h-0 flex flex-col items-center justify-center p-2 sm:p-3 gap-2 sm:gap-3">
        <div class="text-center space-y-1 shrink-0">
          <h2 class="text-sm sm:text-base md:text-xl font-black text-content tracking-tight">{{ appStore.t('compress.start') }}</h2>
          <p class="text-xs md:text-sm text-muted font-bold uppercase tracking-widest">{{ appStore.t('compress.select_to_begin') }}</p>
        </div>
        <div class="flex-1 min-h-0 w-full max-w-4xl flex flex-col sm:flex-row gap-2 sm:gap-3 px-2">
          <EnhancedFileDropzone
            @files-selected="onFilesSelected"
            mode="folder"
            class="shadow-sm flex-1 min-h-[160px] sm:min-h-0"
          />
          <EnhancedFileDropzone
            @files-selected="onFilesSelected"
            mode="file"
            :hint="appStore.t('compress.drop_file_hint')"
            :nativeDrop="false"
            class="shadow-sm flex-1 min-h-[160px] sm:min-h-0"
          />
        </div>
      </div>

      <!-- 底部辅助区 -->
      <div v-if="totalPayload > 0" class="px-3 py-2 border-t border-subtle bg-input/10 grid grid-cols-1 sm:grid-cols-2 gap-2 shrink-0">
        <EnhancedFileDropzone
          @files-selected="onFilesSelected"
          :compact="true"
          mode="folder"
          class="w-full min-w-0 h-9"
        />
        <EnhancedFileDropzone
          @files-selected="onFilesSelected"
          :compact="true"
          mode="file"
          :hint="appStore.t('compress.drop_file_hint')"
          :nativeDrop="false"
          class="w-full min-w-0 h-9"
        />
      </div>
    </div>

    <Modal
      :visible="showRarResolution"
      title="创建 RAR 需要编码器"
      size="md"
      @close="finishRarResolution('cancel')"
    >
      <div class="space-y-4">
        <div class="rounded-2xl border border-amber-500/25 bg-amber-500/10 p-4 text-sm text-content">
          <div class="mb-2 flex items-center gap-2 font-black">
            <i class="pi pi-info-circle text-amber-400"></i>
            RAR 是专有格式
          </div>
          <p class="text-xs leading-6 text-muted">{{ rarResolutionMessage }}</p>
        </div>

        <button type="button" class="w-full rounded-2xl border border-primary/30 bg-primary/10 p-4 text-left transition hover:border-primary" @click="finishRarResolution('use-7z')">
          <div class="font-black text-content"><i class="pi pi-star mr-2 text-primary"></i>改用 7Z（推荐）</div>
          <div class="mt-1 text-xs leading-5 text-muted">无需安装额外软件，支持 AES-256 密码、固实压缩和分卷。</div>
        </button>

        <button type="button" class="w-full rounded-2xl border border-subtle bg-input p-4 text-left transition hover:border-primary disabled:opacity-60" :disabled="installingWinRar" @click="installWinRar">
          <div class="font-black text-content"><i :class="installingWinRar ? 'pi pi-spin pi-spinner' : 'pi pi-download'" class="mr-2 text-primary"></i>使用 winget 安装 WinRAR</div>
          <div class="mt-1 text-xs leading-5 text-muted">从 RARLAB 官方地址安装专有试用软件（试用期最多 40 天）。点击即表示你同意查看并接受其许可条款。</div>
        </button>

        <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
          <button type="button" class="h-10 rounded-xl border border-subtle bg-input text-xs font-bold text-content hover:border-primary" @click="openRarDownloadPage">打开官方下载页</button>
          <button type="button" class="h-10 rounded-xl border border-subtle bg-input text-xs font-bold text-content hover:border-primary" @click="retryRarDetection">已安装，重新检测</button>
        </div>
      </div>
      <template #footer>
        <button type="button" class="h-10 rounded-xl border border-subtle px-5 text-xs font-bold text-muted hover:text-content" @click="finishRarResolution('cancel')">取消本次压缩</button>
      </template>
    </Modal>

    <!-- 全局设置弹窗 -->
    <GlobalSettingsModal
      :visible="showGlobalSettingsModal"
      @update:visible="showGlobalSettingsModal = $event"
      :settings="compressionStore.globalSettings"
      @update:settings="compressionStore.globalSettings = $event"
      :outputPath="compressionStore.globalOutputPath"
      @update:outputPath="compressionStore.globalOutputPath = $event"
      :allow-single-file-formats="canGlobalUseSingleFileFormats"
      @template-draft-created="showGlobalSettingsModal = false"
    />
  </div>
</template>

<style scoped>
.compression-view {
  min-width: 0;
  overflow-x: hidden;
  background: radial-gradient(circle at 100% 100%, color-mix(in srgb, var(--dynamic-accent) 4%, transparent) 0%, transparent 40%);
}

.compression-task-list,
.compression-table-header,
.compression-job-card,
.compression-job-row,
.details-drawer {
  min-width: 0;
  max-width: 100%;
}

.pop-enter-active, .pop-leave-active { transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1); }
.pop-enter-from, .pop-leave-to { opacity: 0; transform: scale(0.8) translateY(20px); }

.compression-job-card {
  box-shadow: 0 1px 2px rgb(0 0 0 / 0.08);
}

.compression-job-row {
  min-height: 46px;
}

.details-drawer {
  background-color: transparent;
}

.compression-detail-card {
  position: relative;
  width: 100%;
  box-sizing: border-box;
  min-width: 0;
  max-width: 100%;
  overflow: hidden;
  border: 1px dashed color-mix(in srgb, var(--dynamic-accent) 24%, transparent);
  border-radius: 1rem;
  background: linear-gradient(
    to bottom,
    color-mix(in srgb, var(--bg-card) 92%, transparent),
    color-mix(in srgb, var(--bg-card) 98%, transparent)
  );
  box-shadow: 0 20px 45px -25px rgb(0 0 0 / 0.55);
  transition: border-color 0.3s ease, box-shadow 0.3s ease;
}

.compression-detail-card:hover {
  border-color: color-mix(in srgb, var(--dynamic-accent) 70%, transparent);
  box-shadow:
    0 24px 48px -24px rgb(0 0 0 / 0.55),
    0 0 18px color-mix(in srgb, var(--dynamic-accent) 12%, transparent);
}

.aero-drawer-enter-active,
.aero-drawer-leave-active {
  overflow: hidden;
  transition:
    height 0.35s cubic-bezier(0.4, 0, 0.2, 1),
    opacity 0.25s linear,
    margin 0.35s cubic-bezier(0.4, 0, 0.2, 1);
}

.aero-drawer-enter-from,
.aero-drawer-leave-to {
  height: 0 !important;
  opacity: 0 !important;
  margin-top: 0 !important;
  margin-bottom: 0 !important;
}

.compression-detail-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 0;
  align-items: stretch;
}

.compression-config-panel {
  flex: 0.85 1 20rem;
  min-width: 0;
  max-height: 26rem;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 1.25rem 1.5rem;
  border-right: 1px solid color-mix(in srgb, var(--border-subtle) 55%, transparent);
  scrollbar-gutter: stable;
}

.compression-detail-grid :deep(.compression-execution-panel) {
  flex: 1 1 20rem;
}

.compression-config-panel.is-submitted :deep(input),
.compression-config-panel.is-submitted :deep(select),
.compression-config-panel.is-submitted :deep(button),
.compression-config-panel.is-submitted :deep(label) {
  pointer-events: none;
}

.compression-config-panel :deep(.horizontal-settings) {
  gap: 0.875rem;
}

.compression-config-panel :deep(.settings-core-grid) {
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.75rem;
}

.compression-config-panel :deep(.advanced-option) {
  padding: 0.625rem;
}

.detail-heading {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
  color: var(--dynamic-accent);
  font-size: 0.75rem;
  font-weight: 900;
  letter-spacing: 0.16em;
}

@media (max-width: 760px) {
  .compression-detail-grid {
    flex-direction: column;
  }

  .compression-config-panel {
    flex: none;
    width: 100%;
    max-width: 100%;
    max-height: 22rem;
    border-right: 0;
    border-bottom: 1px solid color-mix(in srgb, var(--border-subtle) 55%, transparent);
  }

  .compression-config-panel :deep(.settings-core-grid) {
    grid-template-columns: minmax(0, 1fr);
  }

  .compression-detail-grid :deep(.compression-execution-panel) {
    flex: none;
    width: 100%;
    max-width: 100%;
  }

  .compression-table-header,
  .compression-job-row {
    gap: 0.5rem;
    padding-inline: 0.75rem;
  }
}

@media (max-width: 520px) {
  .compression-leading-cell {
    width: 1.25rem;
  }

  .compression-row-actions {
    width: 2.5rem;
  }

  .details-drawer {
    padding-inline: 0;
  }

  .compression-config-panel {
    padding: 1rem;
  }
}

</style>
