import type { ZipPasswordEncoding } from './zipPassword'

export interface BatchConfigPatch {
  format?: 'zip' | 'zipx' | '7z' | 'rar' | 'wim' | 'zpaq' | 'tar.lz' | 'lzma' | 'tar.lzma' | 'lzma.aes' | 'tar.lzma.aes'
  wimMethod?: 'store' | 'xpress' | 'lzx' | 'lzms'
  zipMethod?: 'deflate' | 'zstd' | 'bzip2'
  level?: number
  rarRecoveryPercent?: number
  rarRecoveryVolumes?: number
  /** undefined keeps each destination; empty string uses each source directory. */
  outputDirectory?: string
  extractToSubfolder?: boolean
  /** ZIP/ZIPX input or output, according to the caller. Only input accepts auto. */
  zipPasswordEncoding?: ZipPasswordEncoding | 'auto'
}
