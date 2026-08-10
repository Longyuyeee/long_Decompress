/**
 * 压缩配置组类型定义
 */

export interface CompressionProfile {
  id: string
  name: string
  icon: string
  description: string
  config: CompressionConfig
  autoApply: AutoApplyRule
  passwordStrategy: PasswordStrategy
  stats: ProfileStats
  createdAt: number
  lastUsedAt: number | null
}

export interface CompressionConfig {
  format: string
  level: number
  password: string | null
  splitArchive: boolean
  splitSize: number | null
  keepStructure: boolean
  deleteAfter: boolean
  verifyAfter: boolean
  createSolidArchive: boolean
  filenameTemplate: string | null
  extraParams: Record<string, string>
}

export interface AutoApplyRule {
  enabled: boolean
  mode: AutoApplyMode
  filePatterns: string[]
  sizeRange: [number, number] | null
}

export enum AutoApplyMode {
  None = 'none',
  All = 'all',
  Pattern = 'pattern',
  SizeRange = 'size_range'
}

export type PasswordStrategy =
  | { type: 'none' }
  | { type: 'fixed' }
  | { type: 'from_vault'; categoryId: string | null }
  | { type: 'auto_generate'; length: number; saveToVault: boolean }

export interface ProfileStats {
  useCount: number
  successCount: number
  failureCount: number
  totalFilesProcessed: number
  totalBytesProcessed: number
}

/**
 * 创建配置组请求
 */
export interface CreateProfileRequest {
  name: string
  icon: string
  description: string
  config: CompressionConfig
}

/**
 * 更新配置组请求
 */
export interface UpdateProfileRequest {
  id: string
  profile: Partial<CompressionProfile>
}

/**
 * 配置组排序请求
 */
export interface ReorderProfilesRequest {
  ids: string[]
}

/**
 * 应用配置组到任务
 */
export interface ApplyProfileRequest {
  profileId: string
  filePaths: string[]
}

/**
 * 推荐配置组请求
 */
export interface SuggestProfileRequest {
  filePath: string
}

export type TaskTemplateSourceMode = 'manual_selection' | 'all' | 'pattern' | 'size_range'
export type TaskTemplateTargetMode = 'same_directory' | 'choose_at_runtime'

export type TaskTemplatePasswordStrategy =
  | { mode: 'none' }
  | { mode: 'prompt_at_runtime' }
  | { mode: 'from_vault'; categoryId: string | null }
  | { mode: 'auto_generate'; length: number; saveToVault: boolean }

export interface TaskTemplate {
  schema: 'long-decompress-task-template'
  version: 1
  name: string
  icon: string
  description: string
  sourceRules: {
    mode: TaskTemplateSourceMode
    includePatterns: string[]
    sizeRangeMib: [number, number] | null
  }
  targetRule: {
    mode: TaskTemplateTargetMode
    filenameTemplate: string | null
  }
  compression: {
    format: string
    level: number
    splitArchive: boolean
    splitSizeMib: number | null
    keepStructure: boolean
    verifyAfter: boolean
    createSolidArchive: boolean
  }
  passwordStrategy: TaskTemplatePasswordStrategy
  exportNotes: string[]
}

export interface TaskTemplatePreview {
  template: TaskTemplate
  warnings: string[]
  contentSha256: string
}
