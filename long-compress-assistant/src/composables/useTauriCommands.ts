import { invoke } from '@tauri-apps/api/tauri'
import { message, open, save } from '@tauri-apps/api/dialog'
import { fs } from '@tauri-apps/api'
import { useAppStore } from '@/stores/app'
import { useTaskStore } from '@/stores/task'

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
}

export interface RarCompressionSupport {
  available: boolean
  encoder_path?: string | null
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
            name: '压缩文件',
            extensions: ['zip', 'rar', '7z', 'tar', 'gz', 'bz2']
          },
          {
            name: '所有文件',
            extensions: ['*']
          }
        ]
      })

      if (!selected) return []

      const files = Array.isArray(selected) ? selected : [selected]
      const fileInfos: FileInfo[] = []

      for (const filePath of files) {
        const info = await getFileInfo(filePath)
        if (info) fileInfos.push(info)
      }

      return fileInfos
    } catch (error) {
      console.error('Failed to select files:', error)
      await message(`选择文件失败: ${error}`, { type: 'error' })
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
      await message(`选择目录失败: ${error}`, { type: 'error' })
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

      return await invoke<string>('extract_file', {
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
          file_filter: options.fileFilter || null,
          enable_bruteforce: appStore.settings.enableBruteForce,
          bruteforce_wordlists: appStore.settings.bruteForceWordlists
        }
      })
    } catch (error: any) {
      taskStore.updateTaskStatus(taskId, 'failed')
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
      const entries = await fs.readDir(dirPath)
      const fileInfos: FileInfo[] = []

      for (const entry of entries) {
        const info = await getFileInfo(entry.path)
        if (info) fileInfos.push(info)
      }

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
    try {
      const result = await invoke('check_file_format', { filePath })
      return result as any
    } catch (error) {
      console.error('Failed to check file format:', error)
      return {
        supported: false,
        encrypted: false,
        error: error instanceof Error ? error.message : String(error)
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
            name: '文本文件',
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
            name: '密码本 (Wordlist)',
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
        await message(`Skipped ${invalidResults.length} invalid wordlist file(s):\n${details}${suffix}`, {
          title: 'Wordlist validation',
          type: 'warning'
        })
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
  const cancelCompression = async (taskId: string) => {
    try {
      await invoke('cancel_compression', { taskId })
      taskStore.updateTaskStatus(taskId, 'cancelled')
    } catch (error) {
      console.error('Failed to cancel task:', error)
    }
  }

  return {
    selectFiles,
    selectDirectory,
    selectWordlists,
    decompressFile,
    decompressFiles,
    compressFiles,
    checkRarCompressionSupport,
    getFileInfo,
    listDirectory,
    checkFileFormat,
    getSystemInfo,
    showMessage,
    saveFile,
    cancelCompression
  }
}
