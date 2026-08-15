import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { translations } from '../i18n'
import { invoke } from '@tauri-apps/api/tauri'

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
  archiveTaskConcurrencyVersion: number
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
  bruteForceWordlists: string[]
  autoStart: boolean
  contextMenuEnabled: boolean
  closeToTray: boolean
  autoCheckUpdates: boolean
  conflictPolicy: 'ask' | 'overwrite' | 'skip' | 'rename'
  autoDeleteSource: boolean
  preserveMarkOfWeb: boolean
  uiScale: number
  accessibility?: {
    fontSize: 'normal' | 'large' | 'x-large'
    highContrast: boolean
    colorBlindMode: 'none' | 'protanopia' | 'deuteranopia' | 'tritanopia'
    reduceMotion: boolean
    focusIndicator: boolean
  }
}

export const useAppStore = defineStore('app', () => {
  const theme = ref<'light' | 'dark' | 'auto' | 'cyberpunk' | 'twilight' | 'sepia'>('auto')
  const language = ref('zh-CN')
  const accentColor = ref('#0ea5e9')
  const error = ref<string | null>(null)
  const successMessage = ref<string | null>(null)
  const recentFiles = ref<string[]>([])
  const errorMessage = ref<string | null>(null)
  const decompressTasks = ref<DecompressTask[]>([])
  const pendingContextActions = ref<Array<{ action: string; files: string[] }>>([])
  const pendingArchiveBrowserPath = ref('')
  let errorTimer: ReturnType<typeof setTimeout> | null = null
  let successTimer: ReturnType<typeof setTimeout> | null = null

  const t = (key: string, fallback?: string): string => {
    return translations[language.value]?.[key] || translations['zh-CN']?.[key] || fallback || key
  }

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
    maxConcurrentTasks: 1, archiveTaskConcurrencyVersion: 1, scanForViruses: true, checkFileExtensions: true, warnLargeFiles: true,
    savePasswords: false, encryptPasswords: true, autoClearPasswords: true, collectUsageData: false,
    sendCrashReports: true, cacheSize: 200, logLevel: 'info', enableBruteForce: false,
    bruteForceCharset: '0123456789abcdefghijklmnopqrstuvwxyz', bruteForceMaxLen: 6,
    bruteForceWordlists: [], autoStart: false, contextMenuEnabled: true, closeToTray: true, autoCheckUpdates: true, conflictPolicy: 'ask', autoDeleteSource: false, preserveMarkOfWeb: true,
    uiScale: 100,
    accessibility: {
      fontSize: 'normal',
      highContrast: false,
      colorBlindMode: 'none',
      reduceMotion: false,
      focusIndicator: true,
    }
  })

  // UI 缩放 - 通过调整根字体大小实现真正的字体/界面缩放（非僵硬 CSS zoom）
  // rem 单位基于 html font-size，修改它会让所有 rem 值（Tailwind 间距、标准字号等）等比缩放
  watch(() => settings.value.uiScale, (scale) => {
    const root = document.documentElement
    const factor = scale / 100
    const fontSize = settings.value.accessibility?.fontSize || 'normal'
    const accessibilityFactor = fontSize === 'x-large' ? 1.25 : fontSize === 'large' ? 1.125 : 1
    root.style.setProperty('--ui-scale-factor', String(factor))
    root.style.fontSize = `${scale * accessibilityFactor}%`
  }, { immediate: true })

  watch(() => settings.value.autoStart, async (newVal) => {
    try {
      await invoke('set_auto_start', { enable: newVal })
    } catch (e) {
      console.error('Failed to set auto start:', e)
    }
  })

  watch(() => settings.value.closeToTray, async (enabled) => {
    try {
      await invoke('set_close_to_tray', { enabled })
    } catch (e) {
      console.warn('Failed to update close-to-tray behavior:', e)
    }
  }, { immediate: true })

  // Serialize Explorer menu updates so startup repair and user toggles cannot
  // overwrite each other. The saved preference is the desired state; registry
  // inspection is only used to repair drift and never changes that preference.
  let contextMenuSyncQueue: Promise<void> = Promise.resolve()
  const synchronizeContextMenu = (enabled = settings.value.contextMenuEnabled) => {
    const operation = contextMenuSyncQueue.then(async () => {
      const registered = await invoke<boolean>('is_context_menu_registered')
      if (registered !== enabled) {
        await invoke(enabled ? 'register_context_menu' : 'unregister_context_menu')
      }
    })
    contextMenuSyncQueue = operation.catch(() => {})
    return operation
  }

  const setContextMenuEnabled = async (enabled: boolean) => {
    await synchronizeContextMenu(enabled)
    updateSettings({ contextMenuEnabled: enabled })
  }

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
    const merged = { ...settings.value, ...newSettings }
    // 输入校验：防止非法值
    merged.maxConcurrentTasks = Math.max(1, Math.min(16, merged.maxConcurrentTasks || 1))
    merged.cacheSize = Math.max(50, Math.min(1000, merged.cacheSize || 200))
    merged.bruteForceMaxLen = Math.max(1, Math.min(20, merged.bruteForceMaxLen || 6))
    merged.uiScale = Math.max(60, Math.min(200, merged.uiScale || 100))
    settings.value = merged
    saveSettingsToStorage()
  }

  const resetSettings = () => {
    settings.value = {
      theme: 'auto', language: 'zh-CN', accentColor: '#0ea5e9', defaultOutputPath: '',
      maxConcurrentTasks: 1, archiveTaskConcurrencyVersion: 1, scanForViruses: true, checkFileExtensions: true, warnLargeFiles: true,
      savePasswords: false, encryptPasswords: true, autoClearPasswords: true, collectUsageData: false,
      sendCrashReports: true, cacheSize: 200, logLevel: 'info', enableBruteForce: false,
      bruteForceCharset: '0123456789abcdefghijklmnopqrstuvwxyz', bruteForceMaxLen: 6,
      bruteForceWordlists: [], autoStart: false, contextMenuEnabled: true, closeToTray: true, autoCheckUpdates: true, conflictPolicy: 'ask', autoDeleteSource: false, preserveMarkOfWeb: true,
      uiScale: 100,
      accessibility: {
        fontSize: 'normal',
        highContrast: false,
        colorBlindMode: 'none',
        reduceMotion: false,
        focusIndicator: true,
      }
    }
    saveSettingsToStorage()
    void synchronizeContextMenu(settings.value.contextMenuEnabled).catch(e => {
      console.warn('Failed to synchronize Explorer context menu after reset:', e)
    })
  }

  const saveSettingsToStorage = () => {
    try {
      const json = JSON.stringify(settings.value)
      localStorage.setItem('app-settings', json)
      localStorage.setItem('app-theme', theme.value)
      localStorage.setItem('app-language', language.value)
      localStorage.setItem('app-accent', accentColor.value)
      // 同时持久化到后端数据目录
      invoke('save_app_settings', { settingsJson: json }).catch(() => {})
    } catch (e) { console.error(e) }
  }

  const mergeStoredSettings = (parsed: Partial<AppSettings>) => {
    const merged = { ...settings.value, ...parsed }
    // Before concurrency v1 this setting was visible but the queues were always
    // serial. Preserve that effective behavior on upgrade; users can explicitly
    // raise the value after migration.
    if (parsed.archiveTaskConcurrencyVersion !== 1) {
      merged.maxConcurrentTasks = 1
      merged.archiveTaskConcurrencyVersion = 1
    }
    merged.maxConcurrentTasks = Math.max(1, Math.min(16, merged.maxConcurrentTasks || 1))
    return merged
  }

  const loadSettingsFromStorage = () => {
    try {
      const savedSettings = localStorage.getItem('app-settings')
      if (savedSettings) {
        const parsed = JSON.parse(savedSettings)
        settings.value = mergeStoredSettings(parsed)
        if (parsed.archiveTaskConcurrencyVersion !== 1) saveSettingsToStorage()
      }
      theme.value = (localStorage.getItem('app-theme') as any) || 'auto'
      language.value = localStorage.getItem('app-language') || 'zh-CN'
      accentColor.value = localStorage.getItem('app-accent') || '#0ea5e9'
    } catch (e) { console.error(e) }
    // 如果 localStorage 为空，尝试从后端恢复
    if (!localStorage.getItem('app-settings')) {
      invoke<string>('load_app_settings').then(json => {
        if (json && json !== '{}') {
          const parsed = JSON.parse(json)
          settings.value = mergeStoredSettings(parsed)
          saveSettingsToStorage()
          void synchronizeContextMenu(settings.value.contextMenuEnabled).catch(e => {
            console.warn('Failed to synchronize Explorer context menu:', e)
          })
        }
      }).catch(() => {})
    }
  }

  const addRecentFile = (path: string) => {
    recentFiles.value = [path, ...recentFiles.value.filter(f => f !== path)].slice(0, 10)
    try { localStorage.setItem('recent-files', JSON.stringify(recentFiles.value)) } catch { }
  }

  // 压缩预设
  const compressionPresets = ref<Array<{ name: string; format: string; level: number; password?: string }>>([])
  const saveCompressionPreset = (name: string, format: string, level: number, password?: string) => {
    compressionPresets.value = [{ name, format, level, password }, ...compressionPresets.value].slice(0, 8)
    try { localStorage.setItem('compression-presets', JSON.stringify(compressionPresets.value)) } catch { }
  }
  const deleteCompressionPreset = (index: number) => {
    compressionPresets.value.splice(index, 1)
    try { localStorage.setItem('compression-presets', JSON.stringify(compressionPresets.value)) } catch { }
  }

  try {
    const saved = localStorage.getItem('compression-presets')
    if (saved) compressionPresets.value = JSON.parse(saved)
  } catch { }

  try {
    const saved = localStorage.getItem('recent-files')
    if (saved) recentFiles.value = JSON.parse(saved)
  } catch { }

  loadSettingsFromStorage()
  void synchronizeContextMenu(settings.value.contextMenuEnabled).catch(e => {
    console.warn('Failed to synchronize Explorer context menu:', e)
  })

  return {
    theme, language, accentColor, error, successMessage, errorMessage, decompressTasks, settings,
    pendingContextActions, pendingArchiveBrowserPath,
    recentFiles, addRecentFile,
    compressionPresets, saveCompressionPreset, deleteCompressionPreset,
    currentTheme, activeTasks, completedTasks, totalProgress, t,
    setError: (msg: string | null) => {
      error.value = msg
      if (errorTimer) clearTimeout(errorTimer)
      if (msg) errorTimer = setTimeout(() => { error.value = null }, 5000)
    },
    setSuccess: (msg: string | null) => {
      successMessage.value = msg
      if (successTimer) clearTimeout(successTimer)
      if (msg) successTimer = setTimeout(() => { successMessage.value = null }, 3000)
    },
    clearError: () => { error.value = null; if (errorTimer) clearTimeout(errorTimer) },
    enqueueContextAction: (action: { action: string; files: string[] }) => pendingContextActions.value.push(action),
    takeContextActions: () => pendingContextActions.value.splice(0),
    openArchiveInBrowser: (path: string) => { pendingArchiveBrowserPath.value = path },
    takeArchiveBrowserPath: () => {
      const path = pendingArchiveBrowserPath.value
      pendingArchiveBrowserPath.value = ''
      return path
    },
    createDecompressTask, updateTaskProgress, markTaskAsError, clearCompletedTasks, updateSettings, resetSettings, saveSettingsToStorage,
    synchronizeContextMenu, setContextMenuEnabled
  }
})
