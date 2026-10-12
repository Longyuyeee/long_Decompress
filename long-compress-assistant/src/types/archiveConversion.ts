export interface ConversionSettings {
  format: string
  level: number
  outputDirectory: string
  outputPath: string
  sourcePassword: string
  password: string
  splitMiB: number
  encryptFilenames: boolean
  zipEncryption: 'aes256' | 'zipcrypto'
  zipPasswordEncoding: 'utf8' | 'windows_local'
}
export interface ConversionDraft {
  id: string
  sourcePath: string
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
}
export type ConversionPatch = Partial<ConversionSettings>
