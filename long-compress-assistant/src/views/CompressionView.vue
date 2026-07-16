<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { listen } from '@tauri-apps/api/event'
import { useAppStore } from '@/stores/app'
import { useCompressionStore } from '@/stores/compression'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { useTaskStore } from '@/stores/task'
import { extractErrorMessage, generateId } from '@/utils'
import { extensionForFormat, isPasswordSupportedFormat, isSingleFileStreamFormat } from '@/utils/compressionFormat'
import CompressionSettingsPanel from '@/components/compression/CompressionSettingsPanel.vue'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'

const appStore = useAppStore()
const compressionStore = useCompressionStore()
const tauriCommands = useTauriCommands()
const taskStore = useTaskStore()

const selectedRows = ref<Set<string>>(new Set())
const rarSupport = ref<{ available: boolean; encoder_path?: string | null; message: string } | null>(null)
const checkingRarSupport = ref(false)
const router = useRouter()

const compUnlisteners: Array<() => void> = []

onMounted(async () => {
  // 右键菜单 → 打开压缩对话框（用户手动配置）
  const u1 = await listen<string[]>('context-compress-custom', (event) => {
    const files = event.payload.filter(f => f && !f.startsWith('%'))
    if (files.length > 0) {
      router.push('/compress')
      files.forEach(f => {
        const name = f.split(/[\\/]/).pop() || f
        compressionStore.addFile({ name, path: f, size: 0, type: 'file', isDirectory: false })
      })
      appStore.setSuccess(appStore.t('compress.context_menu_custom').replace('{0}', String(files.length)))
    }
  })
  compUnlisteners.push(u1)

  // 右键菜单 → 直接压缩为 ZIP
  const u2 = await listen<string[]>('context-compress-zip', (event) => {
    const files = event.payload.filter(f => f && !f.startsWith('%'))
    if (files.length > 0) {
      router.push('/compress')
      files.forEach(f => {
        const name = f.split(/[\\/]/).pop() || f
        compressionStore.addFile({ name, path: f, size: 0, type: 'file', isDirectory: false })
      })
      compressionStore.globalSettings.format = 'zip'
      appStore.setSuccess(appStore.t('compress.context_menu_zip').replace('{0}', String(files.length)))
      // 自动开始压缩
      setTimeout(() => handleCompress(), 500)
    }
  })
  compUnlisteners.push(u2)

  // 右键菜单 → 直接压缩为 7Z
  const u3 = await listen<string[]>('context-compress-7z', (event) => {
    const files = event.payload.filter(f => f && !f.startsWith('%'))
    if (files.length > 0) {
      router.push('/compress')
      files.forEach(f => {
        const name = f.split(/[\\/]/).pop() || f
        compressionStore.addFile({ name, path: f, size: 0, type: 'file', isDirectory: false })
      })
      compressionStore.globalSettings.format = '7z'
      appStore.setSuccess(appStore.t('compress.context_menu_7z').replace('{0}', String(files.length)))
      // 自动开始压缩
      setTimeout(() => handleCompress(), 500)
    }
  })
  compUnlisteners.push(u3)
})

onUnmounted(() => {
  compUnlisteners.forEach(fn => fn())
})

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

