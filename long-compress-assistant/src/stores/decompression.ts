import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import type { FileItem } from './app'

export interface DecompressSettings {
  outputPath: string
  password: string
  options: {
    keepStructure: boolean
    overwriteStrategy: 'ask' | 'overwrite' | 'skip' | 'rename'
    deleteAfter: boolean
    preserveTimestamps: boolean
    skipCorrupted: boolean
    extractOnlyNewer: boolean
    createSubdirectory: boolean
    fileFilter: string
  }
  passwordOptions: {
    rememberForSession: boolean
    autoTryCommon: boolean
    maxAttempts: number
  }
}

export interface DecompressTask {
  id: string
  file: FileItem
  settings: DecompressSettings
  status: 'pending' | 'processing' | 'completed' | 'failed'
  progress: number
  startTime?: Date
  endTime?: Date
  error?: string
  resultPath?: string
  extractedFiles?: string[]
}

export interface DecompressHistory {
  id: string
  taskId: string
  timestamp: Date
  filePath: string
  settings: DecompressSettings
  outputPath: string
  resultPath?: string
  status: 'success' | 'error'
  duration?: number
  extractedCount?: number
  totalSize?: number
}

export const useDecompressionStore = defineStore('decompression', () => {
  // 状态
  const selectedFile = ref<FileItem | null>(null)
  const decompressSettings = ref<DecompressSettings>({
    outputPath: '',
    password: '',
    options: {
      keepStructure: true,
      overwriteStrategy: 'ask',
      deleteAfter: false,
      preserveTimestamps: true,
      skipCorrupted: false,
      extractOnlyNewer: false,
      createSubdirectory: false,
      fileFilter: ''
    },
    passwordOptions: {
      rememberForSession: false,
      autoTryCommon: false,
      maxAttempts: 3
    }
  })
  const isProcessing = ref(false)
  const currentProgress = ref(0)
  const decompressTasks = ref<DecompressTask[]>([])
  const decompressHistory = ref<DecompressHistory[]>([])
  const errorMessage = ref('')
  const successMessage = ref('')
  const showPassword = ref(false)

  // 计算属性
  const canStart = computed(() => {
    return selectedFile.value !== null
  })

  const activeTask = computed(() => {
    return decompressTasks.value.find(task =>
      task.status === 'processing' || task.status === 'pending'
    )
  })

  const recentHistory = computed(() => {
    return [...decompressHistory.value]
      .sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime())
      .slice(0, 10)
  })

  const requiresPassword = computed(() => {
    // 这里需要检测文件是否加密
    // 暂时返回false，实际应用中需要从文件信息中获取
    return false
  })

  const isPasswordValid = computed(() => {
    if (!requiresPassword.value) return true
    return decompressSettings.value.password.length > 0
  })

  // 方法 - 文件操作
  const setSelectedFile = (file: FileItem | null) => {
    selectedFile.value = file
  }

  const clearSelectedFile = () => {
    selectedFile.value = null
  }

  // 方法 - 解压设置
  const updateDecompressSettings = (settings: Partial<DecompressSettings>) => {
    decompressSettings.value = { ...decompressSettings.value, ...settings }
  }

  const setOutputPath = (path: string) => {
    decompressSettings.value.outputPath = path
  }

  const resetSettings = () => {
    decompressSettings.value = {
      outputPath: '',
      password: '',
      options: {
        keepStructure: true,
        overwriteStrategy: 'ask',
        deleteAfter: false,
        preserveTimestamps: true,
        skipCorrupted: false,
        extractOnlyNewer: false,
        createSubdirectory: false,
        fileFilter: ''
      },
      passwordOptions: {
        rememberForSession: false,
        autoTryCommon: false,
        maxAttempts: 3
      }
    }
  }

  // 方法 - 解压操作
  const startDecompression = async () => {
    if (!canStart.value || !selectedFile.value) {
      throw new Error('无法开始解压：请选择要解压的文件')
    }

    if (requiresPassword.value && !isPasswordValid.value) {
      throw new Error('检测到加密文件，请输入解压密码')
    }

    isProcessing.value = true
    currentProgress.value = 0
    errorMessage.value = ''
    successMessage.value = ''

    const taskId = generateId()
    const task: DecompressTask = {
      id: taskId,
      file: selectedFile.value,
      settings: { ...decompressSettings.value },
      status: 'processing',
      progress: 0,
      startTime: new Date()
    }

    decompressTasks.value.push(task)

    try {
      // 准备解压参数
      const outputPath = decompressSettings.value.outputPath ||
        selectedFile.value.path.replace(/\.[^/.]+$/, '') // 默认解压到同名目录

      // 调用Tauri解压API
      const result = await invoke('extract_file', {
        filePath: selectedFile.value.path,
        outputPath: outputPath,
        password: decompressSettings.value.password || null,
        options: {
          preserve_paths: decompressSettings.value.options.keepStructure,
          overwrite_existing: decompressSettings.value.options.overwriteStrategy === 'overwrite',
          delete_after: decompressSettings.value.options.deleteAfter,
          preserve_timestamps: decompressSettings.value.options.preserveTimestamps,
          skip_corrupted: decompressSettings.value.options.skipCorrupted,
          extract_only_newer: decompressSettings.value.options.extractOnlyNewer,
          create_subdirectory: decompressSettings.value.options.createSubdirectory,
          file_filter: decompressSettings.value.options.fileFilter || null
        }
      })

      console.log('解压结果:', result)

      // 更新任务状态
      const taskIndex = decompressTasks.value.findIndex(t => t.id === taskId)
      if (taskIndex !== -1) {
        decompressTasks.value[taskIndex].status = 'completed'
        decompressTasks.value[taskIndex].endTime = new Date()
        decompressTasks.value[taskIndex].resultPath = outputPath
        decompressTasks.value[taskIndex].progress = 100
        // 这里可以解析result获取解压的文件列表
      }

      // 添加到历史记录
      addToHistory({
        taskId,
        filePath: selectedFile.value.path,
        settings: decompressSettings.value,
        outputPath: outputPath,
        resultPath: outputPath,
        status: 'success'
      })

      successMessage.value = '解压完成'
      setTimeout(() => { successMessage.value = '' }, 5000)

    } catch (error) {
      console.error('解压失败:', error)

      // 更新任务状态
      const taskIndex = decompressTasks.value.findIndex(t => t.id === taskId)
      if (taskIndex !== -1) {
        decompressTasks.value[taskIndex].status = 'failed'
        decompressTasks.value[taskIndex].endTime = new Date()
        decompressTasks.value[taskIndex].error = error instanceof Error ? error.message : String(error)
      }

      // 添加到历史记录
      addToHistory({
        taskId,
        filePath: selectedFile.value.path,
        settings: decompressSettings.value,
        outputPath: decompressSettings.value.outputPath,
        status: 'error'
      })

      errorMessage.value = `解压失败: ${error}`
      setTimeout(() => { errorMessage.value = '' }, 5000)

      throw error
    } finally {
      isProcessing.value = false
    }
  }

  const cancelDecompression = (taskId: string) => {
    const taskIndex = decompressTasks.value.findIndex(t => t.id === taskId)
    if (taskIndex !== -1) {
      decompressTasks.value[taskIndex].status = 'failed'
      decompressTasks.value[taskIndex].endTime = new Date()
      decompressTasks.value[taskIndex].error = '用户取消'
    }
  }

  // 方法 - 密码管理
  const togglePasswordVisibility = () => {
    showPassword.value = !showPassword.value
  }

  const clearPassword = () => {
    decompressSettings.value.password = ''
  }

  // 方法 - 历史记录
  const addToHistory = (historyData: Omit<DecompressHistory, 'id' | 'timestamp' | 'duration' | 'extractedCount' | 'totalSize'>) => {
    const history: DecompressHistory = {
      id: generateId(),
      timestamp: new Date(),
      duration: 0, // 实际应用中需要计算
      extractedCount: 0, // 实际应用中需要获取
      totalSize: 0, // 实际应用中需要获取
      ...historyData
    }

    decompressHistory.value.unshift(history)

    // 保持历史记录不超过100条
    if (decompressHistory.value.length > 100) {
      decompressHistory.value = decompressHistory.value.slice(0, 100)
    }

    saveHistoryToStorage()
  }

  const clearHistory = () => {
    decompressHistory.value = []
    localStorage.removeItem('decompression-history')
  }

  // 方法 - 预设应用
  const applyPreset = (presetType: 'quick' | 'safe' | 'batch' | 'deep') => {
    switch (presetType) {
      case 'quick': // 快速解压
        decompressSettings.value.options.overwriteStrategy = 'overwrite'
        decompressSettings.value.options.skipCorrupted = true
        decompressSettings.value.options.extractOnlyNewer = false
        break
      case 'safe': // 安全解压
        decompressSettings.value.options.overwriteStrategy = 'ask'
        decompressSettings.value.options.skipCorrupted = false
        decompressSettings.value.options.preserveTimestamps = true
        break
      case 'batch': // 批量解压
        decompressSettings.value.options.createSubdirectory = true
        decompressSettings.value.options.keepStructure = true
        break
      case 'deep': // 深度扫描
        decompressSettings.value.options.skipCorrupted = false
        break
    }
  }

  // 工具函数
  const generateId = (): string => {
    return Date.now().toString(36) + Math.random().toString(36).substr(2)
  }

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 B'

    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))

    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
  }

  // 存储相关方法
  const saveHistoryToStorage = () => {
    try {
      const historyData = decompressHistory.value.map(history => ({
        ...history,
        timestamp: history.timestamp.toISOString()
      }))
      localStorage.setItem('decompression-history', JSON.stringify(historyData))
    } catch (err) {
      console.error('Failed to save history to storage:', err)
    }
  }

  const loadHistoryFromStorage = () => {
    try {
      const savedHistory = localStorage.getItem('decompression-history')
      if (savedHistory) {
        const historyData = JSON.parse(savedHistory)
        decompressHistory.value = historyData.map((item: any) => ({
          ...item,
          timestamp: new Date(item.timestamp)
        }))
      }
    } catch (err) {
      console.error('Failed to load history from storage:', err)
    }
  }

  const saveSettingsToStorage = () => {
    try {
      localStorage.setItem('decompression-settings', JSON.stringify(decompressSettings.value))
    } catch (err) {
      console.error('Failed to save settings to storage:', err)
    }
  }

  const loadSettingsFromStorage = () => {
    try {
      const savedSettings = localStorage.getItem('decompression-settings')
      if (savedSettings) {
        const settingsData = JSON.parse(savedSettings)
        decompressSettings.value = settingsData
      }
    } catch (err) {
      console.error('Failed to load settings from storage:', err)
    }
  }

  // 初始化
  const initialize = () => {
    loadHistoryFromStorage()
    loadSettingsFromStorage()
  }

  // 执行初始化
  initialize()

  return {
    // 状态
    selectedFile,
    decompressSettings,
    isProcessing,
    currentProgress,
    decompressTasks,
    decompressHistory,
    errorMessage,
    successMessage,
    showPassword,

    // 计算属性
    canStart,
    activeTask,
    recentHistory,
    requiresPassword,
    isPasswordValid,

    // 文件操作方法
    setSelectedFile,
    clearSelectedFile,

    // 解压设置方法
    updateDecompressSettings,
    setOutputPath,
    resetSettings,

    // 解压操作方法
    startDecompression,
    cancelDecompression,

    // 密码管理方法
    togglePasswordVisibility,
    clearPassword,

    // 历史记录方法
    addToHistory,
    clearHistory,

    // 预设方法
    applyPreset,

    // 工具方法
    formatFileSize,

    // 存储方法
    saveHistoryToStorage,
    loadHistoryFromStorage,
    saveSettingsToStorage,
    loadSettingsFromStorage
  }
})
