import { computed } from 'vue'
import { useArchiveEngine } from '@/composables/useArchiveEngine'

const imageCategoryExtensions = new Set(['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'svg', 'ico', 'avif'])
const documentCategoryExtensions = new Set([
  'txt', 'md', 'pdf', 'doc', 'docx', 'xls', 'xlsx', 'ppt', 'pptx', 'csv', 'json', 'xml',
])

const normalizedExtension = (name: string) => name.includes('.')
  ? name.split('.').pop()!.toLocaleLowerCase()
  : ''

export const useArchiveBrowserCapabilities = () => {
  const engine = useArchiveEngine()
  const browseExtensions = computed(() => new Set(engine.capabilities.value?.browseExtensions ?? []))
  const nestedExtensions = computed(() => new Set(engine.capabilities.value?.nestedExtensions ?? []))
  const imagePreviewExtensions = computed(() => new Set(engine.capabilities.value?.imagePreviewExtensions ?? []))
  const textPreviewExtensions = computed(() => new Set(engine.capabilities.value?.textPreviewExtensions ?? []))
  const boundedPreviewFormats = computed(() => engine.capabilities.value?.boundedPreviewFormats ?? [])

  const entryCategory = (name: string): 'image' | 'document' | 'archive' | 'other' => {
    const extension = normalizedExtension(name)
    if (imageCategoryExtensions.has(extension)) return 'image'
    if (documentCategoryExtensions.has(extension)) return 'document'
    if (browseExtensions.value.has(extension)) return 'archive'
    return 'other'
  }

  const previewKind = (name: string): 'image' | 'text' | null => {
    const extension = normalizedExtension(name)
    if (imagePreviewExtensions.value.has(extension)) return 'image'
    if (textPreviewExtensions.value.has(extension)) return 'text'
    return null
  }

  const supportsBoundedPreview = (format?: string | null) => {
    if (!format) return false
    const normalized = format.toLocaleUpperCase()
    return boundedPreviewFormats.value.some(candidate =>
      normalized === candidate.toLocaleUpperCase()
      || normalized.startsWith(`${candidate.toLocaleUpperCase()}.`),
    )
  }

  return {
    capabilities: engine.capabilities,
    loading: engine.loading,
    refresh: engine.refresh,
    browseExtensions,
    entryCategory,
    previewKind,
    supportsBoundedPreview,
    isNestedArchiveName: (name: string) => nestedExtensions.value.has(normalizedExtension(name)),
  }
}
