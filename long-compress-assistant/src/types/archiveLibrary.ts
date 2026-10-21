export interface LibrarySource {
  id: string; path: string; name: string; format: string
  status: 'pending' | 'indexing' | 'ready' | 'failed' | 'interrupted' | 'locked'
  availability?: string; availabilityNote?: string; checkedAt?: string | null; encrypted?: boolean; sourceEncoding?: string | null; memberCount?: number
  note: string; revision: string; indexedAt: string | null; entryCount: number
}
export interface LibraryCandidate { path: string; name: string; format: string }
export interface LibraryScan { candidates: LibraryCandidate[]; visited: number; inaccessible: number; stopped: boolean; limited: boolean }
export interface LibraryHit { sourceId: string; sourcePath: string; sourceName: string; format: string; path: string; name: string; extension: string; size: number; modified: string | null; sizeKnown?: boolean; availability?: string; sourceStatus?: string; encrypted?: boolean; indexedAt?: string | null; revision?: string }
export interface LibraryResults { items: LibraryHit[]; total: number; page: number }
export const libraryStatus: Record<LibrarySource['status'], string> = { pending: '待建索引', indexing: '正在建库', ready: '可搜索', failed: '需要处理', interrupted: '已中断', locked: '需要密码' }
export const libraryBytes = (value: number) => {
  if (value < 1024) return `${value} B`
  const level = Math.min(4, Math.floor(Math.log(value) / Math.log(1024)))
  return `${(value / 1024 ** level).toFixed(1)} ${['B', 'KB', 'MB', 'GB', 'TB'][level]}`
}

export const libraryAvailability: Record<string, string> = { unchecked: '尚未检查来源', available: '上次检查可用', changed: '来源已变化 · 上次索引', missing: '来源离线 · 上次索引', incomplete: '缺少分卷 · 上次索引', unreadable: '来源不可读 · 上次索引' }
export interface LibraryCredentials { password: string; encoding?: import('@/types/zipPassword').ZipPasswordEncoding }

export interface LibraryOrganization { sourceId: string; favorite: boolean; tags: string[] }
export interface LibrarySearchFilter { query: string; sourceId: string; category: string; tag: string; favoritesOnly: boolean; page: number }
export interface LibrarySavedSearch { id: string; name: string; filters: LibrarySearchFilter }
export interface LibraryOrganizationState { sources: LibraryOrganization[]; searches: LibrarySavedSearch[] }
export const libraryHitKey = (hit: Pick<LibraryHit, 'sourceId' | 'path'>) => JSON.stringify([hit.sourceId, hit.path])
