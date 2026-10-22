import type { ArchiveReadOptions } from '@/stores/app'

/** A session-only request. Passwords never become a saved file-manager location. */
export interface ArchivePaneSource {
  id: number
  path: string
  options?: ArchiveReadOptions
}
