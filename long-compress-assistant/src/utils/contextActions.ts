export interface ContextAction {
  action: string
  files: string[]
}

export const groupContextActions = (actions: ContextAction[]): ContextAction[] => {
  const grouped = new Map<string, Set<string>>()
  actions.forEach(({ action, files }) => {
    const paths = grouped.get(action) || new Set<string>()
    files.filter(Boolean).forEach(file => paths.add(file))
    grouped.set(action, paths)
  })
  return Array.from(grouped, ([action, files]) => ({ action, files: Array.from(files) }))
}

export const createContextCompressionEntry = (
  path: string,
  info: { size: number; is_dir: boolean } | null,
) => ({
  name: path.split(/[\\/]/).pop() || path,
  path,
  size: info?.size || 0,
  type: info?.is_dir ? 'folder' : 'file',
  isDirectory: info?.is_dir || false,
})
