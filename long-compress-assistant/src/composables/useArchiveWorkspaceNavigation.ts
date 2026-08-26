import { computed, ref, type Ref } from 'vue'
import type { ArchiveBrowseResult, ArchiveEntryInfo } from '@/composables/useTauriCommands'

export const useArchiveWorkspaceNavigation = (
  result: Ref<ArchiveBrowseResult | null>,
  onLocationChanged?: () => void,
) => {
  const activeDirectory = ref('')
  const focusedEntryPath = ref('')
  const navigationBack = ref<string[]>([])
  const navigationForward = ref<string[]>([])
  const expandedDirectories = ref(new Set<string>())

  const directories = computed(() => {
    const values = new Set<string>()
    result.value?.entries.forEach(entry => {
      const normalized = entry.path.replace(/\\/g, '/').replace(/\/+$/, '')
      const parts = normalized.split('/')
      if (!entry.isDir) parts.pop()
      for (let index = 1; index <= parts.length; index++) values.add(parts.slice(0, index).join('/'))
    })
    return [...values].sort((left, right) => left.localeCompare(right))
  })

  const visibleDirectories = computed(() => directories.value
    .filter(path => {
      const parts = path.split('/')
      return parts.slice(0, -1).every((_, index) => expandedDirectories.value.has(parts.slice(0, index + 1).join('/')))
    })
    .map(path => ({
      path,
      name: path.split('/').pop() || path,
      depth: path.split('/').length - 1,
      hasChildren: directories.value.some(candidate => candidate.startsWith(`${path}/`) && candidate.split('/').length === path.split('/').length + 1),
    })))

  const directoryEntries = computed<ArchiveEntryInfo[]>(() => directories.value.map(path => ({
    path: `${path}/`,
    name: path.split('/').pop() || path,
    size: 0,
    compressedSize: 0,
    modified: null,
    crc: null,
    encrypted: false,
    isDir: true,
  })))

  const canNavigateBack = computed(() => navigationBack.value.length > 0)
  const canNavigateForward = computed(() => navigationForward.value.length > 0)
  const canNavigateUp = computed(() => activeDirectory.value.length > 0)
  const breadcrumbs = computed(() => {
    const parts = activeDirectory.value ? activeDirectory.value.split('/') : []
    return [
      { name: '根目录', path: '' },
      ...parts.map((name, index) => ({ name, path: parts.slice(0, index + 1).join('/') })),
    ]
  })

  const expandDirectoryAncestors = (path: string) => {
    if (!path) return
    const next = new Set(expandedDirectories.value)
    const parts = path.split('/')
    parts.forEach((_, index) => next.add(parts.slice(0, index + 1).join('/')))
    expandedDirectories.value = next
  }

  const navigateToDirectory = (path: string, recordHistory = true) => {
    const normalized = path.replace(/\\/g, '/').replace(/^\/+|\/+$/g, '')
    if (normalized === activeDirectory.value) return
    if (recordHistory) {
      navigationBack.value = [...navigationBack.value, activeDirectory.value]
      navigationForward.value = []
    }
    activeDirectory.value = normalized
    focusedEntryPath.value = ''
    onLocationChanged?.()
    expandDirectoryAncestors(normalized)
  }

  const goBack = () => {
    const target = navigationBack.value.at(-1)
    if (target === undefined) return
    navigationBack.value = navigationBack.value.slice(0, -1)
    navigationForward.value = [activeDirectory.value, ...navigationForward.value]
    navigateToDirectory(target, false)
  }

  const goForward = () => {
    const [target, ...remaining] = navigationForward.value
    if (target === undefined) return
    navigationForward.value = remaining
    navigationBack.value = [...navigationBack.value, activeDirectory.value]
    navigateToDirectory(target, false)
  }

  const goUp = () => {
    if (!activeDirectory.value) return
    const parts = activeDirectory.value.split('/')
    parts.pop()
    navigateToDirectory(parts.join('/'))
  }

  const reset = () => {
    activeDirectory.value = ''
    focusedEntryPath.value = ''
    navigationBack.value = []
    navigationForward.value = []
    expandedDirectories.value = new Set()
    onLocationChanged?.()
  }

  const reconcile = () => {
    const availableDirectories = new Set(directories.value)
    if (activeDirectory.value && !availableDirectories.has(activeDirectory.value)) activeDirectory.value = ''
    navigationBack.value = navigationBack.value.filter(path => !path || availableDirectories.has(path))
    navigationForward.value = navigationForward.value.filter(path => !path || availableDirectories.has(path))
    expandDirectoryAncestors(activeDirectory.value)
  }

  const toggleDirectory = (path: string) => {
    const next = new Set(expandedDirectories.value)
    next.has(path) ? next.delete(path) : next.add(path)
    expandedDirectories.value = next
  }

  return {
    activeDirectory,
    focusedEntryPath,
    navigationBack,
    navigationForward,
    expandedDirectories,
    directories,
    visibleDirectories,
    directoryEntries,
    canNavigateBack,
    canNavigateForward,
    canNavigateUp,
    breadcrumbs,
    navigateToDirectory,
    selectDirectory: navigateToDirectory,
    goBack,
    goForward,
    goUp,
    reset,
    reconcile,
    toggleDirectory,
    expandDirectoryAncestors,
  }
}
