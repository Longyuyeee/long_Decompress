import type { ZipEditCatalog, ZipEditDraft, ZipEditRow, ZipEditChange, ZipOriginalEdit, ZipStructureOperation, ZipStructureRow, ZipWorkspaceNode } from '@/types/zipEdit'
import { resolveZipEdits } from './zipEditPlan'
export const zipPathKey = (path: string) => path.replace(/\/$/, '').toLowerCase()
export function validZipPath(path: string, isDir: boolean): string {
  const clean = path.replace(/\\/g, '/').replace(/\/+$/, '')
  if (!clean || clean.startsWith('/')) throw new Error('请输入相对包内路径，不能使用磁盘或根路径')
  for (const part of clean.split('/')) {
    if (!part || part === '.' || part === '..' || /[. ]$/.test(part) || /[<>:"|?*\x00-\x1f\x7f]/.test(part) || /^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)/i.test(part)) throw new Error(`包内名称不可用：${part || clean}`)
  }
  return clean + (isDir ? '/' : '')
}
function ensureParents(nodes: Map<string, ZipWorkspaceNode>) {
  for (const node of [...nodes.values()]) {
    const parts = node.path.replace(/\/$/, '').split('/')
    for (let i = 1; i < parts.length; i++) {
      const path = parts.slice(0, i).join('/') + '/', old = nodes.get(zipPathKey(path))
      if (old && (!old.isDir || old.path !== path)) throw new Error(`文件/目录或大小写位置冲突：${path}`)
      if (!old) nodes.set(zipPathKey(path), { id: 'directory:' + zipPathKey(path), path, isDir: true, size: 0 })
    }
  }
}
export function previewZipStructure(nodes: ZipWorkspaceNode[], operation: ZipStructureOperation) {
  const affected: ZipStructureRow['affected'] = []
  try {
    if (!operation.targets.length) throw new Error('请先选择需要整理的文件或目录')
    const byId = new Map(nodes.map(node => [node.id, node]))
    const picked = operation.targets.map(target => {
      const node = byId.get(target.id)
      if (!node) throw new Error('所选来源已撤销或移除，请撤销此操作后重新选择')
      const path = operation.kind === 'delete' ? null : validZipPath(target.path || '', node.isDir)
      if (path && node.isDir && zipPathKey(path) !== zipPathKey(node.path) && zipPathKey(path).startsWith(zipPathKey(node.path) + '/')) throw new Error(`不能将目录移入自身子目录：${node.path}`)
      return { node, path }
    })
    if (new Set(picked.map(item => item.node.id)).size !== picked.length) throw new Error('所选条目重复，请重新选择')
    const selectedDirectories = new Set(picked.filter(item=>item.node.isDir).map(item=>item.node.path))
    const roots = picked.filter(item => {
      const clean=item.node.path.replace(/\/$/,'')
      for (let at=clean.indexOf('/');at>=0;at=clean.indexOf('/',at+1)) if (selectedDirectories.has(clean.slice(0,at+1))) return false
      return true
    })
    const rootsByPath = new Map(roots.map(root=>[root.node.path,root]))
    const next = new Map<string, ZipWorkspaceNode>()
    let changed = false
    for (const node of nodes) {
      let root = rootsByPath.get(node.path)
      if (!root) { const clean=node.path.replace(/\/$/,'');for (let at=clean.indexOf('/');at>=0;at=clean.indexOf('/',at+1)) {root=rootsByPath.get(clean.slice(0,at+1));if(root)break} }
      const path = root ? root.path === null ? null : root.path + node.path.slice(root.node.path.length) : node.path
      if (root) { affected.push({ before: node.path, after: path, isDir: node.isDir }); if (path !== node.path) changed = true }
      if (path !== null) {
        const key = zipPathKey(path)
        if (next.has(key)) throw new Error(`目标同名，不能覆盖：${path}`)
        next.set(key, { ...node, path })
      }
    }
    if (!changed) throw new Error('新路径与当前路径相同，请调整名称或目录')
    ensureParents(next)
    return { nodes: [...next.values()], affected }
  } catch (error) { return { nodes, affected, issue: error instanceof Error ? error.message : String(error) } }
}
export function resolveZipWorkspace(catalog: ZipEditCatalog, drafts: ZipEditDraft[], operations: ZipStructureOperation[] = []) {
  let nodes = catalog.entries.map(entry => ({ ...entry, id: 'original:' + entry.path, originalPath: entry.path } as ZipWorkspaceNode))
  const rows: ZipEditRow[] = [], structureRows: ZipStructureRow[] = []
  const events = [...drafts.map((draft, index) => ({ order: draft.order ?? index, draft, operation: undefined as ZipStructureOperation | undefined })), ...operations.map(operation => ({ order: operation.order, draft: undefined as ZipEditDraft | undefined, operation }))].sort((a,b) => a.order - b.order)
  let pending: ZipEditDraft[] = [], unresolved = 0
  function addPending() {
    if (!pending.length) return
    const segment = resolveZipEdits({ ...catalog, entries: nodes }, pending)
    rows.push(...segment.rows); unresolved += segment.unresolved
    const map = new Map(nodes.map(node => [zipPathKey(node.path), node]))
    for (const row of segment.rows) if (row.status === 'add' || row.status === 'replace') {
      const { sourcePath, sourceRevision, isDir, size } = row.draft
      const previous=map.get(zipPathKey(row.target))
      map.set(zipPathKey(row.target), { originalAlias: previous?.originalPath || previous?.originalAlias, id: 'input:' + row.draft.id, path: row.target, isDir, size, input: { sourcePath, sourceRevision, isDir, size, targetPath: row.target } })
    }
    try { ensureParents(map) } catch (error) { unresolved++; rows[rows.length-1].issue = String(error) }
    nodes = [...map.values()]; pending = []
  }
  for (const event of events) {
    if (event.draft) pending.push(event.draft)
    else if (event.operation) {
      addPending(); const preview = previewZipStructure(nodes, event.operation)
      structureRows.push({ operation: event.operation, affected: preview.affected, issue: preview.issue })
      if (preview.issue) unresolved++
      else nodes = preview.nodes
    }
  }
  addPending()
  const originals = new Map(nodes.filter(node => node.originalPath).map(node => [node.originalPath!, node]))
  const originalEdits: ZipOriginalEdit[] = catalog.entries.flatMap(entry => {
    const node = originals.get(entry.path)
    return node?.path === entry.path ? [] : [{ sourcePath: entry.path, targetPath: node?.path ?? null }]
  })
  const changes: ZipEditChange[] = nodes.filter(node => node.input).map(node => ({ ...node.input!, targetPath: node.path, action: 'add' }))
  const changeCount = new Set([...originalEdits.map(edit => zipPathKey(edit.sourcePath)), ...changes.map(change => zipPathKey(change.targetPath))]).size
  return { nodes, rows, structureRows, originalEdits, changes, unresolved, changeCount, hasChanges: !!(originalEdits.length || changes.length) }
}
export interface ZipBatchRule { prefix: string; suffix: string; search: string; replacement: string; destination: string }
export function zipBatchTargets(nodes: ZipWorkspaceNode[], rule: ZipBatchRule) {
  return nodes.map(node => {
    const clean = node.path.replace(/\/$/, ''), slash = clean.lastIndexOf('/'), name = clean.slice(slash + 1)
    const dot = name.lastIndexOf('.'), extension = !node.isDir && dot > 0 ? name.slice(dot) : '', stem = extension ? name.slice(0,-extension.length) : name
    const renamed = (rule.search ? stem.split(rule.search).join(rule.replacement) : stem)
    const parent = rule.destination.trim() ? rule.destination.trim().replace(/\\/g,'/').replace(/\/+$/,'') + '/' : slash < 0 ? '' : clean.slice(0,slash+1)
    return { id: node.id, path: parent + rule.prefix + renamed + rule.suffix + extension + (node.isDir ? '/' : '') }
  })
}
