// Settings, profiles and retry drafts store MiB; compression IPC expects bytes.
export const splitSizeBytes = (enabled: boolean, sizeMiB: string | number): number | null =>
  enabled ? Number(sizeMiB) * 1024 * 1024 : null