const handleCompress = async () => {
  if (compressionStore.groups.length === 0 && compressionStore.selectedFiles.length === 0) return

  const jobs = [
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

  // 第一阶段：预校验并添加所有任务到列表（status: pending）
  const validJobs: Array<{ job: typeof jobs[0], taskId: string }> = []
  let failed = 0

  for (const job of jobs) {
    // 校验失败：跳过当前任务
    if (isSingleFileStreamFormat(job.settings.format) && !canUseSingleFileFormats(job.files)) {
      appStore.setError(appStore.t('compress.error.single_file').replace('{0}', job.settings.format.toUpperCase()))
      failed++
      continue
    }

    if (job.settings.password && !isPasswordSupportedFormat(job.settings.format)) {
      appStore.setError(appStore.t('compress.error.no_password').replace('{0}', job.settings.format.toUpperCase()))
      failed++
      continue
    }

    if (job.settings.format === 'rar') {
      const support = await ensureRarSupport()
      if (!support?.available) {
        appStore.setError(support?.message || appStore.t('compress.error.rar_requires'))
        failed++
        continue
      }
    }

    // 添加任务到列表（pending 状态）
    const taskId = taskStore.addTask({
      id: generateId(),
      name: job.name,
      type: 'compression',
      sourceFiles: job.files.map(file => file.path),
      outputPath: job.outputPath,
      format: job.settings.format
    })

    validJobs.push({ job, taskId })
  }

  // 第二阶段：依次执行所有任务
  let succeeded = 0

  for (const { job, taskId } of validJobs) {
    try {
      taskStore.updateTaskStatus(taskId, 'compressing')
      await tauriCommands.compressFiles(
        taskId,
        job.files.map(file => file.path),
        job.outputPath,
        {
          format: job.settings.format,
          level: job.settings.level,
          password: job.settings.password || undefined,
          split_size: job.settings.splitArchive ? Number(job.settings.splitSize) : null,
          preserve_paths: job.settings.keepStructure,
          delete_after: job.settings.deleteAfter
        }
      )
      taskStore.updateTaskStatus(taskId, 'completed')
      succeeded++
    } catch (error) {
      taskStore.updateTaskStatus(taskId, 'failed')
      const task = taskStore.tasks.find(t => t.id === taskId)
      if (task) {
        task.error = extractErrorMessage(error)
      }
      appStore.setError(`${appStore.t('common.error')}: ${extractErrorMessage(error)}`)
      failed++
      // 继续处理下一个任务，不中断整个批次
    }
  }

  if (succeeded > 0 && failed === 0) {
    appStore.setSuccess(appStore.t('compress.status_success').replace('{0}', String(succeeded)).replace('{1}', succeeded === 1 ? '' : 's'))
  } else if (succeeded > 0) {
    appStore.setSuccess(appStore.t('compress.status_result').replace('{0}', String(succeeded)).replace('{1}', String(failed)))
  }
}

const totalPayload = computed(() => {
  return compressionStore.selectedFiles.length + compressionStore.groups.reduce((acc, g) => acc + g.files.length, 0)
})
</script>

<template>
  <div class="compression-view p-4 md:p-6 h-full flex flex-col gap-4 transition-colors duration-700 overflow-hidden relative">
    <header class="flex flex-wrap justify-between items-center gap-3 shrink-0">
      <div>
        <h1 class="text-xl md:text-2xl font-extrabold text-content tracking-tight">{{ appStore.t('nav.compress') }}</h1>
      </div>
      <div class="flex items-center gap-2 md:gap-3">
        <!-- 磁吸成组按钮 (浮现式，次要操作) -->
        <transition name="pop">
          <button
            v-if="selectedRows.size > 0"
            @click="handleCreateGroup"
            class="h-8 md:h-9 px-3 md:px-5 rounded-lg bg-input border border-subtle text-content text-xs md:text-xs font-bold uppercase tracking-wider hover:bg-primary hover:text-white hover:border-primary transition-all flex items-center gap-2"
          >
            <i class="pi pi-box text-xs"></i>
            <span class="hidden sm:inline">{{ appStore.t('compress.create_group') }} ({{ selectedRows.size }})</span>
            <span class="sm:hidden">({{ selectedRows.size }})</span>
          </button>
        </transition>

        <button
          v-if="totalPayload > 0"
          @click="handleCompress"
          class="h-8 md:h-9 px-4 md:px-6 rounded-lg bg-primary text-white text-xs md:text-xs font-bold uppercase tracking-wider shadow-lg shadow-primary/25 hover:brightness-110 active:scale-[0.98] transition-all flex items-center gap-2"
        >
          <i class="pi pi-play-circle text-xs"></i>
          <span class="hidden sm:inline">{{ appStore.t('compress.start') }}</span>
          <span class="sm:hidden">开始</span>
        </button>
      </div>
    </header>

    <!-- 主工作区 (减小 mb 以使区域向下延展) -->
    <div class="flex-1 min-h-0 aero-card overflow-hidden flex flex-col relative border border-subtle bg-card/40 shadow-2xl">
      <div v-if="totalPayload > 0" class="flex-1 overflow-y-auto custom-scrollbar p-3 md:p-6 space-y-4 md:space-y-6">
        <div class="rounded-2xl border border-primary/20 bg-primary/5 p-6">
          <div class="flex items-start gap-3 mb-4">
            <div class="w-8 h-8 rounded-lg bg-primary/20 flex items-center justify-center shrink-0">
              <i class="pi pi-cog text-primary text-xs"></i>
            </div>
            <div>
              <h4 class="text-sm font-black text-primary uppercase tracking-widest">{{ appStore.t('compress.global_settings') }}</h4>
              <p class="text-xs text-muted mt-1 uppercase tracking-tighter">{{ appStore.t('compress.global_settings.desc') }}</p>
            </div>
          </div>
          <CompressionSettingsPanel
            v-model="compressionStore.globalSettings"
            v-model:outputPath="compressionStore.globalOutputPath"
            :allow-single-file-formats="canGlobalUseSingleFileFormats"
          />
          <div
            v-if="usesRarFormat && rarSupport && !rarSupport.available"
            class="mt-4 flex flex-wrap items-center gap-3 rounded-xl border border-yellow-500/30 bg-yellow-500/10 px-4 py-3 text-sm font-bold text-yellow-600"
          >
            <span class="min-w-0 flex-1">{{ rarSupport.message }}</span>
            <button
              type="button"
              class="h-7 rounded-lg border border-yellow-500/30 px-3 text-xs font-black uppercase tracking-widest transition-colors hover:bg-yellow-500/10"
              @click="openRarDownloadPage"
            >
              {{ appStore.t('compress.download') }}
            </button>
            <button
              type="button"
              class="h-7 rounded-lg border border-yellow-500/30 px-3 text-xs font-black uppercase tracking-widest transition-colors hover:bg-yellow-500/10 disabled:opacity-85"
              :disabled="checkingRarSupport"
              @click="refreshRarSupport"
            >
              {{ appStore.t('compress.recheck') }}
            </button>
          </div>
          <div
            v-else-if="usesRarFormat && rarSupport?.available"
            class="mt-4 flex flex-wrap items-center gap-3 rounded-xl border border-emerald-500/30 bg-emerald-500/10 px-4 py-3 text-sm font-bold text-emerald-600"
          >
            <span class="min-w-0 flex-1">{{ rarSupport.message }}</span>
            <button
              type="button"
              class="h-7 rounded-lg border border-emerald-500/30 px-3 text-xs font-black uppercase tracking-widest transition-colors hover:bg-emerald-500/10 disabled:opacity-85"
              :disabled="checkingRarSupport"
              @click="refreshRarSupport"
            >
              {{ appStore.t('compress.recheck') }}
            </button>
          </div>
        </div>
        
        <!-- 1. 压缩组列表 -->
        <div v-for="group in compressionStore.groups" :key="group.id" 
             class="group-container rounded-[2rem] border transition-all duration-500 overflow-hidden"
             :class="group.expanded ? 'bg-input/40 border-primary/30 shadow-lg' : 'bg-input/20 border-subtle hover:border-primary/20'"
             :style="{ borderColor: group.expanded ? group.themeColor : '' }">
          
          <!-- 组头部 -->
          <div class="flex items-center px-8 py-5 cursor-pointer group/header" @click="group.expanded = !group.expanded">
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
               @click="toggleSelection(file.path)"
                class="flex flex-wrap items-center justify-between px-8 py-4 rounded-2xl bg-input border border-subtle group/row hover:border-primary/30 transition-all cursor-pointer"
               :class="{ 'border-primary/50 bg-primary/5 shadow-inner': selectedRows.has(file.path) }">
            
            <div class="w-6 flex shrink-0">
              <div class="w-4 h-4 rounded border border-subtle flex items-center justify-center transition-all"
                   :class="selectedRows.has(file.path) ? 'bg-primary border-primary' : 'bg-card'">
                <i v-if="selectedRows.has(file.path)" class="pi pi-check text-xs text-white"></i>
              </div>
            </div>

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
              <i class="pi pi-cog text-sm"></i>
            </button>

            <transition name="slide-down">
              <div v-if="file.expanded" class="w-full mt-4 pt-4 border-t border-subtle/30" @click.stop>
                <CompressionSettingsPanel
                  :modelValue="compressionStore.getEffectiveSettings(file.settings)"
                  :outputPath="compressionStore.getEffectiveOutputPath(file.outputPath)"
                  :allow-single-file-formats="canUseSingleFileFormats([file])"
                  @update:modelValue="compressionStore.updateFileSettings(file.path, $event)"
                  @update:outputPath="compressionStore.updateFileOutputPath(file.path, $event)"
                />
              </div>
            </transition>
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
          class="w-full h-7"
        />
        <EnhancedFileDropzone
          @files-selected="onFilesSelected"
          :compact="true"
          mode="file"
          class="w-full h-7"
        />
      </div>
    </div>
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
