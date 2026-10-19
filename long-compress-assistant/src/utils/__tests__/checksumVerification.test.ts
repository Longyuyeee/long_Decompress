import { describe, expect, it } from 'vitest'
import { checksumPendingList, mergeChecksumVerification } from '../checksumVerification'
import type { ChecksumEntryStatus, ChecksumVerificationReport } from '@/types/checksum'
const report = (statuses: ChecksumEntryStatus[], id = 'original'): ChecksumVerificationReport => ({
  valid: false, message: '', checksumPath: 'C:/checks.sha256', manifestId: id,
  entries: statuses.map((status, i) => ({ line: i + 1, fileName: `part.${i + 1}`, path: `C:/part.${i + 1}`, status, reason: status,
    expectedChecksum: 'a'.repeat(64), actualChecksum: status === 'matched' ? 'a'.repeat(64) : null, algorithm: 'sha256', checkedAt: status === 'not_checked' ? null : 123 })),
})
describe('checksum verification observations', () => {
  it('retains prior matched observations while replacing only the checked rows', () => {
    const old = report(['matched', 'missing', 'mismatch']), next = report(['not_checked', 'matched', 'not_checked'])
    const merged = mergeChecksumVerification(old, next)
    expect(merged.entries[0]).toBe(old.entries[0]); expect(merged.entries[2]).toBe(old.entries[2])
    expect(merged.entries[1].status).toBe('matched'); expect(merged.valid).toBe(false)
  })
  it('does not infer success for unvisited entries, including an empty manifest', () => {
    expect(mergeChecksumVerification(null, report(['matched', 'not_checked'])).valid).toBe(false)
    expect(mergeChecksumVerification(null, report([])).valid).toBe(false)
    expect(mergeChecksumVerification(report(['matched','missing']), report(['not_checked','matched'])).valid).toBe(true)
  })
  it('discards all older observations when manifest bytes or source path change', () => {
    const old = report(['matched','matched']), next = report(['not_checked','matched'], 'changed')
    expect(mergeChecksumVerification(old,next).valid).toBe(false)
    next.manifestId=old.manifestId;next.checksumPath='D:/checks.sha256'
    expect(mergeChecksumVerification(old,next).entries[0].status).toBe('not_checked')
  })
  it('copies missing and mismatched names only, preserving subdirectories and deduplicating', () => {
    const value=report(['missing','mismatch','unreadable','invalid_entry','not_checked','matched','missing'])
    value.entries[0].fileName='子目录/part.001';value.entries[6].fileName='子目录/part.001'
    expect(checksumPendingList(value.entries)).toBe('子目录/part.001\npart.2')
  })
})
