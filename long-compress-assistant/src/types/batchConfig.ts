import type { ZipPasswordEncoding } from './zipPassword'

export interface BatchConfigPatch {
  format?: 'zip' | 'zipx' | '7z' | 'rar' | 'tar.lz'
  zipMethod?: 'deflate' | 'zstd'
  level?: number
  rarRecoveryPercent?: number
  rarRecoveryVolumes?: number
  /** undefined keeps each destination; empty string uses each source directory. */
  outputDirectory?: string
  extractToSubfolder?: boolean
  /** Only ZIP/ZIPX input tasks; auto explicitly clears an old override. */
  zipPasswordEncoding?: ZipPasswordEncoding | 'auto'
}
