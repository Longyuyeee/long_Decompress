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
      return await invoke<CompressionProfile[]>('get_compression_profiles')
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
      return await invoke<CompressionProfile | null>('get_compression_profile', { id })
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
      await invoke<void>('update_compression_profile', { id: profile.id, profile })
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
