export interface ContextAction {
  action: string
  files: string[]
}

export const groupContextActions = (actions: ContextAction[]): ContextAction[] => {
  const grouped = new Map<string, Set<string>>()
  actions.forEach(({ action, files }) => {
    files.filter(Boolean).forEach(file => {
      // Legacy Explorer verbs may launch once per selected item. Keep quick-pack
      // invocations separated by directory so selections from two Explorer
      // windows can never be merged into one unexpected archive.
      const key = action === 'context-quick-pack'
        ? `${action}\u0000${getParentPath(file).toLocaleLowerCase()}`
        : action
      const paths = grouped.get(key) || new Set<string>()
      paths.add(file)
      grouped.set(key, paths)
    })
  })
  return Array.from(grouped, ([key, files]) => ({
    action: key.split('\u0000', 1)[0],
    files: Array.from(files),
  }))
}

export const getParentPath = (path: string) => {
  const index = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'))
  return index >= 0 ? path.substring(0, index) : '.'
}

const getFileName = (path: string) => path.split(/[\\/]/).pop() || 'archive'

const removeExtension = (name: string) => name.replace(/\.[^/.]+$/, '') || name

export const createQuickPackPlan = (files: string[]) => {
  const outputDirectory = getParentPath(files[0] || '.')
  const folderName = getFileName(outputDirectory)
  const archiveName = files.length === 1
    ? removeExtension(getFileName(files[0]))
    : (folderName && !/^[A-Za-z]:$/.test(folderName) ? folderName : 'archive')
  return { outputDirectory, archiveName }
}

export const createQuickPackCandidate = (
  plan: { outputDirectory: string; archiveName: string },
  collisionIndex = 0,
) => {
  const suffix = collisionIndex > 0 ? ` (${collisionIndex})` : ''
  const archiveName = `${plan.archiveName}${suffix}`
  const separator = plan.outputDirectory.includes('\\') ? '\\' : '/'
  const outputPath = plan.outputDirectory.endsWith('/') || plan.outputDirectory.endsWith('\\')
    ? `${plan.outputDirectory}${archiveName}.zip`
    : `${plan.outputDirectory}${separator}${archiveName}.zip`
  return { archiveName, outputPath }
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
