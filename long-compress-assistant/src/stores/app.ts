import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { translations } from '../i18n'

export interface DecompressTask {
  id: string
  fileId: string
  filePath: string
  fileName: string
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
  createdAt: Date
}

export interface AppSettings {
  theme: 'light' | 'dark' | 'auto' | 'cyberpunk' | 'twilight' | 'sepia'
  language: string
  accentColor: string
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
  enableBruteForce: boolean 
  bruteForceCharset: string 
  bruteForceMaxLen: number 
  autoDeleteSource: boolean 
  conflictPolicy: 'ask' | 'overwrite' | 'skip' | 'rename'
}

export const useAppStore = defineStore('app', () => {
  const theme = ref<'light' | 'dark' | 'auto' | 'cyberpunk' | 'twilight' | 'sepia'>('auto')
  const language = ref('zh-CN')
  const accentColor = ref('#0ea5e9')
  const error = ref<string | null>(null)
  const successMessage = ref<string | null>(null)
  const errorMessage = ref<string | null>(null)
  const decompressTasks = ref<DecompressTask[]>([])

  const t = (key: string): string => translations[language.value]?.[key] || translations['zh-CN']?.[key] || key

  const currentTheme = computed(() => {
    if (theme.value === 'auto') return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
    return theme.value
  })

  watch([accentColor, theme], () => {
    const root = document.documentElement
    root.style.setProperty('--dynamic-accent', accentColor.value)
    root.classList.remove('light', 'dark', 'mode-cyberpunk', 'mode-twilight', 'mode-sepia')
    
    let mode = theme.value === 'auto' ? currentTheme.value : theme.value
    if (mode === 'cyberpunk') root.classList.add('mode-cyberpunk')
    else if (mode === 'twilight') root.classList.add('mode-twilight')
    else if (mode === 'sepia') root.classList.add('mode-sepia')
    else root.classList.add(mode)
  }, { immediate: true })

  const settings = ref<AppSettings>({
    theme: 'auto', language: 'zh-CN', accentColor: '#0ea5e9', defaultOutputPath: '',
    maxConcurrentTasks: 4, scanForViruses: true, checkFileExtensions: true, warnLargeFiles: true,
    savePasswords: false, encryptPasswords: true, autoClearPasswords: true, collectUsageData: false,
    sendCrashReports: true, cacheSize: 200, logLevel: 'info', enableBruteForce: false,
    bruteForceCharset: '0123456789abcdefghijklmnopqrstuvwxyz', bruteForceMaxLen: 6,
    autoDeleteSource: false, conflictPolicy: 'ask'
  })

  const activeTasks = computed(() => decompressTasks.value.filter(t => t.status === 'processing' || t.status === 'pending'))
  const completedTasks = computed(() => decompressTasks.value.filter(t => t.status === 'completed'))
  const totalProgress = computed(() => {
    if (activeTasks.value.length === 0) return 0
    return Math.round(activeTasks.value.reduce((acc, t) => acc + t.progress, 0) / activeTasks.value.length)
  })

  const createDecompressTask = (fileId: string, filePath: string, outputPath: string, password?: string, options?: any) => {
    const task: DecompressTask = {
      id: Math.random().toString(36).substr(2, 9),
      fileId, filePath, fileName: filePath.split(/[\\/]/).pop() || '',
      outputPath, password,
      options: { keepStructure: true, overwrite: false, deleteAfter: false, ...options },
      status: 'pending', progress: 0, createdAt: new Date()
    }
    decompressTasks.value.push(task)
    return task.id
  }

  const updateTaskProgress = (taskId: string, progress: number) => {
    const task = decompressTasks.value.find(t => t.id === taskId)
    if (task) {
      task.progress = Math.min(100, Math.max(0, progress))
      if (progress >= 100) { task.status = 'completed'; task.endTime = new Date(); }
    }
  }

  const markTaskAsError = (taskId: string, errMsg: string) => {
    const task = decompressTasks.value.find(t => t.id === taskId)
    if (task) { task.status = 'error'; task.error = errMsg; task.endTime = new Date(); }
  }

  const clearCompletedTasks = () => {
    decompressTasks.value = decompressTasks.value.filter(t => t.status !== 'completed' && t.status !== 'error')
  }

  const updateSettings = (newSettings: Partial<AppSettings>) => {
    settings.value = { ...settings.value, ...newSettings }
    saveSettingsToStorage()
  }

  const resetSettings = () => {
    settings.value = {
      theme: 'auto', language: 'zh-CN', accentColor: '#0ea5e9', defaultOutputPath: '',
      maxConcurrentTasks: 4, scanForViruses: true, checkFileExtensions: true, warnLargeFiles: true,
      savePasswords: false, encryptPasswords: true, autoClearPasswords: true, collectUsageData: false,
      sendCrashReports: true, cacheSize: 200, logLevel: 'info', enableBruteForce: false,
      bruteForceCharset: '0123456789abcdefghijklmnopqrstuvwxyz', bruteForceMaxLen: 6,
      autoDeleteSource: false, conflictPolicy: 'ask'
    }
    saveSettingsToStorage()
  }

  const saveSettingsToStorage = () => {
    try {
      localStorage.setItem('app-settings', JSON.stringify(settings.value))
      localStorage.setItem('app-theme', theme.value)
      localStorage.setItem('app-language', language.value)
      localStorage.setItem('app-accent', accentColor.value)
    } catch (e) { console.error(e) }
  }

  const loadSettingsFromStorage = () => {
    try {
      const savedSettings = localStorage.getItem('app-settings')
      if (savedSettings) settings.value = JSON.parse(savedSettings)
      theme.value = (localStorage.getItem('app-theme') as any) || 'auto'
      language.value = localStorage.getItem('app-language') || 'zh-CN'
      accentColor.value = localStorage.getItem('app-accent') || '#0ea5e9'
    } catch (e) { console.error(e) }
  }

  loadSettingsFromStorage()

  return {
    theme, language, accentColor, error, successMessage, errorMessage, decompressTasks, settings,
    currentTheme, activeTasks, completedTasks, totalProgress, t,
    setError: (m: string | null) => error.value = m,
    clearError: () => error.value = null,
    createDecompressTask, updateTaskProgress, markTaskAsError, clearCompletedTasks, updateSettings, resetSettings, saveSettingsToStorage
  }
})
