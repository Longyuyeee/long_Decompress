<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useTaskStore } from '@/stores/task'
import { useAppStore } from '@/stores/app'
import { usePasswordStore } from '@/stores/password'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { extractErrorMessage, generateId } from '@/utils'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/api/dialog'
import AeroTable from '@/components/tasks/AeroTable.vue'
import ConflictResolutionModal from '@/components/tasks/ConflictResolutionModal.vue'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'

const taskStore = useTaskStore()
const appStore = useAppStore()
const passwordStore = usePasswordStore()
const tauriCommands = useTauriCommands()

const selectedConflictTaskId = ref<string | null>(null)
const showConflictModal = ref(false)
const selectedTaskIds = ref<Set<string>>(new Set())
const supportedArchiveAccept = '.zip,.7z,.rar,.tar,.tar.gz,.tgz,.tar.bz2,.tbz,.tbz2,.tar.xz,.txz,.gz,.bz2,.xz,.zst,.zstd,.tzst,.iso,.img,.cab,.lzh,.lha,.arj,.dmg,.wim,.vhd,.vhdx,.chm,.deb,.rpm,.squashfs,.sfs,.msi,.nsis,.xar,.lzma,.cpio,.udf,.fat,.ntfs'
const supportedArchiveHint = 'ZIP, 7Z, RAR, TAR, Zstd, ISO, CAB, DEB, RPM, DMG, MSI + 30 more'

// 全局配置状态
const globalOutputPath = ref('')
const isGlobalSameDir = ref(true) // 默认同目录，用户可通过按钮手动选择
const globalExtractToSubfolder = ref(false)

onMounted(async () => {
  await taskStore.initListeners()
  // 监听右键菜单传入的文件路径
  await listen<string>('context-menu-open', (event) => {
    const filePath = event.payload
    const files = [{ path: filePath }]
    onFilesSelected(files as any)
  })
})

