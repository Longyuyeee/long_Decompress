<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { useAppStore } from '@/stores/app'
import { useTauriCommands, type ArchiveBrowseResult, type ArchiveEntryInfo, type ArchiveImagePreview } from '@/composables/useTauriCommands'

const appStore = useAppStore()
const commands = useTauriCommands()
const archivePath = ref('')
const outputPath = ref('')
const password = ref('')
const result = ref<ArchiveBrowseResult | null>(null)
const selected = ref(new Set<string>())
const query = ref('')
const typeFilter = ref('all')
const activeDirectory = ref('')
const focusedEntryPath = ref('')
const navigationBack = ref<string[]>([])
const navigationForward = ref<string[]>([])
const loading = ref(false)
const extracting = ref(false)
const imagePreview = ref<ArchiveImagePreview | null>(null)
const previewEntry = ref<ArchiveEntryInfo | null>(null)
const previewLoading = ref(false)
const previewError = ref('')
const expandedDirectories = ref(new Set<string>())
const isDraggingArchive = ref(false)
const contextMenu = ref<{
  entry: ArchiveEntryInfo | null
  entries: ArchiveEntryInfo[]
  left: number
  top: number
} | null>(null)
const detailEntries = ref<ArchiveEntryInfo[]>([])
let previewSequence = 0
let selectionAnchorPath = ''
let unlistenDrop: (() => void) | null = null
let unlistenHover: (() => void) | null = null
let unlistenCancel: (() => void) | null = null

