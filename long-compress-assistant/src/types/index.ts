/**
 * 应用类型定义
 */

/**
 * 文件项类?
 */
export interface FileItem {
  id: string
  name: string
  path: string
  size: number
  type: string
  status: 'pending' | 'processing' | 'completed' | 'error'
  progress: number
  error?: string
  createdAt: Date
  updatedAt: Date
}

/**
 * 解压任务类型
 */
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

/**
 * 应用设置类型
 */
export interface AppSettings {
  // 常规设置
  theme: 'light' | 'dark' | 'auto'
  language: string
  startMinimized: boolean
  autoCheckUpdates: boolean
  showWelcome: boolean

  // 解压设置
  defaultOutputPath: string
  defaultKeepStructure: boolean
  defaultOverwrite: boolean
  defaultDeleteAfter: boolean
  maxConcurrentTasks: number

  // 安全设置
  scanForViruses: boolean
  checkFileExtensions: boolean
  warnLargeFiles: boolean
  savePasswords: boolean
  encryptPasswords: boolean
  autoClearPasswords: boolean
  collectUsageData: boolean
  sendCrashReports: boolean

  // 高级设置
  cacheSize: number
  logLevel: 'error' | 'warn' | 'info' | 'debug' | 'trace'
}

/**
 * 用户配置类型
 */
export interface UserPreferences {
  recentFiles: string[]
  favoritePaths: string[]
  savedPasswords: Array<{
    id: string
    name: string
    password: string
    createdAt: Date
  }>
  uiState: {
    sidebarOpen: boolean
    sidebarWidth: number
    lastView: string
  }
}

/**
 * 系统信息类型
 */
export interface SystemInfo {
  platform: string
  arch: string
  version: string
  memory: {
    total: number
    used: number
    free: number
  }
  storage: {
    total: number
    used: number
    free: number
  }
  cpu: {
    cores: number
    usage: number
  }
}

/**
 * 文件格式检查结?
 */
export interface FileFormatCheck {
  supported: boolean
  format?: string
  encrypted: boolean
  error?: string
  fileCount?: number
  totalSize?: number
}

/**
 * 解压结果类型
 */
export interface DecompressResult {
  success: boolean
  outputPath: string
  extractedFiles: string[]
  totalFiles: number
  totalSize: number
  duration: number
  error?: string
}

/**
 * 进度回调类型
 */
export type ProgressCallback = (progress: number, message?: string) => void

/**
 * 事件类型
 */
export interface AppEvent {
  type: string
  data: any
  timestamp: Date
}

/**
 * 通知类型
 */
export interface Notification {
  id: string
  type: 'info' | 'success' | 'warning' | 'error' | 'title' | 'message'
  title: string
  message: string
  duration?: number
  createdAt: Date
}

/**
 * 路由元信息类?
 */
export interface RouteMeta {
  title: string
  requiresAuth?: boolean
  icon?: string
  breadcrumb?: string[]
}

/**
 * 组件属性类?
 */
export interface ComponentProps {
  className?: string
  style?: Record<string, string | number>
  disabled?: boolean
  loading?: boolean
  size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl'
  width?: string | number
  height?: string | number
  variant?: 'default' | 'primary' | 'secondary' | 'success' | 'warning' | 'danger' | 'info'
}

/**
 * API响应类型
 */
export interface ApiResponse<T = any> {
  success: boolean
  data?: T
  error?: string
  message?: string
  timestamp: Date
}

/**
 * 分页参数类型
 */
export interface PaginationParams {
  page: number
  pageSize: number
  total: number
}

/**
 * 分页响应类型
 */
export interface PaginatedResponse<T> {
  items: T[]
  pagination: PaginationParams
}

/**
 * 排序参数类型
 */
export interface SortParams {
  field: string
  direction: 'asc' | 'desc'
}

/**
 * 过滤参数类型
 */
export interface FilterParams {
  field: string
  operator: 'eq' | 'neq' | 'gt' | 'gte' | 'lt' | 'lte' | 'contains' | 'startsWith' | 'endsWith'
  value: any
}

/**
 * 查询参数类型
 */
export interface QueryParams {
  pagination?: PaginationParams
  sort?: SortParams[]
  filters?: FilterParams[]
  search?: string
}

/**
 * 主题配置类型
 */
export interface ThemeConfig {
  colors: {
    primary: string
    secondary: string
    success: string
    warning: string
    danger: string
    info: string
    background: string
    foreground: string
    card: string
    border: string
  }
  fonts: {
    sans: string[]
    mono: string[]
  }
  spacing: Record<string, string>
  borderRadius: Record<string, string>
  shadows: Record<string, string>
}

/**
 * 国际化消息类?
 */
export interface I18nMessages {
  [key: string]: string | I18nMessages
}

/**
 * 语言配置类型
 */
export interface LanguageConfig {
  code: string
  name: string
  nativeName: string
  direction: 'ltr' | 'rtl'
  messages: I18nMessages
}

/**
 * 插件配置类型
 */
export interface PluginConfig {
  id: string
  name: string
  version: string
  description: string
  author: string
  enabled: boolean
  settings: Record<string, any>
}

/**
 * 快捷键配置类?
 */
export interface ShortcutConfig {
  id: string
  key: string
  description: string
  action: () => void
  enabled: boolean
}
