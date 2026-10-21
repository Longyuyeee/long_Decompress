import type { ZipEditCatalog, ZipEditChange, ZipEditDraft, ZipEditEntry, ZipEditRow } from '@/types/zipEdit'
const key = (path: string) => path.replace(/\/$/, '').toLocaleLowerCase()
export function resolveZipEdits(catalog: ZipEditCatalog, drafts: ZipEditDraft[]) {
  const original = new Map(catalog.entries.map(entry => [key(entry.path), entry]))
  const nodes = new Map<string, ZipEditEntry>(original)
  const rows: ZipEditRow[] = []
  const accepted = new Map<string, { row: ZipEditRow; change: ZipEditChange }>()
  const mappings: { batch: string; from: string; to: string | null; blocked?: boolean }[] = []
  const unique = (path: string, directory: boolean) => {
    const clean = directory ? path.slice(0, -1) : path, slash = clean.lastIndexOf('/'), dot = clean.lastIndexOf('.')
    const extension = !directory && dot > slash + 1 ? clean.slice(dot) : ''
    const stem = extension ? clean.slice(0, -extension.length) : clean
    for (let n = 2; ; n++) { const next = `${stem} (${n})${extension}${directory ? '/' : ''}`; if (!nodes.has(key(next))) return next }
  }
  for (const draft of drafts) {
    let mapping: typeof mappings[number] | undefined
    for (let i = mappings.length - 1; i >= 0; i--) { const candidate = mappings[i]; if (candidate.batch === draft.batch && draft.targetPath.startsWith(candidate.from)) { mapping = candidate; break } }
    const row: ZipEditRow = { draft, target: draft.targetPath, status: 'pending' }; rows.push(row)
    if (mapping?.to === null) { row.status = 'skip'; continue }
    if (mapping?.blocked) { row.issue = '先处理上级目录的同名项'; continue }
    let target = mapping ? mapping.to! + draft.targetPath.slice(mapping.from.length) : draft.targetPath
    // Keep the existing directory spelling when merging into it.
    const parts = target.replace(/\/$/, '').split('/')
    for (let i = 1; i < parts.length; i++) {
      const parent = nodes.get(key(parts.slice(0, i).join('/')))
      if (parent?.isDir) parts.splice(0, i, ...parent.path.replace(/\/$/, '').split('/'))
    }
    target = parts.join('/') + (draft.isDir ? '/' : ''); row.target = target
    const initialCollision = nodes.get(key(target))
    if (initialCollision && !(draft.isDir && initialCollision.isDir)) row.collision = initialCollision.isDir ? 'directory' : 'file'
    if (draft.choice === 'skip') {
      row.status = 'skip'; if (draft.isDir) mappings.push({ batch: draft.batch, from: draft.targetPath, to: null }); continue
    }
    let old = nodes.get(key(target))
    if (draft.isDir && old?.isDir) {
      row.target = old.path; row.status = 'merge'; mappings.push({ batch: draft.batch, from: draft.targetPath, to: old.path }); continue
    }
    if (old) {
      row.collision = old.isDir ? 'directory' : 'file'
      if (draft.choice === 'keep-both') { target = unique(target, draft.isDir); row.target = target; old = undefined }
      else if (draft.choice !== 'replace' || draft.isDir || old.isDir) {
        row.issue = draft.isDir || old.isDir ? '文件与目录同名，请跳过或保留两份' : '同名文件，请选择处理方式'
        if (draft.isDir) mappings.push({ batch: draft.batch, from: draft.targetPath, to: target, blocked: true })
        continue
      }
    }
    if (draft.isDir) mappings.push({ batch: draft.batch, from: draft.targetPath, to: target })
    const k = key(target), previous = accepted.get(k)
    if (previous) previous.row.status = 'superseded'
    row.status = old ? 'replace' : 'add'
    const { sourcePath, isDir, size, sourceRevision, comparison } = draft
    accepted.set(k, { row, change: { sourcePath, targetPath: target, isDir, size, sourceRevision, comparison, action: original.has(k) ? 'replace' : 'add' } })
    nodes.set(k, { path: target, isDir, size })
  }
  return { rows, changes: [...accepted.values()].map(item => item.change), unresolved: rows.filter(row => row.status === 'pending').length }
}
