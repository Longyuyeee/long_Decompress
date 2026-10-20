import type { ChecksumEntryStatus, ChecksumVerificationEntry, ChecksumVerificationReport } from '@/types/checksum'

export const checksumStatusLabels: Record<ChecksumEntryStatus, string> = {
  matched: '匹配', missing: '缺失', mismatch: '内容不符', unreadable: '无法读取', invalid_entry: '清单项有误', not_checked: '未检查', interrupted: '已停止',
}
export const checksumUnfinished = (entry: ChecksumVerificationEntry) => ['not_checked', 'interrupted'].includes(entry.status)
export const checksumProblem = (entry: ChecksumVerificationEntry) => !['matched', 'not_checked', 'interrupted'].includes(entry.status)

/** Retain earlier observations only for the exact same manifest, never infer unchecked success. */
export function mergeChecksumVerification(previous: ChecksumVerificationReport | null, next: ChecksumVerificationReport): ChecksumVerificationReport {
  const same = previous?.checksumPath === next.checksumPath && previous?.manifestId === next.manifestId
  const old = new Map(same ? previous!.entries.map(entry => [entry.line, entry]) : [])
  const entries = next.entries.map(entry => entry.status === 'not_checked' && old.has(entry.line) ? old.get(entry.line)! : entry)
  return { ...next, entries, valid: entries.length > 0 && entries.every(entry => entry.status === 'matched') }
}

/** Locked/unreadable files need access fixed; they must not be presented as files to download. */
export function checksumPendingList(entries: ChecksumVerificationEntry[]): string {
  return [...new Set(entries.filter(entry => ['missing', 'mismatch'].includes(entry.status)).map(entry => entry.fileName))].join('\n')
}
