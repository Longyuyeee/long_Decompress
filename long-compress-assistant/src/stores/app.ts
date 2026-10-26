import { uniqueContextPaths, type ExplorerChecksumRequest } from '@/utils/contextActions'
import { defineStore } from 'pinia'
import { ref, computed, watch, onScopeDispose } from 'vue'
import { APP_THEMES, parseAppTheme, accentForeground, type AppTheme } from '@/utils/themes'
import { translations } from '../i18n'
import { invoke } from '@tauri-apps/api/tauri'
import type { ZipPasswordEncoding } from '@/types/zipPassword'
import { isArchiveOpenBehavior, type ArchiveOpenBehavior } from '@/utils/archiveOpening'

export interface PendingContextAction { action: string; files: string[]; outputPath?: string; extractToSubfolder?: boolean; openResultFolder?: boolean }

export interface ArchiveReadOptions { librarySourceId?: string; focusEntryPath?: string; entryAction?: 'locate' | 'preview' | 'extract' | 'read'; resumeReading?: boolean; zpaqSnapshot?: import('@/types/zpaqSnapshot').ZpaqSnapshot; password?: string; zipPasswordEncoding?: ZipPasswordEncoding }
export interface ArchiveDiagnosisRequest extends ArchiveReadOptions {
  path: string; paths?: string[]; taskId?: string; outputPath?: string; extractToSubfolder?: boolean
}

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
  theme: AppTheme
  language: string
  accentColor: string
  defaultOutputPath: string
  maxConcurrentTasks: number
  archiveTaskConcurrencyVersion: number
  defaultCompressionFormat: 'zip' | '7z' | 'tar.gz'
  defaultCompressionLevel: number
  defaultExtractToSubfolder: boolean
  smartExtractionDirectory: boolean
  parallelZipExtraction: boolean
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
  archiveOpenBehavior: ArchiveOpenBehavior
  archiveAfterExtract: 'show-results' | 'open-folder'
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
  const theme = ref<AppTheme>('auto')
  const language = ref('zh-CN')
  const accentColor = ref('#0ea5e9')
  const error = ref<string | null>(null)
  const successMessage = ref<string | null>(null)
  const recentFiles = ref<string[]>([])
  const errorMessage = ref<string | null>(null)
  const decompressTasks = ref<DecompressTask[]>([])
  const pendingContextActions = ref<PendingContextAction[]>([])
  const pendingArchiveOpenFiles = ref<string[]>([])
  const settingsLoaded = ref(false)
  const pendingArchiveBrowserPath = ref('')
  const pendingArchiveBrowserOptions = ref<ArchiveReadOptions>({})
  const pendingArchiveDiagnosis = ref<ArchiveDiagnosisRequest | null>(null)
  const pendingExplorerChecksums = ref<ExplorerChecksumRequest[]>([])
  const pendingChecksumSources = ref<{ paths: string[]; label: string } | null>(null)
  let errorTimer: ReturnType<typeof setTimeout> | null = null
  let successTimer: ReturnType<typeof setTimeout> | null = null

  const t = (key: string, fallback?: string): string => {
    return translations[language.value]?.[key] || translations['zh-CN']?.[key] || fallback || key
  }

  const systemScheme = window.matchMedia('(prefers-color-scheme: dark)')
  const systemDark = ref(systemScheme.matches)
  const followSystemScheme = (event: MediaQueryListEvent) => { systemDark.value = event.matches }
  systemScheme.addEventListener?.('change', followSystemScheme)
  onScopeDispose(() => systemScheme.removeEventListener?.('change', followSystemScheme))
  const currentTheme = computed(() => theme.value === 'auto' ? (systemDark.value ? 'dark' : 'light') : theme.value)

  watch([accentColor, currentTheme], () => {
    const root = document.documentElement
    root.style.setProperty('--dynamic-accent', accentColor.value)
    root.style.setProperty('--accent-ink', accentForeground(accentColor.value))
    root.classList.remove(...APP_THEMES.map(item => ['light', 'dark'].includes(item.value) ? item.value : `mode-${item.value}`))
    const mode = currentTheme.value
    root.classList.add(mode === 'light' || mode === 'dark' ? mode : `mode-${mode}`)
    root.dataset.colorScheme = ['light', 'sepia', 'glacier'].includes(mode) ? 'light' : 'dark'
    root.style.colorScheme = root.dataset.colorScheme
  }, { immediate: true })

  const settings = ref<AppSettings>({
    smartExtractionDirectory: false,
    parallelZipExtraction: false,
    theme: 'auto', language: 'zh-CN', accentColor: '#0ea5e9', defaultOutputPath: '',
    maxConcurrentTasks: 1, archiveTaskConcurrencyVersion: 1, defaultCompressionFormat: 'zip', defaultCompressionLevel: 6, defaultExtractToSubfolder: false, scanForViruses: true, checkFileExtensions: true, warnLargeFiles: true,
    savePasswords: false, encryptPasswords: true, autoClearPasswords: true, collectUsageData: false,
    sendCrashReports: true, cacheSize: 200, logLevel: 'info', enableBruteForce: false,
    bruteForceCharset: '0123456789abcdefghijklmnopqrstuvwxyz', bruteForceMaxLen: 6,
    bruteForceWordlists: [], autoStart: false, contextMenuEnabled: true, closeToTray: true, autoCheckUpdates: true, conflictPolicy: 'ask', autoDeleteSource: false, preserveMarkOfWeb: true,
    uiScale: 100, archiveOpenBehavior: 'ask', archiveAfterExtract: 'show-results',
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
    // Resetting application preferences must not silently change a Windows
    // persistence choice. Only the dedicated auto-start switch may do that.
    const autoStart = settings.value.autoStart
    theme.value = 'auto'
    accentColor.value = '#0ea5e9'
    language.value = 'zh-CN'
    settings.value = {
      smartExtractionDirectory: false,
      parallelZipExtraction: false,
      theme: 'auto', language: 'zh-CN', accentColor: '#0ea5e9', defaultOutputPath: '',
      maxConcurrentTasks: 1, archiveTaskConcurrencyVersion: 1, defaultCompressionFormat: 'zip', defaultCompressionLevel: 6, defaultExtractToSubfolder: false, scanForViruses: true, checkFileExtensions: true, warnLargeFiles: true,
      savePasswords: false, encryptPasswords: true, autoClearPasswords: true, collectUsageData: false,
      sendCrashReports: true, cacheSize: 200, logLevel: 'info', enableBruteForce: false,
      bruteForceCharset: '0123456789abcdefghijklmnopqrstuvwxyz', bruteForceMaxLen: 6,
      bruteForceWordlists: [], autoStart, contextMenuEnabled: true, closeToTray: true, autoCheckUpdates: true, conflictPolicy: 'ask', autoDeleteSource: false, preserveMarkOfWeb: true,
      uiScale: 100, archiveOpenBehavior: 'ask', archiveAfterExtract: 'show-results',
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
      settings.value.theme = theme.value
      settings.value.language = language.value
      settings.value.accentColor = accentColor.value
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
    merged.archiveOpenBehavior = isArchiveOpenBehavior(parsed.archiveOpenBehavior) ? parsed.archiveOpenBehavior : 'ask'
    merged.archiveAfterExtract = parsed.archiveAfterExtract === 'open-folder' ? 'open-folder' : 'show-results'
    merged.smartExtractionDirectory = parsed.smartExtractionDirectory === true
    merged.parallelZipExtraction = parsed.parallelZipExtraction === true
    // autoStart is display state only. Loading it must never register or repair
    // a Windows startup entry; the settings page refreshes the actual read-only
    // state and only an explicit toggle may invoke set_auto_start.
    // Before concurrency v1 this setting was visible but the queues were always
    // serial. Preserve that effective behavior on upgrade; users can explicitly
    // raise the value after migration.
    if (parsed.archiveTaskConcurrencyVersion !== 1) {
      merged.maxConcurrentTasks = 1
      merged.archiveTaskConcurrencyVersion = 1
    }
    merged.maxConcurrentTasks = Math.max(1, Math.min(16, merged.maxConcurrentTasks || 1))
    if (!['zip', '7z', 'tar.gz'].includes(merged.defaultCompressionFormat)) merged.defaultCompressionFormat = 'zip'
    merged.defaultCompressionLevel = Math.max(1, Math.min(9, merged.defaultCompressionLevel || 6))
    return merged
  }

  const loadSettingsFromStorage = () => {
    try {
      let needsMigration = false
      const savedSettings = localStorage.getItem('app-settings')
      if (savedSettings) {
        const parsed = JSON.parse(savedSettings)
        settings.value = mergeStoredSettings(parsed)
        if (parsed.archiveTaskConcurrencyVersion !== 1) {
          needsMigration = true
        }
      }
      theme.value = parseAppTheme(localStorage.getItem('app-theme') || settings.value.theme)
      language.value = localStorage.getItem('app-language') || settings.value.language
      accentColor.value = localStorage.getItem('app-accent') || settings.value.accentColor
      if (needsMigration) saveSettingsToStorage()
    } catch (e) { console.error(e) }
    // 如果 localStorage 为空，尝试从后端恢复
    if (!localStorage.getItem('app-settings')) {
      invoke<string>('load_app_settings').then(json => {
        if (json && json !== '{}') {
          const parsed = JSON.parse(json)
          settings.value = mergeStoredSettings(parsed)
          theme.value = parseAppTheme(parsed.theme)
          language.value = parsed.language || 'zh-CN'
          accentColor.value = parsed.accentColor || '#0ea5e9'
          saveSettingsToStorage()
          void synchronizeContextMenu(settings.value.contextMenuEnabled).catch(e => {
            console.warn('Failed to synchronize Explorer context menu:', e)
          })
        }
      }).catch(() => {}).finally(() => { settingsLoaded.value = true })
    } else {
      settingsLoaded.value = true
    }
  }

  const addRecentFile = (path: string) => {
    recentFiles.value = [path, ...recentFiles.value.filter(f => f !== path)].slice(0, 10)
    try { localStorage.setItem('recent-files', JSON.stringify(recentFiles.value)) } catch { }
  }

  // 压缩预设
  const compressionPresets = ref<Array<{ name: string; format: string; level: number; password?: string; sevenZipFilter?: 'none' | 'delta' | 'bcj_x86' | 'bcj_arm32' | 'bcj_arm_thumb' | 'bcj_arm64' | 'bcj_riscv'; sevenZipDeltaDistance?: number; sevenZipMethod?: 'lzma2' | 'lzma' | 'bzip2' | 'deflate' | 'ppmd' | 'copy'; zipMethod?: 'deflate' | 'zstd' | 'bzip2' | 'ppmd' | 'lzma' | 'store'; wimMethod?: 'store' | 'xpress' | 'lzx' | 'lzms' | 'solid' }>>([])
  const saveCompressionPreset = (name: string, format: string, level: number, password?: string, zipMethod?: 'deflate' | 'zstd' | 'bzip2' | 'ppmd' | 'lzma' | 'store', wimMethod?: 'store' | 'xpress' | 'lzx' | 'lzms' | 'solid', sevenZipMethod?: 'lzma2' | 'lzma' | 'bzip2' | 'deflate' | 'ppmd' | 'copy', sevenZipFilter?: 'none' | 'delta' | 'bcj_x86' | 'bcj_arm32' | 'bcj_arm_thumb' | 'bcj_arm64' | 'bcj_riscv', sevenZipDeltaDistance?: number) => {
    compressionPresets.value = [{ name, format, level, password, zipMethod, wimMethod, sevenZipMethod, sevenZipFilter, sevenZipDeltaDistance }, ...compressionPresets.value].slice(0, 8)
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
    theme, language, accentColor, error, successMessage, errorMessage, decompressTasks, settings, settingsLoaded, pendingArchiveOpenFiles,
    enqueueArchiveOpen: (files: string[]) => { pendingArchiveOpenFiles.value = uniqueContextPaths([...pendingArchiveOpenFiles.value, ...files]) },
    pendingContextActions, pendingArchiveBrowserPath, pendingArchiveDiagnosis, pendingChecksumSources, pendingExplorerChecksums,
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
    enqueueContextAction: (action: PendingContextAction) => pendingContextActions.value.push(action),
    takeContextActions: () => pendingContextActions.value.splice(0),
    openArchiveInBrowser: (path: string, options: ArchiveReadOptions = {}) => { pendingArchiveBrowserOptions.value = { ...options }; pendingArchiveBrowserPath.value = path },
    takeArchiveBrowserOptions: () => { const options = pendingArchiveBrowserOptions.value; pendingArchiveBrowserOptions.value = {}; return options },
    enqueueExplorerChecksums: (request: ExplorerChecksumRequest) => {
      const previous = pendingExplorerChecksums.value.at(-1)
      if (previous?.kind === request.kind) previous.paths = uniqueContextPaths([...previous.paths, ...request.paths])
      else pendingExplorerChecksums.value.push({ ...request, paths: uniqueContextPaths(request.paths) })
    },
    openArchiveForDiagnosis: (request: ArchiveDiagnosisRequest) => { pendingArchiveDiagnosis.value = request },
    takeArchiveBrowserPath: () => {
      const path = pendingArchiveBrowserPath.value
      pendingArchiveBrowserPath.value = ''
      pendingArchiveBrowserOptions.value = {}
      return path
    },
    createDecompressTask, updateTaskProgress, markTaskAsError, clearCompletedTasks, updateSettings, resetSettings, saveSettingsToStorage,
    synchronizeContextMenu, setContextMenuEnabled
  }
})
