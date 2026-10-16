export interface BatchConfigPatch {
  format?: 'zip' | '7z' | 'rar' | 'tar.lz'
  level?: number
  rarRecoveryPercent?: number
  rarRecoveryVolumes?: number
  /** undefined keeps each destination; empty string uses each source directory. */
  outputDirectory?: string
  extractToSubfolder?: boolean
}
