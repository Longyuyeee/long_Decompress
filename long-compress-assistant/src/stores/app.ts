import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export interface FileItem {
  id: string
  name: string
  path: string
  size: number
  type: string
  status: 'pending' | 'processing' | 'completed' | 'error'
  progress: number
  error?: string
}

export interface DecompressTask {
  id: string
  fileId: string
  outputPath: string
  password?: string
  options: {
    keepStructure: boolean
    overwrite: boolean
    deleteAfter: boolean
  }
  status: 'pending' | 'processing' | 'completed' | 'error'
  progress: number
  startTime?: Date
  endTime?: Date
  error?: string
}

export interface AppSettings {
  theme: 'light' | 'dark' | 'auto'
  language: string
  defaultOutputPath: string
  maxConcurrentTasks: number
  scanForViruses: boolean
  checkFileExtensions: boolean
  warnLargeFiles: boolean
  savePasswords: boolean
  encryptPasswords: boolean
  autoClearPasswords: boolean
  collectUsageData: boolean
  sendCrashReports: boolean
  cacheSize: number
  logLevel: 'error' | 'warn' | 'info' | 'debug' | 'trace'
}

export const useAppStore = defineStore('app', () => {
  // 状�?
  const theme = ref<'light' | 'dark' | 'auto'>('auto')
  const language = ref('zh-CN')
  const error = ref<string | null>(null)

  // 解压任务管理
  const decompressTasks = ref<DecompressTask[]>([])

  // 设置
  const settings = ref<AppSettings>({
    theme: 'auto',
    language: 'zh-CN',
    defaultOutputPath: '',
    maxConcurrentTasks: 4,
    scanForViruses: true,
    checkFileExtensions: true,
    warnLargeFiles: true,
    savePasswords: false,
    encryptPasswords: true,
    autoClearPasswords: true,
    collectUsageData: false,
    sendCrashReports: true,
    cacheSize: 200,
    logLevel: 'info'
  })

  // 计算属�?
  const currentTheme = computed(() => {
    if (theme.value === 'auto') {
      return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
    }
    return theme.value
  })

  const activeTasks = computed(() => {
    return decompressTasks.value.filter(task =>
      task.status === 'processing' || task.status === 'pending'
    )
  })

  const completedTasks = computed(() => {
    return decompressTasks.value.filter(task => task.status === 'completed')
  })

  const totalProgress = computed(() => {
    if (activeTasks.value.length === 0) return 0
    const sum = activeTasks.value.reduce((acc, task) => acc + task.progress, 0)
    return Math.round(sum / activeTasks.value.length)
  })

  const createDecompressTask = (
    fileId: string,
    outputPath: string,
    password?: string,
    options?: Partial<DecompressTask['options']>
  ) => {
    const task: DecompressTask = {
      id: generateId(),
      fileId,
      outputPath,
      password,
      options: {
        keepStructure: options?.keepStructure ?? true,
        overwrite: options?.overwrite ?? false,
        deleteAfter: options?.deleteAfter ?? false
      },
      status: 'pending',
      progress: 0,
      startTime: new Date()
    }

    decompressTasks.value.push(task)
    return task.id
  }

  const updateTaskProgress = (taskId: string, progress: number) => {
    const task = decompressTasks.value.find(t => t.id === taskId)
    if (task) {
      task.progress = Math.min(100, Math.max(0, progress))
      if (progress >= 100) {
        task.status = 'completed'
        task.endTime = new Date()
      }
    }
  }

  const markTaskAsError = (taskId: string, errorMessage: string) => {
    const task = decompressTasks.value.find(t => t.id === taskId)
    if (task) {
      task.status = 'error'
      task.error = errorMessage
      task.endTime = new Date()
    }
  }

  const clearCompletedTasks = () => {
    decompressTasks.value = decompressTasks.value.filter(task =>
      task.status !== 'completed' && task.status !== 'error'
    )
  }

  const updateSettings = (newSettings: Partial<AppSettings>) => {
    settings.value = { ...settings.value, ...newSettings }
    // 这里可以添加保存到本地存储的逻辑
    saveSettingsToStorage()
  }

  const resetSettings = () => {
    settings.value = {
      theme: 'auto',
      language: 'zh-CN',
      defaultOutputPath: '',
      maxConcurrentTasks: 4,
      scanForViruses: true,
      checkFileExtensions: true,
      warnLargeFiles: true,
      savePasswords: false,
      encryptPasswords: true,
      autoClearPasswords: true,
      collectUsageData: false,
      sendCrashReports: true,
      cacheSize: 200,
      logLevel: 'info'
    }
    saveSettingsToStorage()
  }

  // 辅助函数
  const generateId = () => {
    return Date.now().toString(36) + Math.random().toString(36).substr(2)
  }

  const saveSettingsToStorage = () => {
    try {
      localStorage.setItem('app-settings', JSON.stringify(settings.value))
      localStorage.setItem('app-theme', theme.value)
      localStorage.setItem('app-language', language.value)
    } catch (error) {
      console.error('Failed to save settings to storage:', error)
    }
  }

  const loadSettingsFromStorage = () => {
    try {
      const savedSettings = localStorage.getItem('app-settings')
      const savedTheme = localStorage.getItem('app-theme')
      const savedLanguage = localStorage.getItem('app-language')

      if (savedSettings) {
        settings.value = JSON.parse(savedSettings)
      }
      if (savedTheme) {
        theme.value = savedTheme as 'light' | 'dark' | 'auto'
      }
      if (savedLanguage) {
        language.value = savedLanguage
      }
    } catch (error) {
      console.error('Failed to load settings from storage:', error)
    }
  }

  // 初始�?
  loadSettingsFromStorage()

  return {
    // 状�?
    theme,
    language,
    error,
    decompressTasks,
    settings,

    // 计算属�?
    currentTheme,
    activeTasks,
    completedTasks,
    totalProgress,

    // 方法
    createDecompressTask,
    updateTaskProgress,
    markTaskAsError,
    clearCompletedTasks,
    updateSettings,
    resetSettings,
    loadSettingsFromStorage,
    saveSettingsToStorage
  }
})
