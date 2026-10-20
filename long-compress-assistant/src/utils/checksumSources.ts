import type { PublishedArchiveOutput } from '@/types/archiveOutput'
import { detectArchiveGroup } from './archiveSource'
import { isDecompressArchivePath, isPotentialSplitArchivePath } from './compressionFormat'
import { parentDirectory, pathKey } from './resultLocation'

// A published snapshot is authoritative: never pick up stale same-prefix files.
export async function checksumSources(path: string, output?: PublishedArchiveOutput | null): Promise<string[]> {
  let paths = output?.files.length ? output.files.map(file => file.path) : [path]
  if (!output?.files.length && (isDecompressArchivePath(path) || isPotentialSplitArchivePath(path))) {
    const group = await detectArchiveGroup(path)
    if (group.is_split) {
      if (!group.is_complete) {
        const missing = group.missing_parts.slice(0, 4).map(value => value.split(/[\\/]/).pop() || value).join('、')
        throw new Error(`原分卷组不完整，请补齐后再生成清单。缺少：${missing || '部分成员'}${group.missing_parts.length > 4 ? ` 等 ${group.missing_parts.length} 卷` : ''}`)
      }
      if (!group.parts.length) throw new Error('没有找到分卷文件，请确认结果仍在原位置。')
      paths = group.parts
    }
  }
  const seen = new Set<string>()
  return paths.filter(value => value.trim() && !seen.has(pathKey(value)) && !!seen.add(pathKey(value)))
}

// Suggest a directory containing every source, not the first file's subdirectory.
export function checksumBaseDirectory(paths: string[]): string {
  if (!paths.length) return ''
  let base = parentDirectory(paths[0])
  for (const path of paths.slice(1)) {
    while (base && pathKey(path) !== pathKey(base) && !pathKey(path).startsWith(pathKey(base).replace(/\/$/, '') + '/')) {
      const parent = parentDirectory(base)
      if (parent === base || pathKey(parent) === pathKey(base)) return ''
      base = parent
    }
    if (!base) return ''
  }
  return base
}
