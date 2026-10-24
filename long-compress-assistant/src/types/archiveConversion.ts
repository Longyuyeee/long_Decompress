import type { ZipPasswordEncoding } from './zipPassword'

export interface ConversionSettings {
  updateSources?: string[]
  archivePrefix?: string
  sourceZpaqSnapshot?: import('./zpaqSnapshot').ZpaqSnapshot
  sourceZipPasswordEncoding?: ZipPasswordEncoding
  format: string
  level: number
  outputDirectory: string
  outputPath: string
  sourcePassword: string
  password: string
  splitMiB: number
  encryptFilenames: boolean
  rarRecoveryPercent?: number
  rarRecoveryVolumes?: number
  wimMethod?: 'store' | 'xpress' | 'lzx' | 'lzms' | 'solid'
  sevenZipMethod?: 'lzma2' | 'lzma' | 'bzip2'
  zipMethod?: 'deflate' | 'zstd' | 'bzip2' | 'ppmd' | 'lzma'
  zipEncryption: 'aes256' | 'zipcrypto'
  zipSplitMode?: 'numeric' | 'standard'
  zipPasswordEncoding: ZipPasswordEncoding
}
export interface ConversionDraft {
  id: string
  sourcePath: string
  sourceParts?: string[]
  sourceBytes?: number
  /** Derived from the confirmed source reference, rechecked before execution. */
  sourceNativeParts?: number
  sourceReference?: string
  /** Content detection, never inferred from a filename suffix. Rechecked before execution. */
  sourceFormat?: string
  intent: 'convert' | 'reconfigure' | 'zpaq-update'
  defaultFormat?: string
  expanded: boolean
  selected: boolean
  settings?: ConversionSettings
  taskId?: string
  retryOfTaskId?: string
  error?: string
  sourceIssue?: string
  passwordRequired?: boolean
  sourcePasswordRequired?: boolean
  targetPasswordRequired?: boolean
}
export type ConversionPatch = Partial<ConversionSettings>
