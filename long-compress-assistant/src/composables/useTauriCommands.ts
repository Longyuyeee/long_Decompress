import { invoke } from '@tauri-apps/api/tauri'
import { message } from '@tauri-apps/api/dialog'
import { open, save } from '@tauri-apps/api/dialog'
import { fs } from '@tauri-apps/api'
import { useAppStore } from '@/stores/app'

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
        try {
          const metadata = await invoke<any>('get_file_info', { path: filePath })
          fileInfos.push({
            path: filePath,
            name: metadata.name,
            size: metadata.size,
            isDir: metadata.is_dir,
            modified: metadata.modified ? new Date(metadata.modified).getTime() : Date.now()
          })
        } catch (error) {
          console.error(`Failed to get file info for ${filePath}:`, error)
        }
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
   * 解压文件
   */
  const decompressFile = async (
    filePath: string,
    options: DecompressOptions,
    onProgress?: (progress: number) => void
  ) => {
    try {
      appStore.setError('')

      // 检查输出目录是否存在
      try {
        await fs.createDir(options.outputPath, { recursive: true })
      } catch (error) {
        console.error('Failed to create output directory:', error)
        throw new Error(`无法创建输出目录: ${error}`)
      }

      // 调用Rust后端解压命令
      const result = await invoke('extract_file', {
        filePath,
        outputPath: options.outputPath,
        password: options.password || null,
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

      console.log('Decompress result:', result)

      // 模拟进度更新（实际应该从后端获取）
      if (onProgress) {
        for (let i = 0; i <= 100; i += 10) {
          setTimeout(() => onProgress(i), i * 20)
        }
      }

      return result
    } catch (error) {
      console.error('Failed to decompress file:', error)
      const errorMessage = `解压失败: ${error}`
      appStore.setError(errorMessage)
      throw error
    }
  }

  /**
   * 批量解压文件
   */
  const decompressFiles = async (
    files: Array<{ path: string, options: DecompressOptions }>,
    onProgress?: (fileIndex: number, progress: number) => void
  ) => {
    const results = []

    for (let i = 0; i < files.length; i++) {
      const { path, options } = files[i]

      try {
        if (onProgress) {
          onProgress(i, 0)
        }

        const result = await decompressFile(path, options, (progress) => {
          if (onProgress) {
            onProgress(i, progress)
          }
        })

        results.push({
          file: path,
          success: true,
          result
        })

        if (onProgress) {
          onProgress(i, 100)
        }
      } catch (error) {
        results.push({
          file: path,
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
        try {
          const metadata = await invoke<any>('get_file_info', { path: entry.path })
          fileInfos.push({
            path: entry.path,
            name: entry.name || metadata.name,
            size: metadata.size,
            isDir: metadata.is_dir,
            modified: metadata.modified ? new Date(metadata.modified).getTime() : Date.now()
          })
        } catch (error) {
          console.error(`Failed to get info for ${entry.path}:`, error)
        }
      }

      return fileInfos.sort((a, b) => {
        // 目录在前，文件在后
        if (a.isDir && !b.isDir) return -1
        if (!a.isDir && b.isDir) return 1
        // 按名称排序
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

  return {
    selectFiles,
    selectDirectory,
    decompressFile,
    decompressFiles,
    getFileInfo,
    listDirectory,
    checkFileFormat,
    getSystemInfo,
    showMessage,
    saveFile
  }
}