const onFilesSelected = async (files: any[]) => {
  for (const file of files) {
    const sourcePath = file.path
    // 去重：如果任务列表中已有相同文件，跳过
    if (taskStore.tasks.some(t => t.sourceFiles.includes(sourcePath))) continue

    const parentDir = sourcePath.substring(0, Math.max(sourcePath.lastIndexOf('/'), sourcePath.lastIndexOf('\\')))

    const taskId = taskStore.addTask({
      id: generateId(),
      name: file.name || sourcePath.split(/[\\/]/).pop() || 'Unknown',
      type: 'decompression',
      sourceFiles: [sourcePath],
      outputPath: isGlobalSameDir.value ? parentDir : globalOutputPath.value,
      extractToSubfolder: globalExtractToSubfolder.value
    })
    appStore.addRecentFile(sourcePath)

    // Smart extraction: auto-detect if subfolder is needed
    tauriCommands.listArchiveContents(sourcePath).then((contents: string[]) => {
      const task = taskStore.tasks.find(t => t.id === taskId)
      if (!task || task.status !== 'pending') return
      const rootEntries = contents.filter(item => !item.includes('/')).length
      if (rootEntries > 1) {
        task.extractToSubfolder = true
        appStore.setSuccess(appStore.t('decompress.smart_extract'))
      } else if (rootEntries === 1) {
        task.extractToSubfolder = false
      }
    }).catch(() => {
      // Cannot read contents (encrypted, unsupported format) — keep default behavior
    })
  }
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
      taskStore.tasks.forEach(t => {
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
  taskStore.tasks.forEach(t => {
    if (t.status === 'pending' && t.sourceFiles.length > 0) {
      const sp = t.sourceFiles[0]
      t.outputPath = sp.substring(0, Math.max(sp.lastIndexOf('/'), sp.lastIndexOf('\\')))
    }
  })
}

const toggleGlobalSubfolder = () => {
  globalExtractToSubfolder.value = !globalExtractToSubfolder.value
  taskStore.tasks.forEach(t => {
    if (t.status === 'pending') t.extractToSubfolder = globalExtractToSubfolder.value
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
  const ids = taskStore.tasks.filter(t => t.status === 'pending').map(t => t.id)
  selectedTaskIds.value = new Set(ids)
}

const deselectAll = () => {
  selectedTaskIds.value = new Set()
}

const startDecompression = async () => {
  // 如果有选中的任务，优先处理选中的；否则处理所有 pending 任务
  const pendingTasks = selectedTaskIds.value.size > 0
    ? taskStore.tasks.filter(t => selectedTaskIds.value.has(t.id) && t.status === 'pending')
    : taskStore.tasks.filter(t => t.status === 'pending')
  if (pendingTasks.length === 0) return

  // 启动后清除选择
  selectedTaskIds.value = new Set()

  for (const task of pendingTasks) {
    const options = {
      outputPath: task.outputPath,
      keepStructure: true,
      overwrite: false,
      deleteAfter: appStore.settings.autoDeleteSource,
      createSubdirectory: task.extractToSubfolder ?? false,
      password: task.password || undefined,
      fileFilter: task.fileFilter || null
    }
    try {
      taskStore.updateTaskStatus(task.id, 'preparing')
      task.passwordRequired = false
      await tauriCommands.decompressFile(task.sourceFiles[0], options, task.id)
    } catch (error) {
      // 如果任务被 password-required 事件标记，自动尝试密码保险箱
      if (task.passwordRequired) {
        const fileName = task.name || task.sourceFiles[0]?.split(/[\\/]/).pop() || ''
        const candidates = passwordStore.findCandidatePasswords(fileName)

        if (candidates.length > 0) {
          taskStore.updateTaskStatus(task.id, 'extracting')
          task.logs.push({
            task_id: task.id,
            message: appStore.t('decompress.auto_trying').replace('{0}', String(candidates.length)),
            severity: 'info',
            timestamp: new Date().toISOString()
          })

          let succeeded = false
          for (const candidatePassword of candidates) {
            if (succeeded) break
            try {
              task.password = candidatePassword
              task.passwordRequired = false
              task.currentPassword = candidatePassword
              await tauriCommands.decompressFile(
                task.sourceFiles[0],
                { ...options, password: candidatePassword },
                task.id
              )
              succeeded = true

              // 递增保险箱中匹配密码的 use_count
              const matchedEntry = passwordStore.entries.find(e => e.password === candidatePassword)
              if (matchedEntry) {
                try {
                  await passwordStore.updateEntry(matchedEntry.id, { use_count: (matchedEntry.use_count || 0) + 1 })
                } catch { /* 非关键操作 */ }
              }
            } catch {
              // 尝试下一个候选密码
              continue
            }
          }

          if (!succeeded) {
            // 所有候选密码均失败，标记为失败并记录错误
            taskStore.updateTaskStatus(task.id, 'failed')
            task.error = appStore.t('decompress.all_failed').replace('{0}', String(candidates.length))
            task.logs.push({
              task_id: task.id,
              message: task.error!,
              severity: 'error',
              timestamp: new Date().toISOString()
            })
          }
        } else {
          // 保险箱中没有候选密码
          taskStore.updateTaskStatus(task.id, 'failed')
          task.error = extractErrorMessage(error) || appStore.t('decompress.no_vault_passwords')
          task.logs.push({
            task_id: task.id,
            message: task.error!,
            severity: 'error',
            timestamp: new Date().toISOString()
          })
        }
      } else {
        taskStore.updateTaskStatus(task.id, 'failed')
        task.error = extractErrorMessage(error) || String(error)
        appStore.setError(`${appStore.t('common.error')}: ${task.error}`)
      }
    }
  }
}

const hasPendingTasks = computed(() => taskStore.tasks.some(t => t.status === 'pending'))
const isRunning = computed(() => taskStore.tasks.some(t => ['running', 'extracting', 'compressing', 'preparing'].includes(t.status)))

const cancelAllTasks = async () => {
  let cancelled = 0
  let failed = 0
  for (const t of taskStore.tasks) {
    if (['running', 'extracting', 'compressing', 'preparing'].includes(t.status)) {
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

taskStore.$subscribe((mutation, state) => {
  const taskWithConflict = state.tasks.find(t => t.conflicts.length > 0)
  if (taskWithConflict && !showConflictModal.value) {
    handleConflict(taskWithConflict.id)
  }
})
</script>

<template>
  <div class="decompress-view p-6 h-full flex flex-col gap-4 transition-colors duration-700 relative overflow-hidden">
    <header class="flex justify-between items-center shrink-0">
      <div>
        <h1 class="text-3xl font-black text-content tracking-tighter mb-0.5">{{ appStore.t('nav.decompress') }}</h1>
        <p class="text-muted text-[0.5625rem] font-bold uppercase tracking-[0.2em] ml-0.5">{{ appStore.t('decompress.add_files') }}</p>
      </div>
      <div class="flex gap-3">
        <button
          v-if="!isRunning && taskStore.tasks.some(t => ['completed', 'failed', 'cancelled'].includes(t.status))"
          @click="taskStore.clearFinishedTasks()"
          class="h-10 px-6 rounded-xl bg-input border border-subtle text-muted text-[0.625rem] font-black uppercase tracking-widest hover:text-red-400 transition-all shadow-sm flex items-center gap-2"
        >
          <i class="pi pi-trash"></i>
          {{ appStore.t('decompress.clear_finished') }}
        </button>
        <button
          v-if="isRunning"
          @click="cancelAllTasks"
          class="h-10 px-6 rounded-xl bg-red-500/10 text-red-500 border border-red-500/20 text-[0.625rem] font-black uppercase tracking-widest hover:bg-red-500 hover:text-white transition-all shadow-sm flex items-center gap-2"
        >
          <i class="pi pi-stop-circle"></i>
          {{ appStore.t('common.cancel') }}
        </button>
        <button
          v-if="hasPendingTasks && !isRunning"
          @click="startDecompression"
          class="h-10 px-6 rounded-xl bg-primary text-white text-[0.625rem] font-black uppercase tracking-widest hover:brightness-110 transition-all shadow-lg flex items-center gap-2"
        >
          <i class="pi pi-play-circle animate-pulse"></i>
          {{ appStore.t('decompress.start_queue') }}
        </button>
      </div>
    </header>

    <div class="flex-1 min-h-0 aero-card overflow-hidden flex flex-col relative border border-subtle bg-card/40 shadow-2xl">
      <div class="flex-1 overflow-hidden flex flex-col relative">
        <!-- 核心逻辑：有内容时 100% 空间给表格 -->
        <div v-if="taskStore.tasks.some(t => t.status === 'pending')" class="flex-1 min-h-0">
          <AeroTable
          :selectedTaskIds="selectedTaskIds"
          statusFilter="pending"
          @toggle-task="toggleTaskSelection"
          @select-all-pending="selectAllPending"
          @deselect-all="deselectAll"
        />
        </div>

        <!-- 空状态 -->
        <div v-else class="flex-1 flex flex-col items-center justify-center p-8">
          <EnhancedFileDropzone
            @files-selected="onFilesSelected"
            :accept="supportedArchiveAccept"
            :sub-hint="supportedArchiveHint"
            class="w-full max-w-lg shadow-sm"
          />
        </div>
      </div>

      <!-- 底部操作区 -->
      <div v-if="taskStore.tasks.length > 0" class="border-t border-subtle bg-input/10 px-3 py-3 flex items-center gap-4 flex-wrap shrink-0">
        <span class="text-[0.5625rem] font-black text-primary uppercase tracking-widest opacity-80 shrink-0">{{ appStore.t('decompress.config.output') }}</span>

        <button @click="handleGlobalSelectDir"
                class="h-6 px-2.5 rounded-lg bg-primary text-white hover:brightness-110 active:scale-95 transition-all text-[0.5625rem] font-black flex items-center gap-1 shadow-sm shadow-primary/20">
          <i class="pi pi-folder-open text-[0.5625rem]"></i>
          {{ appStore.t('decompress.config.output_select') }}
        </button>

        <button @click="handleGlobalSetSameDir"
                :class="isGlobalSameDir ? 'bg-primary/10 text-primary border-primary/20 shadow-inner' : 'bg-input/30 text-muted border-subtle/50'"
                class="h-6 px-2.5 rounded-lg border text-[0.5625rem] font-bold transition-all hover:bg-primary/5">
          {{ appStore.t('decompress.config.output_same') }}
        </button>

        <span class="text-[0.5625rem] font-mono text-content font-bold truncate flex-1 min-w-[100px] max-w-[240px]">
          {{ isGlobalSameDir ? appStore.t('decompress.config.output_auto') : (globalOutputPath || appStore.t('decompress.config.output_auto')) }}
        </span>

        <div class="flex items-center gap-2 cursor-pointer" @click="toggleGlobalSubfolder">
          <div class="w-3 h-3 rounded border border-primary/30 flex items-center justify-center"
               :class="globalExtractToSubfolder ? 'bg-primary border-primary' : 'bg-transparent'">
            <i v-if="globalExtractToSubfolder" class="pi pi-check text-[0.375rem] text-white"></i>
          </div>
          <span class="text-[0.5625rem] font-black text-muted uppercase tracking-widest">{{ appStore.t('decompress.config.output_sub') }}</span>
        </div>

        <div class="w-px h-5 bg-subtle/20 mx-1"></div>

        <EnhancedFileDropzone
          @files-selected="onFilesSelected"
          :compact="true"
          :accept="supportedArchiveAccept"
          class="flex-1 min-w-[140px] max-w-[240px] h-7"
        />
      </div>
    </div>

    <ConflictResolutionModal
      v-if="selectedConflictTaskId"
      v-model:visible="showConflictModal"
      :taskId="selectedConflictTaskId"
    />
  </div>
</template>

<style scoped>
.decompress-view {
  background: radial-gradient(circle at 0% 0%, color-mix(in srgb, var(--dynamic-accent) 4%, transparent) 0%, transparent 40%);
}

.fade-morph-enter-active, .fade-morph-leave-active { transition: all 0.6s cubic-bezier(0.34, 1.56, 0.64, 1); }
.fade-morph-enter-from { opacity: 0; transform: scale(0.98); }
.fade-morph-leave-to { opacity: 0; transform: scale(1.02); }
</style>
