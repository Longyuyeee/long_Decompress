export type ChecksumEntryStatus = 'matched' | 'missing' | 'mismatch' | 'unreadable' | 'invalid_entry' | 'not_checked'
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
}
