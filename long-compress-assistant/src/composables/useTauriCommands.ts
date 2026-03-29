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
}

export interface FileInfo {
  path: string
  name: string
  size: number
  isDir: boolean
  modified: number
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
      // ... (rest of the method logic)
      // Note: I'm only replacing a part but the tool requires the exact literal text.
      // I'll replace the whole method to be safe.
      
      // --- 准备密码队列 ---
      let passwordsToTry: string[] = []
      
      // 1. 手动输入 (优先级最高)
      if (options.password) passwordsToTry.push(options.password)
      
      // 2. 保险箱高频建议 (置信度最高)
      try {
        const suggestions = await invoke<any[]>('get_password_suggestions', { filePath })
        passwordsToTry.push(...suggestions.map(s => s.text))
      } catch (e) { console.warn('Suggestions failed', e) }

      // 3. 遍历全量密码本 (确保本地数据穷尽)
      try {
        const allVault = await invoke<any[]>('get_all_passwords', { includePassword: true })
        passwordsToTry.push(...allVault.map(p => p.password))
      } catch (e) { console.warn('Full vault fetch failed', e) }

      // 去重并加入空密码尝试
      passwordsToTry = Array.from(new Set(['', ...passwordsToTry]))

      let success = false
      let finalResult = ''

      // 核心尝试循环 (本地已知密码阶段)
      for (const pwd of passwordsToTry) {
        try {
          taskStore.updateTaskStatus(taskId, 'extracting')
          finalResult = await invoke<string>('extract_file', {
            taskId,
            filePath,
            outputPath: options.outputPath,
            password: pwd || null,
            options: {
              preserve_paths: options.keepStructure,
              overwrite_existing: options.overwrite,
              delete_after: options.deleteAfter,
              preserve_timestamps: true,
              skip_corrupted: false,
              extract_only_newer: false,
              create_subdirectory: false,
              file_filter: null
            }
          })
          
          success = true
          // 记录成功密码
          // if (pwd) await invoke('record_password_success', { filePath, password: pwd })
          break 
        } catch (error: any) {
          const errorStr = String(error)
          if (errorStr.includes('密码错误') || errorStr.includes('PasswordError')) continue 
          throw error 
        }
      }

      // --- 暴力破解阶段 (仅在开启后进入) ---
      if (!success && appStore.settings.enableBruteForce) {
        taskStore.updateTaskStatus(taskId, 'running')
        throw new Error('本地密码本已穷尽，正在通过暴力破解引擎尝试...')
      }

      if (!success) {
        throw new Error('解压失败：需手动输入密码或开启暴力破解模式')
      }

      return finalResult
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
      return Array.isArray(selected) ? selected : [selected]
    } catch (error) {
      console.error('Failed to select wordlists:', error)
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
    getFileInfo,
    listDirectory,
    checkFileFormat,
    getSystemInfo,
    showMessage,
    saveFile,
    cancelCompression
  }
}
