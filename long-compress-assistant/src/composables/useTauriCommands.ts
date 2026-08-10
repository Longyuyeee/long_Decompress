import { invoke } from '@tauri-apps/api/tauri'
import { message, open, save, ask } from '@tauri-apps/api/dialog'
import { useAppStore } from '@/stores/app'
import { useTaskStore } from '@/stores/task'
import { extractErrorMessage } from '@/utils'

export interface DecompressOptions {
  outputPath: string
  password?: string
  keepStructure: boolean
  overwrite: boolean
  deleteAfter: boolean
  preserveTimestamps?: boolean
  skipCorrupted?: boolean
  extractOnlyNewer?: boolean
  createSubdirectory?: boolean
  fileFilter?: string | null
  conflictPolicy?: 'ask' | 'overwrite' | 'skip' | 'rename'
}

export interface FileInfo {
  path: string
  name: string
  size: number
  isDir: boolean
  modified: number
}

export interface CompressOptions {
  format?: string
  level: number
  password?: string
  split_size?: number | null
  preserve_paths?: boolean
  delete_after?: boolean
  allow_insecure_password_cli?: boolean
}

export interface RarCompressionSupport {
  available: boolean
  encoder_path?: string | null
  message: string
}

export interface ArchiveEngineFormatCapability {
  name: string
  extensions: string[]
  canCreate: boolean
}

export interface ArchiveEngineCapabilities {
  available: boolean
  command?: string | null
  version?: string | null
  fullEngine: boolean
  formats: ArchiveEngineFormatCapability[]
  message: string
}

interface WordlistValidationResult {
  path: string
  valid: boolean
  valid_password_count: number
  error?: string | null
}

