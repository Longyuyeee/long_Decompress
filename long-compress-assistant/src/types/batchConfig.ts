export interface BatchConfigPatch {
  format?: 'zip' | 'zipx' | '7z' | 'rar' | 'tar.lz'
  zipMethod?: 'deflate' | 'zstd'
  level?: number
  rarRecoveryPercent?: number
  rarRecoveryVolumes?: number
  /** undefined keeps each destination; empty string uses each source directory. */
  outputDirectory?: string
  extractToSubfolder?: boolean
}
