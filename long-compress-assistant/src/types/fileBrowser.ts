import type { ArchiveReadOptions } from '@/stores/app'

/** A session-only request. Passwords never become a saved file-manager location. */
export interface ArchivePaneSource {
  id: number
  path: string
  options?: ArchiveReadOptions
}

export interface FilePanePosition {
  path: string
  selectedPaths: string[]
  history: string[]
  historyIndex: number
  scrollTop: number
  selectionMode: boolean
}

/** The disk view where this quick pack was started. Lives with the current task. */
export interface BrowserCompressionOrigin {
  source: 'left' | 'right'
  left: FilePanePosition
  right: FilePanePosition
}

export interface BrowserReturnRequest {
  origin: BrowserCompressionOrigin
  outputPath?: string
}
