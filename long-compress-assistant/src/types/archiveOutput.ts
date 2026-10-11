/** Snapshot of files measured after archive publication, not an output prefix. */
export interface PublishedArchiveOutput {
  primaryPath: string
  files: Array<{ path: string; bytes: number }>
  totalBytes: number
  split: boolean
}
