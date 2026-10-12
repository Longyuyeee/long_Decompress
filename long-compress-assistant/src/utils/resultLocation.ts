import type { PublishedArchiveOutput } from '@/types/archiveOutput'

export interface ResultLocation { path: string; directory: boolean }
export interface ResultSource {
  status: string
  type?: string
  taskType?: string
  outputPath: string
  archiveOutput?: PublishedArchiveOutput | null
  workloadKind?: string
  metrics?: unknown
}
export function resultLocation(source: ResultSource): ResultLocation | null {
  if (source.status !== 'completed') return null
  // A skipped image task records its planned path but publishes no new result.
  if (source.workloadKind === 'image' && !source.metrics) return null
  const path = source.archiveOutput?.primaryPath || source.outputPath
  return path.trim() ? { path, directory: (source.type || source.taskType) === 'decompression' } : null
}
export function parentDirectory(path: string): string {
  const index = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'))
  if (index === 2 && path[1] === ':') return path.slice(0, 3)
  if (index === 0) return path.slice(0, 1)
  return index >= 0 ? path.slice(0, index) : '.'
}
function pathKey(path: string) {
  const windows = /^[a-z]:[/\\]|^[/\\]{2}/i.test(path)
  const key = path.replace(/\\/g, '/').replace(/\/+$/, '')
  return windows ? key.toLowerCase() : key
}
export function uniqueResultDirectories(locations: ResultLocation[]): string[] {
  const directories = new Map<string, string>()
  for (const location of locations) {
    if (!location.path.trim()) continue
    const directory = location.directory ? location.path : parentDirectory(location.path)
    if (!directories.has(pathKey(directory))) directories.set(pathKey(directory), directory)
  }
  return [...directories.values()]
}
export function uniqueResultPaths(locations: ResultLocation[]): string[] {
  return [...new Map(locations.filter(location => location.path.trim()).map(location => [pathKey(location.path), location.path])).values()]
}
