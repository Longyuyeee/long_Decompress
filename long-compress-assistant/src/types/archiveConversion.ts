import type { ZipPasswordEncoding } from './zipPassword'

export interface ConversionSettings {
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
  zipMethod?: 'deflate' | 'zstd'
  zipEncryption: 'aes256' | 'zipcrypto'
  zipSplitMode?: 'numeric' | 'standard'
  zipPasswordEncoding: ZipPasswordEncoding
}
export interface ConversionDraft {
  id: string
  sourcePath: string
  sourceParts?: string[]
  sourceReference?: string
  intent: 'convert' | 'reconfigure'
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
