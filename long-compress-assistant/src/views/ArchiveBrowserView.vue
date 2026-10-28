<script setup lang="ts">
import type { ArchivePaneSource } from '@/types/fileBrowser'
import FilePaneNavigation from '@/components/file-manager/FilePaneNavigation.vue'
import FilePaneList, { type FilePaneEntry } from '@/components/file-manager/FilePaneList.vue'
import { browserBreadcrumbs } from '@/utils/browserPath'
import type { useSavedLocations } from '@/composables/useSavedLocations'
import { parentDirectory as diskParentDirectory } from '@/utils/resultLocation'
import {prepareZpaqSource,type ZpaqGroup} from '@/utils/zpaqMultipart'
import ArchiveImageReader from '@/components/archive/ArchiveImageReader.vue'
import {readerError} from '@/composables/useArchiveImageReader'
import ReaderRetrievalDialog from '@/components/archive/ReaderRetrievalDialog.vue'
import {useArchiveReadingStore} from '@/stores/archiveReading'
import type {ArchiveReadingRecord,ReaderSelection} from '@/types/archiveReading'
import { useArchiveLibraryStore } from '@/stores/archiveLibrary'
import { invoke } from '@tauri-apps/api/tauri'
import ZipEditBar from '@/components/archive/ZipEditBar.vue'
import { useZipEditStore } from '@/stores/zipEdit'
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { useAppStore, type ArchiveReadOptions } from '@/stores/app'
import ZipPasswordEncodingField from '@/components/ui/ZipPasswordEncodingField.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
import { zipPasswordEncodingArguments, type ZipPasswordEncoding } from '@/types/zipPassword'
import { supportsZipPasswordEncoding, zipPasswordEncodingForSource } from '@/utils/zipPasswordSource'
import { useTauriCommands, type ArchiveBrowseResult, type ArchiveEntryInfo, type ArchiveImagePreview, type ArchiveTextPreview } from '@/composables/useTauriCommands'
import { useArchiveBrowserCapabilities } from '@/composables/useArchiveBrowserCapabilities'
import { describeArchiveBrowseError, useArchiveBrowseRequest } from '@/composables/useArchiveBrowseRequest'
import { useArchiveWorkspaceNavigation } from '@/composables/useArchiveWorkspaceNavigation'
import ArchiveDirectoryPane from '@/components/archive/ArchiveDirectoryPane.vue'
import ZpaqSnapshotPicker from '@/components/archive/ZpaqSnapshotPicker.vue'
import ZpaqRecoveryDialog from '@/components/archive/ZpaqRecoveryDialog.vue'
import ZpaqSnapshotDiff from '@/components/archive/ZpaqSnapshotDiff.vue'
import { type ZpaqSnapshot } from '@/types/zpaqSnapshot'
import { useArchiveConversionStore } from '@/stores/archiveConversion'

const props = withDefaults(defineProps<{ embedded?: boolean; source?: ArchivePaneSource; destination?: string; active?: boolean; paneId?:string; locations?:Array<{name:string;path:string;kind:string}>; savedLocations?:ReturnType<typeof useSavedLocations> }>(), { embedded: false, active: true, paneId:'left', locations:()=>[] })
const emit = defineEmits<{ leave: []; 'navigate-disk':[path:string]; extracted: [destination: string]; 'pane-state': [state: { busy: boolean; selected: number }] }>()
const browserRoot = ref<HTMLElement | null>(null)
const paneToolsOpen = ref(false), sourceError = ref(''), customOutput = ref(false)
function closePaneTools(restoreFocus=false){paneToolsOpen.value=false;if(restoreFocus)browserRoot.value?.querySelector<HTMLButtonElement>('[data-testid="archive-pane-tools"]')?.focus()}
function dismissPaneTools(event:Event){
  const target=event.target as Element|null
  if(props.embedded&&paneToolsOpen.value&&!target?.closest('[data-testid="archive-pane-tools"]')&&!browserRoot.value?.querySelector('.archive-options')?.contains(target))closePaneTools()
}
let disposed = false, sourceSequence = 0
const leavePane = () => {
  if (extracting.value || zipEditBusy.value || openingEntryPath.value) return
  browseRequest.cancel(false); archiveLoadSequence++; emit('leave')
}

interface ArchiveWorkspaceFrame {
  archivePath: string
  displayName: string
  password: string
  zipPasswordEncoding?: ZipPasswordEncoding
  outputPath: string
  result: ArchiveBrowseResult
  selected: string[]
  paneSelected: string[]
  paneSelectionMode: boolean
  query: string
  typeFilter: string
  activeDirectory: string
  focusedEntryPath: string
  navigationBack: string[]
  navigationForward: string[]
  expandedDirectories: string[]
  contentSha256: string
}

const appStore = useAppStore()
const commands = useTauriCommands()
const archiveCapabilities = useArchiveBrowserCapabilities()
const capabilitiesReady = archiveCapabilities.refresh()
const browseRequest = useArchiveBrowseRequest(commands.browseArchive, commands.cancelArchiveBrowse)
const archivePath = ref('')
const nativeZpaqGroup=ref<ZpaqGroup|null>(null)
const librarySourceId = ref('')
const libraryEntryPath = ref('')
const outputPath = ref('')
const password = ref('')
const zipPasswordEncoding = ref<ZipPasswordEncoding>()
const usesPeerOutput = computed(() => props.embedded && !customOutput.value && !!props.destination)
const inputEncoding = computed(() => result.value
  ? zipPasswordEncodingForSource(archivePath.value, zipPasswordEncoding.value, result.value.format) : zipPasswordEncoding.value)
const result = ref<ArchiveBrowseResult | null>(null)
const requestedSnapshot = ref<ZpaqSnapshot>()
const selectedSnapshot = computed(() => result.value?.zpaqHistory?.selected || requestedSnapshot.value)
const snapshotReadArguments = computed((): [ZipPasswordEncoding?, ZpaqSnapshot?] => selectedSnapshot.value
  ? [inputEncoding.value, selectedSnapshot.value] : zipPasswordEncodingArguments(inputEncoding.value))
const conversion = useArchiveConversionStore()
const zipEditing = useZipEditStore()
const zipEditor = ref<InstanceType<typeof ZipEditBar> | null>(null)
const canEditZip = computed(() => (['ZIP','ZIPX'].includes(result.value?.format || '') || !!zipEditing.sessionFor(archivePath.value)?.editing) && !parentFrames.value.length)
const editingZip = computed(() => canEditZip.value && zipEditing.sessionFor(archivePath.value)?.editing)
const zipEditBusy = computed(() => !!zipEditing.sessionFor(archivePath.value)?.busy)
const selected = ref(new Set<string>())
const paneSelection = ref(new Set<string>()), paneSelectionMode = ref(false)
const query = ref('')
const typeFilter = ref('all')
let selectionAnchorPath = ''
const navigation = useArchiveWorkspaceNavigation(result, () => { selectionAnchorPath = ''; if(props.embedded){paneSelection.value=new Set();selected.value=new Set()} })
const {
  activeDirectory, focusedEntryPath, navigationBack, navigationForward, expandedDirectories,
  directories, visibleDirectories, directoryEntries, canNavigateBack, canNavigateForward,
  canNavigateUp, breadcrumbs, navigateToDirectory, selectDirectory, goBack, goForward, goUp,
  toggleDirectory,
} = navigation
const loading = browseRequest.loading
const extracting = ref(false)
const checkingSnapshot = ref(false)
const diffVisible = ref(false)
const recoveryVisible = ref(false)
const openingEntryPath = ref('')
const inputBusy = computed(() => zipEditBusy.value || loading.value || extracting.value || checkingSnapshot.value || !!openingEntryPath.value)
const dangerousOpenEntry = ref<ArchiveEntryInfo | null>(null)
const reading=useArchiveReadingStore()
const readerOpen=ref(false),readerInitialPath=ref(''),readerResume=ref<ArchiveReadingRecord>(),readerCollectPaths=ref<string[]|null>(null),showAllReading=ref(false)
const readingSource=computed(()=>result.value?.sourcePath||archivePath.value)
const currentReadingRecord=computed(()=>!parentFrames.value.length?reading.find(readingSource.value,selectedSnapshot.value):undefined)
let readerReturnFocus:HTMLElement|null=null
const readerCandidates=computed(()=>files.value.filter(entry=>archiveCapabilities.entryCategory(entry.name)==='image'))
function openImageReader(entry?:ArchiveEntryInfo,resume?:ArchiveReadingRecord){
 const first=entry||readerCandidates.value.find(entry=>entryParent(entry)===activeDirectory.value)||readerCandidates.value[0]
 if(!first&&!resume)return;readerReturnFocus=document.activeElement as HTMLElement;readerInitialPath.value=resume?.path||first!.path;readerResume.value=resume;closePreview();readerOpen.value=true
}
async function closeImageReader(){if(!readerOpen.value)return;readerOpen.value=false;const target=readerReturnFocus;readerReturnFocus=null;await nextTick();if(target?.isConnected)target.focus({preventScroll:true})}
async function locateReaderImage(path:string){await closeImageReader();query.value='';typeFilter.value='all';const normalized=path.replace(/\\/g,'/'),slash=normalized.lastIndexOf('/');navigateToDirectory(slash<0?'':normalized.slice(0,slash));focusedEntryPath.value=path;await nextTick();const row=Array.from(browserRoot.value?.querySelectorAll<HTMLElement>('[data-entry-path]') || []).find(row=>row.dataset.entryPath===path);row?.scrollIntoView?.({block:'nearest'});row?.focus({preventScroll:true})}
async function extractReaderImage(path:string){const selection:ReaderSelection={revision:result.value?.revision||'',paths:[path],keepStructure:true,conflictPolicy:'rename'};await closeImageReader();const target=await requestOutputDirectory(outputPath.value);if(target)await extractPaths([path],target,selection)}
async function refreshReaderSource(){await closeImageReader();await refreshDirectory()}
async function collectReaderImages(paths:string[],position?:ArchiveReadingRecord){readerResume.value=position;readerCollectPaths.value=[...paths];await closeImageReader()}
function closeReaderCollection(resume=true){readerCollectPaths.value=null;if(resume)openImageReader(undefined,currentReadingRecord.value||readerResume.value)}
async function continueCurrentReading(){const row=currentReadingRecord.value;if(row&&await refreshDirectory())openImageReader(undefined,row)}
async function continueReading(row:ArchiveReadingRecord){await openArchivePath(row.filePath,{focusEntryPath:row.path,entryAction:'read',resumeReading:true,zpaqSnapshot:row.snapshot,zipPasswordEncoding:row.encoding})}
const imagePreview = ref<ArchiveImagePreview | null>(null)
const textPreview = ref<ArchiveTextPreview | null>(null)
const previewEntry = ref<ArchiveEntryInfo | null>(null)
const previewLoading = ref(false)
const previewError = ref('')
const parentFrames = ref<ArchiveWorkspaceFrame[]>([])
const currentContentSha256 = ref('')
const nestedLoadError = ref('')
const nestedLoadRevision = ref(0)
const browseNotice = browseRequest.notice
const isDraggingArchive = ref(false)
const contextMenu = ref<{
  entry: ArchiveEntryInfo | null
  entries: ArchiveEntryInfo[]
  left: number
  top: number
} | null>(null)
const detailEntries = ref<ArchiveEntryInfo[]>([])
let previewSequence = 0
let archiveLoadSequence = 0
let unlistenDrop: (() => void) | null = null
let unlistenHover: (() => void) | null = null
let unlistenCancel: (() => void) | null = null

const files = computed(() => result.value?.entries.filter(entry => !entry.isDir) ?? [])
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
    if (typeFilter.value === 'other') {
      return archiveCapabilities.entryCategory(entry.name) === 'other'
    }
    return archiveCapabilities.entryCategory(entry.name) === typeFilter.value
  }).sort((left, right) => Number(right.isDir) - Number(left.isDir) || left.name.localeCompare(right.name))
})

const visibleFiles = computed(() => filteredEntries.value.filter(entry => !entry.isDir))
const visibleSelected = computed(() => visibleFiles.value.length > 0 && visibleFiles.value.every(entry => selected.value.has(entry.path)))
const previewRouteSupported = computed(() => archiveCapabilities.supportsBoundedPreview(result.value?.format))
const entryPreviewSupported = (entry: ArchiveEntryInfo) => previewKind(entry) === 'text' ? archiveCapabilities.supportsTextPreview(result.value?.format) : previewRouteSupported.value
let entryPreviewRequestId: string | undefined
const cancelEntryPreview = () => { if (entryPreviewRequestId) { void commands.cancelArchiveBrowse(entryPreviewRequestId).catch(() => {}); entryPreviewRequestId = undefined } }
const detailFiles = computed(() => detailEntries.value.flatMap(entry => entry.isDir
  ? files.value.filter(file => file.path.replace(/\\/g, '/').startsWith(entry.path.replace(/\\/g, '/')))
  : [entry]))
