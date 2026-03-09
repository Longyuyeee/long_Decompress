import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'
import type { FileItem } from './app'

export interface CompressionOptions {
  format: 'zip' | '7z' | 'tar' | 'gz' | 'bz2' | 'tar.gz' | 'tar.bz2' | 'xz' | 'tar.xz' | 'rar'
  level: number
  password: string
  filename: string
  splitArchive: boolean
  splitSize: string
  keepStructure: boolean
  deleteAfter: boolean
  createSolidArchive: boolean
}

export interface CompressionTask {
  id: string
  files: FileItem[]
  options: CompressionOptions
  outputPath: string
  status: 'pending' | 'processing' | 'completed' | 'failed'
  progress: number
  startTime?: Date
  endTime?: Date
  error?: string
  resultPath?: string
}

export interface CompressionHistory {
  id: string
  taskId: string
  timestamp: Date
  operation: 'compress' | 'decompress'
  files: string[]
  options: CompressionOptions
  outputPath: string
  resultPath?: string
  status: 'success' | 'error'
  duration?: number
  originalSize: number
  compressedSize?: number
  compressionRatio?: number
}

// 辅助函数
const generateId = () => Math.random().toString(36).substring(2, 15)

const getFormatExtension = (format: string) => {
  switch (format) {
    case 'tar.gz': return 'tar.gz'
    case 'tar.bz2': return 'tar.bz2'
    case 'tar.xz': return 'tar.xz'
    default: return format
  }
}

