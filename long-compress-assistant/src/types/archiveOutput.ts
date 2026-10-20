/** Snapshot of files measured after archive publication, not an output prefix. */
export interface PublishedArchiveOutput {
  updateSummary?: { updatedEntries: number; newFragments: number; baseSnapshots: number; snapshots: number; unchanged: boolean; copiedBytes: number; inputBytes: number }
  primaryPath: string
  files: Array<{ path: string; bytes: number }>
  totalBytes: number
  split: boolean
}
