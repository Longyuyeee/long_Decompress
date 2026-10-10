import { ref } from 'vue'

const key = 'long-saved-locations-v1'
const identity = (path: string) => path.replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase()
const clean = (items: unknown, limit: number): string[] => {
  if (!Array.isArray(items)) return []
  const result: string[] = []
  for (const path of items) {
    if (typeof path !== 'string' || !path.trim() || path.length > 32768) continue
    if (!result.some(item => identity(item) === identity(path))) result.push(path)
    if (result.length === limit) break
  }
  return result
}

export function useSavedLocations(storage: Storage = localStorage) {
  const bookmarks = ref<string[]>([])
  const recent = ref<string[]>([])
  const error = ref('')
  let readable = true
  try {
    const saved = JSON.parse(storage.getItem(key) || '{}')
    bookmarks.value = clean(saved?.bookmarks, 50)
    recent.value = clean(saved?.recent, 20)
  } catch { readable = false; error.value = '无法读取已保存位置，已保留原数据；当前目录仍可正常使用。' }
  const save = (nextBookmarks: string[], nextRecent: string[]) => {
    if (!readable) return false
    try {
      storage.setItem(key, JSON.stringify({ bookmarks: nextBookmarks, recent: nextRecent }))
      bookmarks.value = nextBookmarks
      recent.value = nextRecent
      error.value = ''
      return true
    } catch {
      error.value = '位置保存失败，未更改已保存的书签；请检查可用空间后重试。'
      return false
    }
  }
  const isBookmarked = (path: string) => bookmarks.value.some(item => identity(item) === identity(path))
  const toggleBookmark = (path: string) => {
    if (!path.trim()) return false
    if (isBookmarked(path)) return save(bookmarks.value.filter(item => identity(item) !== identity(path)), recent.value)
    if (bookmarks.value.length >= 50) { error.value = '最多保存 50 个书签，请先移除不用的位置。'; return false }
    return save([...bookmarks.value, path], recent.value)
  }
  const remember = (path: string) => save(bookmarks.value, clean([path, ...recent.value], 20))
  return { bookmarks, recent, error, isBookmarked, toggleBookmark, remember }
}