const detailTotalSize = computed(() => detailFiles.value.reduce((sum, entry) => sum + entry.size, 0))
const detailCompressedSize = computed(() => detailFiles.value.reduce((sum, entry) => sum + (entry.compressedSize ?? 0), 0))
const detailTitle = computed(() => detailEntries.value.length === 1 ? detailEntries.value[0].name : `${detailEntries.value.length} 个条目`)
const contextHasLink = computed(() => !!contextMenu.value?.entries.some(entry=>entry.linkTarget))
const contextDisplayEntries = computed(() => {
  if (!contextMenu.value) return []
  if (contextMenu.value.entry?.isDir) return [contextMenu.value.entry]
  return contextMenu.value.entries
})

const previewKind = (entry: ArchiveEntryInfo): 'image' | 'text' | null => {
  if (entry.isDir) return null
  return archiveCapabilities.previewKind(entry.name)
}
const canPreviewEntry = (entry: ArchiveEntryInfo) => previewKind(entry) !== null
const isNestedArchiveEntry = (entry: ArchiveEntryInfo) => {
  if (entry.isDir) return false
  return archiveCapabilities.isNestedArchiveName(entry.name)
}

const formatBytes = (value: number) => {
  if (value < 1024) return `${value} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let size = value / 1024
  let index = 0
  while (size >= 1024 && index < units.length - 1) { size /= 1024; index++ }
  return `${size.toFixed(size >= 100 ? 0 : size >= 10 ? 1 : 2)} ${units[index]}`
}

const parentDirectory = diskParentDirectory

const archiveName = computed(() => archivePath.value.split(/[\\/]/).pop() || archivePath.value)
const archiveChain = computed(() => [
  ...parentFrames.value.map((frame, index) => ({ name: frame.displayName, index, current: false })),
  { name: archiveName.value, index: parentFrames.value.length, current: true },
])
const archiveDepth = computed(() => parentFrames.value.length + 1)

const cancelArchiveLoad = () => {
  if (!loading.value) return
  browseRequest.cancel()
  archiveLoadSequence++
}

const openArchivePath = async (path: string, options: ArchiveReadOptions = {}) => {
  if (!path || inputBusy.value) return
  const opening = ++sourceSequence
  sourceError.value = ''; customOutput.value = false
  try {
    if (options.librarySourceId) await invoke('library_check', { sourceId: options.librarySourceId, entryPath: options.focusEntryPath || null })
    librarySourceId.value = options.librarySourceId || ''
    libraryEntryPath.value = options.focusEntryPath || ''
    readerCollectPaths.value=null
    path=await prepareZpaqSource(path)
    if (disposed || opening !== sourceSequence) return
    nativeZpaqGroup.value=/\.zpaq$/i.test(path)?await invoke<ZpaqGroup|null>('zpaq_multipart_details',{path}):null
    if (disposed || opening !== sourceSequence) return
    archivePath.value = path
    password.value = options.password || ''
    zipPasswordEncoding.value = options.zipPasswordEncoding
    requestedSnapshot.value = options.zpaqSnapshot
    parentFrames.value = []
    currentContentSha256.value = ''
    nestedLoadError.value = ''
    nestedLoadRevision.value = 0
    browseRequest.clearNotice()
    outputPath.value = props.embedded ? props.destination || '' : parentDirectory(nativeZpaqGroup.value?.members[0]?.path||path)
    query.value = ''
    typeFilter.value = 'all'
    await loadArchive()
    if (disposed || opening !== sourceSequence) return
    if (options.focusEntryPath && result.value) {
      const entry = result.value.entries.find(row => row.path === options.focusEntryPath && !row.isDir)
      if (!entry && (options.entryAction!=='read'||!options.resumeReading)) { selected.value = new Set(); throw new Error('当前归档已找不到所选条目，请返回资源库刷新索引') }
      selected.value = new Set(entry?[entry.path]:[])
      if(props.embedded)paneSelection.value=new Set(selected.value)
      if(entry){navigateToDirectory(entryParent(entry));focusedEntryPath.value=entry.path}
      if(options.entryAction==='read'){
        await capabilitiesReady
        const saved=options.resumeReading?reading.find(readingSource.value,selectedSnapshot.value):undefined
        openImageReader(entry,saved)
      }else if (options.entryAction === 'preview' && entry) {
        await capabilitiesReady
        if (canPreviewEntry(entry) && entryPreviewSupported(entry)) await openPreview(entry)
        else browseNotice.value = '已定位所选文件；此类型暂不提供内部预览，可提取后打开'
      } else if (options.entryAction === 'extract' && entry) {
        const destination = await requestOutputDirectory(outputPath.value)
        if (destination) { outputPath.value = destination; await extractPaths([entry.path], destination) }
      }
    }
  } catch (error) {
    if (disposed || opening !== sourceSequence) return
    sourceError.value = String(error)
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

let lastBrowseError = ''
const loadArchive = async (snapshot = requestedSnapshot.value, selectAll = !props.embedded) => {
  if (!archivePath.value || loading.value) return
  const sequence = ++archiveLoadSequence
  browseRequest.clearNotice()
  sourceError.value = ''
  result.value = null
  selected.value = new Set()
  navigation.reset()
  closePreview()
  try {
    const loaded = await browseRequest.run(archivePath.value, password.value, inputEncoding.value, snapshot)
    if (sequence !== archiveLoadSequence) return
    if (lastBrowseError && appStore.error === lastBrowseError) appStore.clearError()
    lastBrowseError = ''
    result.value = loaded
    if (props.embedded && loaded.zpaqHistory?.interruptedTail) paneToolsOpen.value = false
    requestedSnapshot.value = loaded.zpaqHistory?.selected
    selected.value = new Set(selectAll && !props.embedded ? loaded.entries.filter(entry => !entry.isDir).map(entry => entry.path) : [])
    paneSelection.value = new Set()
    nestedLoadError.value = ''
    nestedLoadRevision.value = 0
  } catch (error) {
    if (sequence !== archiveLoadSequence) return
    const message = describeArchiveBrowseError(error)
    if (parentFrames.value.length > 0) {
      nestedLoadError.value = message
      nestedLoadRevision.value++
    }
    else { lastBrowseError = message; sourceError.value = message; if (props.embedded) paneToolsOpen.value = true; appStore.setError(message) }
  } finally {
    if (sequence === archiveLoadSequence) loading.value = false
  }
}

const changeSnapshot = async (snapshot: ZpaqSnapshot) => {
  if (inputBusy.value) return
  requestedSnapshot.value = { ...snapshot }; query.value = ''; typeFilter.value = 'all'
  closeContextMenu(); detailEntries.value = []; dangerousOpenEntry.value = null
  await loadArchive(snapshot, false)
}
const checkSnapshot = async () => {
  if (inputBusy.value || !selectedSnapshot.value) return
  checkingSnapshot.value = true
  try { await commands.testArchiveIntegrity(archivePath.value, password.value, ...snapshotReadArguments.value); appStore.setSuccess(`快照 ${selectedSnapshot.value.version} 内容检查通过`) }
  catch (error) { appStore.setError(String(error)) }
  finally { checkingSnapshot.value = false }
}
const convertSnapshot = () => {
  if (selectedSnapshot.value && !inputBusy.value) conversion.openSnapshot(archivePath.value, selectedSnapshot.value, password.value)
}
const closePreview = () => {
  cancelEntryPreview()
  readerOpen.value=false
  previewSequence++
  imagePreview.value = null
  textPreview.value = null
  previewEntry.value = null
  previewLoading.value = false
  previewError.value = ''
}

const openPreview = async (entry: ArchiveEntryInfo) => {
  if (!canPreviewEntry(entry) || !entryPreviewSupported(entry)) return
  if (props.embedded) paneToolsOpen.value = false
  cancelEntryPreview()
  const sequence = ++previewSequence
  previewEntry.value = entry
  imagePreview.value = null
  textPreview.value = null
  previewError.value = ''
  previewLoading.value = true
  try {
    if (librarySourceId.value) await invoke('library_check', { sourceId: librarySourceId.value, entryPath: entry.path })
    const context = ['ALZ','EGG','7Z','TAR.LZO'].includes(result.value?.format.toUpperCase() || '')
      ? { previewId: result.value?.format.toLowerCase()+'-'+previewKind(entry)+'-'+crypto.randomUUID(), sourceRevision: result.value?.revision } : undefined
    entryPreviewRequestId = context?.previewId
    if (previewKind(entry) === 'image') {
      const value = context
        ? await commands.previewArchiveImage(archivePath.value, entry.path, password.value, inputEncoding.value, selectedSnapshot.value, context)
        : await commands.previewArchiveImage(archivePath.value, entry.path, password.value, ...snapshotReadArguments.value)
      if (sequence === previewSequence) imagePreview.value = value
    } else {
      const value = context
        ? await commands.previewArchiveText(archivePath.value, entry.path, password.value, inputEncoding.value, selectedSnapshot.value, context)
        : await commands.previewArchiveText(archivePath.value, entry.path, password.value, ...snapshotReadArguments.value)
      if (sequence === previewSequence) textPreview.value = value
    }
  } catch (error) {
    if (sequence === previewSequence) {
      const raw = String(error).replace(/^Error:\s*/i, '').trim()
      previewError.value = /ALZ|EGG/.test(result.value?.format || '') && raw.includes('文本前缀读取超过 64 MiB')
        ? '读取这段文本需要扫描超过 64 MiB，已停止预览。请先提取后打开。' : raw.includes('IMAGE_READER_CHANGED|')
        ? '来源已变化或无法确认，请关闭预览并刷新此栏后重试。' : /^provided password is incorrect$/i.test(raw)
        ? '密码不正确，请修改密码后重试。' : ['EGG','7Z'].includes(result.value?.format || '')
        ? (/appears to be binary|supported text encoding/.test(raw) ? '这份文件不是可识别的文本，请先提取后打开。' : readerError(raw.replace('[sevenz-preview:password] ',''))) : String(error)
    }
  } finally {
    if (sequence === previewSequence) { previewLoading.value = false; entryPreviewRequestId = undefined }
  }
}

const chooseOutput = async () => {
  const picked = await requestOutputDirectory(outputPath.value)
  if (picked) { outputPath.value = picked; customOutput.value = picked !== props.destination }
}

const requestOutputDirectory = async (initialPath: string) => {
  const queued = takeDesktopDialogSelection()
  const picked = queued === undefined
    ? await commands.selectDirectory(initialPath || undefined)
    : typeof queued === 'string' ? queued : null
  return typeof picked === 'string' ? picked : null
}

const refreshDirectory = async () => {
  if (!archivePath.value || loading.value) return
  const sequence = ++archiveLoadSequence
  browseRequest.clearNotice()
  focusedEntryPath.value = ''
  selectionAnchorPath = ''
  closePreview()
  try {
    const refreshed = await browseRequest.run(archivePath.value, password.value, inputEncoding.value, requestedSnapshot.value)
    if (sequence !== archiveLoadSequence) return
    result.value = refreshed
    const availableFiles = new Set(refreshed.entries.filter(entry => !entry.isDir).map(entry => entry.path))
    if(props.embedded){
      const available=new Set([...availableFiles,...directoryEntries.value.map(entry=>entry.path)])
      setPaneSelection(new Set([...paneSelection.value].filter(path=>available.has(path))))
    }else selected.value = new Set([...selected.value].filter(path => availableFiles.has(path)))
    navigation.reconcile()
    return true
  } catch (error) {
    if (sequence === archiveLoadSequence) appStore.setError(`刷新压缩包失败：${describeArchiveBrowseError(error)}`)
  } finally {
    if (sequence === archiveLoadSequence) loading.value = false
  }
}

const openWithDefaultApplication = async (entry: ArchiveEntryInfo, allowDangerous = false) => {
  if(entry.linkTarget){appStore.setError("链接内容当前支持只读预览，完整打开将在后续版本接通");return}
  if (entry.isDir || openingEntryPath.value) return
  closeContextMenu()
  openingEntryPath.value = entry.path
  try {
    const opened = await commands.openArchiveEntry(archivePath.value, entry.path, password.value, allowDangerous, ...snapshotReadArguments.value)
    if (opened.status === 'confirmationRequired') {
      dangerousOpenEntry.value = entry
      return
    }
    dangerousOpenEntry.value = null
    appStore.setSuccess(`已使用默认应用打开：${entry.name}`)
  } catch (error) {
    appStore.setError(`无法打开归档内文件：${String(error)}`)
  } finally {
    openingEntryPath.value = ''
  }
}

const confirmDangerousOpen = async () => {
  const entry = dangerousOpenEntry.value
  if (!entry) return
  await openWithDefaultApplication(entry, true)
}

const captureCurrentFrame = (contentSha256: string): ArchiveWorkspaceFrame | null => {
  if (!result.value) return null
  return {
    archivePath: archivePath.value,
    displayName: archiveName.value,
    password: password.value,
    zipPasswordEncoding: zipPasswordEncoding.value,
    outputPath: outputPath.value,
    result: result.value,
    selected: [...selected.value],
    paneSelected: [...paneSelection.value],
    paneSelectionMode: paneSelectionMode.value,
    query: query.value,
    typeFilter: typeFilter.value,
    activeDirectory: activeDirectory.value,
    focusedEntryPath: focusedEntryPath.value,
    navigationBack: [...navigationBack.value],
    navigationForward: [...navigationForward.value],
    expandedDirectories: [...expandedDirectories.value],
    contentSha256,
  }
}

const restoreFrame = (index: number) => {
  const frame = parentFrames.value[index]
  if (!frame) return
  browseRequest.cancel(false)
  archiveLoadSequence++
  parentFrames.value = parentFrames.value.slice(0, index)
  archivePath.value = frame.archivePath
  password.value = frame.password
  zipPasswordEncoding.value = frame.zipPasswordEncoding
  outputPath.value = frame.outputPath
  result.value = frame.result
  requestedSnapshot.value = frame.result.zpaqHistory?.selected
  selected.value = new Set(frame.selected)
  paneSelection.value = new Set(frame.paneSelected)
  paneSelectionMode.value = frame.paneSelectionMode
  query.value = frame.query
  typeFilter.value = frame.typeFilter
  activeDirectory.value = frame.activeDirectory
  focusedEntryPath.value = frame.focusedEntryPath
  navigationBack.value = [...frame.navigationBack]
  navigationForward.value = [...frame.navigationForward]
  expandedDirectories.value = new Set(frame.expandedDirectories)
  currentContentSha256.value = frame.contentSha256
  nestedLoadError.value = ''
  nestedLoadRevision.value = 0
  browseRequest.clearNotice()
  closeContextMenu()
  closePreview()
}

const enterNestedArchive = async (entry: ArchiveEntryInfo) => {
  if(entry.linkTarget){appStore.setError("链接内容当前支持只读预览，嵌套打开将在后续版本接通");return}
  if (!isNestedArchiveEntry(entry) || openingEntryPath.value || !result.value) return
  if (archiveDepth.value >= 3) {
    appStore.setError('嵌套归档最多允许 3 层，请返回上层后再打开其他归档')
    return
  }
  closeContextMenu()
  openingEntryPath.value = entry.path
  try {
    const ancestorHashes = [
      ...parentFrames.value.map(frame => frame.contentSha256),
      currentContentSha256.value,
    ].filter(Boolean)
    const materialized = await commands.materializeNestedArchive(
      archivePath.value,
      entry.path,
      password.value,
      archiveDepth.value + 1,
      ancestorHashes,
      ...snapshotReadArguments.value,
    )
    const frame = captureCurrentFrame(currentContentSha256.value || materialized.parentSha256)
    if (!frame) throw new Error('外层归档状态不可用')
    parentFrames.value = [...parentFrames.value, frame]
    librarySourceId.value = ''; libraryEntryPath.value = ''
    archivePath.value = materialized.cachePath
    password.value = ''
    zipPasswordEncoding.value = undefined
    requestedSnapshot.value = undefined
    currentContentSha256.value = materialized.contentSha256
    outputPath.value = frame.outputPath
    nestedLoadError.value = ''
    nestedLoadRevision.value = 0
    await loadArchive()
  } catch (error) {
    appStore.setError(`无法进入嵌套归档：${String(error)}`)
  } finally {
    openingEntryPath.value = ''
  }
}

const activateEntry = (entry: ArchiveEntryInfo) => {
  if(entry.linkTarget){if(canPreviewEntry(entry)&&entryPreviewSupported(entry))void openPreview(entry);else appStore.setError('链接内容当前支持文本和图片预览，提取将在后续版本接通');return}
  focusedEntryPath.value = entry.path
  if (entry.isDir) navigateToDirectory(entry.path.replace(/\/+$/, ''))
  else if (isNestedArchiveEntry(entry)) void enterNestedArchive(entry)
  else if (props.embedded && canPreviewEntry(entry) && entryPreviewSupported(entry)) void openPreview(entry)
  else void openWithDefaultApplication(entry)
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

const addingToLibrary=ref(false),addedToLibrary=ref(false)
async function addToLibrary(){
  if(!archivePath.value||addingToLibrary.value)return
  addingToLibrary.value=true
  try{const library=useArchiveLibraryStore();if(await library.add([archivePath.value])){addedToLibrary.value=true;appStore.setSuccess('已加入资料库，请前往资料库手动建立或刷新索引')}else appStore.setError(library.error)}finally{addingToLibrary.value=false}
}
const extractPaths = async (paths: string[], destination: string, readerSelection?:ReaderSelection) => {
  if (!archivePath.value || !destination || paths.length === 0 || extracting.value) return
  if(result.value?.entries.some(entry=>entry.linkTarget&&paths.some(path=>entry.path===path||entry.path.startsWith(path.replace(/\/+$/,'')+'/')))){appStore.setError('链接内容当前支持只读预览，提取将在后续版本接通');return}
  if(result.value?.zpaqHistory?.interruptedTail){recoveryVisible.value=true;return}
  extracting.value = true
  closeContextMenu()
  try {
    if (librarySourceId.value) await invoke('library_check', { sourceId: librarySourceId.value, entryPath: null })
    const zpaqCatalogId=nativeZpaqGroup.value&&selectedSnapshot.value&&!readerSelection
      ? await invoke<string>('zpaq_snapshot_catalog_id',{filePath:archivePath.value,password:password.value||null,snapshot:selectedSnapshot.value}) : undefined
    await commands.decompressFile(archivePath.value, {
      ...(zpaqCatalogId?{zpaqCatalogId}:{}),
      ...(readerSelection?{readerSelection}:{}),
      sourceFormat: result.value?.format, zpaqSnapshot: selectedSnapshot.value,
      outputPath: destination,
      password: password.value || undefined,
      ...(inputEncoding.value !== undefined ? { zipPasswordEncoding: inputEncoding.value } : {}),
      keepStructure: true,
      overwrite: false,
      deleteAfter: false,
      preserveTimestamps: true,
      selectedEntries: paths,
      conflictPolicy: 'rename'
    })
    appStore.setSuccess('已取回所选内容')
    emit('extracted', destination)
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

const defaultOpenContextEntry = (entry: ArchiveEntryInfo) => {
  closeContextMenu()
  void openWithDefaultApplication(entry)
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
  if (event.defaultPrevented || readerOpen.value || props.embedded && !props.active) return
  if (document.querySelector('[role="dialog"], [role="alertdialog"]') && !previewEntry.value && !dangerousOpenEntry.value && !detailEntries.value.length) return
  if (event.key === 'Escape') {
    if (previewEntry.value) closePreview()
    else if (dangerousOpenEntry.value) dangerousOpenEntry.value = null
    else if (contextMenu.value) closeContextMenu()
    else if (detailEntries.value.length > 0) closeDetails()
    else if (paneToolsOpen.value) closePaneTools(true)
    else if (props.embedded && paneSelectionMode.value) { paneSelectionMode.value=false;setPaneSelection(new Set()) }
    return
  }
  if (isEditableKeyboardTarget(event.target)) return
  const focusedEntry = [...directoryEntries.value, ...files.value].find(entry => entry.path === focusedEntryPath.value) ?? null
  const keyboardEntries = contextEntriesFor(focusedEntry)
  const keyboardDisplayEntries = focusedEntry?.isDir ? [focusedEntry] : keyboardEntries
  if (props.embedded && (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'a') {
    event.preventDefault(); paneSelectionMode.value=true;setPaneSelection(new Set(filteredEntries.value.map(entry => entry.path)))
  } else if ((event.shiftKey && event.key === 'F10') || event.key === 'ContextMenu') {
    event.preventDefault()
    const focusedElement = focusedEntryPath.value
      ? Array.from(browserRoot.value?.querySelectorAll<HTMLElement>('[data-entry-path]') || [])
          .find(element => element.dataset.entryPath === focusedEntryPath.value) ?? null
      : null
    const rect = focusedElement?.getBoundingClientRect()
    openContextMenu(focusedEntry, new MouseEvent('contextmenu', {
      clientX: rect ? rect.left + Math.min(rect.width / 2, 160) : window.innerWidth / 2,
      clientY: rect ? rect.top + Math.min(rect.height, 36) : window.innerHeight / 2,
    }))
  } else if (event.ctrlKey && event.key === 'Enter' && focusedEntry && canPreviewEntry(focusedEntry) && entryPreviewSupported(focusedEntry)) {
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
    if (props.embedded && !canNavigateUp.value) { if (parentFrames.value.length) restoreFrame(parentFrames.value.length-1); else leavePane() }
    else goUp()
  }
}

const droppedBrowserFiles = (event: DragEvent) => {
  event.preventDefault()
  isDraggingArchive.value = false
  const paths = Array.from(event.dataTransfer?.files || []).map(file => (file as File & { path?: string }).path).filter((path): path is string => !!path)
  if (editingZip.value) { void zipEditor.value?.addDropped(paths); return }
  if (paths[0]) void openArchivePath(paths[0])
}

watch(() => appStore.pendingArchiveBrowserPath, path => {
  if (path && !props.embedded) {
    const options = appStore.takeArchiveBrowserOptions()
    void openArchivePath(appStore.takeArchiveBrowserPath(), options)
  }
}, { immediate: true })

watch(() => props.source, source => { if (props.embedded && source) void openArchivePath(source.path, source.options) }, { immediate: true })
watch(() => props.destination, value => { if (props.embedded && !customOutput.value) outputPath.value = value || '' })
watch(() => props.active, active => { if (!active) { closeContextMenu(); isDraggingArchive.value = false; paneToolsOpen.value = false } })
watch([inputBusy, () => selected.value.size, () => paneSelection.value.size], () => emit('pane-state', { busy: inputBusy.value, selected: props.embedded ? paneSelection.value.size : selected.value.size }), { immediate: true })

watch([password, zipPasswordEncoding], () => { void closeImageReader();closePreview(); dangerousOpenEntry.value = null })

onMounted(() => {
  if (import.meta.env.MODE === 'test') return
  window.addEventListener('keydown', handleWorkspaceKeydown)
  window.addEventListener('pointerdown', dismissPaneTools)
  void Promise.all([
    listen('tauri://file-drop-hover', () => { if (!props.embedded || props.active) isDraggingArchive.value = true }).then(value => { if (disposed) value(); else unlistenHover = value }),
    listen<string[]>('tauri://file-drop', event => {
      isDraggingArchive.value = false
      if (props.embedded && !props.active) return
      if (editingZip.value) void zipEditor.value?.addDropped(event.payload)
      else if (event.payload[0]) void openArchivePath(event.payload[0])
    }).then(value => { if (disposed) value(); else unlistenDrop = value }),
    listen('tauri://file-drop-cancelled', () => { isDraggingArchive.value = false }).then(value => { if (disposed) value(); else unlistenCancel = value })
  ]).catch(error => console.warn('Archive browser drag-and-drop is unavailable:', error))
})

onUnmounted(() => {
  window.removeEventListener('pointerdown', dismissPaneTools)
  cancelEntryPreview()
  disposed = true; sourceSequence++; archiveLoadSequence++
  readerOpen.value=false
  previewSequence++
  browseRequest.cancel(false)
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
const paneEntries = computed<FilePaneEntry[]>(() => filteredEntries.value.map(entry => ({
  ...entry, extension:entry.isDir?'':entry.name.split('.').pop(), icon:entry.isDir?'pi pi-folder':archiveCapabilities.entryCategory(entry.name)==='archive'?'pi pi-box':'pi pi-file',
  preview:canPreviewEntry(entry),previewDisabled:!entryPreviewSupported(entry),
})))
const paneCrumbs = computed(() => {
  const source = nativeZpaqGroup.value?.members[0]?.path || props.source?.path || archivePath.value
  const disk = browserBreadcrumbs(diskParentDirectory(source)).map(crumb=>({...crumb,kind:'disk',frame:-1}))
  const frames = [...parentFrames.value.map(frame=>frame.displayName),archiveName.value || source.split(/[\\/]/).pop() || '压缩包']
  const packages = frames.map((name,index)=>({name,path:source+(index?'/'+frames.slice(1,index+1).join('/'):''),kind:'archive',frame:index}))
  const root = packages.at(-1)!.path
  return [...disk,...packages,...breadcrumbs.value.slice(1).map(crumb=>({name:crumb.name,path:`${root}/${crumb.path}`,kind:'directory',frame:-1,directory:crumb.path}))]
})
const paneVirtualRoot = computed(() => paneCrumbs.value.filter(crumb=>crumb.kind==='archive').at(-1)?.path || props.source?.path || '')
const paneVirtualPath = computed(() => paneVirtualRoot.value+(activeDirectory.value?'/'+activeDirectory.value:''))
function paneBreadcrumb(index:number) {
  const crumb=paneCrumbs.value[index];if(!crumb||inputBusy.value)return
  if(crumb.kind==='disk')emit('navigate-disk',crumb.path)
  else if(crumb.kind==='archive'){if(crumb.frame<parentFrames.value.length)restoreFrame(crumb.frame);else navigateToDirectory('')}
  else navigateToDirectory('directory' in crumb?String(crumb.directory):'')
}
function navigatePanePath(path:string) {
  const normalized=path.replace(/\\/g,'/').replace(/\/+$/,''),root=paneVirtualRoot.value.replace(/\\/g,'/').replace(/\/+$/,'')
  if(normalized.toLowerCase()===root.toLowerCase())navigateToDirectory('')
  else if(normalized.toLowerCase().startsWith(root.toLowerCase()+'/')){
    const directory=normalized.slice(root.length+1)
    if(directories.value.includes(directory))navigateToDirectory(directory)
    else appStore.setError('压缩包中找不到此文件夹')
  }else emit('navigate-disk',path)
}
function setPaneSelection(paths:Set<string>) {
  paneSelection.value=paths
  const underSelection=(path:string)=>paths.has(path)||[...paths].some(parent=>path.startsWith(parent.replace(/\/+$/,'')+'/'))
  const emptyDirectories=directoryEntries.value.filter(entry=>underSelection(entry.path)&&!files.value.some(file=>file.path.startsWith(entry.path))).map(entry=>entry.path)
  const emptyRoots=emptyDirectories.filter(path=>!emptyDirectories.some(parent=>parent!==path&&path.startsWith(parent)))
  selected.value=new Set([...emptyRoots,...files.value.filter(entry=>underSelection(entry.path)).map(entry=>entry.path)])
}
function selectPaneEntry(entry:FilePaneEntry,event:MouseEvent) {
  focusedEntryPath.value=entry.path
  const additive=paneSelectionMode.value||event.ctrlKey||event.metaKey
  const next=new Set(additive?paneSelection.value:[])
  if(additive&&next.has(entry.path))next.delete(entry.path);else next.add(entry.path)
  setPaneSelection(next)
}
const sourceEntry = (entry:FilePaneEntry) => filteredEntries.value.find(item=>item.path===entry.path)
function activatePaneEntry(entry:FilePaneEntry) { const source=sourceEntry(entry);if(source)void activateEntry(source) }
function previewPaneEntry(entry:FilePaneEntry) { const source=sourceEntry(entry);if(source)void openPreview(source) }
function contextPaneEntry(entry:FilePaneEntry|null,event:MouseEvent) {
  if(entry&&!paneSelection.value.has(entry.path))setPaneSelection(new Set([entry.path]))
  openContextMenu(entry?sourceEntry(entry)||null:null,event)
}
async function extractToDestination(destination:string){if(inputBusy.value||!destination)return;customOutput.value=false;outputPath.value=destination;await extractSelected()}
defineExpose({extractToDestination})
</script>

<template>
  <div
    ref="browserRoot"
    :class="{ 'archive-pane-browser': embedded }"
    :data-testid="embedded ? 'archive-pane-browser' : undefined"
    class="browser-page relative h-full min-w-0 overflow-hidden p-responsive p-8 flex flex-col gap-3"
    @dragenter.prevent="isDraggingArchive = true"
    @dragover.prevent="isDraggingArchive = true"
    @dragleave.self="isDraggingArchive = false"
    @drop="droppedBrowserFiles"
    @keydown="handleWorkspaceKeydown"
  >
    <header v-if="!embedded" class="shrink-0 flex flex-wrap items-center justify-between gap-3">
      <div class="min-w-0">
        <h1 class="browser-title font-black text-content tracking-tighter">压缩包浏览中心</h1>
        <p class="text-muted text-xs font-bold mt-1">像文件夹一样浏览压缩包，只解压真正需要的内容</p>
      </div>
      <div class="browser-header-actions flex min-w-0 flex-wrap items-center gap-2">
        <slot name="header-actions" />
        <button v-if="result && !parentFrames.length" class="browser-input" type="button" data-testid="browser-add-library" :disabled="inputBusy || addingToLibrary" @click="addToLibrary"><i class="pi pi-database"/>{{ addingToLibrary ? '正在加入…' : '加入 / 刷新资料库' }}</button><RouterLink v-if="addedToLibrary" to="/library" class="text-primary text-xs font-bold">前往资料库建索引</RouterLink>
        <button class="browser-primary" type="button" :disabled="inputBusy" @click="chooseArchive">
          <i class="pi pi-folder-open"></i><span>打开压缩包</span>
        </button>
      </div>
    </header>

    <FilePaneNavigation v-if="embedded" :pane-id="paneId" :path="paneVirtualPath" :crumbs="paneCrumbs" :folders="filteredEntries.filter(entry=>entry.isDir).length" :files="filteredEntries.filter(entry=>!entry.isDir).length" :locations="locations" :saved="savedLocations" archive :loading="inputBusy" :selection-mode="paneSelectionMode" :tools-open="paneToolsOpen" :filter-active="!!query || typeFilter!=='all'" :can-back="true" :can-forward="canNavigateForward" :can-up="true" @back="canNavigateBack?goBack():parentFrames.length?restoreFrame(parentFrames.length-1):leavePane()" @forward="goForward" @up="canNavigateUp?goUp():parentFrames.length?restoreFrame(parentFrames.length-1):leavePane()" @refresh="result?refreshDirectory():loadArchive()" @selection="paneSelectionMode=!paneSelectionMode;!paneSelectionMode&&setPaneSelection(new Set())" @tools="paneToolsOpen=!paneToolsOpen" @navigate="navigatePanePath" @breadcrumb="paneBreadcrumb"/>
    <section v-if="result?.zpaqHistory?.interruptedTail" class="mx-2 mb-2 flex shrink-0 flex-wrap items-center gap-2 rounded-xl border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs" data-testid="zpaq-interrupted-tail">
      <i class="pi pi-exclamation-triangle text-amber-400"/><strong>尾部更新中断</strong><span class="text-muted">当前显示旧快照 {{result.zpaqHistory.selected.version}}</span><HelpHint label="ZPAQ 中断尾部">当前列表来自中断之前的完整快照。中断更新的内容与条目范围无法确定；不代表最新备份完整，也不会自动拼入未知尾卷。</HelpHint>
      <button class="browser-secondary ml-auto" :disabled="inputBusy" data-testid="zpaq-recovery-open" @click="recoveryVisible=true">取回完整快照</button>
    </section>
    <section v-if="!embedded&&!result&&!loading&&reading.records.length" class="reading-history shrink-0" data-testid="archive-reading-history"><div><strong><i class="pi pi-bookmark"/>继续上次阅读</strong><small>本机阅读位置 · 不保存密码或图片数据</small><button v-if="reading.records.length>4" class="text-primary" @click="showAllReading=!showAllReading">{{ showAllReading?'收起':'更多阅读记录' }}</button></div><div class="reading-recent"><article v-for="row in reading.records.slice(0,showAllReading?30:4)" :key="row.filePath+String(row.snapshot?.version||'')"><button class="reading-recent-open" :disabled="inputBusy" :title="row.filePath" data-testid="reading-recent-open" @click="continueReading(row)"><strong>{{ row.filePath.replace(/\\/g,'/').split('/').pop() }}<small v-if="row.snapshot"> · 快照 {{ row.snapshot.version }}</small></strong><span>{{ row.path }}</span></button><button class="reading-recent-remove" aria-label="移除此阅读记录" @click="reading.remove(row)"><i class="pi pi-times"/></button></article></div></section>
    <p v-if="result && browseNotice" class="shrink-0 rounded-xl border border-primary/20 bg-primary/5 px-4 py-3 text-xs text-primary" data-testid="archive-result-notice">{{ browseNotice }}</p>
    <div v-if="librarySourceId" class="shrink-0 flex flex-wrap items-center justify-between gap-2 rounded-xl border border-primary/20 bg-primary/5 px-4 py-3 text-xs" data-testid="archive-library-origin"><span class="min-w-0 break-all text-muted">来自归档资源库 · {{ libraryEntryPath }}</span><RouterLink to="/library" class="font-bold text-primary">返回搜索结果</RouterLink></div>

    <nav v-if="archivePath && !embedded" class="archive-chain shrink-0" aria-label="嵌套归档链" data-testid="archive-chain">
      <template v-for="(item, index) in archiveChain" :key="`${item.index}-${item.name}`">
        <i v-if="index > 0" class="pi pi-angle-right" aria-hidden="true"></i>
        <button type="button" :disabled="item.current" :class="{ current: item.current }" :title="item.name" @click="restoreFrame(item.index)">
          <i :class="item.current ? 'pi pi-box' : 'pi pi-folder-open'"></i><span class="truncate">{{ item.name }}</span>
        </button>
      </template>
      <span class="archive-depth">{{ archiveDepth }} / 3 层</span>
    </nav>

    <div v-show="!embedded || paneToolsOpen" class="archive-options custom-scrollbar" @keydown.esc.stop.prevent="paneToolsOpen=false">
      <div v-if="embedded" class="pane-search-options"><label class="browser-search"><i class="pi pi-search"/><input v-model="query" placeholder="搜索文件名或路径" aria-label="搜索包内文件"/></label><select v-model="typeFilter" class="browser-select" aria-label="包内文件类型"><option value="all">全部类型</option><option value="image">图片</option><option value="document">文档</option><option value="archive">压缩包</option><option value="other">其他</option></select><button v-if="query||typeFilter!=='all'" type="button" @click="query='';typeFilter='all'">清除筛选</button></div>
      <div v-if="embedded && result" class="pane-source-facts"><span>{{result.format}} · {{formatBytes(result.totalUncompressedSize)}}<i v-if="result.encrypted" class="pi pi-lock"/></span><button v-if="readerCandidates.length" type="button" @click="openImageReader()">阅读图片</button><button type="button" @click="chooseOutput">选择其他提取位置…</button></div>
      <div v-if="embedded && result && !parentFrames.length" class="pane-option-actions"><button class="browser-secondary" type="button" data-testid="browser-add-library" :disabled="inputBusy || addingToLibrary" @click="addToLibrary"><i class="pi pi-database"/>加入 / 刷新资料库</button><button v-if="result.format==='ZPAQ'" class="browser-secondary" type="button" data-testid="zpaq-diagnose-current" :disabled="inputBusy" @click="appStore.openArchiveForDiagnosis({path:archivePath,password,zpaqSnapshot:selectedSnapshot})"><i class="pi pi-search"/>诊断文件影响</button><RouterLink v-if="addedToLibrary" to="/library">前往资料库</RouterLink></div>
    <details v-if="nativeZpaqGroup && !parentFrames.length" class="native-zpaq-source shrink-0" data-testid="zpaq-multipart-source"><summary><i class="pi pi-link"/> 已确认 ZPAQ 来源 · {{nativeZpaqGroup.members.length}} 个部分 <span>查看成员</span></summary><ul><li v-for="member in nativeZpaqGroup.members" :key="member.path">{{member.path}}</li></ul></details>
    <ZpaqSnapshotPicker v-if="result?.zpaqHistory && !result.zpaqHistory.interruptedTail && result.zpaqHistory.snapshots.length > 1" :history="result.zpaqHistory" :disabled="inputBusy" class="shrink-0" @change="changeSnapshot">
      <button type="button" data-testid="zpaq-snapshot-diff" class="browser-secondary" :disabled="inputBusy" @click="diffVisible=true">对比快照</button>
      <button type="button" data-testid="zpaq-snapshot-check" class="browser-secondary" :disabled="inputBusy" @click="checkSnapshot">{{ checkingSnapshot ? '检查中…' : '检查此快照' }}</button>
      <button type="button" data-testid="zpaq-snapshot-convert" class="browser-secondary" :disabled="inputBusy" @click="convertSnapshot">转换此快照</button>
    </ZpaqSnapshotPicker>
    <ZpaqSnapshotDiff v-if="result?.zpaqHistory && !result.zpaqHistory.interruptedTail" v-model:visible="diffVisible" :path="archivePath" :password="password" :history="result.zpaqHistory" :output-directory="outputPath" />
    <p v-if="result?.format === 'WIM'" data-testid="wim-image-hint" class="text-xs leading-relaxed text-muted">固实 WIM/ESD 的预览与所选提取可能需要读取同块内容。多镜像按 1、2 等编号目录展示，可进入后选择文件；根目录的 [1].xml 为镜像说明。原生 SWM 请将同组 install.swm、install2.swm 等放在同一目录，可从任意一卷进入。提取与转换保留当前目录视图，不修改原镜像。</p>
    <ZipEditBar v-if="canEditZip" ref="zipEditor" :source-path="archivePath" :directory="activeDirectory" :password="password" :source-encoding="inputEncoding" :selected-paths="[...selected]" :selected-file="selected.size === 1 ? [...selected][0] : undefined" :disabled="inputBusy" @open-result="(path,options)=>openArchivePath(path,options)" />

    <section class="aero-card shrink-0 p-3 grid gap-3 browser-toolbar" :class="{'has-zip-encoding': archivePath && supportsZipPasswordEncoding(archivePath, result?.format)}">
      <div class="min-w-0 browser-field">
        <span class="browser-label">当前压缩包</span>
        <button class="browser-input text-left truncate" type="button" :disabled="inputBusy" :title="archivePath" @click="chooseArchive">{{ archiveName || '选择或拖入需要浏览的压缩包' }}</button>
      </div>
      <div class="min-w-0 browser-field">
        <span class="browser-label">{{ result?.format === 'LZOP' ? '读取目录' : '密码' }}</span>
        <div class="flex min-w-0 gap-2">
          <input v-if="result?.format !== 'LZOP'" v-model="password" :disabled="inputBusy" class="browser-input min-w-0 flex-1" type="password" placeholder="留空则尝试密码保险箱" @keyup.enter="loadArchive()">
          <button class="browser-icon-button" type="button" :disabled="!archivePath || loading" @click="loadArchive()" title="重新读取">
            <i :class="loading ? 'pi pi-spin pi-spinner' : 'pi pi-refresh'"></i>
          </button>
        </div>
      </div>
      <ZipPasswordEncodingField v-if="archivePath && supportsZipPasswordEncoding(archivePath, result?.format)" v-model="zipPasswordEncoding" :source-path="archivePath" :source-format="result?.format" :disabled="inputBusy" test-id="browser-zip-password-encoding" />
    </section>

      <section v-if="result" class="browser-summary shrink-0" aria-label="压缩包摘要">
        <button v-if="currentReadingRecord" class="browser-input browser-continue-reading" data-testid="archive-reader-continue" :disabled="inputBusy" :title="currentReadingRecord.path" @click="continueCurrentReading"><i class="pi pi-bookmark"/>继续上次阅读</button>
        <button class="browser-primary browser-reader-button" data-testid="archive-reader-open" :disabled="inputBusy||!readerCandidates.length" @click="openImageReader()"><i class="pi pi-images"/>阅读图片 · {{ readerCandidates.length }}</button>
        <span><b>{{ result.format }}</b> 格式</span>
        <span v-if="result.format === 'ZPAQ'" data-testid="zpaq-current-view">{{result.zpaqHistory?.interruptedTail?`完整旧快照 ${selectedSnapshot?.version}`:'当前文件'}}<HelpHint label="ZPAQ 当前文件视图">按当前所选快照显示有效文件，其他版本不会混入。日志包支持历史选择、版本对比与变化提取，普通和原生密码数字卷可继续读取；压缩中心支持另存增量更新。流式包仅显示当前内容，文件检查不能检测所有未认证头部变化。</HelpHint></span>
          <span v-if="result.format === 'ALZ'" data-testid="alz-read-view">只读<HelpHint label="ALZ 读取范围">支持已验证的存储、Deflate、ALZ 变体 Bzip2、ZipCrypto 及原生 .alz/.aNN 卷组。文件名与密码默认按 CP949，可明确选择原工具代码页，不能无损转换时会说明原因。需补齐原卷后解压；不提供 ALZ 创建或损坏修复。</HelpHint></span>
          <span v-if="result.format === 'EGG'" data-testid="egg-read-view">只读<HelpHint label="EGG 读取范围">支持已验证的存储、Deflate、Bzip2、LZMA、AZO、固实及 ZipCrypto、AES/LEA 密码读取。UTF-8 名称与原生 .volN.egg 卷组按真实组合支持，需补齐原卷。固实所选提取仍要读取共享流，单个文件的压缩体积不单独计算。不提供 EGG 创建、文件名/全局加密或损坏修复。</HelpHint></span>
          <span><b>{{ result.totalFiles }}</b> 个文件</span>
        <span>展开后 <b>{{ result.entries.some(entry => entry.sizeKnown === false && !entry.isDir) ? '大小未知' : formatBytes(result.totalUncompressedSize) }}</b></span>
        <span><i :class="result.encrypted ? 'pi pi-lock' : 'pi pi-lock-open'"></i> {{ result.encrypted ? '已加密' : '未加密' }}</span>
      </section>

      <details v-if="embedded && result" class="pane-directory-options"><summary>目录树</summary><ArchiveDirectoryPane :active-directory="activeDirectory" :directories="visibleDirectories" :expanded-directories="expandedDirectories" @select="selectDirectory" @toggle="toggleDirectory"/></details>
    </div>
    <div v-if="loading" class="aero-card flex-1 min-h-0 grid place-items-center">
      <div class="text-center text-muted">
        <i class="pi pi-spin pi-spinner text-primary text-3xl"></i>
        <p class="mt-4 font-bold">正在读取压缩包结构…</p>
        <p class="mt-2 text-xs">大型或异常归档最多等待 30 秒，你也可以立即取消。</p>
        <button class="browser-icon-button browser-cancel-read mt-4 mx-auto px-4" data-testid="archive-browse-cancel" type="button" @click="cancelArchiveLoad">
          <i class="pi pi-times"></i><span>取消读取</span>
        </button>
      </div>
    </div>

    <div v-else-if="!result && parentFrames.length > 0" class="aero-card flex-1 min-h-0 grid place-items-center">
      <div class="max-w-lg text-center px-6"><i class="pi pi-lock text-amber-500 text-5xl"></i><h2 class="mt-5 text-xl font-black text-content">内层归档尚未打开</h2><p class="mt-2 text-sm text-muted leading-6 break-words" data-testid="archive-nested-error" :data-error-revision="nestedLoadRevision">{{ nestedLoadError || '如果内层归档已加密，请只输入这一层的密码后重试；外层密码不会自动继承。' }}</p><div class="mt-5 flex justify-center gap-3"><button class="browser-primary" data-testid="archive-nested-retry" type="button" @click="loadArchive()"><i class="pi pi-refresh"></i><span>使用当前内层密码重试</span></button><button class="browser-icon-button nested-return" type="button" title="返回上一层" @click="restoreFrame(parentFrames.length - 1)"><i class="pi pi-arrow-left"></i></button></div></div>
    </div>

    <div v-else-if="!result && embedded" class="pane-read-error"><i :class="sourceError ? 'pi pi-info-circle' : 'pi pi-box'"/><strong>{{ sourceError ? '暂时无法读取此压缩包' : '尚未读取压缩包' }}</strong><p data-testid="archive-pane-error">{{ sourceError || browseNotice || '可以重新读取，或返回此栏文件夹。' }}</p><div><button class="browser-secondary" @click="paneToolsOpen=true">填写密码 / 调整编码</button><button class="browser-secondary" :disabled="inputBusy" @click="loadArchive()">重新读取</button></div></div>
    <div v-else-if="!result" class="aero-card browser-empty flex-1 min-h-0 grid place-items-center border-dashed" @click="chooseArchive">
      <div class="max-w-md text-center px-6"><i class="pi pi-cloud-upload text-primary text-5xl"></i><h2 class="mt-5 text-xl font-black text-content">把压缩包拖到这里直接浏览</h2><p v-if="browseNotice" class="mt-2 text-sm font-bold text-primary" data-testid="archive-browse-notice">{{ browseNotice }}</p><p class="mt-2 text-sm text-muted leading-6">也可以点击此区域选择文件。内容只在本机读取，密码不会写入命令行。</p><span class="browser-drop-hint">支持 ZIP、7Z、RAR、TAR 等已接入格式</span></div>
    </div>

    <template v-else>
      <section class="aero-card flex-1 min-h-0 min-w-0 overflow-hidden browser-workspace">
        <ArchiveDirectoryPane v-if="!embedded"
          :active-directory="activeDirectory"
          :directories="visibleDirectories"
          :expanded-directories="expandedDirectories"
          @select="selectDirectory"
          @toggle="toggleDirectory"
        />

        <div class="min-h-0 min-w-0 overflow-hidden flex flex-col">
          <FilePaneList v-if="embedded" :entries="paneEntries" :selected="paneSelection" :selection-mode="paneSelectionMode" @select="selectPaneEntry" @activate="activatePaneEntry" @preview="previewPaneEntry" @context="contextPaneEntry"/>
          <template v-else>
          <nav class="browser-navigation shrink-0 border-b border-subtle/70" aria-label="压缩包目录导航">
            <div class="browser-navigation-actions">
              <button type="button" data-testid="archive-nav-back" :disabled="!canNavigateBack" title="后退 (Alt+左箭头)" aria-label="后退" @click="goBack"><i class="pi pi-arrow-left"></i></button>
              <button type="button" data-testid="archive-nav-forward" :disabled="!canNavigateForward" title="前进 (Alt+右箭头)" aria-label="前进" @click="goForward"><i class="pi pi-arrow-right"></i></button>
              <button type="button" data-testid="archive-nav-up" :disabled="!canNavigateUp && !embedded" title="上一级 (Backspace)" aria-label="上一级" @click="canNavigateUp ? goUp() : parentFrames.length ? restoreFrame(parentFrames.length-1) : leavePane()"><i class="pi pi-arrow-up"></i></button>
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
                <span class="min-w-0 flex-1"><strong class="block truncate text-content">{{ entry.name }}</strong><small v-if="!embedded || query" class="block truncate text-muted mt-0.5">{{ entry.path }}</small></span>
                <button
                  v-if="canPreviewEntry(entry)"
                  type="button"
                  class="preview-trigger"
                  :disabled="!entryPreviewSupported(entry)"
                  :title="entryPreviewSupported(entry) ? `预览 ${entry.name}` : '此格式暂不支持此类预览，请先提取后打开'"
                  :aria-label="`预览 ${entry.name}`"
                  @click.stop="openPreview(entry)"
                ><i class="pi pi-eye"></i></button>
              </span>
              <span class="hidden md:block text-left text-muted">{{ entry.isDir ? '文件夹' : entry.sizeKnown === false ? '大小未知' : formatBytes(entry.size) }}</span>
              <span class="hidden lg:block text-left text-muted truncate">{{ entry.modified || '—' }}</span>
              <span class="hidden xl:block text-left font-mono text-muted truncate">{{ entry.crc || '—' }}</span>
            </div>
            <div v-if="filteredEntries.length === 0" class="h-full grid place-items-center text-muted text-sm">没有符合条件的文件</div>
          </div>
          </template>
        </div>
      </section>

      <footer v-if="!embedded" class="shrink-0 flex flex-wrap items-center justify-between gap-3">
        <div class="footer-status min-w-0">
          <span class="text-sm font-bold text-muted">已选择 <strong class="text-primary">{{ selected.size }}</strong> / {{ result.totalFiles }} 个文件</span>
          <button class="output-target" type="button" :title="outputPath" @click="chooseOutput"><i class="pi pi-folder"></i><span class="truncate">解压到：{{ outputPath || '选择输出目录' }}</span><i class="pi pi-pencil"></i></button>
        </div>
        <button class="browser-primary" type="button" :disabled="selected.size === 0 || !outputPath || extracting" @click="extractSelected">
          <i :class="extracting ? 'pi pi-spin pi-spinner' : 'pi pi-download'"></i><span>{{ extracting ? '正在解压' : usesPeerOutput ? '解压到另一栏' : '解压所选文件' }}</span>
        </button>
      </footer>
      <footer v-if="embedded" class="pane-folder-footer"><span>{{paneSelection.size?`已选择 ${paneSelection.size} 项`:paneSelectionMode?'多选模式 · 单击切换选择 · Esc 退出':'单击选择 · Ctrl 多选 · 双击打开 · 右键操作'}}</span><button v-if="paneSelectionMode" type="button" @click="paneSelectionMode=false;setPaneSelection(new Set())">退出多选</button></footer>
    </template>

    <div v-if="isDraggingArchive" class="browser-drop-overlay" aria-live="polite">
      <div><i class="pi pi-cloud-upload"></i><strong>松开即可浏览压缩包</strong><span>将替换当前打开的压缩包</span></div>
    </div>

    <ZpaqRecoveryDialog v-if="recoveryVisible && result?.zpaqHistory?.interruptedTail" :key="archivePath" :path="archivePath" :password="password" :history="result.zpaqHistory" :output-path="outputPath" @close="recoveryVisible=false" />
    <ReaderRetrievalDialog v-if="readerCollectPaths&&result" :file-path="readingSource" :format="result.format" :revision="result.revision||''" :paths="readerCollectPaths" :output-path="outputPath" :password="password" :encoding="inputEncoding" :snapshot="selectedSnapshot" :password-required="result.entries.some(entry=>readerCollectPaths!.includes(entry.path)&&entry.encrypted)" @close="closeReaderCollection" @selection="readerResume&&(readerResume.selected=$event)"/>
    <ArchiveImageReader v-if="readerOpen&&result" :resume="readerResume" :remember="!parentFrames.length" :format="result.format" :file-path="readingSource" :entries="result.entries" :initial-path="readerInitialPath" :directory="activeDirectory" :revision="result.revision" :password="password" :encoding="inputEncoding" :snapshot="selectedSnapshot" :supported="previewRouteSupported" :image-extensions="archiveCapabilities.capabilities.value?.imagePreviewExtensions||[]" @close="closeImageReader" @locate="locateReaderImage" @extract="extractReaderImage" @refresh="refreshReaderSource" @collect="collectReaderImages"/>
    <Teleport to="body">
    <div v-if="previewEntry" class="preview-backdrop" data-testid="archive-entry-preview" @click.self="closePreview">
      <section class="preview-dialog" role="dialog" aria-modal="true" :aria-label="`预览 ${previewEntry.name}`">
        <header class="preview-header">
          <div class="min-w-0 flex items-center gap-2"><h2 class="truncate text-lg font-black text-content">{{ previewEntry.name }}</h2><HelpHint label="预览说明"><template v-if="previewKind(previewEntry) === 'image'">只读查看，不修改原文件。7Z Bzip2 支持普通/密码、非固实/固实完整小文本（≤1 MiB）和图片（≤8 MiB），含内容/文件名加密和完整数字卷；固实共享压缩/解码分别 ≤64 MiB；大文本可先看前 1 MiB，前置与开头合计 ≤64 MiB，明确未校验整文件并可提取全文；超限可先提取后打开。7Z Deflate 支持普通/密码包的完整小文本（≤1 MiB）和图片（≤8 MiB），含固实、内容/文件名加密和完整数字卷，沿原手动密码或保险箱入口；固实共享压缩/解码分别 ≤64 MiB；大文本仅显示前 1 MiB，未检查后续内容，可提取全文；固实前置内容与开头合计 ≤64 MiB，超限请先提取。Deflate64 普通/密码非固实及固实包支持完整小文本 ≤1 MiB、图片 ≤8 MiB，含内容/文件名加密和完整数字分卷，沿原手动密码或保险箱入口；固实共享压缩/解码分别 ≤64 MiB。大文本可先看前 1 MiB，含固实（前置内容与开头合计 ≤64 MiB），未校验整文件，可提取全文；超限请先提取。7Z PPMd 支持普通/密码、非固实/小型固实包的完整小文本（≤1 MiB）和图片（≤8 MiB），含内容/文件名加密与完整数字卷，沿原手动密码或保险箱入口；小图文完整读取；大文本只显示前 1 MiB，明确未校验整文件。固实包只跳过所选前置内容（前置与开头合计≤64 MiB），超出请先提取。7Z 支持 LZMA / LZMA2 非固实及固实图片，AES 内容加密、文件名加密和完整数字分卷沿原手动密码或保险箱入口。固实包的共享压缩数据与解码总量须分别不超过 64 MiB，检查整个共享块后显示；超限请先提取。实际读取最多 64 MiB、密码候选合计限时 30 秒。EGG 未加密 Deflate / LZMA 固实包的共享压缩数据与解码总量须分别不超过 64 MiB，完整校验后显示；超限或加密固实包请先提取。支持 PNG、JPEG、GIF、WebP、BMP；图片不超过 8 MiB，单边不超过 8192、总像素不超过 1600 万。超出范围可先提取后查看。</template><template v-else>只读查看，不修改原文件。7Z Bzip2 支持普通/密码、非固实/固实完整小文本（≤1 MiB）和图片（≤8 MiB），含内容/文件名加密和完整数字卷；固实共享压缩/解码分别 ≤64 MiB；大文本可先看前 1 MiB，前置与开头合计 ≤64 MiB，明确未校验整文件并可提取全文；超限可先提取后打开。7Z Deflate 支持普通/密码包的完整小文本（≤1 MiB）和图片（≤8 MiB），含固实、内容/文件名加密和完整数字卷，沿原手动密码或保险箱入口；固实共享压缩/解码分别 ≤64 MiB；大文本仅显示前 1 MiB，未检查后续内容，可提取全文；固实前置内容与开头合计 ≤64 MiB，超限请先提取。Deflate64 普通/密码非固实及固实包支持完整小文本 ≤1 MiB、图片 ≤8 MiB，含内容/文件名加密和完整数字分卷，沿原手动密码或保险箱入口；固实共享压缩/解码分别 ≤64 MiB。大文本可先看前 1 MiB，含固实（前置内容与开头合计 ≤64 MiB），未校验整文件，可提取全文；超限请先提取。7Z PPMd 支持普通/密码、非固实/小型固实包的完整小文本（≤1 MiB）和图片（≤8 MiB），含内容/文件名加密与完整数字卷，沿原手动密码或保险箱入口；小图文完整读取；大文本只显示前 1 MiB，明确未校验整文件。固实包只跳过所选前置内容（前置与开头合计≤64 MiB），超出请先提取。7Z 支持 LZMA / LZMA2、1 MiB 以内的完整文本。大文本仅显示前 1 MiB，未校验整文件、未读尾部或整包密码。支持非固实及固实（均含 AES 密码包）；固实前置内容与开头合计限 64 MiB，超限请先提取。固实小文本可沿原手动密码或保险箱入口读取 AES 内容加密、文件名加密与完整数字分卷；其共享压缩数据与解码总量须分别不超过 64 MiB，检查整个共享块后显示，超限请先提取。实际读取最多 64 MiB、密码候选合计限时 30 秒。EGG 非固实大文本仅显示前 1 MiB，未校验整文件；小文本与 1 MiB 以内的固实文本完整校验后显示。未加密 Deflate / LZMA 固实包的共享压缩数据与解码总量须分别不超过 64 MiB，超限或加密固实包可先提取。ALZ 小文本完整校验后显示；大文本仅显示前 1 MiB，未校验整文件，不能代替完整性检查。其他已支持格式的大文本显示前 1 MiB。需要编辑时，可以提取后打开。</template></HelpHint></div>
          <button type="button" class="preview-close" aria-label="关闭预览" @click="closePreview"><i class="pi pi-times"></i></button>
        </header>
        <div class="preview-stage">
          <div v-if="previewLoading" class="text-center text-muted"><i class="pi pi-spin pi-spinner text-3xl text-primary"></i><p class="mt-3 text-sm font-bold">{{ ['EGG','7Z'].includes(result?.format || '') && previewEntry.compressedSize === null ? '正在读取固实包共享内容，请稍候…' : '正在读取预览…' }}</p></div>
          <div v-else-if="previewError" class="max-w-md text-center"><i class="pi pi-exclamation-triangle text-3xl text-amber-500"></i><p class="mt-3 break-words text-sm font-bold text-content">无法预览</p><p class="mt-2 break-words text-xs leading-5 text-muted">{{ previewError }}</p></div>
          <div v-else-if="imagePreview" data-testid="archive-image-preview" class="contents"><img :src="imagePreview.dataUrl" :alt="previewEntry.name" class="preview-image"></div>
          <pre v-else-if="textPreview" data-testid="archive-text-preview" class="preview-text">{{ textPreview.content }}</pre>
        </div>
        <footer v-if="imagePreview" class="preview-meta">
          <span>{{ imagePreview.width }} × {{ imagePreview.height }}</span><span>{{ formatBytes(imagePreview.byteSize) }}</span><span>{{ imagePreview.mimeType }}</span><span>只读</span><button class="text-primary font-bold" data-testid="archive-preview-read-images" @click="openImageReader(previewEntry)">连续阅读这组图片 <i class="pi pi-arrow-right"/></button>
        </footer>
        <footer v-else-if="textPreview" class="preview-meta">
          <span>{{ textPreview.encoding }}</span><span>已读 {{ formatBytes(textPreview.byteSize) }} / {{ formatBytes(textPreview.totalSize) }}</span><span>{{ textPreview.lineCount }} 行</span><span>{{ textPreview.truncated ? (['ALZ','EGG','7Z'].includes(result?.format || '') ? '部分内容 · 未校验整文件' : '仅显示前 1 MiB') : '完整显示' }}</span><span>只读</span><button v-if="result?.format==='7Z'&&textPreview.truncated" type="button" class="text-primary font-bold" data-testid="archive-prefix-open-full" :disabled="Boolean(openingEntryPath)" @click="openWithDefaultApplication(previewEntry); closePreview()"><i class="pi pi-external-link"/> 提取后打开</button>
        </footer>

      </section>
    </div>

    </Teleport>
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
            <span class="min-w-0"><strong class="block truncate">{{ contextMenu.entry?.isDir ? contextMenu.entry.name : contextDisplayEntries.length > 1 ? `${contextDisplayEntries.length} 个已选文件` : contextMenu.entry?.name || '当前文件区' }}</strong><small>{{ contextMenu.entry?.isDir ? `${contextMenu.entries.length} 个文件` : contextDisplayEntries.length > 1 ? '批量操作' : contextMenu.entry?.linkTarget ? '链接内容 · 当前支持只读预览' : contextMenu.entry && canPreviewEntry(contextMenu.entry) && entryPreviewSupported(contextMenu.entry) ? '可内部只读预览，也可用默认应用打开' : '可安全提取后用默认应用打开' }}</small></span>
          </header>

          <button v-if="contextMenu.entry?.isDir" type="button" role="menuitem" data-testid="archive-context-open" @click="openContextEntry(contextMenu.entry)"><i class="pi pi-folder-open"></i><span>打开文件夹</span><kbd>Enter</kbd></button>
          <button v-if="contextMenu.entry && !contextMenu.entry.isDir" type="button" role="menuitem" data-testid="archive-context-default-open" :disabled="Boolean(openingEntryPath)||!!contextMenu.entry.linkTarget" :title="contextMenu.entry.linkTarget?'当前支持只读预览':''" @click="defaultOpenContextEntry(contextMenu.entry)"><i :class="openingEntryPath === contextMenu.entry.path ? 'pi pi-spin pi-spinner' : 'pi pi-external-link'"></i><span>使用默认应用打开</span><kbd>Enter</kbd></button>
          <button v-if="contextMenu.entry && isNestedArchiveEntry(contextMenu.entry)" type="button" role="menuitem" data-testid="archive-context-enter-nested" :disabled="archiveDepth >= 3 || Boolean(openingEntryPath)" :title="archiveDepth >= 3 ? '已达到最多 3 层嵌套限制' : '在只读子工作区中打开'" @click="enterNestedArchive(contextMenu.entry)"><i class="pi pi-sitemap"></i><span>{{ archiveDepth >= 3 ? '已达到 3 层上限' : '进入压缩包' }}</span><kbd>Enter</kbd></button>
          <button v-if="contextMenu.entry && !contextMenu.entry.isDir && canPreviewEntry(contextMenu.entry) && entryPreviewSupported(contextMenu.entry)" type="button" role="menuitem" data-testid="archive-context-preview" @click="previewContextEntry(contextMenu.entry)"><i class="pi pi-eye"></i><span>内部查看器打开</span><kbd>Ctrl+Enter</kbd></button>

          <button v-if="canEditZip && contextMenu.entry && !contextMenu.entry.isDir" type="button" role="menuitem" data-testid="archive-context-edit-replace" :disabled="inputBusy" @click="zipEditor?.replace(contextMenu.entry.path); closeContextMenu()"><i class="pi pi-file-edit"></i><span>替换此文件并另存…</span></button>
          <button v-if="canEditZip && contextMenu.entry" type="button" role="menuitem" data-testid="archive-context-edit-rename" :disabled="inputBusy" @click="zipEditor?.organize(contextDisplayEntries.map(entry=>entry.path),'rename'); closeContextMenu()"><i class="pi pi-pencil"></i><span>{{ contextDisplayEntries.length>1?'批量改名或移动…':'重命名或移动…' }}</span></button>
          <button v-if="canEditZip && contextMenu.entry" type="button" role="menuitem" data-testid="archive-context-edit-delete" :disabled="inputBusy" @click="zipEditor?.organize(contextDisplayEntries.map(entry=>entry.path),'delete'); closeContextMenu()"><i class="pi pi-trash"></i><span>从新 ZIP 删除…</span></button>
          <button v-if="canEditZip && !contextMenu.entry" type="button" role="menuitem" data-testid="archive-context-edit-add" :disabled="inputBusy" @click="zipEditor?.addFiles(); closeContextMenu()"><i class="pi pi-plus"></i><span>添加文件到当前目录…</span></button>
          <div v-if="contextMenu.entries.length > 0" class="archive-context-separator"></div>
          <button v-if="contextMenu.entries.length > 0" type="button" role="menuitem" data-testid="archive-context-extract-current" :disabled="extracting || !outputPath || contextHasLink" :title="contextHasLink?'链接内容当前支持只读预览':''" @click="extractContextEntries(false)"><i class="pi pi-download"></i><span>{{ usesPeerOutput ? '解压到另一栏' : '解压到当前输出目录' }}</span><kbd>Alt+E</kbd></button>
          <button v-if="contextMenu.entries.length > 0" type="button" role="menuitem" data-testid="archive-context-extract-choose" :disabled="extracting || contextHasLink" :title="contextHasLink?'链接内容当前支持只读预览':''" @click="extractContextEntries(true)"><i class="pi pi-folder-open"></i><span>解压到指定目录…</span><kbd>Alt+Shift+E</kbd></button>

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
            <article><span>展开大小</span><strong>{{ detailFiles.some(entry => entry.sizeKnown === false) ? '大小未知' : formatBytes(detailTotalSize) }}</strong></article>
            <article><span>归档内大小</span><strong>{{ detailCompressedSize ? formatBytes(detailCompressedSize) : '未知' }}</strong></article>
          </div>
          <dl v-if="detailEntries.length === 1" class="archive-details-grid">
            <div><dt>类型</dt><dd>{{ detailEntries[0].isDir ? '文件夹' : '文件' }}</dd></div>
            <div><dt>加密标记</dt><dd>{{ detailEntries[0].encrypted ? '已加密' : '未单独标记' }}</dd></div>
            <div><dt>修改时间</dt><dd>{{ detailEntries[0].modified || '归档未提供' }}</dd></div>
            <div><dt>CRC</dt><dd class="font-mono">{{ detailEntries[0].crc || '归档未提供' }}</dd></div>
            <div v-if="detailEntries[0].linkTarget" class="archive-link-source"><dt>内容来源</dt><dd :title="detailEntries[0].linkTarget">{{detailEntries[0].linkTarget}}</dd></div>
          </dl>
          <section class="archive-details-paths">
            <h3>归档内路径</h3>
            <ul><li v-for="entry in detailEntries" :key="entry.path">{{ entry.path.replace(/\/+$/, '') }}</li></ul>
          </section>
        </section>
      </div>

      <div v-if="dangerousOpenEntry" class="archive-details-backdrop" data-testid="archive-dangerous-open-dialog" @pointerdown.self="dangerousOpenEntry = null">
        <section class="dangerous-open-dialog" role="alertdialog" aria-modal="true" aria-labelledby="dangerous-open-title">
          <div class="dangerous-open-icon"><i class="pi pi-shield"></i></div>
          <div class="min-w-0">
            <p class="dangerous-open-kicker">ACTIVE CONTENT</p>
            <h2 id="dangerous-open-title">此文件可能执行系统操作</h2>
            <p>“{{ dangerousOpenEntry.name }}”属于可执行或脚本内容。它尚未解压，也没有启动。仅在你确认来源可信时继续。</p>
          </div>
          <footer>
            <button type="button" autofocus data-testid="archive-dangerous-cancel" @click="dangerousOpenEntry = null">取消（推荐）</button>
            <button class="dangerous-confirm" type="button" data-testid="archive-dangerous-confirm" :disabled="Boolean(openingEntryPath)" @click="confirmDangerousOpen">仍要打开</button>
          </footer>
        </section>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.native-zpaq-source{font-size:.72rem;border:1px solid var(--border-subtle);border-radius:.9rem;padding:.65rem 1rem;min-width:0}.native-zpaq-source summary{cursor:pointer;color:var(--color-primary,#0ea5e9);font-weight:700}.native-zpaq-source summary span{float:right;opacity:.65}.native-zpaq-source ul{margin-top:.6rem;max-height:7rem;overflow:auto;overflow-wrap:anywhere;list-style:none;line-height:1.8;opacity:.7}

.browser-page { max-width: 100%; }
.archive-chain { min-width: 0; display: flex; align-items: center; gap: .35rem; overflow-x: auto; overflow-y: hidden; padding: .15rem .1rem; scrollbar-width: none; }
.archive-chain::-webkit-scrollbar { display: none; }
.archive-chain > i { flex: 0 0 auto; color: var(--text-muted); font-size: .6rem; }
.archive-chain button { flex: 0 1 auto; min-width: 0; max-width: 16rem; display: inline-flex; align-items: center; gap: .4rem; border: 1px solid var(--border-subtle); border-radius: 999px; padding: .38rem .65rem; background: var(--bg-input); color: var(--text-muted); font-size: .68rem; font-weight: 850; }
.archive-chain button:not(:disabled):hover { border-color: var(--dynamic-accent); color: var(--dynamic-accent); }
.archive-chain button.current { color: var(--dynamic-accent); background: color-mix(in srgb, var(--dynamic-accent) 10%, var(--bg-input)); }
.archive-depth { flex: 0 0 auto; margin-left: auto; color: var(--text-muted); font-size: .65rem; font-weight: 850; }
.nested-return { display: inline-grid; place-items: center; }
.browser-title { font-size: clamp(1.75rem, 3vw, 2.5rem); line-height: 1; }
.browser-toolbar { grid-template-columns: minmax(0, 1fr) minmax(14rem, .42fr); }
.browser-toolbar.has-zip-encoding { grid-template-columns: minmax(0, 1fr) minmax(10rem, .7fr) minmax(10rem, .6fr); align-items:end; }
@media (max-width: 1050px) { .browser-toolbar.has-zip-encoding { grid-template-columns: minmax(0,1fr) minmax(0,1fr); } .browser-toolbar.has-zip-encoding > :first-child { grid-column:1 / -1; } }
@media (max-width: 760px) { .browser-toolbar.has-zip-encoding { grid-template-columns:minmax(0,1fr); } }
.browser-field { display: flex; flex-direction: column; gap: .4rem; }
.browser-label { color: var(--text-muted); font-size: .7rem; font-weight: 900; letter-spacing: .12em; }
.browser-input, .browser-search, .browser-select { height: 2.75rem; border: 1px solid var(--border-subtle); border-radius: .85rem; background: var(--bg-input); color: var(--text-content); padding: 0 .9rem; outline: none; }
.browser-input:focus, .browser-search:focus-within, .browser-select:focus { border-color: var(--dynamic-accent); }
.browser-icon-button { width: 2.75rem; height: 2.75rem; border-radius: .85rem; border: 1px solid var(--border-subtle); color: var(--dynamic-accent); background: var(--bg-input); }
.browser-cancel-read { width: auto; display: inline-flex; align-items: center; justify-content: center; gap: .5rem; white-space: nowrap; }
.browser-primary { min-height: 2.85rem; padding: 0 1.15rem; border-radius: .9rem; display: inline-flex; align-items: center; justify-content: center; gap: .55rem; color: white; font-size: .8rem; font-weight: 900; background: var(--dynamic-accent); box-shadow: 0 8px 24px color-mix(in srgb, var(--dynamic-accent) 25%, transparent); }
.browser-primary:disabled, .browser-icon-button:disabled { opacity: .45; cursor: not-allowed; }
.browser-secondary { min-height: 2rem; padding: .4rem .75rem; border: 1px solid var(--border-subtle); border-radius: .65rem; background: var(--bg-card); color: var(--dynamic-accent); font-size: .75rem; font-weight: 700; }
.browser-secondary:hover { background: var(--bg-input); }
.browser-secondary:disabled { opacity: .45; cursor: not-allowed; }
.browser-empty { flex: 1 0 auto; padding-block: 1.5rem; overflow-wrap: anywhere; cursor: pointer; transition: border-color .2s ease, background .2s ease, transform .2s ease; }
.browser-empty:hover { border-color: var(--dynamic-accent); background: color-mix(in srgb, var(--dynamic-accent) 5%, var(--bg-card)); transform: translateY(-1px); }
.browser-drop-hint { display: inline-flex; margin-top: 1rem; padding: .45rem .75rem; border-radius: 999px; background: color-mix(in srgb, var(--dynamic-accent) 10%, transparent); color: var(--dynamic-accent); font-size: .7rem; font-weight: 800; }
.browser-reader-button{width:auto;min-height:2.25rem;padding:0 .85rem;flex-shrink:0;white-space:nowrap;font-size:.7rem}
.browser-summary { display: flex; align-items: center; flex-wrap: wrap; gap: .45rem; min-height: 2.25rem; }
.browser-summary span { display: inline-flex; align-items: center; gap: .35rem; padding: .38rem .65rem; border: 1px solid var(--border-subtle); border-radius: 999px; background: color-mix(in srgb, var(--bg-card) 82%, transparent); color: var(--text-muted); font-size: .68rem; font-weight: 800; }
.browser-summary b { color: var(--text-content); }
.browser-workspace { display: grid; grid-template-columns: minmax(10rem, 15rem) minmax(0, 1fr); }
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
.preview-backdrop { position: fixed; inset: 0; z-index: 300; display: grid; place-items: center; min-width: 0; padding: clamp(.75rem, 3vw, 2rem); background: color-mix(in srgb, #08141f 58%, transparent); backdrop-filter: blur(14px); overflow-x: hidden; }
.preview-dialog { width: min(52rem, 100%); max-height: 100%; min-width: 0; overflow-x: hidden; overflow-y: auto; border: 1px solid color-mix(in srgb, var(--dynamic-accent) 24%, var(--border-subtle)); border-radius: 1.5rem; background: color-mix(in srgb, var(--bg-card) 94%, transparent); box-shadow: 0 28px 80px rgba(0, 0, 0, .34); }
.preview-header { min-width: 0; display: flex; align-items: center; justify-content: space-between; gap: 1rem; padding: 1rem 1.25rem; border-bottom: 1px solid var(--border-subtle); }
.preview-close { flex: 0 0 auto; width: 2.4rem; height: 2.4rem; display: grid; place-items: center; border-radius: .75rem; color: var(--text-muted); background: var(--bg-input); }
.preview-stage { min-height: min(54vh, 30rem); display: grid; place-items: center; padding: clamp(1rem, 3vw, 2rem); background: radial-gradient(circle at 50% 42%, color-mix(in srgb, var(--dynamic-accent) 12%, transparent), transparent 65%); overflow: hidden; }
.preview-image { display: block; max-width: 100%; max-height: min(54vh, 30rem); object-fit: contain; border-radius: 1rem; box-shadow: 0 18px 50px rgba(0, 0, 0, .25); }
.preview-text { width: 100%; min-height: min(46vh, 24rem); max-height: min(54vh, 30rem); overflow: auto; border: 1px solid var(--border-subtle); border-radius: 1rem; background: color-mix(in srgb, var(--bg-input) 92%, transparent); padding: 1rem; color: var(--text-content); font-family: ui-monospace, SFMono-Regular, Consolas, monospace; font-size: .75rem; line-height: 1.65; text-align: left; white-space: pre-wrap; overflow-wrap: anywhere; tab-size: 2; }
.preview-meta { padding-bottom:1rem; display: flex; flex-wrap: wrap; gap: .5rem; padding: .9rem 1.25rem 0; }
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
.dangerous-open-dialog { width: min(34rem, calc(100vw - 2rem)); display: grid; grid-template-columns: auto minmax(0, 1fr); gap: 1rem; padding: 1.4rem; border: 1px solid color-mix(in srgb, #f59e0b 38%, var(--border-subtle)); border-radius: 1.5rem; background: var(--bg-modal); box-shadow: 0 26px 80px rgba(5, 18, 28, .36); }
.dangerous-open-icon { width: 3.4rem; height: 3.4rem; display: grid; place-items: center; border-radius: 1rem; color: #d97706; background: color-mix(in srgb, #f59e0b 14%, var(--bg-soft)); font-size: 1.45rem; }
.dangerous-open-kicker { color: #d97706; font-size: .64rem; font-weight: 900; letter-spacing: .18em; }
.dangerous-open-dialog h2 { margin-top: .25rem; color: var(--text-content); font-size: 1.18rem; font-weight: 900; }
.dangerous-open-dialog p:not(.dangerous-open-kicker) { margin-top: .55rem; color: var(--text-muted); font-size: .78rem; font-weight: 650; line-height: 1.65; overflow-wrap: anywhere; }
.dangerous-open-dialog footer { grid-column: 1 / -1; display: flex; justify-content: flex-end; gap: .7rem; padding-top: .25rem; }
.dangerous-open-dialog footer button { min-height: 2.7rem; padding: 0 1rem; border: 1px solid var(--border-subtle); border-radius: .8rem; color: var(--text-content); background: var(--bg-soft); font-size: .72rem; font-weight: 850; }
.dangerous-open-dialog footer button:focus-visible { outline: 3px solid color-mix(in srgb, var(--dynamic-accent) 32%, transparent); outline-offset: 2px; }
.dangerous-open-dialog footer .dangerous-confirm { color: #fff; border-color: #d97706; background: #d97706; }
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
.archive-details-grid .archive-link-source { grid-column:1 / -1; }
.archive-details-grid .archive-link-source dd { white-space:normal; overflow:visible; overflow-wrap:anywhere; line-height:1.7; }
.archive-details-paths { padding: 1rem 1.25rem 1.25rem; }
.archive-details-paths h3 { color: var(--text-muted); font-size: .65rem; font-weight: 900; letter-spacing: .12em; }
.archive-details-paths ul { max-height: 12rem; margin-top: .55rem; overflow-y: auto; overflow-x: hidden; border: 1px solid var(--border-subtle); border-radius: .85rem; background: var(--bg-input); }
.archive-details-paths li { overflow-wrap: anywhere; padding: .55rem .7rem; border-bottom: 1px solid color-mix(in srgb, var(--border-subtle) 65%, transparent); color: var(--text-content); font-family: ui-monospace, SFMono-Regular, Consolas, monospace; font-size: .66rem; }
.archive-details-paths li:last-child { border-bottom: 0; }
@media (max-width: 1050px) { .browser-table-head, .browser-row { grid-template-columns: 1.5rem minmax(8rem, 1fr) minmax(5rem, .28fr) minmax(7rem, .4fr); } }
/* On short desktop windows, scroll the page instead of shrinking the file
   list to a fraction of one row behind its navigation and search controls. */
@media (min-width: 761px) and (max-height: 720px) {
  .browser-page { overflow-y: auto; overflow-x: hidden; }
  .browser-workspace { flex: 0 0 24rem; min-height: 24rem; }
}
@media (max-width: 760px) { .browser-page { padding: 1rem; overflow-y: auto; overflow-x: hidden; } .browser-workspace { flex: 0 0 34rem; min-height: 34rem; grid-template-columns: 1fr; grid-template-rows: minmax(7rem, 10rem) minmax(22rem, 1fr); } .browser-toolbar { grid-template-columns: 1fr; } .browser-table-head, .browser-row { grid-template-columns: 1.5rem minmax(0, 1fr); } .output-target { max-width: 80vw; } .archive-details-metrics { grid-template-columns: repeat(2, minmax(0, 1fr)); } .archive-details-grid { grid-template-columns: 1fr; } }
.reading-history{border:1px solid var(--border-subtle);border-radius:16px;padding:16px;background:var(--bg-card);max-height:40%;overflow:auto}.reading-history>div:first-child{display:flex;align-items:center;justify-content:space-between;flex-wrap:wrap;gap:8px;font-size:12px}.reading-history strong i{color:var(--dynamic-accent);margin-right:7px}.reading-history small{font-size:10px;font-weight:400;color:var(--text-muted)}.reading-recent{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:8px;margin-top:12px}.reading-recent article{display:flex;gap:9px;align-items:center;padding:10px 12px;border:1px solid var(--border-subtle);border-radius:11px;min-width:0}.reading-recent-open{flex:1;text-align:left;min-width:0;font-size:11px}.reading-recent-open strong,.reading-recent-open span{display:block;overflow:hidden;white-space:nowrap;text-overflow:ellipsis}.reading-recent-open span{color:var(--text-muted);font-size:10px;margin-top:5px}.reading-recent-open:hover strong{color:var(--dynamic-accent)}.reading-recent-remove{color:var(--text-muted);font-size:10px;padding:5px}.browser-continue-reading{white-space:nowrap;font-size:.7rem}@media(max-width:800px){.reading-recent{grid-template-columns:minmax(0,1fr)}.reading-history{padding:12px}}

.archive-options{display:contents}
.archive-pane-browser{container:archive-pane / inline-size;padding:0!important;gap:0!important;min-height:0;overflow:hidden!important;background:transparent}
.archive-pane-browser .pane-archive-nav{display:flex;align-items:center;gap:6px;flex-wrap:nowrap;padding:9px;border-bottom:1px solid var(--border-subtle);flex-shrink:0}
.pane-archive-nav>button{display:grid;place-items:center;width:30px;height:30px;border-radius:8px;flex-shrink:0;color:var(--text-muted);font-size:12px}.pane-archive-nav>button:hover,.pane-archive-nav>button.active{color:var(--dynamic-accent);background:var(--bg-input)}.pane-archive-nav>button:disabled{opacity:.4}
.pane-archive-location{display:flex;align-items:center;gap:8px;min-width:0;flex:1;background:var(--bg-input);border:1px solid var(--border-subtle);border-radius:9px;padding:8px 10px;font-size:11px}.pane-archive-location>i{color:var(--dynamic-accent);flex-shrink:0}.pane-archive-location strong{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.pane-archive-status{display:flex;align-items:center;justify-content:space-between;gap:8px;padding:8px 12px;border-bottom:1px solid var(--border-subtle);font-size:10px;color:var(--text-muted);flex-shrink:0}.pane-archive-status>span{min-width:0;overflow-wrap:anywhere}.pane-archive-status i{margin-left:5px}.pane-archive-status>button{color:var(--dynamic-accent);flex-shrink:0;font-weight:700}
.archive-pane-browser .archive-options{position:absolute;z-index:20;top:56px;left:8px;right:8px;display:flex;flex-direction:column;gap:10px;min-width:0;min-height:0;max-height:calc(100% - 72px);overflow-y:auto;padding:12px;border:1px solid var(--border-subtle);border-radius:12px;background:var(--bg-card);box-shadow:0 14px 40px rgb(0 0 0 / .3)}
.archive-pane-browser .archive-options>*{flex-shrink:0;min-width:0;max-width:100%}.pane-option-actions{display:flex;gap:8px;align-items:center;flex-wrap:wrap;font-size:11px;color:var(--dynamic-accent)}.archive-pane-browser .browser-toolbar,.archive-pane-browser .browser-toolbar.has-zip-encoding{grid-template-columns:minmax(0,1fr);padding:0;background:transparent;border:0;box-shadow:none}.archive-pane-browser .browser-toolbar>.browser-field:first-child{display:none}.archive-pane-browser .browser-toolbar.has-zip-encoding>:first-child{grid-column:auto}
.archive-pane-browser .browser-input,.archive-pane-browser .browser-search,.archive-pane-browser .browser-select,.archive-pane-browser .browser-icon-button{height:34px;font-size:11px;border-radius:9px}.archive-pane-browser .browser-summary{gap:6px}.archive-pane-browser .browser-summary>span{font-size:10px;padding:5px 8px}.archive-pane-browser .browser-primary{font-size:11px;min-height:34px;padding:7px 10px;border-radius:9px;box-shadow:none}
.archive-pane-browser .browser-workspace{display:flex;flex:1 1 0;min-height:0;border:0;border-radius:0;background:transparent}.archive-pane-browser .browser-workspace>div{flex:1}.archive-pane-browser .browser-navigation{padding:8px;gap:7px}.archive-pane-browser .browser-navigation-actions{gap:3px}.archive-pane-browser .browser-navigation-actions button{width:26px;height:26px;font-size:10px}.archive-pane-browser .browser-breadcrumbs{font-size:10px}.archive-pane-browser .browser-search{padding:0 9px}.archive-pane-browser .browser-select{max-width:7rem;min-width:0;flex:0 1 6rem;padding:0 6px}.archive-pane-browser .browser-search{min-width:0;flex:1 1 0}
.archive-pane-browser .browser-table-head,.archive-pane-browser .browser-row{grid-template-columns:20px minmax(0,1fr) 55px;gap:7px;padding:9px 10px;font-size:11px}.archive-pane-browser .browser-table-head>span:nth-last-child(-n+2),.archive-pane-browser .browser-row>span:nth-last-child(-n+2){display:none}.archive-pane-browser .browser-table-head>span:nth-child(3),.archive-pane-browser .browser-row>span:nth-child(3){display:block!important;font-size:10px}.archive-pane-browser .browser-row small{font-size:10px}.archive-pane-browser .browser-checkbox{width:16px;height:16px;border-radius:5px}.archive-pane-browser .preview-trigger{width:24px;height:26px;font-size:11px;border-radius:7px}
.archive-pane-browser>footer{padding:9px 10px;border-top:1px solid var(--border-subtle);gap:8px;flex-wrap:nowrap}.archive-pane-browser>footer>.browser-primary{flex-shrink:0;white-space:nowrap}.archive-pane-browser .footer-status{display:flex;flex-direction:column;align-items:stretch;gap:3px;flex:1 1 0;min-width:0}.archive-pane-browser .footer-status>span{font-size:10px}.archive-pane-browser .output-target{width:100%;max-width:100%;font-size:10px;padding:4px 6px;min-height:26px}.archive-pane-browser .archive-chain{padding:6px 10px}.archive-pane-browser .archive-chain button{font-size:10px;padding:4px 7px}.pane-directory-options{font-size:11px;color:var(--text-muted)}.pane-directory-options>summary{cursor:pointer;padding:6px}.pane-directory-options :deep(aside){max-height:180px;overflow:auto}.pane-read-error{flex:1;min-height:0;overflow:auto;display:flex;align-items:center;flex-direction:column;justify-content:center;gap:12px;text-align:center;padding:16px;font-size:12px}.pane-read-error>i{font-size:24px;color:var(--dynamic-accent)}.pane-read-error>p{font-size:11px;color:var(--text-muted);line-height:1.8;overflow-wrap:anywhere}.pane-read-error>div{display:flex;gap:8px;flex-wrap:wrap;justify-content:center}
@container archive-pane (max-width:360px){.archive-pane-browser .browser-table-head,.archive-pane-browser .browser-row{grid-template-columns:18px minmax(0,1fr) 44px;gap:5px;padding:8px}.archive-pane-browser .browser-navigation{flex-wrap:wrap}.archive-pane-browser .browser-navigation-actions{flex-shrink:0}.archive-pane-browser .browser-breadcrumbs{flex:1;min-width:0}}

.archive-pane-browser>.pane-folder-footer{display:flex;min-height:2rem;align-items:center;justify-content:space-between;gap:.5rem;padding:.4rem .7rem;border-top:1px solid var(--border-subtle);color:var(--text-muted);font-size:.6rem;font-weight:700}.pane-folder-footer>span{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.pane-folder-footer>button{flex-shrink:0;color:var(--dynamic-accent);font-weight:800}.pane-search-options{display:flex;flex-wrap:wrap;gap:8px}.pane-search-options>.browser-search{flex:1 1 150px}.pane-search-options>button,.pane-source-facts>button{color:var(--dynamic-accent);font-size:11px}.pane-source-facts{display:flex;align-items:center;flex-wrap:wrap;gap:10px;font-size:11px;color:var(--text-muted)}
</style>
