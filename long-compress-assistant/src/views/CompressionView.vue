<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useAppStore } from '@/stores/app'
import { useCompressionStore } from '@/stores/compression'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { useTaskStore } from '@/stores/task'
import { extractErrorMessage, generateId } from '@/utils'
import { effectiveFormatForPassword, extensionForFormat, isPasswordSupportedFormat, isSingleFileStreamFormat } from '@/utils/compressionFormat'
import CompressionSettingsPanel from '@/components/compression/CompressionSettingsPanel.vue'
import GlobalSettingsModal from '@/components/compression/GlobalSettingsModal.vue'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'
import AeroTable from '@/components/tasks/AeroTable.vue'
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

const runCompression = async () => {
  if (compressionStore.groups.length === 0 && compressionStore.selectedFiles.length === 0) return

  let jobs = [
    ...compressionStore.groups.map(group => {
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
    ...compressionStore.selectedFiles.map(file => {
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
        allow_insecure_password_cli: effectiveFormat === 'rar' && allowRarPasswordCli
      }
    })

    validJobs.push({ job, taskId, effectiveFormat })
  }

  // A validated draft becomes a task exactly once. Keeping it in the draft area
  // allowed a second click to submit the same output while the first run was active.
  compressionStore.removeSubmittedJobs(validJobs.map(({ job }) => job.id))
  validJobs.forEach(({ job }) => selectedRows.value.delete(job.id))

  // 第二阶段：依次执行所有任务
  let succeeded = 0

  for (const { job, taskId, effectiveFormat } of validJobs) {
    const queuedTask = taskStore.tasks.find(task => task.id === taskId)
    if (!queuedTask || queuedTask.status === 'cancelled') {
      continue
    }
    try {
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
          allow_insecure_password_cli: effectiveFormat === 'rar' && allowRarPasswordCli
        }
      )
      taskStore.updateTaskStatus(taskId, 'completed')
      succeeded++
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
  taskStore.clearFinishedTasks('compression')
}
</script>

<template>
  <div class="compression-view p-4 md:p-6 h-full flex flex-col gap-4 transition-colors duration-700 overflow-hidden relative">
    <header class="flex flex-wrap justify-between items-center gap-3 shrink-0">
      <div>
        <h1 class="text-2xl md:text-3xl font-black text-content tracking-tight">{{ appStore.t('nav.compress') }}</h1>
        <p class="text-xs md:text-sm text-muted font-semibold mt-1">{{ appStore.t('compress.subtitle') }}</p>
      </div>
      <div class="flex items-center gap-2 md:gap-3">
        <button
          v-if="hasFinishedCompressionTasks"
          @click="clearFinishedCompressionTasks"
          class="h-8 md:h-9 px-3 rounded-lg bg-input border border-subtle text-muted text-xs font-bold hover:text-content hover:border-primary transition-all flex items-center gap-2"
        >
          <i class="pi pi-trash text-xs"></i>
          <span class="hidden md:inline">清除已结束</span>
        </button>
        <button
          v-if="activeCompressionTasks.length > 0"
          @click="cancelAllCompressionTasks"
          class="h-8 md:h-9 px-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-xs font-bold hover:bg-red-500/20 transition-all flex items-center gap-2"
        >
          <i class="pi pi-stop-circle text-xs"></i>
          <span class="hidden md:inline">取消进行中</span>
        </button>
        <!-- 全局设置按钮 -->
        <button
          @click="showGlobalSettingsModal = true"
          class="h-8 md:h-9 px-3 md:px-5 rounded-lg bg-input border border-subtle text-content text-xs font-bold uppercase tracking-wider hover:bg-primary/10 hover:border-primary transition-all flex items-center gap-2"
        >
          <i class="pi pi-cog text-xs"></i>
          <span class="hidden sm:inline">全局设置</span>
        </button>

        <!-- 磁吸成组按钮 -->
        <transition name="pop">
          <button
            v-if="selectedRows.size > 0"
            @click="handleCreateGroup"
            class="h-8 md:h-9 px-3 md:px-5 rounded-lg bg-input border border-subtle text-content text-xs font-bold uppercase tracking-wider hover:bg-primary hover:text-white hover:border-primary transition-all flex items-center gap-2"
          >
            <i class="pi pi-box text-xs"></i>
            <span class="hidden sm:inline">{{ appStore.t('compress.create_group') }} ({{ selectedRows.size }})</span>
            <span class="sm:hidden">({{ selectedRows.size }})</span>
          </button>
        </transition>

        <!-- 开始压缩按钮 -->
        <button
          v-if="totalPayload > 0"
          @click="handleCompress"
          :disabled="isCompressing"
          class="h-8 md:h-9 px-4 md:px-6 rounded-lg bg-primary text-white text-xs font-bold uppercase tracking-wider shadow-lg shadow-primary/25 hover:brightness-110 active:scale-[0.98] transition-all flex items-center gap-2 disabled:opacity-60 disabled:cursor-wait"
        >
          <i :class="isCompressing ? 'pi pi-spin pi-spinner' : 'pi pi-play-circle'" class="text-xs"></i>
          <span class="hidden sm:inline">{{ appStore.t('compress.start') }}</span>
          <span class="sm:hidden">开始</span>
        </button>
      </div>
    </header>

    <!-- 主工作区 -->
    <div class="flex-1 min-h-0 aero-card overflow-hidden flex flex-col relative border border-subtle bg-card/40 shadow-2xl">
      <div v-if="totalPayload > 0" class="flex-1 overflow-y-auto custom-scrollbar p-3 md:p-6 space-y-4 md:space-y-6">
        <!-- 1. 压缩组列表 -->
        <div v-for="group in compressionStore.groups" :key="group.id" 
             class="group-container rounded-[2rem] border transition-all duration-500 overflow-hidden"
             :class="group.expanded ? 'bg-input/40 border-primary/30 shadow-lg' : 'bg-input/20 border-subtle hover:border-primary/20'"
             :style="{ borderColor: group.expanded ? group.themeColor : '' }">
          
          <!-- 组头部 -->
          <div class="flex items-center px-8 py-5 cursor-pointer group/header"
               role="button" tabindex="0" :aria-expanded="group.expanded"
               @click="group.expanded = !group.expanded"
               @keydown.enter="group.expanded = !group.expanded"
               @keydown.space.prevent="group.expanded = !group.expanded">
            <div class="w-10 h-10 rounded-xl flex items-center justify-center mr-6 shadow-sm transition-transform group-hover/header:rotate-6"
                 :style="{ backgroundColor: `${group.themeColor}20`, color: group.themeColor, border: `1px solid ${group.themeColor}40` }">
              <i class="pi pi-briefcase text-sm"></i>
            </div>
            
            <div class="flex-1">
              <div class="text-sm font-black text-content tracking-tight">{{ group.name }}</div>
              <div class="flex items-center gap-2 mt-1">
                <span class="text-xs font-bold text-muted uppercase tracking-widest">{{ group.files.length }} {{ appStore.t('compress.group_count') }}</span>
                <div class="w-1 h-1 rounded-full bg-subtle"></div>
                <span class="text-xs font-mono text-primary font-black uppercase">{{ compressionStore.getEffectiveSettings(group.settings).format }}</span>
              </div>
            </div>

            <div class="flex items-center gap-4">
              <button @click.stop="compressionStore.dissolveGroup(group.id)" class="text-muted hover:text-red-500 transition-colors">
                <i class="pi pi-trash text-xs"></i>
              </button>
              <i class="pi transition-transform duration-500 text-muted text-sm" :class="group.expanded ? 'pi-chevron-up' : 'pi-chevron-down'"></i>
            </div>
          </div>

          <!-- 组展开：独立配置面板 -->
          <transition name="slide-down">
            <div v-if="group.expanded" class="px-8 pb-8 pt-4 border-t border-subtle/30">
              <div class="mb-6">
                <h4 class="text-xs font-black text-muted uppercase tracking-widest mb-4">{{ appStore.t('compress.settings') }}</h4>
                <!-- 使用横向配置组件，适配该组 -->
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
                <div v-for="file in group.files" :key="file.path" class="text-sm text-muted font-mono py-1 px-3 bg-card/40 rounded-lg border border-subtle/50 flex items-center justify-between group/file">
                  <div class="flex items-center gap-2 overflow-hidden min-w-0">
                    <i :class="file.isDirectory ? 'pi pi-folder text-primary/60' : 'pi pi-file text-muted/60'" class="text-xs shrink-0"></i>
                    <span class="truncate">{{ file.name }}</span>
                  </div>
                  <div class="flex items-center gap-2 shrink-0">
                    <span class="opacity-75 italic ml-2">{{ file.path }}</span>
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
          </transition>
        </div>

        <!-- 2. 未分组文件列表 (待分配) -->
        <div v-if="compressionStore.selectedFiles.length > 0" class="space-y-3">
          <h3 class="text-xs font-black text-muted uppercase tracking-[0.3em] px-4">{{ appStore.t('compress.add_files') }}</h3>
          <div v-for="file in compressionStore.selectedFiles" :key="file.path" 
               data-testid="compression-draft-row"
               @click="file.expanded = !file.expanded"
                class="flex flex-wrap items-center justify-between px-8 py-4 rounded-2xl bg-input border border-subtle group/row hover:border-primary/30 transition-all cursor-pointer"
               :class="{ 'border-primary/50 bg-primary/5 shadow-inner': file.expanded }">
            
            <button
              type="button"
              data-testid="compression-group-checkbox"
              class="w-6 flex shrink-0"
              :aria-label="`选择 ${file.name} 用于打组`"
              @click.stop="toggleSelection(file.path)"
            >
              <div class="w-4 h-4 rounded border border-subtle flex items-center justify-center transition-all"
                   :class="selectedRows.has(file.path) ? 'bg-primary border-primary' : 'bg-card'">
                <i v-if="selectedRows.has(file.path)" class="pi pi-check text-xs text-white"></i>
              </div>
            </button>

            <div class="flex-1 min-w-[200px] overflow-hidden px-4 flex items-center gap-3">
              <div class="w-8 h-8 rounded-lg bg-card border border-subtle flex items-center justify-center shrink-0">
                <i :class="file.isDirectory ? 'pi pi-folder text-primary' : 'pi pi-file text-muted'" class="text-xs"></i>
              </div>
              <div class="overflow-hidden">
                <div class="text-content font-bold truncate text-xs tracking-tight group-hover/row:text-primary transition-colors">{{ file.name }}</div>
                <div class="text-xs text-muted font-mono mt-0.5 opacity-90 truncate">{{ file.path }}</div>
              </div>
            </div>

            <button @click.stop="compressionStore.selectedFiles = compressionStore.selectedFiles.filter(f => f.path !== file.path)" 
                    class="w-8 h-8 rounded-lg flex items-center justify-center text-dim hover:text-red-500 transition-all">
              <i class="pi pi-times text-sm"></i>
            </button>
            <button @click.stop="file.expanded = !file.expanded"
                    class="w-8 h-8 rounded-lg flex items-center justify-center text-dim hover:text-primary transition-all">
              <i class="pi text-sm transition-transform" :class="file.expanded ? 'pi-chevron-up' : 'pi-chevron-down'"></i>
            </button>

            <transition name="slide-down">
              <div v-if="file.expanded" class="w-full mt-4 pt-4 border-t border-subtle/30" @click.stop>
                <CompressionSettingsPanel
                  :modelValue="compressionStore.getEffectiveSettings(file.settings)"
                  :outputPath="compressionStore.getEffectiveOutputPath(file.outputPath)"
                  :allow-single-file-formats="canUseSingleFileFormats([file])"
                  :suggested-filename="getBaseName(file.path)"
                  @update:modelValue="compressionStore.updateFileSettings(file.path, $event)"
                  @update:outputPath="compressionStore.updateFileOutputPath(file.path, $event)"
                />
              </div>
            </transition>
          </div>
        </div>

        <section v-if="compressionTasks.length > 0" class="min-h-[260px] flex-1 rounded-2xl border border-subtle bg-card/30 overflow-hidden">
          <div class="px-5 py-3 border-b border-subtle flex items-center justify-between">
            <div>
              <h3 class="text-xs font-black text-content uppercase tracking-[0.2em]">压缩任务</h3>
              <p class="text-xs text-muted mt-1">点击任务展开左侧配置与右侧实时进度日志</p>
            </div>
            <span class="text-xs font-mono text-primary">{{ activeCompressionTasks.length }} 进行中</span>
          </div>
          <div class="h-[320px]">
            <AeroTable
              task-type="compression"
              @cancel-task="cancelCompressionTask"
            />
          </div>
        </section>
      </div>

      <!-- 3. 空状态 (引导式双列布局) -->
      <div v-else-if="compressionTasks.length === 0" class="flex-1 min-h-0 flex flex-col items-center justify-center p-2 sm:p-3 gap-2 sm:gap-3">
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

      <div v-else class="flex-1 min-h-0">
        <AeroTable
          task-type="compression"
          @cancel-task="cancelCompressionTask"
        />
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
    />
  </div>
</template>

<style scoped>
.compression-view {
  background: radial-gradient(circle at 100% 100%, color-mix(in srgb, var(--dynamic-accent) 4%, transparent) 0%, transparent 40%);
}

.slide-down-enter-active, .slide-down-leave-active { transition: all 0.5s cubic-bezier(0.34, 1.56, 0.64, 1); }
.slide-down-enter-from, .slide-down-leave-to { opacity: 0; transform: translateY(-10px); }

.pop-enter-active, .pop-leave-active { transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1); }
.pop-enter-from, .pop-leave-to { opacity: 0; transform: scale(0.8) translateY(20px); }
</style>
