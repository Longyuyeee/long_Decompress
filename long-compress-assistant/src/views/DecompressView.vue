<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import { useTaskStore } from '@/stores/task'
import { useAppStore } from '@/stores/app'
import { usePasswordStore } from '@/stores/password'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { extractErrorMessage, generateId, isPasswordRelatedError } from '@/utils'
import { open } from '@tauri-apps/api/dialog'
import { confirm } from '@tauri-apps/api/dialog'
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

        const createdTasks = taskStore.tasks.filter(
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
  for (const file of files) {
    const sourcePath = file.path
    // 去重：如果任务列表中已有相同文件，跳过
    if (taskStore.tasks.some(t => t.sourceFiles.includes(sourcePath))) continue

    // 检测是否为分卷文件
    try {
      const splitInfo = await tauriCommands.invoke<any>('detect_split_archive', { path: sourcePath })

      if (splitInfo && splitInfo.is_split) {
        // 检测到分卷文件
        const totalSizeMB = (splitInfo.total_size / 1024 / 1024).toFixed(2)
        const confirmed = await confirm(
          appStore.t('decompress.split.detected', `检测到分卷压缩文件！\n\n分卷数量: ${splitInfo.parts.length}\n总大小: ${totalSizeMB} MB\n\n是否自动添加所有分卷？`),
          { title: appStore.t('decompress.split.title', '分卷检测'), type: 'info' }
        )

        if (confirmed) {
          // 用户确认，添加所有分卷（作为单个任务，使用第一个分卷）
          const parentDir = sourcePath.substring(0, Math.max(sourcePath.lastIndexOf('/'), sourcePath.lastIndexOf('\\')))

          const taskId = taskStore.addTask({
            id: generateId(),
            name: (file.name || sourcePath.split(/[\\/]/).pop() || 'Unknown') + ` (${splitInfo.parts.length} 个分卷)`,
            type: 'decompression',
            sourceFiles: [splitInfo.first_part], // 使用第一个分卷
            outputPath: isGlobalSameDir.value ? parentDir : globalOutputPath.value,
            extractToSubfolder: globalExtractToSubfolder.value
          })
          createdTaskIds.push(taskId)

          appStore.setSuccess(
            appStore.t('decompress.split.added', `已添加分卷文件（${splitInfo.parts.length} 个分卷，总计 ${totalSizeMB} MB）`)
          )
          appStore.addRecentFile(splitInfo.first_part)
          continue // 跳过普通处理
        }
      }
    } catch (e) {
      // 分卷检测失败或不是分卷文件，继续普通处理
      console.debug('Split archive detection skipped:', e)
    }

    const parentDir = sourcePath.substring(0, Math.max(sourcePath.lastIndexOf('/'), sourcePath.lastIndexOf('\\')))

    const taskId = taskStore.addTask({
      id: generateId(),
      name: file.name || sourcePath.split(/[\\/]/).pop() || 'Unknown',
      type: 'decompression',
      sourceFiles: [sourcePath],
      outputPath: isGlobalSameDir.value ? parentDir : globalOutputPath.value,
      extractToSubfolder: globalExtractToSubfolder.value
    })
    createdTaskIds.push(taskId)
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

const startDecompression = async (onlyTaskIds?: string[]) => {
  // 防止重复点击
  if (isProcessing.value) return
  // 如果有选中的任务，优先处理选中的；否则处理所有 pending 任务
  const pendingTasks = onlyTaskIds
    ? taskStore.tasks.filter(t => onlyTaskIds.includes(t.id) && t.status === 'pending')
    : selectedTaskIds.value.size > 0
    ? taskStore.tasks.filter(t => selectedTaskIds.value.has(t.id) && t.status === 'pending')
    : taskStore.tasks.filter(t => t.status === 'pending')
  if (pendingTasks.length === 0) return

  isProcessing.value = true

  // 启动后清除选择
  selectedTaskIds.value = new Set()

  for (const task of pendingTasks) {
    const fileName = task.name || task.sourceFiles[0]?.split(/[\\/]/).pop() || ''

    // 不预先添加密码，先尝试解压，只有明确要求密码时才使用保险箱
    const options = {
      outputPath: task.outputPath,
      keepStructure: true,
      overwrite: false,
      deleteAfter: appStore.settings.autoDeleteSource,
      createSubdirectory: task.extractToSubfolder ?? false,
      password: task.password || undefined, // 只使用用户手动输入的密码
      fileFilter: task.fileFilter || null
      ,conflictPolicy: appStore.settings.conflictPolicy
    }

    try {
      taskStore.updateTaskStatus(task.id, 'preparing')
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
    } catch (error) {
      // 只在后端明确返回密码相关错误时才尝试密码破解
      const errorMsg = extractErrorMessage(error) || String(error)
      const isPasswordError = isPasswordRelatedError(error)

      if (isPasswordError && !task.password) {
        // 密码错误且用户未输入密码，现在尝试保险箱和字典攻击
        const candidates = passwordStore.findCandidatePasswords(fileName)

        if (candidates.length > 0) {
          task.logs.push({
            task_id: task.id,
            message: appStore.t('decompress.auto_trying').replace('{0}', String(candidates.length)),
            severity: 'info',
            timestamp: new Date().toISOString()
          })
        }

        const remainingCandidates = candidates
        if (remainingCandidates.length > 0) {
          taskStore.updateTaskStatus(task.id, 'extracting')
          task.logs.push({
            task_id: task.id,
            message: appStore.t('decompress.auto_trying').replace('{0}', String(remainingCandidates.length)),
            severity: 'info',
            timestamp: new Date().toISOString()
          })

          let succeeded = false
          for (const candidatePassword of remainingCandidates) {
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
            // 保险箱密码全部失败，尝试密码字典攻击
            taskStore.updateTaskStatus(task.id, 'extracting')
            task.logs.push({
              task_id: task.id,
              message: appStore.t('decompress.dictionary.trying', '正在尝试常用密码字典...'),
              severity: 'info',
              timestamp: new Date().toISOString()
            })

            try {
              // 调用密码字典服务
              const dictionaryPasswords = await tauriCommands.invoke<string[]>('get_dictionary_passwords', {
                fileName: fileName,
                strategy: 'recommended'
              })

              if (dictionaryPasswords && dictionaryPasswords.length > 0) {
                task.logs.push({
                  task_id: task.id,
                  message: appStore.t('decompress.dictionary.found', `字典中找到 ${dictionaryPasswords.length} 个候选密码，开始尝试...`),
                  severity: 'info',
                  timestamp: new Date().toISOString()
                })

                let dictSucceeded = false
                for (let i = 0; i < dictionaryPasswords.length; i++) {
                  if (dictSucceeded) break
                  const dictPassword = dictionaryPasswords[i]

                  // 每10个密码更新一次进度
                  if (i % 10 === 0) {
                    task.logs.push({
                      task_id: task.id,
                      message: appStore.t('decompress.dictionary.progress', `字典攻击进度: ${i}/${dictionaryPasswords.length}`),
                      severity: 'info',
                      timestamp: new Date().toISOString()
                    })
                  }

                  try {
                    task.password = dictPassword
                    task.passwordRequired = false
                    task.currentPassword = dictPassword
                    await tauriCommands.decompressFile(
                      task.sourceFiles[0],
                      { ...options, password: dictPassword },
                      task.id
                    )
                    dictSucceeded = true

                    // 成功后保存到密码保险箱
                    task.logs.push({
                      task_id: task.id,
                      message: appStore.t('decompress.dictionary.success', `✓ 密码破解成功！使用密码: ${dictPassword}`),
                      severity: 'success',
                      timestamp: new Date().toISOString()
                    })

                    // 保存到保险箱
                    try {
                      await passwordStore.addEntry({
                        password: dictPassword,
                        hint: `字典破解 - ${fileName}`,
                        tags: ['auto-cracked']
                      })
                    } catch { /* 非关键操作 */ }
                  } catch {
                    // 尝试下一个字典密码
                    continue
                  }
                }

                if (!dictSucceeded) {
                  // 字典攻击也失败
                  taskStore.updateTaskStatus(task.id, 'failed')
                  task.error = appStore.t('decompress.dictionary.all_failed', `所有密码尝试失败（保险箱: ${candidates.length}, 字典: ${dictionaryPasswords.length}）`)
                  task.logs.push({
                    task_id: task.id,
                    message: task.error!,
                    severity: 'error',
                    timestamp: new Date().toISOString()
                  })
                }
              } else {
                // 字典服务未返回密码
                taskStore.updateTaskStatus(task.id, 'failed')
                task.error = appStore.t('decompress.all_failed').replace('{0}', String(candidates.length))
                task.logs.push({
                  task_id: task.id,
                  message: task.error!,
                  severity: 'error',
                  timestamp: new Date().toISOString()
                })
              }
            } catch (dictError) {
              // 字典服务调用失败
              console.error('Dictionary attack failed:', dictError)
              taskStore.updateTaskStatus(task.id, 'failed')
              task.error = appStore.t('decompress.all_failed').replace('{0}', String(candidates.length))
              task.logs.push({
                task_id: task.id,
                message: task.error!,
                severity: 'error',
                timestamp: new Date().toISOString()
              })
            }
          }
        } else {
          // 没有保险箱候选密码，直接尝试字典攻击
          taskStore.updateTaskStatus(task.id, 'extracting')
          task.logs.push({
            task_id: task.id,
            message: appStore.t('decompress.dictionary.trying', '正在尝试常用密码字典...'),
            severity: 'info',
            timestamp: new Date().toISOString()
          })

          try {
            const dictionaryPasswords = await tauriCommands.invoke<string[]>('get_dictionary_passwords', {
              fileName: fileName,
              strategy: 'recommended'
            })

            if (dictionaryPasswords && dictionaryPasswords.length > 0) {
              task.logs.push({
                task_id: task.id,
                message: appStore.t('decompress.dictionary.found', `字典中找到 ${dictionaryPasswords.length} 个候选密码，开始尝试...`),
                severity: 'info',
                timestamp: new Date().toISOString()
              })

              let dictSucceeded = false
              for (let i = 0; i < dictionaryPasswords.length; i++) {
                if (dictSucceeded) break
                const dictPassword = dictionaryPasswords[i]

                if (i % 10 === 0) {
                  task.logs.push({
                    task_id: task.id,
                    message: appStore.t('decompress.dictionary.progress', `字典攻击进度: ${i}/${dictionaryPasswords.length}`),
                    severity: 'info',
                    timestamp: new Date().toISOString()
                  })
                }

                try {
                  task.password = dictPassword
                  task.passwordRequired = false
                  task.currentPassword = dictPassword
                  await tauriCommands.decompressFile(
                    task.sourceFiles[0],
                    { ...options, password: dictPassword },
                    task.id
                  )
                  dictSucceeded = true

                  task.logs.push({
                    task_id: task.id,
                    message: appStore.t('decompress.dictionary.success', `✓ 密码破解成功！使用密码: ${dictPassword}`),
                    severity: 'success',
                    timestamp: new Date().toISOString()
                  })

                  try {
                    await passwordStore.addEntry({
                      password: dictPassword,
                      hint: `字典破解 - ${fileName}`,
                      tags: ['auto-cracked']
                    })
                  } catch { /* 非关键操作 */ }
                } catch {
                  continue
                }
              }

              if (!dictSucceeded) {
                taskStore.updateTaskStatus(task.id, 'failed')
                task.error = appStore.t('decompress.dictionary.all_failed', `所有字典密码尝试失败（${dictionaryPasswords.length} 个）`)
                task.logs.push({
                  task_id: task.id,
                  message: task.error!,
                  severity: 'error',
                  timestamp: new Date().toISOString()
                })
              }
            } else {
              taskStore.updateTaskStatus(task.id, 'failed')
              task.error = extractErrorMessage(error) || appStore.t('decompress.no_vault_passwords')
              task.logs.push({
                task_id: task.id,
                message: task.error!,
                severity: 'error',
                timestamp: new Date().toISOString()
              })
            }
          } catch (dictError) {
            console.error('Dictionary attack failed:', dictError)
            taskStore.updateTaskStatus(task.id, 'failed')
            task.error = extractErrorMessage(error) || appStore.t('decompress.no_vault_passwords')
            task.logs.push({
              task_id: task.id,
              message: task.error!,
              severity: 'error',
              timestamp: new Date().toISOString()
            })
          }
        }
        if (task.status === 'failed') {
          task.passwordRequired = true
          task.password = ''
          task.currentPassword = undefined
        }
      } else if (isPasswordError) {
        taskStore.updateTaskStatus(task.id, 'failed')
        task.passwordRequired = true
        task.error = appStore.t('tasks.password.wrong')
        task.logs.push({
          task_id: task.id,
          message: task.error,
          severity: 'error',
          timestamp: new Date().toISOString()
        })
      } else {
        // 非密码错误，直接失败
        taskStore.updateTaskStatus(task.id, 'failed')
        task.error = errorMsg
        appStore.setError(`${appStore.t('common.error')}: ${task.error}`)
      }
    }
  }

  isProcessing.value = false
}

const hasPendingTasks = computed(() => taskStore.tasks.some(t => t.status === 'pending'))
const isRunning = computed(() => taskStore.tasks.some(t => ['running', 'extracting', 'compressing', 'preparing'].includes(t.status)))

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

  if (action === 'skip') {
    task.conflicts = []
    taskStore.updateTaskStatus(taskId, 'cancelled')
    return
  }

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
    ,conflictPolicy: action
  }
  try {
    await tauriCommands.decompressFile(task.sourceFiles[0], options, taskId)
  } catch (e: any) {
    appStore.setError(appStore.t('decompress.extract_failed').replace('{0}', e))
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
        <h1 class="text-2xl md:text-3xl font-black text-content tracking-tight">{{ appStore.t('nav.decompress') }}</h1>
        <p class="text-xs md:text-sm text-muted font-semibold mt-1">{{ appStore.t('decompress.subtitle') }}</p>
      </div>
      <div class="flex gap-3">
        <button
          v-if="!isRunning && taskStore.tasks.some(t => ['completed', 'failed', 'cancelled'].includes(t.status))"
          @click="taskStore.clearFinishedTasks()"
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
        <div v-if="taskStore.tasks.length > 0" class="flex-1 min-h-0">
          <AeroTable
          :selectedTaskIds="selectedTaskIds"
          statusFilter="all"
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
            class="w-full max-w-lg shadow-sm"
          />
        </div>
      </div>

      <!-- 底部操作区 -->
      <div v-if="taskStore.tasks.length > 0" class="border-t border-subtle bg-input/10 px-3 py-3 flex items-center gap-3 flex-wrap shrink-0">
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

        <div class="w-px h-5 bg-subtle/20 mx-1 hidden md:block"></div>

        <EnhancedFileDropzone
          @files-selected="onFilesSelected"
          :compact="true"
          :accept="supportedArchiveAccept"
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
  background: radial-gradient(circle at 0% 0%, color-mix(in srgb, var(--dynamic-accent) 4%, transparent) 0%, transparent 40%);
}

.fade-morph-enter-active, .fade-morph-leave-active { transition: all 0.6s cubic-bezier(0.34, 1.56, 0.64, 1); }
.fade-morph-enter-from { opacity: 0; transform: scale(0.98); }
.fade-morph-leave-to { opacity: 0; transform: scale(1.02); }
</style>
