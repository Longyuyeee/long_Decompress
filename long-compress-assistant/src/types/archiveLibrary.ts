export interface LibrarySource {
  id: string; path: string; name: string; format: string
  status: 'pending' | 'indexing' | 'ready' | 'failed' | 'interrupted'
  note: string; revision: string; indexedAt: string | null; entryCount: number
}
export interface LibraryCandidate { path: string; name: string; format: string }
export interface LibraryScan { candidates: LibraryCandidate[]; visited: number; inaccessible: number; stopped: boolean; limited: boolean }
export interface LibraryHit { sourceId: string; sourcePath: string; sourceName: string; format: string; path: string; name: string; extension: string; size: number; modified: string | null }
export interface LibraryResults { items: LibraryHit[]; total: number; page: number }
export const libraryStatus: Record<LibrarySource['status'], string> = { pending: '待建索引', indexing: '正在建库', ready: '可搜索', failed: '需要处理', interrupted: '已中断' }
export const libraryBytes = (value: number) => {
  if (value < 1024) return `${value} B`
  const level = Math.min(4, Math.floor(Math.log(value) / Math.log(1024)))
  return `${(value / 1024 ** level).toFixed(1)} ${['B', 'KB', 'MB', 'GB', 'TB'][level]}`
}