export const useCompressionStore = defineStore('compression', () => {
  // 状态
  const selectedFiles = ref<FileItem[]>([])
  const compressionOptions = ref<CompressionOptions>({
    format: 'zip',
    level: 6,
    password: '',
    filename: '',
    splitArchive: false,
    splitSize: '1024',
    keepStructure: true,
    deleteAfter: false,
    createSolidArchive: false
  })
  const outputPath = ref('')
  const isProcessing = ref(false)
  const currentProgress = ref(0)
  const totalProgress = ref(0)
  const processedFiles = ref(0)
  const compressionTasks = ref<CompressionTask[]>([])
  const compressionHistory = ref<CompressionHistory[]>([])
  const errorMessage = ref('')
  const successMessage = ref('')

  // 监听任务队列事件
  listen<{ task_id: string, progress: number }>('task_progress', (event) => {
    const task = compressionTasks.value.find(t => t.id === event.payload.task_id)
    if (task) {
      task.progress = Math.round(event.payload.progress * 100)
      if (task.status === 'processing') {
        currentProgress.value = task.progress
        totalProgress.value = task.progress
      }
    }
  })

  listen<{ task_id: string, status: string }>('task_status', (event) => {
    const task = compressionTasks.value.find(t => t.id === event.payload.task_id)
    if (task) {
      const s = event.payload.status.toLowerCase()
      if (s.includes('completed')) {
        task.status = 'completed'
        task.progress = 100
        task.endTime = new Date()
        
        // 自动加入历史记录
        addToHistory({
          taskId: task.id,
          files: task.files.map(f => f.path),
          options: task.options,
          outputPath: task.outputPath,
          resultPath: task.resultPath || task.outputPath,
          originalSize: task.files.reduce((acc, f) => acc + f.size, 0),
          status: 'success'
        })
        
        checkAllCompleted()
      } else if (s.includes('failed') || s.includes('error')) {
        task.status = 'failed'
        task.endTime = new Date()
        
        // 自动加入历史记录 (失败状态)
        addToHistory({
          taskId: task.id,
          files: task.files.map(f => f.path),
          options: task.options,
          outputPath: task.outputPath,
          originalSize: task.files.reduce((acc, f) => acc + f.size, 0),
          status: 'error'
        })
        
        checkAllCompleted()
      } else if (s.includes('running')) {
        task.status = 'processing'
      }
    }
  })

  const checkAllCompleted = () => {
    const pendingOrProcessing = compressionTasks.value.filter(t => t.status === 'pending' || t.status === 'processing')
    if (pendingOrProcessing.length === 0) {
      isProcessing.value = false
      const failedTasks = compressionTasks.value.filter(task => task.status === 'failed')
      if (failedTasks.length === 0 && compressionTasks.value.length > 0) {
        successMessage.value = '压缩完成'
        setTimeout(() => { successMessage.value = '' }, 5000)
      } else if (failedTasks.length > 0) {
        errorMessage.value = `共 ${failedTasks.length} 个任务压缩失败`
        setTimeout(() => { errorMessage.value = '' }, 5000)
      }
    }
  }

  // 计算属性
  const canStart = computed(() => {
    return selectedFiles.value.length > 0 && outputPath.value.length > 0
  })

  const totalFileSize = computed(() => {
    return selectedFiles.value.reduce((sum, file) => sum + file.size, 0)
  })

  const estimatedCompressionRatio = computed(() => {
    const baseRatios: Record<string, number> = {
      zip: 0.7, '7z': 0.5, tar: 1.0, gz: 0.6, bz2: 0.55,
      'tar.gz': 0.6, 'tar.bz2': 0.55, xz: 0.45, 'tar.xz': 0.45, rar: 0.65
    }
    const levelFactor = 1 - (compressionOptions.value.level / 10) * 0.3
    const baseRatio = baseRatios[compressionOptions.value.format] || 0.7
    return Math.round(baseRatio * levelFactor * 100)
  })

  const estimatedCompressedSize = computed(() => {
    return totalFileSize.value * (estimatedCompressionRatio.value / 100)
  })

  const activeTask = computed(() => {
    return compressionTasks.value.find(task =>
      task.status === 'processing' || task.status === 'pending'
    )
  })

  const recentHistory = computed(() => {
    return [...compressionHistory.value]
      .sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime())
      .slice(0, 10)
  })

  // 方法 - 文件操作
  const addFiles = (files: FileItem[]) => {
    selectedFiles.value = [...selectedFiles.value, ...files]
  }

  const removeFile = (fileId: string) => {
    selectedFiles.value = selectedFiles.value.filter(file => file.id !== fileId)
  }

  const clearFiles = () => {
    selectedFiles.value = []
  }

  // 方法 - 压缩选项
  const updateCompressionOptions = (options: Partial<CompressionOptions>) => {
    compressionOptions.value = { ...compressionOptions.value, ...options }
  }

  const setOutputPath = (path: string) => {
    outputPath.value = path
  }

  // 方法 - 压缩操作
  const startCompression = async () => {
    if (!canStart.value) {
      throw new Error('无法开始压缩：请选择文件和输出路径')
    }

    isProcessing.value = true
    currentProgress.value = 0
    totalProgress.value = 0
    processedFiles.value = 0
    errorMessage.value = ''
    successMessage.value = ''

    try {
      const filePaths = selectedFiles.value.map(file => file.path)

      let outputFileName = compressionOptions.value.filename
      if (!outputFileName) {
        const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19)
        outputFileName = `压缩文件_${timestamp}`
      }

      const extension = getFormatExtension(compressionOptions.value.format)
      const fullOutputPath = `${outputPath.value}/${outputFileName}.${extension}`

      // 将前端格式字符串转换为 Rust 后端的 CompressionFormat 枚举
      const formatMap: Record<string, string> = {
        'zip': 'Zip',
        '7z': 'SevenZip',
        'tar': 'Tar',
        'gz': 'Gzip',
        'bz2': 'Bzip2',
        'xz': 'Xz'
      }
      
      const backendFormat = formatMap[compressionOptions.value.format] || 'Zip'

      const request = {
        source_files: filePaths,
        output_path: fullOutputPath,
        format: backendFormat,
        options: {
          password: compressionOptions.value.password || null,
          compression_level: compressionOptions.value.level,
          split_size: compressionOptions.value.splitArchive && compressionOptions.value.splitSize
            ? parseInt(compressionOptions.value.splitSize) * 1024 * 1024
            : null,
          preserve_paths: compressionOptions.value.keepStructure,
          exclude_patterns: [],
          include_patterns: [],
          create_subdirectories: true,
          overwrite_existing: true
        },
        priority: 'Medium'
      }

      // 将任务加入后端队列，取代原来的同步阻塞执行
      const taskId = await invoke<string>('add_compression_task', { request })

      const task: CompressionTask = {
        id: taskId,
        files: [...selectedFiles.value],
        options: { ...compressionOptions.value },
        outputPath: outputPath.value,
        resultPath: fullOutputPath,
        status: 'pending',
        progress: 0,
        startTime: new Date()
      }

      compressionTasks.value.push(task)

    } catch (error) {
      console.error('添加压缩任务失败:', error)
      errorMessage.value = `添加任务失败: ${error}`
      isProcessing.value = false
      throw error
    }
  }

  const cancelCompression = async (taskId: string) => {
    try {
      await invoke('cancel_task', { taskId })
    } catch (e) {
      console.error('取消任务失败:', e)
    }
  }

  // 方法 - 历史记录
  const addToHistory = (historyData: Omit<CompressionHistory, 'id' | 'timestamp' | 'operation' | 'duration' | 'compressedSize' | 'compressionRatio'>) => {
    const history: CompressionHistory = {
      id: generateId(),
      timestamp: new Date(),
      operation: 'compress',
      duration: 0,
      compressedSize: 0,
      compressionRatio: 0,
      ...historyData
    }

    compressionHistory.value.unshift(history)
    if (compressionHistory.value.length > 100) {
      compressionHistory.value = compressionHistory.value.slice(0, 100)
    }
    saveHistoryToStorage()
  }

  const saveHistoryToStorage = () => {
    try {
      localStorage.setItem('compression-history', JSON.stringify(compressionHistory.value))
    } catch(e) {
      console.error('保存历史记录失败', e)
    }
  }

  const clearHistory = () => {
    compressionHistory.value = []
    localStorage.removeItem('compression-history')
  }

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
  }

  return {
    selectedFiles,
    compressionOptions,
    outputPath,
    isProcessing,
    currentProgress,
    totalProgress,
    processedFiles,
    compressionTasks,
    compressionHistory,
    errorMessage,
    successMessage,
    canStart,
    totalFileSize,
    estimatedCompressionRatio,
    estimatedCompressedSize,
    activeTask,
    recentHistory,
    addFiles,
    removeFile,
    clearFiles,
    updateCompressionOptions,
    setOutputPath,
    startCompression,
    cancelCompression,
    clearHistory,
    formatFileSize
  }
})
