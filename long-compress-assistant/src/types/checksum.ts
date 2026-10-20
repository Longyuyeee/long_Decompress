export type ChecksumEntryStatus = 'matched' | 'missing' | 'mismatch' | 'unreadable' | 'invalid_entry' | 'not_checked' | 'interrupted'
export interface ChecksumVerificationEntry {
  line: number
  fileName: string
  path: string
  algorithm: string | null
  expectedChecksum: string | null
  actualChecksum: string | null
  status: ChecksumEntryStatus
  reason: string
  checkedAt: number | null
}
export interface ChecksumVerificationReport {
  valid: boolean
  message: string
  checksumPath: string
  manifestId: string
  entries: ChecksumVerificationEntry[]
  cancelled?: boolean
}

export interface ChecksumCalculationEntry {
  path: string
  fileName: string
  algorithm: string
  checksum: string
  status: 'success' | 'pending' | 'error' | 'interrupted'
  error?: string | null
}
export interface ChecksumProgress {
  operationId: string
  currentFile: string
  fileBytes: number
  fileSize: number
  completedBytes: number
  totalBytes: number
  completedFiles: number
  totalFiles: number
  entry?: ChecksumVerificationEntry | null
  report?: ChecksumVerificationReport | null
  calculationResults?: ChecksumCalculationEntry[] | null
}