export const useTauriCommands = () => {
  const appStore = useAppStore()
  const taskStore = useTaskStore()

  /**
   * 选择文件
   */
  const selectFiles = async (multiple = true, filters?: { name: string, extensions: string[] }[]) => {
    try {
      const selected = await open({
        multiple,
        filters: filters || [
          {
            name: appStore.t('dialog.compress_files'),
            extensions: [
              'zip', 'zipx', 'rar', '7z',
              'tar', 'gz', 'gzip', 'bz2', 'bzip2', 'xz', 'zst', 'zstd', 'lzma',
              'tgz', 'tpz', 'tbz', 'tbz2', 'txz', 'tzst',
              'jar', 'xpi', 'ipa', 'apk', 'appx',
              'iso', 'img', 'cab', 'lzh', 'lha', 'arj', 'dmg', 'wim', 'vhd', 'vhdx', 'chm',
              'deb', 'rpm', 'squashfs', 'sfs', 'msi', 'nsis', 'xar', 'cpio'
            ]
          },
          {
            name: appStore.t('dialog.all_files'),
            extensions: ['*']
          }
        ]
      })

      if (!selected) return []

      const files = Array.isArray(selected) ? selected : [selected]
      const fileInfos: FileInfo[] = []
      const metadataBatchSize = 16
      for (let start = 0; start < files.length; start += metadataBatchSize) {
        const batch = await Promise.all(
          files.slice(start, start + metadataBatchSize).map(filePath => getFileInfo(filePath))
        )
        fileInfos.push(...batch.filter((info): info is FileInfo => info !== null))
      }
      return fileInfos
    } catch (error) {
      console.error('Failed to select files:', error)
      await message(appStore.t('dialog.select_file_error').replace('{0}', String(error)), { type: 'error' })
      return []
    }
  }

  /**
   * 选择目录
   */
  const selectDirectory = async (defaultPath?: string) => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath
      })

      return selected || null
    } catch (error) {
      console.error('Failed to select directory:', error)
      await message(appStore.t('dialog.select_dir_error').replace('{0}', String(error)), { type: 'error' })
      return null
    }
  }

  /**
   * 智能解压文件 (严密五级密码尝试逻辑)
   */
  const decompressFile = async (
    filePath: string,
    options: DecompressOptions,
    existingTaskId?: string
  ) => {
    const fileName = filePath.split(/[\\/]/).pop() || 'unknown'
    const taskId = existingTaskId || taskStore.addTask({
      id: Date.now().toString(),
      name: fileName,
      type: 'decompression',
      sourceFiles: [filePath],
      outputPath: options.outputPath,
      format: filePath.split('.').pop() || 'zip'
    })

    try {
      taskStore.updateTaskStatus(taskId, 'preparing')
      taskStore.updateTaskStatus(taskId, 'extracting')

      const result = await invoke<string>('extract_file', {
        taskId,
        filePath,
        outputPath: options.outputPath,
        password: options.password || null,
        options: {
          preserve_paths: options.keepStructure,
          overwrite_existing: options.overwrite,
          delete_after: options.deleteAfter,
          preserve_timestamps: options.preserveTimestamps ?? true,
          skip_corrupted: options.skipCorrupted ?? false,
          extract_only_newer: options.extractOnlyNewer ?? false,
          create_subdirectory: options.createSubdirectory ?? false,
          preserve_mark_of_web: appStore.settings.preserveMarkOfWeb,
          file_filter: options.fileFilter || null,
          conflict_policy: options.conflictPolicy || 'rename',
          enable_bruteforce: appStore.settings.enableBruteForce,
          bruteforce_wordlists: appStore.settings.bruteForceWordlists
        }
      })
      const task = taskStore.tasks.find(item => item.id === taskId)
      if (task) task.progress = 100
      taskStore.updateTaskStatus(taskId, 'completed')
      return result
    } catch (error: any) {
      const task = taskStore.tasks.find(item => item.id === taskId)
      if (task && !['cancelled', 'cancelling'].includes(task.status)) {
        taskStore.updateTaskStatus(taskId, 'failed')
      }
      throw error
    }
  }

  /**
   * 批量解压文件
   */
  const decompressFiles = async (
    files: Array<{ path: string, options: DecompressOptions }>
  ) => {
    const results = []
    for (const file of files) {
      try {
        const result = await decompressFile(file.path, file.options)
        results.push({ file: file.path, success: true, result })
      } catch (error) {
        results.push({ 
          file: file.path, 
          success: false, 
          error: error instanceof Error ? error.message : String(error) 
        })
      }
    }
    return results
  }

  const compressFiles = async (
    taskId: string,
    files: string[],
    outputPath: string,
    options: CompressOptions
  ) => {
    return await invoke<string>('compress_files', {
      taskId,
      files,
      outputPath,
      options
    })
  }

  const checkRarCompressionSupport = async (): Promise<RarCompressionSupport> => {
    return await invoke<RarCompressionSupport>('check_rar_compression_support')
  }

  const installWinRarWithWinget = async (): Promise<RarCompressionSupport> => {
    return await invoke<RarCompressionSupport>('install_winrar_with_winget')
  }

  const getArchiveEngineCapabilities = async (): Promise<ArchiveEngineCapabilities> => {
    return await invoke<ArchiveEngineCapabilities>('get_archive_engine_capabilities')
  }

  const openRarDownloadPage = async (): Promise<void> => {
    await invoke<void>('open_rar_download_page')
  }

  /**
   * 获取文件信息
   */
  const getFileInfo = async (filePath: string): Promise<FileInfo | null> => {
    try {
      const metadata = await invoke<any>('get_file_info', { path: filePath })
      return {
        path: filePath,
        name: metadata.name,
        size: metadata.size,
        isDir: metadata.is_dir,
        modified: metadata.modified ? new Date(metadata.modified).getTime() : Date.now()
      }
    } catch (error) {
      console.error('Failed to get file info:', error)
      return null
    }
  }

  /**
   * 列出目录内容
   */
  const listDirectory = async (dirPath: string): Promise<FileInfo[]> => {
    try {
      const fileInfos = await invoke<FileInfo[]>('list_files', { path: dirPath })

      return fileInfos.sort((a, b) => {
        if (a.isDir && !b.isDir) return -1
        if (!a.isDir && b.isDir) return 1
        return a.name.localeCompare(b.name)
      })
    } catch (error) {
      console.error('Failed to list directory:', error)
      return []
    }
  }

  /**
   * 检查文件是否可解压
   */
  const checkFileFormat = async (filePath: string): Promise<{
    supported: boolean
    format?: string
    encrypted: boolean
    error?: string
  }> => {
    // 通过后端 list_archive_contents 检测格式是否可解压
    try {
      await invoke<string[]>('list_archive_contents', { filePath, password: null })
      const ext = filePath.split('.').pop()?.toLowerCase() || 'unknown'
      return { supported: true, format: ext, encrypted: false }
    } catch (error) {
      const errMsg = error instanceof Error ? error.message : String(error)
      return {
        supported: false,
        encrypted: errMsg.toLowerCase().includes('password') || errMsg.toLowerCase().includes('encrypted'),
        error: errMsg
      }
    }
  }

  /**
   * 获取系统信息
   */
  const getSystemInfo = async () => {
    try {
      const result = await invoke('get_system_info')
      return result
    } catch (error) {
      console.error('Failed to get system info:', error)
      return null
    }
  }

  /**
   * 显示消息对话框
   */
  const showMessage = async (title: string, messageText: string, type: 'info' | 'warning' | 'error' = 'info') => {
    try {
      await message(messageText, { title, type })
    } catch (error) {
      console.error('Failed to show message:', error)
    }
  }

  /**
   * 保存文件
   */
  const saveFile = async (defaultPath?: string, filters?: { name: string, extensions: string[] }[]) => {
    try {
      const path = await save({
        defaultPath,
        filters: filters || [
          {
            name: appStore.t('dialog.text_files'),
            extensions: ['txt', 'md', 'json']
          }
        ]
      })
      return path
    } catch (error) {
      console.error('Failed to save file:', error)
      return null
    }
  }

  /**
   * 选择密码本 (TXT文件)
   */
  const selectWordlists = async () => {
    try {
      const selected = await open({
        multiple: true,
        filters: [
          {
            name: appStore.t('dialog.wordlist'),
            extensions: ['txt']
          }
        ]
      })

      if (!selected) return []
      const selectedPaths = Array.from(new Set(Array.isArray(selected) ? selected : [selected]))
      const validation = await invoke<WordlistValidationResult[]>('validate_wordlists', {
        paths: selectedPaths
      })

      const validPaths = validation
        .filter(result => result.valid)
        .map(result => result.path)
      const invalidResults = validation.filter(result => !result.valid)

      if (invalidResults.length > 0) {
        const details = invalidResults
          .slice(0, 5)
          .map(result => `${result.path.split(/[\\/]/).pop() || result.path}: ${result.error || 'Invalid wordlist'}`)
          .join('\n')
        const suffix = invalidResults.length > 5 ? `\n...and ${invalidResults.length - 5} more` : ''
        await message(
          appStore.t('dialog.wordlist_skipped').replace('{0}', String(invalidResults.length)) + '\n' + details + suffix,
          { title: appStore.t('dialog.wordlist_validation'), type: 'warning' }
        )
      }

      return validPaths
    } catch (error) {
      console.error('Failed to select wordlists:', error)
      await message(`Failed to validate wordlists: ${error}`, {
        title: 'Wordlist validation',
        type: 'error'
      })
      return []
    }
  }

  /**
   * 取消压缩/解压任务
   */
  /**
   * 在系统文件管理器中打开路径
   */
  /**
   * 导出密码到JSON文件
   */
  const exportPasswords = async () => {
    try {
      const encrypted = await invoke<any[]>('list_encrypted_passwords', {})
      if (!encrypted || encrypted.length === 0) {
        await message(appStore.t('dialog.no_passwords_export'), { title: appStore.t('vault.export'), type: 'info' })
        return
      }

      // 询问是否加密导出
      const shouldEncrypt = await ask(
        appStore.t('dialog.export_encrypt_prompt') || '是否使用密码加密导出文件？\n\n选择"是"将需要输入密码来加密导出；\n选择"否"将导出为未加密的JSON文件。',
        { title: appStore.t('vault.export'), type: 'info' }
      )

      let exportPassword: string | undefined
      if (shouldEncrypt) {
        // 使用浏览器原生 prompt 作为临时方案
        exportPassword = window.prompt(appStore.t('dialog.export_password_prompt') || '请输入导出密码：') || undefined
        if (!exportPassword) {
          await message(appStore.t('dialog.export_cancelled') || '已取消导出', { type: 'info' })
          return
        }
      }

      const savePath = await save({
        defaultPath: 'password_vault_export.json',
        filters: [{ name: appStore.t('dialog.json_files'), extensions: ['json'] }]
      })
      if (!savePath) return

      // 调用后端的导出命令
      const success = await invoke<boolean>('export_passwords_command', {
        filePath: savePath,
        exportPassword: exportPassword || null,
        encrypt: shouldEncrypt,
        includePasswords: true,
        includeMetadata: true,
        format: 'Json'
      })

      if (success) {
        await message(
          appStore.t('dialog.export_success').replace('{0}', String(encrypted.length)),
          { title: appStore.t('vault.export'), type: 'info' }
        )
      }
    } catch (error) {
      await message(appStore.t('dialog.export_failed').replace('{0}', extractErrorMessage(error)), { type: 'error' })
    }
  }

  /**
   * 从JSON文件导入密码
   */
  const importPasswords = async () => {
    try {
      const selectedPath = await open({
        multiple: false,
        filters: [{ name: appStore.t('dialog.json_files'), extensions: ['json'] }]
      })
      if (!selectedPath) return
      const filePath = Array.isArray(selectedPath) ? selectedPath[0] : selectedPath

      // 询问是否加密文件
      const isEncrypted = await ask(
        appStore.t('dialog.import_encrypted_prompt') || '导入文件是否使用密码加密？\n\n选择"是"将需要输入解密密码；\n选择"否"将作为未加密JSON文件导入。',
        { title: appStore.t('vault.import'), type: 'info' }
      )

      let importPassword: string | undefined
      if (isEncrypted) {
        importPassword = window.prompt(appStore.t('dialog.import_password_prompt') || '请输入导入密码：') || undefined
        if (!importPassword) {
          await message(appStore.t('dialog.import_cancelled') || '已取消导入', { type: 'info' })
          return
        }
      }

      // 调用后端的导入命令
      const imported = await invoke<number>('import_passwords_command', {
        filePath: filePath,
        importPassword: importPassword || null,
        encrypt: isEncrypted,
        format: 'Json'
      })

      await message(appStore.t('dialog.import_success').replace('{0}', String(imported)), { title: appStore.t('vault.import'), type: 'info' })
    } catch (error) {
      await message(appStore.t('dialog.import_failed').replace('{0}', extractErrorMessage(error)), { type: 'error' })
    }
  }

  const openInExplorer = async (path: string) => {
    try {
      await invoke('open_in_explorer', { path })
    } catch (error) {
      console.error('Failed to open in explorer:', error)
    }
  }

  const listArchiveContents = async (filePath: string, password?: string) => {
    return await invoke<string[]>('list_archive_contents', { filePath, password: password || null })
  }

  const testArchiveIntegrity = async (filePath: string, password?: string) => {
    return await invoke<string>('test_archive_integrity', { filePath, password: password || null })
  }

  const repairZip = async (filePath: string) => {
    return await invoke<string>('repair_zip', { filePath })
  }

  const cancelCompression = async (taskId: string) => {
    const task = taskStore.tasks.find(item => item.id === taskId)
    if (!task || ['completed', 'failed', 'cancelled', 'cancelling'].includes(task.status)) return
    const previousStatus = task.status
    taskStore.updateTaskStatus(taskId, 'cancelling')
    try {
      await invoke('cancel_compression', { taskId })
      taskStore.updateTaskStatus(taskId, 'cancelled')
    } catch (error) {
      taskStore.updateTaskStatus(taskId, previousStatus)
      console.error('Failed to cancel task:', error)
    }
  }

  return {
    invoke, // Export raw invoke for custom commands
    selectFiles,
    selectDirectory,
    selectWordlists,
    decompressFile,
    decompressFiles,
    compressFiles,
    checkRarCompressionSupport,
    installWinRarWithWinget,
    getArchiveEngineCapabilities,
    openRarDownloadPage,
    getFileInfo,
    listDirectory,
    checkFileFormat,
    getSystemInfo,
    showMessage,
    saveFile,
    exportPasswords,
    importPasswords,
    openInExplorer,
    listArchiveContents,
    testArchiveIntegrity,
    repairZip,
    cancelCompression,
    registerContextMenu: () => invoke<boolean>('register_context_menu'),
    unregisterContextMenu: () => invoke<boolean>('unregister_context_menu'),
    isContextMenuRegistered: () => invoke<boolean>('is_context_menu_registered')
  }
}
