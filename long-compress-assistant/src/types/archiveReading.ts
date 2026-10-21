import {isZpaqSnapshot, snapshotKey, type ZpaqSnapshot} from './zpaqSnapshot'
import {isZipPasswordEncoding, type ZipPasswordEncoding} from './zipPassword'

export interface ReaderSelection {
  revision: string
  paths: string[]
  keepStructure: boolean
  conflictPolicy: 'rename' | 'skip'
}
export interface ArchiveReadingRecord {
  filePath: string
  format: string
  revision: string
  snapshot?: ZpaqSnapshot
  encoding?: ZipPasswordEncoding
  path: string
  directory: string
  scope: 'directory' | 'archive'
  fit: boolean
  zoom: number
  selected: string[]
  updatedAt: string
}
export const READING_KEY = 'long-decompress-archive-reading-v1'
export const readingKey = (path: string, snapshot?: ZpaqSnapshot) =>
  path.replace(/^\\\\\?\\/, '').replace(/\\/g, '/').toLowerCase() + ':' + snapshotKey(snapshot)
const text = (value: unknown): value is string => typeof value === 'string' && value.length > 0 && value.length <= 32768
export function copyReaderSelection(value: unknown): ReaderSelection | null {
  const row = value as ReaderSelection | null
  if (!row || !/^[a-f0-9]{64}$/.test(row.revision) || !Array.isArray(row.paths)
      || !row.paths.length || row.paths.length > 2000 || JSON.stringify(row.paths).length > 60000
      || !row.paths.every(text) || new Set(row.paths).size !== row.paths.length
      || typeof row.keepStructure !== 'boolean' || !['rename', 'skip'].includes(row.conflictPolicy)) return null
  return {revision: row.revision, paths: [...row.paths], keepStructure: row.keepStructure, conflictPolicy: row.conflictPolicy}
}
export function copyReadingRecord(value: unknown): ArchiveReadingRecord | null {
  const row = value as ArchiveReadingRecord | null
  if (!row || !text(row.filePath) || typeof row.format !== 'string' || row.format.length > 80
      || !/^[a-f0-9]{64}$/.test(row.revision) || !text(row.path)
      || typeof row.directory !== 'string' || row.directory.length > 32768
      || !['directory', 'archive'].includes(row.scope) || typeof row.fit !== 'boolean'
      || !Number.isFinite(row.zoom) || row.zoom < .25 || row.zoom > 4
      || !Array.isArray(row.selected) || row.selected.length > 2000 || !row.selected.every(text)
      || JSON.stringify(row.selected).length > 60000 || !Number.isFinite(Date.parse(row.updatedAt))
      || (row.snapshot != null && !isZpaqSnapshot(row.snapshot))) return null
  return {filePath: row.filePath, format: row.format, revision: row.revision,
    ...(row.snapshot ? {snapshot: {...row.snapshot}} : {}),
    ...(isZipPasswordEncoding(row.encoding) ? {encoding: row.encoding} : {}),
    path: row.path, directory: row.directory, scope: row.scope, fit: row.fit, zoom: row.zoom,
    selected: [...new Set(row.selected)], updatedAt: row.updatedAt}
}