const extensionGroups: Record<string, Set<string>> = {
  image: new Set(['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'svg', 'ico', 'avif']),
  document: new Set(['txt', 'md', 'pdf', 'doc', 'docx', 'xls', 'xlsx', 'ppt', 'pptx', 'csv', 'json', 'xml']),
  archive: new Set(['zip', '7z', 'rar', 'tar', 'gz', 'bz2', 'xz', 'zst', 'iso', 'cab'])
}
const boundedPreviewExtensions = new Set(['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp'])

const files = computed(() => result.value?.entries.filter(entry => !entry.isDir) ?? [])
const directories = computed(() => {
  const values = new Set<string>()
  result.value?.entries.forEach(entry => {
    const normalized = entry.path.replace(/\\/g, '/').replace(/\/+$/, '')
    const parts = normalized.split('/')
    if (!entry.isDir) parts.pop()
    for (let index = 1; index <= parts.length; index++) values.add(parts.slice(0, index).join('/'))
  })
  return [...values].sort((a, b) => a.localeCompare(b))
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
    hasChildren: directories.value.some(candidate => candidate.startsWith(`${path}/`) && candidate.split('/').length === path.split('/').length + 1)
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

const entryParent = (entry: ArchiveEntryInfo) => {
  const normalized = entry.path.replace(/\\/g, '/').replace(/\/+$/, '')
  const index = normalized.lastIndexOf('/')
  return index < 0 ? '' : normalized.slice(0, index)
}

const filteredEntries = computed(() => {
  const search = query.value.trim().toLocaleLowerCase()
  return [...directoryEntries.value, ...files.value].filter(entry => {
    const normalized = entry.path.replace(/\\/g, '/')
    if (!search && entryParent(entry) !== activeDirectory.value) return false
    if (search && !normalized.toLocaleLowerCase().includes(search)) return false
    if (entry.isDir) return typeFilter.value === 'all'
    if (typeFilter.value === 'all') return true
    const extension = entry.name.includes('.') ? entry.name.split('.').pop()!.toLocaleLowerCase() : ''
    if (typeFilter.value === 'other') {
      return !Object.values(extensionGroups).some(group => group.has(extension))
    }
    return extensionGroups[typeFilter.value]?.has(extension) ?? false
  }).sort((left, right) => Number(right.isDir) - Number(left.isDir) || left.name.localeCompare(right.name))
})

const visibleFiles = computed(() => filteredEntries.value.filter(entry => !entry.isDir))
const visibleSelected = computed(() => visibleFiles.value.length > 0 && visibleFiles.value.every(entry => selected.value.has(entry.path)))
const previewRouteSupported = computed(() => result.value?.format === 'ZIP' || result.value?.format.startsWith('TAR'))
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
const detailFiles = computed(() => detailEntries.value.flatMap(entry => entry.isDir
  ? files.value.filter(file => file.path.replace(/\\/g, '/').startsWith(entry.path.replace(/\\/g, '/')))
  : [entry]))
const detailTotalSize = computed(() => detailFiles.value.reduce((sum, entry) => sum + entry.size, 0))
const detailCompressedSize = computed(() => detailFiles.value.reduce((sum, entry) => sum + (entry.compressedSize ?? 0), 0))
const detailTitle = computed(() => detailEntries.value.length === 1 ? detailEntries.value[0].name : `${detailEntries.value.length} 个条目`)
const contextDisplayEntries = computed(() => {
  if (!contextMenu.value) return []
  if (contextMenu.value.entry?.isDir) return [contextMenu.value.entry]
  return contextMenu.value.entries
})

const canPreviewEntry = (entry: ArchiveEntryInfo) => {
  const extension = entry.name.includes('.') ? entry.name.split('.').pop()!.toLocaleLowerCase() : ''
  return boundedPreviewExtensions.has(extension)
}

const formatBytes = (value: number) => {
  if (value < 1024) return `${value} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let size = value / 1024
  let index = 0
  while (size >= 1024 && index < units.length - 1) { size /= 1024; index++ }
  return `${size.toFixed(size >= 100 ? 0 : size >= 10 ? 1 : 2)} ${units[index]}`
}

const parentDirectory = (path: string) => {
  const index = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'))
  return index > 0 ? path.slice(0, index) : ''
}

const archiveName = computed(() => archivePath.value.split(/[\\/]/).pop() || archivePath.value)

const openArchivePath = async (path: string) => {
  if (!path || loading.value) return
  try {
    archivePath.value = path
    outputPath.value = parentDirectory(path)
    query.value = ''
    typeFilter.value = 'all'
    await loadArchive()
  } catch (error) {
    appStore.setError(`无法打开压缩包：${String(error)}`)
  }
}

const takeDesktopDialogSelection = () => import.meta.env.VITE_DESKTOP_E2E
  ? window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection()
  : undefined

const chooseArchive = async () => {
  const queued = takeDesktopDialogSelection()
  const queuedPath = typeof queued === 'string' ? queued : Array.isArray(queued) ? queued[0] : null
  const queuedInfo = queuedPath ? await commands.getFileInfo(queuedPath) : null
  const picked = queued === undefined
    ? await commands.selectFiles(false)
    : queuedInfo ? [queuedInfo] : []
  if (!picked[0]) return
  await openArchivePath(picked[0].path)
}

const loadArchive = async () => {
  if (!archivePath.value || loading.value) return
  loading.value = true
  result.value = null
  selected.value = new Set()
  activeDirectory.value = ''
  focusedEntryPath.value = ''
  navigationBack.value = []
  navigationForward.value = []
  selectionAnchorPath = ''
  expandedDirectories.value = new Set()
  closePreview()
  try {
    result.value = await commands.browseArchive(archivePath.value, password.value)
    selected.value = new Set(result.value.entries.filter(entry => !entry.isDir).map(entry => entry.path))
  } catch (error) {
    appStore.setError(String(error))
  } finally {
    loading.value = false
  }
}

const closePreview = () => {
  previewSequence++
  imagePreview.value = null
  previewEntry.value = null
  previewLoading.value = false
  previewError.value = ''
}

const openPreview = async (entry: ArchiveEntryInfo) => {
  if (!canPreviewEntry(entry) || !previewRouteSupported.value) return
  const sequence = ++previewSequence
  previewEntry.value = entry
  imagePreview.value = null
  previewError.value = ''
  previewLoading.value = true
  try {
    const value = await commands.previewArchiveImage(archivePath.value, entry.path, password.value)
    if (sequence === previewSequence) imagePreview.value = value
  } catch (error) {
    if (sequence === previewSequence) previewError.value = String(error)
  } finally {
    if (sequence === previewSequence) previewLoading.value = false
  }
}

const chooseOutput = async () => {
  const picked = await requestOutputDirectory(outputPath.value)
  if (picked) outputPath.value = picked
}

const requestOutputDirectory = async (initialPath: string) => {
  const queued = takeDesktopDialogSelection()
  const picked = queued === undefined
    ? await commands.selectDirectory(initialPath || undefined)
    : typeof queued === 'string' ? queued : null
  return typeof picked === 'string' ? picked : null
}

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
  selectionAnchorPath = ''
  expandDirectoryAncestors(normalized)
}

const selectDirectory = (path: string) => {
  navigateToDirectory(path)
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

const refreshDirectory = async () => {
  if (!archivePath.value || loading.value) return
  loading.value = true
  focusedEntryPath.value = ''
  selectionAnchorPath = ''
  closePreview()
  try {
    const refreshed = await commands.browseArchive(archivePath.value, password.value)
    result.value = refreshed
    const availableFiles = new Set(refreshed.entries.filter(entry => !entry.isDir).map(entry => entry.path))
    selected.value = new Set([...selected.value].filter(path => availableFiles.has(path)))
    const availableDirectories = new Set(directories.value)
    if (activeDirectory.value && !availableDirectories.has(activeDirectory.value)) activeDirectory.value = ''
    navigationBack.value = navigationBack.value.filter(path => !path || availableDirectories.has(path))
    navigationForward.value = navigationForward.value.filter(path => !path || availableDirectories.has(path))
    expandDirectoryAncestors(activeDirectory.value)
  } catch (error) {
    appStore.setError(`刷新压缩包失败：${String(error)}`)
  } finally {
    loading.value = false
  }
}

const activateEntry = (entry: ArchiveEntryInfo) => {
  focusedEntryPath.value = entry.path
  if (entry.isDir) navigateToDirectory(entry.path.replace(/\/+$/, ''))
}

const contextEntriesFor = (entry: ArchiveEntryInfo | null) => {
  if (!entry) return files.value.filter(file => selected.value.has(file.path))
  if (entry.isDir) {
    const prefix = entry.path.replace(/\\/g, '/')
    return files.value.filter(file => file.path.replace(/\\/g, '/').startsWith(prefix))
  }
  if (selected.value.has(entry.path) && selected.value.size > 1) {
    return files.value.filter(file => selected.value.has(file.path))
  }
  return [entry]
}

const openContextMenu = (entry: ArchiveEntryInfo | null, event: MouseEvent) => {
  event.preventDefault()
  if (entry) focusedEntryPath.value = entry.path
  contextMenu.value = {
    entry,
    entries: contextEntriesFor(entry),
    left: Math.max(8, event.clientX),
    top: Math.max(8, event.clientY),
  }
  void nextTick(() => {
    const menu = document.querySelector<HTMLElement>('[data-testid="archive-context-menu"]')
    if (!menu || !contextMenu.value) return
    const bounds = menu.getBoundingClientRect()
    contextMenu.value.left = Math.max(8, Math.min(contextMenu.value.left, window.innerWidth - bounds.width - 8))
    contextMenu.value.top = Math.max(8, Math.min(contextMenu.value.top, window.innerHeight - bounds.height - 8))
  })
}

const closeContextMenu = () => {
  contextMenu.value = null
}

const showDetails = (entries: ArchiveEntryInfo[]) => {
  if (entries.length === 0) return
  detailEntries.value = entries
  closeContextMenu()
}

const closeDetails = () => {
  detailEntries.value = []
}

const copyContextText = async (kind: 'name' | 'path', entries: ArchiveEntryInfo[]) => {
  if (entries.length === 0) return
  const text = entries.map(entry => kind === 'name' ? entry.name : entry.path.replace(/\/+$/, '')).join('\n')
  closeContextMenu()
  try {
    await navigator.clipboard.writeText(text)
    appStore.setSuccess(`已复制${kind === 'name' ? '名称' : '归档内路径'}`)
  } catch (error) {
    appStore.setError(`复制失败：${String(error)}`)
  }
}

const extractPaths = async (paths: string[], destination: string) => {
  if (!archivePath.value || !destination || paths.length === 0 || extracting.value) return
  extracting.value = true
  closeContextMenu()
  try {
    await commands.decompressFile(archivePath.value, {
      outputPath: destination,
      password: password.value || undefined,
      keepStructure: true,
      overwrite: false,
      deleteAfter: false,
      preserveTimestamps: true,
      selectedEntries: paths,
      conflictPolicy: 'rename'
    })
    appStore.setSuccess(`已解压 ${paths.length} 个所选文件`)
  } catch (error) {
    appStore.setError(String(error))
  } finally {
    extracting.value = false
  }
}

const extractContextEntries = async (chooseDestination: boolean) => {
  const entries = contextMenu.value?.entries ?? []
  const paths = entries.filter(entry => !entry.isDir).map(entry => entry.path)
  if (chooseDestination) closeContextMenu()
  const destination = chooseDestination
    ? await requestOutputDirectory(outputPath.value)
    : outputPath.value
  if (destination) await extractPaths(paths, destination)
}

const openContextEntry = (entry: ArchiveEntryInfo) => {
  closeContextMenu()
  activateEntry(entry)
}

const previewContextEntry = (entry: ArchiveEntryInfo) => {
  closeContextMenu()
  void openPreview(entry)
}

const handleEntryClick = (entry: ArchiveEntryInfo, event: MouseEvent) => {
  focusedEntryPath.value = entry.path
  if (entry.isDir) return
  if (event.shiftKey && selectionAnchorPath) {
    const start = visibleFiles.value.findIndex(item => item.path === selectionAnchorPath)
    const end = visibleFiles.value.findIndex(item => item.path === entry.path)
    if (start >= 0 && end >= 0) {
      const next = new Set(selected.value)
      const [from, to] = start <= end ? [start, end] : [end, start]
      visibleFiles.value.slice(from, to + 1).forEach(item => next.add(item.path))
      selected.value = next
      return
    }
  }
  selectionAnchorPath = entry.path
  if (event.ctrlKey || event.metaKey) toggleEntry(entry)
}

const isEditableKeyboardTarget = (target: EventTarget | null) => {
  const element = target as HTMLElement | null
  return Boolean(element?.closest('input, select, textarea, [contenteditable="true"]'))
}

const handleWorkspaceKeydown = (event: KeyboardEvent) => {
  if (event.defaultPrevented) return
  if (event.key === 'Escape') {
    if (contextMenu.value) closeContextMenu()
    else if (detailEntries.value.length > 0) closeDetails()
    return
  }
  if (isEditableKeyboardTarget(event.target)) return
  const focusedEntry = [...directoryEntries.value, ...files.value].find(entry => entry.path === focusedEntryPath.value) ?? null
  const keyboardEntries = contextEntriesFor(focusedEntry)
  const keyboardDisplayEntries = focusedEntry?.isDir ? [focusedEntry] : keyboardEntries
  if ((event.shiftKey && event.key === 'F10') || event.key === 'ContextMenu') {
    event.preventDefault()
    const focusedElement = focusedEntryPath.value
      ? Array.from(document.querySelectorAll<HTMLElement>('[data-entry-path]'))
          .find(element => element.dataset.entryPath === focusedEntryPath.value) ?? null
      : null
    const rect = focusedElement?.getBoundingClientRect()
    openContextMenu(focusedEntry, new MouseEvent('contextmenu', {
      clientX: rect ? rect.left + Math.min(rect.width / 2, 160) : window.innerWidth / 2,
      clientY: rect ? rect.top + Math.min(rect.height, 36) : window.innerHeight / 2,
    }))
  } else if (event.ctrlKey && event.key === 'Enter' && focusedEntry && canPreviewEntry(focusedEntry) && previewRouteSupported.value) {
    event.preventDefault()
    void openPreview(focusedEntry)
  } else if (event.altKey && event.key === 'Enter' && keyboardDisplayEntries.length > 0) {
    event.preventDefault()
    showDetails(keyboardDisplayEntries)
  } else if (event.altKey && event.shiftKey && event.key.toLocaleLowerCase() === 'e' && keyboardEntries.length > 0) {
    event.preventDefault()
    void (async () => {
      const destination = await requestOutputDirectory(outputPath.value)
      if (destination) await extractPaths(keyboardEntries.map(entry => entry.path), destination)
    })()
  } else if (event.altKey && event.key.toLocaleLowerCase() === 'e' && keyboardEntries.length > 0) {
    event.preventDefault()
    void extractPaths(keyboardEntries.map(entry => entry.path), outputPath.value)
  } else if (event.ctrlKey && event.shiftKey && event.key.toLocaleLowerCase() === 'c' && keyboardDisplayEntries.length > 0) {
    event.preventDefault()
    void copyContextText('path', keyboardDisplayEntries)
  } else if (event.ctrlKey && event.altKey && event.key.toLocaleLowerCase() === 'c' && keyboardDisplayEntries.length > 0) {
    event.preventDefault()
    void copyContextText('name', keyboardDisplayEntries)
  } else if (event.key === 'F5') {
    event.preventDefault()
    void refreshDirectory()
  } else if (event.key === 'Enter' && focusedEntry) {
    event.preventDefault()
    activateEntry(focusedEntry)
  } else if (event.altKey && event.key === 'ArrowLeft') {
    event.preventDefault()
    goBack()
  } else if (event.altKey && event.key === 'ArrowRight') {
    event.preventDefault()
    goForward()
  } else if (event.key === 'Backspace') {
    event.preventDefault()
    goUp()
  }
}

const toggleDirectory = (path: string) => {
  const next = new Set(expandedDirectories.value)
  next.has(path) ? next.delete(path) : next.add(path)
  expandedDirectories.value = next
}

const droppedBrowserFiles = (event: DragEvent) => {
  event.preventDefault()
  isDraggingArchive.value = false
  const path = (event.dataTransfer?.files[0] as (File & { path?: string }) | undefined)?.path
  if (path) void openArchivePath(path)
}

watch(() => appStore.pendingArchiveBrowserPath, path => {
  if (path) void openArchivePath(appStore.takeArchiveBrowserPath())
}, { immediate: true })

onMounted(() => {
  if (import.meta.env.MODE === 'test') return
  window.addEventListener('keydown', handleWorkspaceKeydown)
  void Promise.all([
    listen('tauri://file-drop-hover', () => { isDraggingArchive.value = true }).then(value => { unlistenHover = value }),
    listen<string[]>('tauri://file-drop', event => {
      isDraggingArchive.value = false
      if (event.payload[0]) void openArchivePath(event.payload[0])
    }).then(value => { unlistenDrop = value }),
    listen('tauri://file-drop-cancelled', () => { isDraggingArchive.value = false }).then(value => { unlistenCancel = value })
  ]).catch(error => console.warn('Archive browser drag-and-drop is unavailable:', error))
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleWorkspaceKeydown)
  unlistenHover?.()
  unlistenDrop?.()
  unlistenCancel?.()
})

const toggleEntry = (entry: ArchiveEntryInfo) => {
  const next = new Set(selected.value)
  next.has(entry.path) ? next.delete(entry.path) : next.add(entry.path)
  selected.value = next
}

const toggleVisible = () => {
  const next = new Set(selected.value)
  visibleFiles.value.forEach(entry => visibleSelected.value ? next.delete(entry.path) : next.add(entry.path))
  selected.value = next
}

const extractSelected = async () => {
  await extractPaths([...selected.value], outputPath.value)
}
</script>

<template>
  <div
    class="browser-page relative h-full min-w-0 overflow-hidden p-responsive p-8 flex flex-col gap-3"
    @dragenter.prevent="isDraggingArchive = true"
    @dragover.prevent="isDraggingArchive = true"
    @dragleave.self="isDraggingArchive = false"
    @drop="droppedBrowserFiles"
    @keydown="handleWorkspaceKeydown"
  >
    <header class="shrink-0 flex flex-wrap items-center justify-between gap-3">
      <div class="min-w-0">
        <h1 class="browser-title font-black text-content tracking-tighter">压缩包浏览中心</h1>
        <p class="text-muted text-xs font-bold mt-1">像文件夹一样浏览压缩包，只解压真正需要的内容</p>
      </div>
      <button class="browser-primary" type="button" @click="chooseArchive">
        <i class="pi pi-folder-open"></i><span>打开压缩包</span>
      </button>
    </header>

    <section class="aero-card shrink-0 p-3 grid gap-3 browser-toolbar">
      <div class="min-w-0 browser-field">
        <span class="browser-label">当前压缩包</span>
        <button class="browser-input text-left truncate" type="button" :title="archivePath" @click="chooseArchive">{{ archiveName || '选择或拖入需要浏览的压缩包' }}</button>
      </div>
      <div class="min-w-0 browser-field">
        <span class="browser-label">密码</span>
        <div class="flex min-w-0 gap-2">
          <input v-model="password" class="browser-input min-w-0 flex-1" type="password" placeholder="留空则尝试密码保险箱" @keyup.enter="loadArchive">
          <button class="browser-icon-button" type="button" :disabled="!archivePath || loading" @click="loadArchive" title="重新读取">
            <i :class="loading ? 'pi pi-spin pi-spinner' : 'pi pi-refresh'"></i>
          </button>
        </div>
      </div>
    </section>

    <div v-if="loading" class="aero-card flex-1 min-h-0 grid place-items-center">
      <div class="text-center text-muted"><i class="pi pi-spin pi-spinner text-primary text-3xl"></i><p class="mt-4 font-bold">正在读取压缩包结构…</p></div>
    </div>

    <div v-else-if="!result" class="aero-card browser-empty flex-1 min-h-0 grid place-items-center border-dashed" @click="chooseArchive">
      <div class="max-w-md text-center px-6"><i class="pi pi-cloud-upload text-primary text-5xl"></i><h2 class="mt-5 text-xl font-black text-content">把压缩包拖到这里直接浏览</h2><p class="mt-2 text-sm text-muted leading-6">也可以点击此区域选择文件。内容只在本机读取，密码不会写入命令行。</p><span class="browser-drop-hint">支持 ZIP、7Z、RAR、TAR 等已接入格式</span></div>
    </div>

    <template v-else>
      <section class="browser-summary shrink-0" aria-label="压缩包摘要">
        <span><b>{{ result.format }}</b> 格式</span>
        <span><b>{{ result.totalFiles }}</b> 个文件</span>
        <span>展开后 <b>{{ formatBytes(result.totalUncompressedSize) }}</b></span>
        <span><i :class="result.encrypted ? 'pi pi-lock' : 'pi pi-lock-open'"></i> {{ result.encrypted ? '已加密' : '未加密' }}</span>
      </section>

      <section class="aero-card flex-1 min-h-0 min-w-0 overflow-hidden browser-workspace">
        <aside class="directory-pane min-h-0 min-w-0 overflow-y-auto overflow-x-hidden custom-scrollbar border-r border-subtle/70 p-3">
          <p class="directory-heading">目录树</p>
          <button class="directory-entry" :class="{ active: activeDirectory === '' }" type="button" @click="selectDirectory('')"><i class="pi pi-home"></i><span>全部文件</span></button>
          <div v-for="directory in visibleDirectories" :key="directory.path" class="directory-tree-row" :style="{ paddingLeft: `${directory.depth * 0.9}rem` }">
            <button v-if="directory.hasChildren" class="directory-toggle" type="button" :aria-label="expandedDirectories.has(directory.path) ? '折叠目录' : '展开目录'" @click="toggleDirectory(directory.path)">
              <i :class="expandedDirectories.has(directory.path) ? 'pi pi-chevron-down' : 'pi pi-chevron-right'"></i>
            </button>
            <span v-else class="directory-toggle-spacer"></span>
            <button class="directory-entry" :class="{ active: activeDirectory === directory.path }" type="button" :title="directory.path" @click="selectDirectory(directory.path)">
              <i :class="expandedDirectories.has(directory.path) ? 'pi pi-folder-open' : 'pi pi-folder'"></i><span class="truncate">{{ directory.name }}</span>
            </button>
          </div>
        </aside>

        <div class="min-h-0 min-w-0 overflow-hidden flex flex-col">
          <nav class="browser-navigation shrink-0 border-b border-subtle/70" aria-label="压缩包目录导航">
            <div class="browser-navigation-actions">
              <button type="button" data-testid="archive-nav-back" :disabled="!canNavigateBack" title="后退 (Alt+左箭头)" aria-label="后退" @click="goBack"><i class="pi pi-arrow-left"></i></button>
              <button type="button" data-testid="archive-nav-forward" :disabled="!canNavigateForward" title="前进 (Alt+右箭头)" aria-label="前进" @click="goForward"><i class="pi pi-arrow-right"></i></button>
              <button type="button" data-testid="archive-nav-up" :disabled="!canNavigateUp" title="上一级 (Backspace)" aria-label="上一级" @click="goUp"><i class="pi pi-arrow-up"></i></button>
              <button type="button" data-testid="archive-nav-refresh" title="刷新当前目录" aria-label="刷新当前目录" @click="refreshDirectory"><i class="pi pi-refresh"></i></button>
            </div>
            <div class="browser-breadcrumbs" data-testid="archive-breadcrumbs">
              <template v-for="(crumb, index) in breadcrumbs" :key="crumb.path || '__root__'">
                <i v-if="index > 0" class="pi pi-angle-right" aria-hidden="true"></i>
                <button type="button" :class="{ current: index === breadcrumbs.length - 1 }" :title="crumb.path || '根目录'" @click="navigateToDirectory(crumb.path)">{{ crumb.name }}</button>
              </template>
            </div>
          </nav>
          <div class="shrink-0 p-3 border-b border-subtle/70 flex flex-wrap gap-2">
            <label class="browser-search min-w-0 flex-1"><i class="pi pi-search"></i><input v-model="query" placeholder="搜索文件名或路径"></label>
            <select v-model="typeFilter" class="browser-select">
              <option value="all">全部类型</option><option value="image">图片</option><option value="document">文档</option><option value="archive">压缩包</option><option value="other">其他</option>
            </select>
          </div>
          <div class="browser-table-head shrink-0">
            <button type="button" class="browser-checkbox" :class="{ checked: visibleSelected }" @click="toggleVisible"><i v-if="visibleSelected" class="pi pi-check"></i></button>
            <span>名称与路径</span><span class="hidden md:block">大小</span><span class="hidden lg:block">修改时间</span><span class="hidden xl:block">CRC</span>
          </div>
          <div class="flex-1 min-h-0 overflow-y-auto overflow-x-hidden custom-scrollbar" @contextmenu.self="openContextMenu(null, $event)">
            <div
              v-for="entry in filteredEntries"
              :key="entry.path"
              class="browser-row"
              :class="{ focused: focusedEntryPath === entry.path, directory: entry.isDir }"
              :data-entry-path="entry.path"
              role="row"
              tabindex="0"
              @click="handleEntryClick(entry, $event)"
              @contextmenu="openContextMenu(entry, $event)"
              @dblclick="activateEntry(entry)"
              @keydown.enter.prevent="activateEntry(entry)"
            >
              <button v-if="!entry.isDir" type="button" class="browser-checkbox" :class="{ checked: selected.has(entry.path) }" :aria-label="selected.has(entry.path) ? `取消选择 ${entry.name}` : `选择 ${entry.name}`" @click.stop="focusedEntryPath = entry.path; toggleEntry(entry)"><i v-if="selected.has(entry.path)" class="pi pi-check"></i></button>
              <span v-else class="browser-directory-marker"><i class="pi pi-folder"></i></span>
              <span class="min-w-0 text-left flex items-center gap-2">
                <span class="min-w-0 flex-1"><strong class="block truncate text-content">{{ entry.name }}</strong><small class="block truncate text-muted mt-0.5">{{ entry.path }}</small></span>
                <button
                  v-if="canPreviewEntry(entry)"
                  type="button"
                  class="preview-trigger"
                  :disabled="!previewRouteSupported"
                  :title="previewRouteSupported ? `预览 ${entry.name}` : '当前仅支持 ZIP 与 TAR 系列的有界预览'"
                  :aria-label="`预览 ${entry.name}`"
                  @click.stop="openPreview(entry)"
                ><i class="pi pi-eye"></i></button>
              </span>
              <span class="hidden md:block text-left text-muted">{{ entry.isDir ? '文件夹' : formatBytes(entry.size) }}</span>
              <span class="hidden lg:block text-left text-muted truncate">{{ entry.modified || '—' }}</span>
              <span class="hidden xl:block text-left font-mono text-muted truncate">{{ entry.crc || '—' }}</span>
            </div>
            <div v-if="filteredEntries.length === 0" class="h-full grid place-items-center text-muted text-sm">没有符合条件的文件</div>
          </div>
        </div>
      </section>

      <footer class="shrink-0 flex flex-wrap items-center justify-between gap-3">
        <div class="footer-status min-w-0">
          <span class="text-sm font-bold text-muted">已选择 <strong class="text-primary">{{ selected.size }}</strong> / {{ result.totalFiles }} 个文件</span>
          <button class="output-target" type="button" :title="outputPath" @click="chooseOutput"><i class="pi pi-folder"></i><span class="truncate">解压到：{{ outputPath || '选择输出目录' }}</span><i class="pi pi-pencil"></i></button>
        </div>
        <button class="browser-primary" type="button" :disabled="selected.size === 0 || !outputPath || extracting" @click="extractSelected">
          <i :class="extracting ? 'pi pi-spin pi-spinner' : 'pi pi-download'"></i><span>{{ extracting ? '正在解压' : '解压所选文件' }}</span>
        </button>
      </footer>
    </template>

    <div v-if="isDraggingArchive" class="browser-drop-overlay" aria-live="polite">
      <div><i class="pi pi-cloud-upload"></i><strong>松开即可浏览压缩包</strong><span>将替换当前打开的压缩包</span></div>
    </div>

    <div v-if="previewEntry" class="preview-backdrop" data-testid="archive-image-preview" @click.self="closePreview">
      <section class="preview-dialog" role="dialog" aria-modal="true" :aria-label="`预览 ${previewEntry.name}`">
        <header class="preview-header">
          <div class="min-w-0"><p class="text-xs font-black tracking-widest text-primary">归档内图片预览</p><h2 class="mt-1 truncate text-lg font-black text-content">{{ previewEntry.name }}</h2></div>
          <button type="button" class="preview-close" aria-label="关闭预览" @click="closePreview"><i class="pi pi-times"></i></button>
        </header>
        <div class="preview-stage">
          <div v-if="previewLoading" class="text-center text-muted"><i class="pi pi-spin pi-spinner text-3xl text-primary"></i><p class="mt-3 text-sm font-bold">正在进行有界读取与安全检查…</p></div>
          <div v-else-if="previewError" class="max-w-md text-center"><i class="pi pi-exclamation-triangle text-3xl text-amber-500"></i><p class="mt-3 break-words text-sm font-bold text-content">无法预览</p><p class="mt-2 break-words text-xs leading-5 text-muted">{{ previewError }}</p></div>
          <img v-else-if="imagePreview" :src="imagePreview.dataUrl" :alt="previewEntry.name" class="preview-image">
        </div>
        <footer v-if="imagePreview" class="preview-meta">
          <span>{{ imagePreview.width }} × {{ imagePreview.height }}</span><span>{{ formatBytes(imagePreview.byteSize) }}</span><span>{{ imagePreview.mimeType }}</span><span>只读 · 未写入磁盘</span>
        </footer>
        <p class="preview-safety">预览仅接受经魔数确认的 PNG、JPEG、GIF、WebP、BMP；解压后最大 8 MiB、最多 1600 万像素，TAR 流最多扫描 64 MiB。SVG、截断与扩展名伪装内容不会渲染。</p>
      </section>
    </div>

    <Teleport to="body">
      <div v-if="contextMenu" class="archive-context-layer" @pointerdown.self="closeContextMenu" @contextmenu.prevent>
        <section
          class="archive-context-menu"
          data-testid="archive-context-menu"
          role="menu"
          :aria-label="contextMenu.entry ? `${contextMenu.entry.name} 操作菜单` : '文件区操作菜单'"
          :style="{ left: `${contextMenu.left}px`, top: `${contextMenu.top}px` }"
        >
          <header class="archive-context-header">
            <i :class="contextMenu.entry?.isDir ? 'pi pi-folder' : contextDisplayEntries.length > 1 ? 'pi pi-clone' : 'pi pi-file'"></i>
            <span class="min-w-0"><strong class="block truncate">{{ contextMenu.entry?.isDir ? contextMenu.entry.name : contextDisplayEntries.length > 1 ? `${contextDisplayEntries.length} 个已选文件` : contextMenu.entry?.name || '当前文件区' }}</strong><small>{{ contextMenu.entry?.isDir ? `${contextMenu.entries.length} 个文件` : contextDisplayEntries.length > 1 ? '批量操作' : '只读归档操作' }}</small></span>
          </header>

          <button v-if="contextMenu.entry?.isDir" type="button" role="menuitem" data-testid="archive-context-open" @click="openContextEntry(contextMenu.entry)"><i class="pi pi-folder-open"></i><span>打开文件夹</span><kbd>Enter</kbd></button>
          <button v-if="contextMenu.entry && !contextMenu.entry.isDir && canPreviewEntry(contextMenu.entry) && previewRouteSupported" type="button" role="menuitem" data-testid="archive-context-preview" @click="previewContextEntry(contextMenu.entry)"><i class="pi pi-eye"></i><span>内部查看器打开</span><kbd>Ctrl+Enter</kbd></button>

          <div v-if="contextMenu.entries.length > 0" class="archive-context-separator"></div>
          <button v-if="contextMenu.entries.length > 0" type="button" role="menuitem" data-testid="archive-context-extract-current" :disabled="extracting || !outputPath" @click="extractContextEntries(false)"><i class="pi pi-download"></i><span>解压到当前输出目录</span><kbd>Alt+E</kbd></button>
          <button v-if="contextMenu.entries.length > 0" type="button" role="menuitem" data-testid="archive-context-extract-choose" :disabled="extracting" @click="extractContextEntries(true)"><i class="pi pi-folder-open"></i><span>解压到指定目录…</span><kbd>Alt+Shift+E</kbd></button>

          <div v-if="contextDisplayEntries.length > 0" class="archive-context-separator"></div>
          <button v-if="contextDisplayEntries.length > 0" type="button" role="menuitem" data-testid="archive-context-copy-name" @click="copyContextText('name', contextDisplayEntries)"><i class="pi pi-copy"></i><span>复制名称</span><kbd>Ctrl+Alt+C</kbd></button>
          <button v-if="contextDisplayEntries.length > 0" type="button" role="menuitem" data-testid="archive-context-copy-path" @click="copyContextText('path', contextDisplayEntries)"><i class="pi pi-link"></i><span>复制归档内路径</span><kbd>Ctrl+Shift+C</kbd></button>
          <button v-if="contextDisplayEntries.length > 0" type="button" role="menuitem" data-testid="archive-context-details" @click="showDetails(contextDisplayEntries)"><i class="pi pi-info-circle"></i><span>显示详细信息</span><kbd>Alt+Enter</kbd></button>

          <div class="archive-context-separator"></div>
          <button type="button" role="menuitem" data-testid="archive-context-refresh" @click="closeContextMenu(); refreshDirectory()"><i class="pi pi-refresh"></i><span>刷新压缩包</span><kbd>F5</kbd></button>
        </section>
      </div>

      <div v-if="detailEntries.length > 0" class="archive-details-backdrop" data-testid="archive-entry-details" @pointerdown.self="closeDetails">
        <section class="archive-details-dialog" role="dialog" aria-modal="true" :aria-label="`${detailTitle} 详细信息`">
          <header class="archive-details-header">
            <div class="min-w-0"><p>ARCHIVE ENTRY</p><h2 class="truncate">{{ detailTitle }}</h2><span>只读元数据，不会提取或修改归档内容</span></div>
            <button type="button" aria-label="关闭条目详情" @click="closeDetails"><i class="pi pi-times"></i></button>
          </header>
          <div class="archive-details-metrics">
            <article><span>条目</span><strong>{{ detailEntries.length }}</strong></article>
            <article><span>包含文件</span><strong>{{ detailFiles.length }}</strong></article>
            <article><span>展开大小</span><strong>{{ formatBytes(detailTotalSize) }}</strong></article>
            <article><span>归档内大小</span><strong>{{ detailCompressedSize ? formatBytes(detailCompressedSize) : '未知' }}</strong></article>
          </div>
          <dl v-if="detailEntries.length === 1" class="archive-details-grid">
            <div><dt>类型</dt><dd>{{ detailEntries[0].isDir ? '文件夹' : '文件' }}</dd></div>
            <div><dt>加密标记</dt><dd>{{ detailEntries[0].encrypted ? '已加密' : '未单独标记' }}</dd></div>
            <div><dt>修改时间</dt><dd>{{ detailEntries[0].modified || '归档未提供' }}</dd></div>
            <div><dt>CRC</dt><dd class="font-mono">{{ detailEntries[0].crc || '归档未提供' }}</dd></div>
          </dl>
          <section class="archive-details-paths">
            <h3>归档内路径</h3>
            <ul><li v-for="entry in detailEntries" :key="entry.path">{{ entry.path.replace(/\/+$/, '') }}</li></ul>
          </section>
        </section>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.browser-page { max-width: 100%; }
.browser-title { font-size: clamp(1.75rem, 3vw, 2.5rem); line-height: 1; }
.browser-toolbar { grid-template-columns: minmax(0, 1fr) minmax(14rem, .42fr); }
.browser-field { display: flex; flex-direction: column; gap: .4rem; }
.browser-label { color: var(--text-muted); font-size: .7rem; font-weight: 900; letter-spacing: .12em; }
.browser-input, .browser-search, .browser-select { height: 2.75rem; border: 1px solid var(--border-subtle); border-radius: .85rem; background: var(--bg-input); color: var(--text-content); padding: 0 .9rem; outline: none; }
.browser-input:focus, .browser-search:focus-within, .browser-select:focus { border-color: var(--dynamic-accent); }
.browser-icon-button { width: 2.75rem; height: 2.75rem; border-radius: .85rem; border: 1px solid var(--border-subtle); color: var(--dynamic-accent); background: var(--bg-input); }
.browser-primary { min-height: 2.85rem; padding: 0 1.15rem; border-radius: .9rem; display: inline-flex; align-items: center; justify-content: center; gap: .55rem; color: white; font-size: .8rem; font-weight: 900; background: var(--dynamic-accent); box-shadow: 0 8px 24px color-mix(in srgb, var(--dynamic-accent) 25%, transparent); }
.browser-primary:disabled, .browser-icon-button:disabled { opacity: .45; cursor: not-allowed; }
.browser-empty { cursor: pointer; transition: border-color .2s ease, background .2s ease, transform .2s ease; }
.browser-empty:hover { border-color: var(--dynamic-accent); background: color-mix(in srgb, var(--dynamic-accent) 5%, var(--bg-card)); transform: translateY(-1px); }
.browser-drop-hint { display: inline-flex; margin-top: 1rem; padding: .45rem .75rem; border-radius: 999px; background: color-mix(in srgb, var(--dynamic-accent) 10%, transparent); color: var(--dynamic-accent); font-size: .7rem; font-weight: 800; }
.browser-summary { display: flex; align-items: center; flex-wrap: wrap; gap: .45rem; min-height: 2.25rem; }
.browser-summary span { display: inline-flex; align-items: center; gap: .35rem; padding: .38rem .65rem; border: 1px solid var(--border-subtle); border-radius: 999px; background: color-mix(in srgb, var(--bg-card) 82%, transparent); color: var(--text-muted); font-size: .68rem; font-weight: 800; }
.browser-summary b { color: var(--text-content); }
.browser-workspace { display: grid; grid-template-columns: minmax(10rem, 15rem) minmax(0, 1fr); }
.directory-heading { padding: .2rem .75rem .5rem; color: var(--text-muted); font-size: .64rem; font-weight: 900; letter-spacing: .12em; }
.directory-tree-row { display: flex; min-width: 0; align-items: center; }
.directory-toggle, .directory-toggle-spacer { flex: 0 0 1.35rem; width: 1.35rem; height: 2.15rem; display: grid; place-items: center; color: var(--text-muted); font-size: .58rem; }
.directory-toggle:hover { color: var(--dynamic-accent); }
.directory-entry { flex: 1; width: 100%; min-width: 0; display: flex; align-items: center; gap: .55rem; border-radius: .7rem; padding: .56rem .65rem; color: var(--text-muted); font-size: .74rem; font-weight: 800; text-align: left; }
.directory-entry:hover, .directory-entry.active { background: color-mix(in srgb, var(--dynamic-accent) 13%, transparent); color: var(--dynamic-accent); }
.browser-navigation { min-width: 0; display: flex; align-items: center; gap: .7rem; padding: .55rem .75rem; }
.browser-navigation-actions { flex: 0 0 auto; display: flex; gap: .25rem; }
.browser-navigation-actions button { width: 2rem; height: 2rem; display: grid; place-items: center; border-radius: .6rem; color: var(--text-muted); font-size: .7rem; }
.browser-navigation-actions button:hover:not(:disabled) { color: var(--dynamic-accent); background: color-mix(in srgb, var(--dynamic-accent) 10%, transparent); }
.browser-navigation-actions button:disabled { opacity: .3; cursor: not-allowed; }
.browser-breadcrumbs { min-width: 0; display: flex; align-items: center; gap: .2rem; overflow-x: auto; overflow-y: hidden; scrollbar-width: none; }
.browser-breadcrumbs::-webkit-scrollbar { display: none; }
.browser-breadcrumbs > i { flex: 0 0 auto; color: var(--text-muted); font-size: .55rem; }
.browser-breadcrumbs button { flex: 0 0 auto; max-width: 12rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; border-radius: .55rem; padding: .38rem .5rem; color: var(--text-muted); font-size: .7rem; font-weight: 800; }
.browser-breadcrumbs button:hover, .browser-breadcrumbs button.current { color: var(--dynamic-accent); background: color-mix(in srgb, var(--dynamic-accent) 9%, transparent); }
.browser-search { display: flex; align-items: center; gap: .6rem; padding-inline: .8rem; }
.browser-search input { width: 100%; min-width: 0; background: transparent; outline: none; }
.browser-select { min-width: 8.5rem; }
.browser-table-head, .browser-row { min-width: 0; display: grid; grid-template-columns: 1.5rem minmax(10rem, 1fr) minmax(5rem, .25fr) minmax(8rem, .42fr) minmax(5rem, .25fr); align-items: center; gap: .75rem; padding: .7rem 1rem; }
.browser-table-head { color: var(--text-muted); font-size: .68rem; font-weight: 900; border-bottom: 1px solid var(--border-subtle); }
.browser-row { width: 100%; border-bottom: 1px solid color-mix(in srgb, var(--border-subtle) 60%, transparent); font-size: .75rem; }
.browser-row:hover { background: color-mix(in srgb, var(--dynamic-accent) 7%, transparent); }
.browser-row.focused { outline: 1px solid color-mix(in srgb, var(--dynamic-accent) 68%, transparent); outline-offset: -2px; background: color-mix(in srgb, var(--dynamic-accent) 11%, transparent); }
.browser-row:focus-visible { outline: 2px solid var(--dynamic-accent); outline-offset: -2px; }
.browser-row.directory strong { color: var(--dynamic-accent); }
.browser-directory-marker { width: 1.15rem; height: 1.15rem; display: inline-grid; place-items: center; color: var(--dynamic-accent); font-size: .8rem; }
.preview-trigger { flex: 0 0 auto; width: 2rem; height: 2rem; display: grid; place-items: center; border-radius: .65rem; color: var(--dynamic-accent); background: color-mix(in srgb, var(--dynamic-accent) 10%, transparent); }
.preview-trigger:hover { background: color-mix(in srgb, var(--dynamic-accent) 18%, transparent); }
.preview-trigger:disabled { color: var(--text-muted); background: var(--bg-input); opacity: .45; cursor: not-allowed; }
.browser-checkbox { width: 1.15rem; height: 1.15rem; border: 1px solid var(--border-subtle); border-radius: .35rem; display: inline-grid; place-items: center; color: white; font-size: .55rem; }
.browser-checkbox.checked { background: var(--dynamic-accent); border-color: var(--dynamic-accent); }
.footer-status { display: flex; flex-wrap: wrap; align-items: center; gap: .75rem 1rem; }
.output-target { min-width: 0; max-width: min(36rem, 55vw); display: inline-flex; align-items: center; gap: .45rem; padding: .45rem .7rem; border: 1px solid var(--border-subtle); border-radius: .7rem; color: var(--text-muted); background: var(--bg-input); font-size: .7rem; font-weight: 800; }
.output-target:hover { border-color: var(--dynamic-accent); color: var(--dynamic-accent); }
.browser-drop-overlay { position: absolute; inset: .75rem; z-index: 45; display: grid; place-items: center; border: 2px dashed var(--dynamic-accent); border-radius: 1.5rem; background: color-mix(in srgb, var(--bg-card) 88%, transparent); backdrop-filter: blur(16px); pointer-events: none; }
.browser-drop-overlay > div { display: flex; flex-direction: column; align-items: center; gap: .65rem; color: var(--text-muted); }
.browser-drop-overlay i { color: var(--dynamic-accent); font-size: 3rem; }
.browser-drop-overlay strong { color: var(--text-content); font-size: 1.15rem; }
.browser-drop-overlay span { font-size: .75rem; font-weight: 700; }
.preview-backdrop { position: fixed; inset: 0; z-index: 50; display: grid; place-items: center; min-width: 0; padding: clamp(.75rem, 3vw, 2rem); background: color-mix(in srgb, #08141f 58%, transparent); backdrop-filter: blur(14px); overflow-x: hidden; }
.preview-dialog { width: min(52rem, 100%); max-height: 100%; min-width: 0; overflow-x: hidden; overflow-y: auto; border: 1px solid color-mix(in srgb, var(--dynamic-accent) 24%, var(--border-subtle)); border-radius: 1.5rem; background: color-mix(in srgb, var(--bg-card) 94%, transparent); box-shadow: 0 28px 80px rgba(0, 0, 0, .34); }
.preview-header { min-width: 0; display: flex; align-items: center; justify-content: space-between; gap: 1rem; padding: 1rem 1.25rem; border-bottom: 1px solid var(--border-subtle); }
.preview-close { flex: 0 0 auto; width: 2.4rem; height: 2.4rem; display: grid; place-items: center; border-radius: .75rem; color: var(--text-muted); background: var(--bg-input); }
.preview-stage { min-height: min(54vh, 30rem); display: grid; place-items: center; padding: clamp(1rem, 3vw, 2rem); background: radial-gradient(circle at 50% 42%, color-mix(in srgb, var(--dynamic-accent) 12%, transparent), transparent 65%); overflow: hidden; }
.preview-image { display: block; max-width: 100%; max-height: min(54vh, 30rem); object-fit: contain; border-radius: 1rem; box-shadow: 0 18px 50px rgba(0, 0, 0, .25); }
.preview-meta { display: flex; flex-wrap: wrap; gap: .5rem; padding: .9rem 1.25rem 0; }
.preview-meta span { border-radius: 999px; padding: .35rem .65rem; background: var(--bg-input); color: var(--text-muted); font-size: .68rem; font-weight: 800; }
.preview-safety { padding: .8rem 1.25rem 1.15rem; color: var(--text-muted); font-size: .68rem; line-height: 1.55; }
.archive-context-layer { position: fixed; inset: 0; z-index: 80; }
.archive-context-menu { position: fixed; width: min(17rem, calc(100vw - 1rem)); max-height: calc(100vh - 1rem); overflow-y: auto; overflow-x: hidden; padding: .45rem; border: 1px solid color-mix(in srgb, var(--dynamic-accent) 25%, var(--border-subtle)); border-radius: 1rem; background: color-mix(in srgb, var(--bg-modal) 96%, transparent); box-shadow: 0 20px 60px rgba(5, 18, 28, .28); backdrop-filter: blur(20px); }
.archive-context-header { min-width: 0; display: flex; align-items: center; gap: .7rem; padding: .65rem .7rem .75rem; color: var(--dynamic-accent); }
.archive-context-header > i { flex: 0 0 auto; font-size: 1rem; }
.archive-context-header strong { color: var(--text-content); font-size: .76rem; }
.archive-context-header small { display: block; margin-top: .15rem; color: var(--text-muted); font-size: .62rem; font-weight: 700; }
.archive-context-menu > button { width: 100%; min-width: 0; display: grid; grid-template-columns: 1.2rem minmax(0, 1fr) auto; align-items: center; gap: .55rem; padding: .58rem .65rem; border-radius: .65rem; color: var(--text-content); text-align: left; font-size: .7rem; font-weight: 800; }
.archive-context-menu > button:hover:not(:disabled), .archive-context-menu > button:focus-visible { color: var(--dynamic-accent); background: color-mix(in srgb, var(--dynamic-accent) 11%, transparent); outline: none; }
.archive-context-menu > button:disabled { opacity: .4; cursor: not-allowed; }
.archive-context-menu kbd { color: var(--text-muted); font-family: inherit; font-size: .56rem; font-weight: 700; white-space: nowrap; }
.archive-context-separator { height: 1px; margin: .35rem .45rem; background: var(--border-subtle); }
.archive-details-backdrop { position: fixed; inset: 0; z-index: 85; display: grid; place-items: center; min-width: 0; padding: clamp(.75rem, 3vw, 2rem); background: color-mix(in srgb, #08141f 58%, transparent); backdrop-filter: blur(14px); overflow: hidden; }
.archive-details-dialog { width: min(46rem, 100%); max-height: 100%; min-width: 0; overflow-y: auto; overflow-x: hidden; border: 1px solid color-mix(in srgb, var(--dynamic-accent) 25%, var(--border-subtle)); border-radius: 1.5rem; background: var(--bg-modal); box-shadow: 0 28px 80px rgba(0, 0, 0, .34); }
.archive-details-header { min-width: 0; display: flex; align-items: center; justify-content: space-between; gap: 1rem; padding: 1.25rem; border-bottom: 1px solid var(--border-subtle); background: radial-gradient(circle at 90% 0, color-mix(in srgb, var(--dynamic-accent) 13%, transparent), transparent 48%); }
.archive-details-header p { color: var(--dynamic-accent); font-size: .62rem; font-weight: 900; letter-spacing: .2em; }
.archive-details-header h2 { margin-top: .3rem; color: var(--text-content); font-size: 1.2rem; font-weight: 900; }
.archive-details-header span { display: block; margin-top: .2rem; color: var(--text-muted); font-size: .68rem; font-weight: 700; }
.archive-details-header button { flex: 0 0 auto; width: 2.5rem; height: 2.5rem; display: grid; place-items: center; border-radius: .8rem; color: var(--text-muted); background: var(--bg-input); }
.archive-details-metrics { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: .7rem; padding: 1rem 1.25rem 0; }
.archive-details-metrics article { min-width: 0; padding: .8rem; border: 1px solid var(--border-subtle); border-radius: .9rem; background: var(--bg-input); }
.archive-details-metrics span, .archive-details-grid dt { display: block; color: var(--text-muted); font-size: .62rem; font-weight: 800; }
.archive-details-metrics strong { display: block; margin-top: .3rem; overflow: hidden; color: var(--text-content); font-size: .82rem; text-overflow: ellipsis; white-space: nowrap; }
.archive-details-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: .6rem 1rem; padding: 1rem 1.25rem 0; }
.archive-details-grid > div { min-width: 0; padding: .65rem .8rem; border-bottom: 1px solid var(--border-subtle); }
.archive-details-grid dd { margin-top: .25rem; overflow: hidden; color: var(--text-content); font-size: .7rem; font-weight: 800; text-overflow: ellipsis; white-space: nowrap; }
.archive-details-paths { padding: 1rem 1.25rem 1.25rem; }
.archive-details-paths h3 { color: var(--text-muted); font-size: .65rem; font-weight: 900; letter-spacing: .12em; }
.archive-details-paths ul { max-height: 12rem; margin-top: .55rem; overflow-y: auto; overflow-x: hidden; border: 1px solid var(--border-subtle); border-radius: .85rem; background: var(--bg-input); }
.archive-details-paths li { overflow-wrap: anywhere; padding: .55rem .7rem; border-bottom: 1px solid color-mix(in srgb, var(--border-subtle) 65%, transparent); color: var(--text-content); font-family: ui-monospace, SFMono-Regular, Consolas, monospace; font-size: .66rem; }
.archive-details-paths li:last-child { border-bottom: 0; }
@media (max-width: 1050px) { .browser-table-head, .browser-row { grid-template-columns: 1.5rem minmax(8rem, 1fr) minmax(5rem, .28fr) minmax(7rem, .4fr); } }
@media (max-width: 760px) { .browser-page { padding: 1rem; overflow-y: auto; overflow-x: hidden; } .browser-workspace { flex: 0 0 34rem; min-height: 34rem; grid-template-columns: 1fr; grid-template-rows: minmax(7rem, 10rem) minmax(22rem, 1fr); } .browser-workspace aside { border-right: 0; border-bottom: 1px solid var(--border-subtle); } .browser-toolbar { grid-template-columns: 1fr; } .browser-table-head, .browser-row { grid-template-columns: 1.5rem minmax(0, 1fr); } .output-target { max-width: 80vw; } .archive-details-metrics { grid-template-columns: repeat(2, minmax(0, 1fr)); } .archive-details-grid { grid-template-columns: 1fr; } }
</style>
