import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'

export interface ArchiveEngineFormatCapability {
  name: string
  extensions: string[]
  canCreate: boolean
}

export interface ArchiveEngineCapabilities {
  available: boolean
  command?: string | null
  version?: string | null
  fullEngine: boolean
  formats: ArchiveEngineFormatCapability[]
  browseExtensions: string[]
  nestedExtensions: string[]
  boundedPreviewFormats: string[]
  imagePreviewExtensions: string[]
  textPreviewExtensions: string[]
  message: string
}

const capabilities = ref<ArchiveEngineCapabilities | null>(null)
const loading = ref(false)
let pendingLoad: Promise<ArchiveEngineCapabilities | null> | null = null

export const useArchiveEngine = () => {
  const refresh = async () => {
    if (pendingLoad) return pendingLoad
    loading.value = true
    pendingLoad = invoke<ArchiveEngineCapabilities>('get_archive_engine_capabilities')
      .then(result => {
        capabilities.value = result || {
          available: false,
          fullEngine: false,
          formats: [],
          browseExtensions: [],
          nestedExtensions: [],
          boundedPreviewFormats: [],
          imagePreviewExtensions: [],
          textPreviewExtensions: [],
          message: '压缩引擎没有返回能力信息',
        }
        return capabilities.value
      })
      .catch(() => {
        capabilities.value = {
          available: false,
          fullEngine: false,
          formats: [],
          browseExtensions: [],
          nestedExtensions: [],
          boundedPreviewFormats: [],
          imagePreviewExtensions: [],
          textPreviewExtensions: [],
          message: '当前运行环境无法读取归档引擎能力',
        }
        return capabilities.value
      })
      .finally(() => {
        loading.value = false
        pendingLoad = null
      })
    return pendingLoad
  }

  const canCreate = (engineFormat?: string) => {
    if (!engineFormat) return true
    if (!capabilities.value) return false
    return capabilities.value.formats.some(format =>
      format.canCreate && (
        format.name.toLowerCase() === engineFormat.toLowerCase() ||
        format.extensions.some(extension => extension.toLowerCase() === engineFormat.toLowerCase())
      )
    )
  }

  return {
    capabilities: computed(() => capabilities.value),
    loading: computed(() => loading.value),
    refresh,
    canCreate,
  }
}
