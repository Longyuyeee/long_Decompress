import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useCompressionProfiles } from '@/composables/useCompressionProfiles'
import type { CompressionProfile, CreateProfileRequest } from '@/types'

/**
 * 压缩配置组状态管理
 */
export const useCompressionProfileStore = defineStore('compressionProfile', () => {
  const { getAllProfiles, getProfileById, createProfile, updateProfile, deleteProfile, recordProfileUsage, suggestProfile, initializeDefaultProfiles } = useCompressionProfiles()

  // 状态
  const profiles = ref<CompressionProfile[]>([])
  const currentProfile = ref<CompressionProfile | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  // 计算属性
  const profilesCount = computed(() => profiles.value.length)

  const sortedProfiles = computed(() => {
    return [...profiles.value].sort((a, b) => {
      // 按最后使用时间降序排序，未使用的排在后面
      if (a.lastUsedAt && b.lastUsedAt) {
        return b.lastUsedAt - a.lastUsedAt
      }
      if (a.lastUsedAt) return -1
      if (b.lastUsedAt) return 1
      // 都没有使用过，按创建时间降序
      return b.createdAt - a.createdAt
    })
  })

  const mostUsedProfiles = computed(() => {
    return [...profiles.value]
      .filter(p => p.stats.useCount > 0)
      .sort((a, b) => b.stats.useCount - a.stats.useCount)
      .slice(0, 5)
  })

  // 操作方法
  const loadAllProfiles = async () => {
    loading.value = true
    error.value = null
    try {
      profiles.value = await getAllProfiles()
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      loading.value = false
    }
  }

  const loadProfileById = async (id: string) => {
    loading.value = true
    error.value = null
    try {
      const profile = await getProfileById(id)
      if (profile) {
        currentProfile.value = profile
        return profile
      }
      return null
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      loading.value = false
    }
  }

  const addProfile = async (input: CreateProfileRequest) => {
    loading.value = true
    error.value = null
    try {
      const newProfileId = await createProfile(input)
      // 创建成功后重新加载配置组列表
      await loadAllProfiles()
      return newProfileId
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      loading.value = false
    }
  }

  const modifyProfile = async (profile: CompressionProfile) => {
    loading.value = true
    error.value = null
    try {
      await updateProfile(profile)
      // 更新成功后重新加载配置组列表
      await loadAllProfiles()
      if (currentProfile.value?.id === profile.id) {
        currentProfile.value = profile
      }
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      loading.value = false
    }
  }

  const removeProfile = async (id: string) => {
    loading.value = true
    error.value = null
    try {
      await deleteProfile(id)
      profiles.value = profiles.value.filter(p => p.id !== id)
      if (currentProfile.value?.id === id) {
        currentProfile.value = null
      }
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      loading.value = false
    }
  }

  const applyProfile = async (profileId: string, success: boolean, filesCount: number, bytesProcessed: number) => {
    try {
      await recordProfileUsage({ profile_id: profileId, success, files_count: filesCount, bytes_processed: bytesProcessed })
      // 重新加载配置组列表以更新统计
      await loadAllProfiles()
    } catch (err) {
      console.error('[compressionProfileStore] Failed to apply profile:', err)
      throw err
    }
  }

  const getSuggestedProfile = async (filePath: string, fileSize: number) => {
    try {
      return await suggestProfile({ file_path: filePath, file_size: fileSize })
    } catch (err) {
      console.error('[compressionProfileStore] Failed to suggest profile:', err)
      return null
    }
  }

  const initDefaults = async () => {
    loading.value = true
    error.value = null
    try {
      await initializeDefaultProfiles()
      await loadAllProfiles()
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      loading.value = false
    }
  }

  const setCurrentProfile = (profile: CompressionProfile | null) => {
    currentProfile.value = profile
  }

  const clearError = () => {
    error.value = null
  }

  return {
    // 状态
    profiles,
    currentProfile,
    loading,
    error,
    // 计算属性
    profilesCount,
    sortedProfiles,
    mostUsedProfiles,
    // 方法
    loadAllProfiles,
    loadProfileById,
    addProfile,
    modifyProfile,
    removeProfile,
    applyProfile,
    getSuggestedProfile,
    initDefaults,
    setCurrentProfile,
    clearError,
  }
})
