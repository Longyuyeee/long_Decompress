export interface BatchConfigPatch {
  format?: 'zip' | '7z' | 'rar'
  level?: number
  /** undefined keeps each destination; empty string uses each source directory. */
  outputDirectory?: string
  extractToSubfolder?: boolean
}
