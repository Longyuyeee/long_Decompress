import { ref } from 'vue'
import type { ArchiveBrowseResult } from '@/composables/useTauriCommands'

type BrowseArchive = (filePath: string, password?: string, browseId?: string) => Promise<ArchiveBrowseResult>
type CancelArchiveBrowse = (browseId: string) => Promise<void>

const createBrowseId = () => globalThis.crypto?.randomUUID?.()
  ?? `archive-browse-${Date.now()}-${Math.random().toString(16).slice(2)}`

export const describeArchiveBrowseError = (error: unknown) => {
  const raw = String(error).replace(/^Error:\s*/i, '')
  const marker = raw.indexOf('ARCHIVE_BROWSE_')
  if (marker < 0) return '无法读取压缩包结构，请确认文件完整且格式受支持'
  const [, message] = raw.slice(marker).split('|')
  return message || '无法读取压缩包结构，请稍后重试'
}

export const useArchiveBrowseRequest = (
  browseArchive: BrowseArchive,
  cancelArchiveBrowse: CancelArchiveBrowse,
) => {
  const loading = ref(false)
  const notice = ref('')
  let activeBrowseId = ''

  const run = async (filePath: string, password?: string) => {
    const browseId = createBrowseId()
    activeBrowseId = browseId
    loading.value = true
    notice.value = ''
    try {
      return await browseArchive(filePath, password, browseId)
    } finally {
      if (activeBrowseId === browseId) {
        activeBrowseId = ''
        loading.value = false
      }
    }
  }

  const cancel = (showNotice = true) => {
    const browseId = activeBrowseId
    activeBrowseId = ''
    loading.value = false
    if (showNotice) notice.value = '已取消读取压缩包内容'
    if (browseId) void cancelArchiveBrowse(browseId).catch(() => undefined)
    return Boolean(browseId)
  }

  return {
    loading,
    notice,
    run,
    cancel,
    clearNotice: () => { notice.value = '' },
  }
}
