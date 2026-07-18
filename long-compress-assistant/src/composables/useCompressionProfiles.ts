import { invoke } from '@tauri-apps/api/tauri'
import { extractErrorMessage } from '@/utils'
import type { CompressionProfile, CreateProfileRequest } from '@/types'

interface ApplyProfileParams {
  profile_id: string
  success: boolean
  files_count: number
  bytes_processed: number
}

interface SuggestProfileParams {
  file_path: string
  file_size: number
}

const normalizeProfile = (raw: any): CompressionProfile => {
  const autoApply = raw.autoApply ?? raw.auto_apply ?? {}
  return ({
  id: raw.id,
  name: raw.name,
  icon: raw.icon,
  description: raw.description,
  config: {
    format: raw.config.format,
    level: raw.config.level,
    password: raw.config.password ?? null,
    splitArchive: raw.config.splitArchive ?? raw.config.split_archive ?? false,
    splitSize: raw.config.splitSize ?? raw.config.split_size ?? null,
    keepStructure: raw.config.keepStructure ?? raw.config.keep_structure ?? true,
    deleteAfter: raw.config.deleteAfter ?? raw.config.delete_after ?? false,
    createSolidArchive: raw.config.createSolidArchive ?? raw.config.create_solid_archive ?? false,
    filenameTemplate: raw.config.filenameTemplate ?? raw.config.filename_template ?? null,
    extraParams: raw.config.extraParams ?? raw.config.extra_params ?? {},
  },
  autoApply: {
    enabled: autoApply.enabled ?? false,
    mode: autoApply.mode ?? 'none',
    filePatterns: autoApply.filePatterns ?? autoApply.file_patterns ?? [],
    sizeRange: autoApply.sizeRange ?? autoApply.size_range ?? null,
  },
  passwordStrategy: raw.passwordStrategy ?? raw.password_strategy ?? 'none',
  stats: {
    useCount: raw.stats?.useCount ?? raw.stats?.use_count ?? 0,
    successCount: raw.stats?.successCount ?? raw.stats?.success_count ?? 0,
    failureCount: raw.stats?.failureCount ?? raw.stats?.failure_count ?? 0,
    totalFilesProcessed: raw.stats?.totalFilesProcessed ?? raw.stats?.total_files_processed ?? 0,
    totalBytesProcessed: raw.stats?.totalBytesProcessed ?? raw.stats?.total_bytes_processed ?? 0,
  },
  createdAt: raw.createdAt ?? raw.created_at ?? 0,
  lastUsedAt: raw.lastUsedAt ?? raw.last_used_at ?? null,
  })
}

const toBackendProfile = (profile: CompressionProfile) => ({
  id: profile.id,
  name: profile.name,
  icon: profile.icon,
  description: profile.description,
  config: {
    format: profile.config.format,
    level: profile.config.level,
    password: profile.config.password,
    split_archive: profile.config.splitArchive,
    split_size: profile.config.splitSize,
    keep_structure: profile.config.keepStructure,
    delete_after: profile.config.deleteAfter,
    create_solid_archive: profile.config.createSolidArchive,
    filename_template: profile.config.filenameTemplate,
    extra_params: profile.config.extraParams,
  },
  auto_apply: {
    enabled: profile.autoApply.enabled,
    mode: profile.autoApply.mode,
    file_patterns: profile.autoApply.filePatterns,
    size_range: profile.autoApply.sizeRange,
  },
  password_strategy: profile.passwordStrategy,
  stats: {
    use_count: profile.stats.useCount,
    success_count: profile.stats.successCount,
    failure_count: profile.stats.failureCount,
    total_files_processed: profile.stats.totalFilesProcessed,
    total_bytes_processed: profile.stats.totalBytesProcessed,
  },
  created_at: profile.createdAt,
  last_used_at: profile.lastUsedAt,
})

/**
 * 压缩配置组 Composable
 * 封装所有与配置组相关的 Tauri 命令调用
 */
export const useCompressionProfiles = () => {
  /**
   * 获取所有配置组
   */
  const getAllProfiles = async (): Promise<CompressionProfile[]> => {
    try {
      const profiles = await invoke<any[]>('get_compression_profiles')
      return profiles.map(normalizeProfile)
    } catch (error) {
      console.error('[useCompressionProfiles] Failed to get all profiles:', error)
      throw new Error(extractErrorMessage(error))
    }
  }

  /**
   * 根据 ID 获取配置组
   */
  const getProfileById = async (id: string): Promise<CompressionProfile | null> => {
    try {
      const profile = await invoke<any | null>('get_compression_profile', { id })
      return profile ? normalizeProfile(profile) : null
    } catch (error) {
      console.error(`[useCompressionProfiles] Failed to get profile ${id}:`, error)
      throw new Error(extractErrorMessage(error))
    }
  }

  /**
   * 创建新配置组
   */
  const createProfile = async (input: CreateProfileRequest): Promise<string> => {
    try {
      return await invoke<string>('create_compression_profile', { profile: input })
    } catch (error) {
      console.error('[useCompressionProfiles] Failed to create profile:', error)
      throw new Error(extractErrorMessage(error))
    }
  }

  /**
   * 更新配置组
   */
  const updateProfile = async (profile: CompressionProfile): Promise<void> => {
    try {
      await invoke<void>('update_compression_profile', { id: profile.id, profile: toBackendProfile(profile) })
    } catch (error) {
      console.error(`[useCompressionProfiles] Failed to update profile ${profile.id}:`, error)
      throw new Error(extractErrorMessage(error))
    }
  }

  /**
   * 删除配置组
   */
  const deleteProfile = async (id: string): Promise<void> => {
    try {
      await invoke<void>('delete_compression_profile', { id })
    } catch (error) {
      console.error(`[useCompressionProfiles] Failed to delete profile ${id}:`, error)
      throw new Error(extractErrorMessage(error))
    }
  }

  /**
   * 记录配置组应用（更新统计数据）
   */
  const recordProfileUsage = async (params: ApplyProfileParams): Promise<void> => {
    try {
      await invoke<void>('apply_compression_profile', {
        profileId: params.profile_id,
        success: params.success,
        filesCount: params.files_count,
        bytesProcessed: params.bytes_processed,
      })
    } catch (error) {
      console.error(
        `[useCompressionProfiles] Failed to record usage for profile ${params.profile_id}:`,
        error
      )
      throw new Error(extractErrorMessage(error))
    }
  }

  /**
   * 根据文件路径推荐配置组
   */
  const suggestProfile = async (
    params: SuggestProfileParams
  ): Promise<CompressionProfile | null> => {
    try {
      return await invoke<CompressionProfile | null>('suggest_compression_profile', {
        filePath: params.file_path,
        fileSize: params.file_size,
      })
    } catch (error) {
      console.error(
        `[useCompressionProfiles] Failed to suggest profile for ${params.file_path}:`,
        error
      )
      throw new Error(extractErrorMessage(error))
    }
  }

  /**
   * 初始化默认配置组（应用启动时自动执行，无需手动调用）
   */
  const initializeDefaultProfiles = async (): Promise<void> => {
    // 后端在 main.rs 中自动初始化，前端无需调用
    // 保留此方法以兼容现有代码
    console.warn('[useCompressionProfiles] initializeDefaultProfiles is deprecated - profiles are auto-initialized on app startup')
  }

  return {
    getAllProfiles,
    getProfileById,
    createProfile,
    updateProfile,
    deleteProfile,
    recordProfileUsage,
    suggestProfile,
    initializeDefaultProfiles,
  }
}
