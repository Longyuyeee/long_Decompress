import type { ZipPasswordEncoding } from './zipPassword'

export interface BatchConfigPatch {
  format?: 'zip' | 'zipx' | '7z' | 'rar' | 'swm' | 'wim' | 'esd' | 'zpaq' | 'tar.lz' | 'lzma' | 'tar.lzma' | 'lzma.aes' | 'tar.lzma.aes'
  wimMethod?: 'store' | 'xpress' | 'lzx' | 'lzms' | 'solid'
  sevenZipFilter?: 'none' | 'delta'; sevenZipDeltaDistance?: number; sevenZipMethod?: 'lzma2' | 'lzma' | 'bzip2' | 'deflate' | 'ppmd' | 'copy'
  zipMethod?: 'deflate' | 'zstd' | 'bzip2' | 'ppmd' | 'lzma' | 'store'
  password?: string
  encryptFilenames?: boolean
  splitArchive?: boolean
  splitSize?: string
  zipSplitMode?: 'numeric' | 'standard'
  level?: number
  rarRecoveryPercent?: number
  rarRecoveryVolumes?: number
  /** undefined keeps each destination; empty string uses each source directory. */
  outputDirectory?: string
  extractToSubfolder?: boolean
  /** ZIP/ZIPX input or output, according to the caller. Only input accepts auto. */
  zipPasswordEncoding?: ZipPasswordEncoding | 'auto'
}
