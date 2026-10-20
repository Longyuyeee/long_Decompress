import type { ZipPasswordEncoding } from './zipPassword'

export interface BatchConfigPatch {
  format?: 'zip' | 'zipx' | '7z' | 'rar' | 'tar.lz' | 'lzma'
  zipMethod?: 'deflate' | 'zstd'
  level?: number
  rarRecoveryPercent?: number
  rarRecoveryVolumes?: number
  /** undefined keeps each destination; empty string uses each source directory. */
  outputDirectory?: string
  extractToSubfolder?: boolean
  /** ZIP/ZIPX input or output, according to the caller. Only input accepts auto. */
  zipPasswordEncoding?: ZipPasswordEncoding | 'auto'
}
