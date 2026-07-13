<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
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
import { DECOMPRESS_ARCHIVE_ACCEPT, DECOMPRESS_ARCHIVE_HINT } from '@/utils/compressionFormat'

const taskStore = useTaskStore()
const appStore = useAppStore()
const passwordStore = usePasswordStore()
const tauriCommands = useTauriCommands()

const selectedConflictTaskId = ref<string | null>(null)
const showConflictModal = ref(false)
const selectedTaskIds = ref<Set<string>>(new Set())
const supportedArchiveAccept = DECOMPRESS_ARCHIVE_ACCEPT
const supportedArchiveHint = DECOMPRESS_ARCHIVE_HINT

// 全局配置状态
const globalOutputPath = ref('')
const isGlobalSameDir = ref(true) // 默认同目录，用户可通过按钮手动选择
const globalExtractToSubfolder = ref(false)

const unlisteners: Array<() => void> = []

onMounted(async () => {
  await taskStore.initListeners()
  // taskStore listeners are auto-managed by Pinia

  // 右键菜单 / CLI：添加文件到解压队列
  const handleContextFiles = (files: string[]) => {
    const fileObjs = files.filter(f => f && !f.startsWith('%')).map(f => ({ path: f }))
    if (fileObjs.length > 0) {
      onFilesSelected(fileObjs as any)
    } else if (files.length > 0) {
      // 所有文件都被过滤掉（均为 %V 占位符等）
      appStore.setError(appStore.t('decompress.no_file_paths'))
    }
  }

  // 右键菜单：直接解压到此处
  await listen<string[]>('context-extract-here', (event) => {
    const files = event.payload.filter(f => f && !f.startsWith('%'))
    const fileObjs = files.map(f => ({ path: f }))
    if (fileObjs.length > 0) {
      onFilesSelected(fileObjs as any)
      appStore.setSuccess(appStore.t('decompress.context_menu_added').replace('{0}', String(fileObjs.length)))
      setTimeout(() => {
        const pending = taskStore.tasks.filter(t => t.status === 'pending')
        if (pending.length > 0) {
          pending.forEach(t => { t.extractToSubfolder = false })
          setTimeout(() => startDecompression(), 300)
        }
      }, 600)
    }
  })

  // 右键菜单：解压到同名文件夹
  await listen<string[]>('context-extract-to', (event) => {
    const files = event.payload.filter(f => f && !f.startsWith('%'))
    const fileObjs = files.map(f => ({ path: f }))
    if (fileObjs.length > 0) {
      onFilesSelected(fileObjs as any)
      appStore.setSuccess(appStore.t('decompress.context_menu_folder').replace('{0}', String(fileObjs.length)))
      setTimeout(() => {
        const pending = taskStore.tasks.filter(t => t.status === 'pending')
        if (pending.length > 0) {
          pending.forEach(t => { t.extractToSubfolder = true })
          setTimeout(() => startDecompression(), 300)
        }
      }, 600)
    }
  })

  // 右键菜单：测试完整性
  await listen<string[]>('context-test-archive', (event) => {
    const files = event.payload.filter(f => f && !f.startsWith('%'))
    files.forEach(async (path) => {
      try {
        const result = await tauriCommands.testArchiveIntegrity(path)
        appStore.setSuccess(appStore.t('decompress.integrity_passed').replace('{0}', result))
      } catch (e: any) {
        appStore.setError(appStore.t('decompress.integrity_failed').replace('{0}', String(e)))
      }
    })
  })

  // 向后兼容旧版
  const unlistenOpen = await listen<string>('context-open', (event) => {
    handleContextFiles([event.payload])
  })
  unlisteners.push(unlistenOpen)
})

onUnmounted(() => {
  unlisteners.forEach(fn => fn())
  unsubConflict()
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
    }).catch((e) => {
      // Archiving listing failed (encrypted/unsupported format) — keep default behavior
      console.debug('Smart extract skipped (unable to list contents):', sourcePath, e)
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

const isProcessing = ref(false)

const startDecompression = async () => {
  // 防止重复点击
  if (isProcessing.value) return
  // 如果有选中的任务，优先处理选中的；否则处理所有 pending 任务
  const pendingTasks = selectedTaskIds.value.size > 0
    ? taskStore.tasks.filter(t => selectedTaskIds.value.has(t.id) && t.status === 'pending')
    : taskStore.tasks.filter(t => t.status === 'pending')
  if (pendingTasks.length === 0) return

  isProcessing.value = true

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

  isProcessing.value = false
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

// 处理冲突解决：取消当前任务并用新策略重试
const handleConflictResolve = async (action: 'overwrite' | 'skip' | 'rename', applyToAll: boolean) => {
  const taskId = selectedConflictTaskId.value
  if (!taskId) return
  const task = taskStore.tasks.find(t => t.id === taskId)
  if (!task) return

  // 应用冲突策略
  if (applyToAll) {
    appStore.updateSettings({ conflictPolicy: action })
  }
  showConflictModal.value = false
  selectedConflictTaskId.value = null

  // 取消当前任务并用新策略重试
  try {
    await taskStore.cancelTask(taskId)
  } catch { /* ignore cancel errors */ }

  const options = {
    outputPath: task.outputPath,
    keepStructure: true,
    overwrite: action === 'overwrite',
    deleteAfter: appStore.settings.autoDeleteSource,
    createSubdirectory: task.extractToSubfolder ?? false,
    password: task.password || undefined,
    fileFilter: task.fileFilter || null
  }
  try {
    await tauriCommands.decompressFile(task.sourceFiles[0], options, taskId)
  } catch (e: any) {
    appStore.setError(`解压失败: ${e}`)
  }
}

// 监听冲突事件（仅当无弹窗显示时才打开，避免重复）
const unsubConflict = taskStore.$subscribe((_mutation, state) => {
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
        <h1 class="text-2xl font-extrabold text-content tracking-tight mb-1">{{ appStore.t('nav.decompress') }}</h1>
        <p class="text-dim text-[0.5rem] font-semibold uppercase tracking-[0.15em]">{{ appStore.t('decompress.add_files') }}</p>
      </div>
      <div class="flex gap-3">
        <button
          v-if="!isRunning && taskStore.tasks.some(t => ['completed', 'failed', 'cancelled'].includes(t.status))"
          @click="taskStore.clearFinishedTasks()"
          class="h-9 px-5 rounded-lg bg-input border border-subtle text-muted text-[0.5625rem] font-bold uppercase tracking-wider hover:text-red-500 hover:border-red-500/30 transition-all flex items-center gap-2"
        >
          <i class="pi pi-trash text-xs"></i>
          {{ appStore.t('decompress.clear_finished') }}
        </button>
        <button
          v-if="isRunning"
          @click="cancelAllTasks"
          class="h-9 px-5 rounded-lg bg-red-500/10 text-red-500 border border-red-500/30 text-[0.5625rem] font-bold uppercase tracking-wider hover:bg-red-500 hover:text-white transition-all flex items-center gap-2"
        >
          <i class="pi pi-stop-circle text-xs"></i>
          {{ appStore.t('common.cancel') }}
        </button>
        <button
          v-if="hasPendingTasks && !isRunning"
          @click="startDecompression"
          class="h-9 px-6 rounded-lg bg-primary text-white text-[0.5625rem] font-bold uppercase tracking-wider hover:brightness-110 active:scale-[0.98] transition-all shadow-lg shadow-primary/25 flex items-center gap-2"
        >
          <i class="pi pi-play-circle text-xs"></i>
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
      @resolve="handleConflictResolve"
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
