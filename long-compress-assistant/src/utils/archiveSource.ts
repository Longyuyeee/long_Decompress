import { invoke } from '@tauri-apps/api/tauri'
import { FORMAT_CAPABILITIES, EXTRACT_ONLY_FORMATS } from './compressionFormat'

export interface ArchiveVolumeGroup {
  is_split: boolean
  format?: string | null
  archive_format?: string | null
  base_name?: string | null
  first_part?: string | null
  parts: string[]
  total_parts: number
  is_complete: boolean
  missing_parts: string[]
}

export const detectArchiveGroup = (path: string) => invoke<ArchiveVolumeGroup>('detect_split_archive', { path })

export function archiveGroupKey(path: string): string {
  return path.replace(/\\/g, '/').toLowerCase()
    .replace(/\.part\d+\.rar$/, '.rar')
    .replace(/\.r\d{2,3}$/, '.rar')
    .replace(/\.z\d{2,5}$/, '.zip')
    .replace(/\.\d{3,}$/, '.001')
    .replace(/\.part\d+$/, '.part1')
}

export function archiveSourceFormat(path: string): string | undefined {
  const name = path.toLowerCase().replace(/\.\d{3,}$|\.part\d+$/, '')
  if (/\.z\d{2,5}$/.test(name)) return 'zip'
  if (/\.r\d{2,3}$|\.part\d+\.rar$/.test(name)) return 'rar'
  const formats = FORMAT_CAPABILITIES.flatMap(format => format.extensions.map(extension => ({ extension, format: format.format })))
    .sort((a, b) => b.extension.length - a.extension.length)
  return formats.find(format => name.endsWith('.' + format.extension))?.format
    ?? EXTRACT_ONLY_FORMATS.flatMap(format => format.extensions).find(extension => name.endsWith('.' + extension))
}

export function archiveSourceStem(path: string): string {
  let name = path.split(/[\\/]/).pop() || 'archive'
  name = name.replace(/\.part\d+\.rar$/i, '.rar').replace(/\.r\d{2,3}$/i, '.rar').replace(/\.z\d{2,5}$/i, '.zip')
    .replace(/\.\d{3,}$|\.part\d+$/i, '')
  const extensions = [...FORMAT_CAPABILITIES.flatMap(format => format.extensions), ...EXTRACT_ONLY_FORMATS.flatMap(format => format.extensions)].sort((a, b) => b.length - a.length)
  const extension = extensions.find(extension => name.toLowerCase().endsWith('.' + extension))
  return extension ? name.slice(0, -(extension.length + 1)) : name
}
