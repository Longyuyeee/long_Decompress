<script setup lang="ts">
import HelpHint from '@/components/ui/HelpHint.vue'
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { open } from '@tauri-apps/api/dialog'
import { listen } from '@tauri-apps/api/event'
import type { ArchiveReadOptions } from '@/stores/app'
import type { ArchivePaneSource } from '@/types/fileBrowser'
import ArchiveBrowserView from '@/views/ArchiveBrowserView.vue'
import LocalFilePreview from './LocalFilePreview.vue'
import FilePaneNavigation from './FilePaneNavigation.vue'
import FilePaneList from './FilePaneList.vue'
import { browserBreadcrumbs } from '@/utils/browserPath'
import { archiveSourceDirectory } from '@/utils/zpaqMultipart'
import { localPreviewKind } from '@/utils/filePreview'
import { useSavedLocations } from '@/composables/useSavedLocations'
import { DECOMPRESS_ARCHIVE_PICKER_EXTENSIONS, isDecompressArchivePath, isPotentialSfxPath, isPotentialSplitArchivePath } from '@/utils/compressionFormat'

export interface LocalFileEntry {
  path: string
  name: string
  size: number
  isDir: boolean
  extension?: string | null
  modified?: string | number | { secs_since_epoch: number; nanos_since_epoch?: number } | null
}

interface Location { name: string; path: string; kind: string }
interface PaneState {
  id: 'left' | 'right'
  path: string
  pathDraft: string
  locationMenuOpen: boolean
  entries: LocalFileEntry[]
  selected: Set<string>
  history: string[]
  historyIndex: number
  selectionMode: boolean
  loading: boolean
  error: string
  archive: ArchivePaneSource | null
  archiveCache?: ArchivePaneSource
  archiveBusy: boolean
  archiveSelected: number
  scrollTop: number
}

const props = defineProps<{ archiveRequest?: ArchivePaneSource }>()
const managerRoot = ref<HTMLElement | null>(null)
const previewEntry = ref<LocalFileEntry | null>(null)
let archiveId = 0, unmounted = false, unlistenDrop: (() => void) | undefined
const emit = defineEmits<{
  openArchive: [path: string]
  compress: [entries: LocalFileEntry[], destination: string]
  extract: [paths: string[], destination: string]
}>()

const locations = ref<Location[]>([])
const savedLocations = useSavedLocations()
const paneRequest = { left: 0, right: 0 }
const activePane = ref<'left' | 'right'>('left')
const busy = ref(false)
const notice = ref('')
const context = ref<{ pane: PaneState; entry: LocalFileEntry | null; x: number; y: number } | null>(null)
const properties = ref<any | null>(null)
const editor = reactive<{ type: '' | 'rename' | 'mkdir' | 'delete'; pane: PaneState | null; entry: LocalFileEntry | null; value: string }>({ type: '', pane: null, entry: null, value: '' })

const createPane = (id: 'left' | 'right'): PaneState => reactive({ id, path: '', pathDraft: '', locationMenuOpen: false, entries: [], selected: new Set<string>(), history: [], historyIndex: -1, selectionMode: false, loading: false, error: '', archive: null, archiveBusy: false, archiveSelected: 0, scrollTop: 0 })
const left = createPane('left')
const right = createPane('right')
const archiveApis = new Map<string, InstanceType<typeof ArchiveBrowserView>>()
const bindArchiveApi = (id:string, component:unknown) => { if(component) archiveApis.set(id,component as InstanceType<typeof ArchiveBrowserView>);else archiveApis.delete(id) }
const canResumeArchive = (pane:PaneState) => !!pane.archiveCache && parentPath(pane.archiveCache.path).replace(/\\/g,'/').toLowerCase()===pane.path.replace(/\\/g,'/').toLowerCase()
const otherPane = (pane: PaneState) => pane.id === 'left' ? right : left
const isArchive = (entry: LocalFileEntry) => !entry.isDir && (isDecompressArchivePath(entry.path) || isPotentialSfxPath(entry.path) || isPotentialSplitArchivePath(entry.path))
const selectedEntries = (pane: PaneState) => pane.entries.filter(entry => pane.selected.has(entry.path))
const operationEntries = (pane: PaneState, entry?: LocalFileEntry | null) => {
  const selected = selectedEntries(pane)
  return entry && !pane.selected.has(entry.path) ? [entry] : selected.length ? selected : entry ? [entry] : []
}

const normalizeEntry = (raw: any): LocalFileEntry => ({
  path: raw.path,
  name: raw.name,
  size: Number(raw.size || 0),
  isDir: Boolean(raw.isDir ?? raw.is_dir),
  extension: raw.extension,
  modified: raw.modified,
})

const sortEntries = (entries: LocalFileEntry[]) => entries.sort((a, b) => a.isDir === b.isDir
  ? a.name.localeCompare(b.name, 'zh-CN', { numeric: true, sensitivity: 'base' })
  : a.isDir ? -1 : 1)

const loadPane = async (pane: PaneState, path: string, pushHistory = true) => {
  const request = ++paneRequest[pane.id]
  pane.loading = true
  pane.error = ''
  try {
    const raw = await invoke<any[]>('list_files', { path })
    if (request !== paneRequest[pane.id]) return false
    pane.entries = sortEntries(raw.map(normalizeEntry))
    pane.path = path
    pane.pathDraft = path
    pane.locationMenuOpen = false
    pane.selected = new Set()
    if (pushHistory && pane.history[pane.historyIndex] !== path) {
      pane.history = pane.history.slice(0, pane.historyIndex + 1)
      pane.history.push(path)
      pane.historyIndex = pane.history.length - 1
    }
    savedLocations.remember(path)
    return true
  } catch (error) {
    if (request !== paneRequest[pane.id]) return false
    pane.error = String(error)
    return false
  } finally { if (request === paneRequest[pane.id]) pane.loading = false }
}

const cleanPathInput = (value: string) => value.trim().replace(/^['"]|['"]$/g, '')
const submitPath = async (pane: PaneState) => {
  const path = cleanPathInput(pane.pathDraft)
  if (!path) {
    pane.pathDraft = pane.path
    pane.locationMenuOpen = false
    return
  }
  await loadPane(pane, path)
}
const selectLocation = async (pane: PaneState, path: string) => {
  pane.pathDraft = path
  await loadPane(pane, path)
}
const toggleLocationMenu = (pane: PaneState) => {
  otherPane(pane).locationMenuOpen = false
  pane.locationMenuOpen = !pane.locationMenuOpen
}
const cancelPathEditing = (pane: PaneState) => {
  pane.pathDraft = pane.path
  pane.locationMenuOpen = false
}
const locationIcon = (location: Location) => location.kind === 'drive'
  ? 'pi pi-database'
  : location.kind === 'home' ? 'pi pi-home' : 'pi pi-folder'

const navigateHistory = (pane: PaneState, delta: number) => {
  const index = pane.historyIndex + delta
  if (delta > 0 && index === pane.history.length && canResumeArchive(pane)) { pane.archive = pane.archiveCache!; return }
  if (index < 0 || index >= pane.history.length) return
  pane.historyIndex = index
  void loadPane(pane, pane.history[index], false)
}

const parentPath = (path: string) => {
  const normalized = path.replace(/[\\/]+$/, '')
  const index = Math.max(normalized.lastIndexOf('\\'), normalized.lastIndexOf('/'))
  if (index < 0) return path
  const parent = normalized.slice(0, index)
  return /^[A-Za-z]:$/.test(parent) ? `${parent}\\` : parent || '/'
}

const breadcrumbs = (path: string) => {
  const windowsDrive = path.match(/^([A-Za-z]:)[\\/]*(.*)$/)
  if (windowsDrive) {
    const root = `${windowsDrive[1]}\\`
    const result = [{ name: windowsDrive[1], path: root }]
    let current = root.replace(/[\\/]+$/, '')
    for (const name of windowsDrive[2].split(/[\\/]+/).filter(Boolean)) {
      current = `${current}\\${name}`
      result.push({ name, path: current })
    }
    return result
  }
  if (/^[\\/]{2}/.test(path)) {
    const parts = path.replace(/^[\\/]+/, '').split(/[\\/]+/).filter(Boolean)
    const result: Array<{ name: string; path: string }> = []
    let current = '\\\\'
    for (const name of parts) {
      current = `${current}${current.endsWith('\\') ? '' : '\\'}${name}`
      result.push({ name, path: current })
    }
    return result
  }
  const result = [{ name: '/', path: '/' }]
  let current = ''
  for (const name of path.split('/').filter(Boolean)) {
    current += `/${name}`
    result.push({ name, path: current })
  }
  return result
}

const openInPane = (pane: PaneState, path: string, options?: ArchiveReadOptions) => {
  if (pane.archiveBusy) { notice.value = '此栏仍在处理，请完成或停止后再打开其他压缩包。'; return }
  context.value = null; pane.locationMenuOpen = false
  pane.history = pane.history.slice(0,pane.historyIndex+1)
  pane.scrollTop = managerRoot.value?.querySelector(`[data-file-pane="${pane.id}"] > .file-pane-content .file-list`)?.scrollTop || 0
  activePane.value = pane.id
  pane.archive = { id: ++archiveId, path, options }
  pane.archiveCache = pane.archive
  emit('openArchive', path)
  const request = pane.archive.id
  void archiveSourceDirectory(path).then(directory => {
    if (!unmounted && pane.archive?.id === request && directory && directory !== pane.path) return loadPane(pane, directory)
  }).catch(() => { /* The archive pane reports an unavailable source itself. */ })
}
const leaveArchive = async (pane: PaneState) => {
  pane.archive = null; pane.archiveBusy = false; pane.archiveSelected = 0
  await nextTick()
  const list = managerRoot.value?.querySelector(`[data-file-pane="${pane.id}"] > .file-pane-content .file-list`)
  if (list) list.scrollTop = pane.scrollTop
}
const navigateDisk = async (pane:PaneState,path:string) => {
  if (pane.archiveBusy) return
  if (isDecompressArchivePath(path) || isPotentialSplitArchivePath(path)) { openInPane(pane,path); return }
  pane.pathDraft = path
  if (await loadPane(pane,path)) { pane.scrollTop=0; await leaveArchive(pane) }
}
const transferArchiveSelection = (pane:PaneState) => {
  if (requireDiskDestination(pane)) void archiveApis.get(pane.id)?.extractToDestination(otherPane(pane).path)
}
const refreshDestination = async (pane: PaneState, destination: string) => {
  const other = otherPane(pane)
  if (!other.archive && other.path.replace(/\\/g,'/').toLowerCase() === destination.replace(/\\/g,'/').toLowerCase()) await loadPane(other, other.path, false)
}
const requireDiskDestination = (pane: PaneState) => {
  if (!otherPane(pane).archive) return true
  notice.value = '另一栏正在浏览压缩包，请先返回目标文件夹，再放入文件。'
  return false
}
const previewLocal = (entry: LocalFileEntry) => { context.value = null; previewEntry.value = entry }
watch(() => props.archiveRequest, request => { if (request?.path) openInPane(activePane.value === 'left' ? left : right, request.path, request.options) }, { immediate: true })

const activate = (pane: PaneState, entry: LocalFileEntry) => {
  activePane.value = pane.id
  if (entry.isDir) void loadPane(pane, entry.path)
  else if (isArchive(entry)) openInPane(pane, entry.path)
  else if (localPreviewKind(entry.name)) previewLocal(entry)
  else void openInSystemManager(entry)
}

const openInSystemManager = async (entry: LocalFileEntry) => {
  context.value = null
  notice.value = ''
  try {
    await invoke('open_in_explorer', { path: entry.path })
  } catch {
    notice.value = entry.isDir
      ? '无法在 Windows 文件管理器中打开此文件夹，请确认该位置仍然存在且有权访问。'
      : '无法在 Windows 文件管理器中定位此文件，请确认该文件仍然存在且有权访问。'
  }
}

const toggle = (pane: PaneState, entry: LocalFileEntry, event: MouseEvent) => {
  activePane.value = pane.id
  const additive = pane.selectionMode || event.ctrlKey || event.metaKey
  const next = new Set(additive ? pane.selected : [])
  if (additive && next.has(entry.path)) next.delete(entry.path)
  else next.add(entry.path)
  pane.selected = next
}

const toggleSelectionMode = (pane: PaneState) => {
  activePane.value = pane.id
  pane.selectionMode = !pane.selectionMode
  if (!pane.selectionMode) pane.selected = new Set()
  context.value = null
}

const selectAll = (pane: PaneState) => {
  activePane.value = pane.id
  pane.selectionMode = true
  pane.selected = new Set(pane.entries.map(entry => entry.path))
  context.value = null
}

const openContext = (pane: PaneState, entry: LocalFileEntry, event: MouseEvent) => {
  event.preventDefault()
  activePane.value = pane.id
  if (!pane.selected.has(entry.path)) pane.selected = new Set([entry.path])
  context.value = { pane, entry, x: Math.min(event.clientX, window.innerWidth - 260), y: Math.min(event.clientY, window.innerHeight - 390) }
}

const openBlankContext = (pane: PaneState, event: MouseEvent) => {
  event.preventDefault()
  activePane.value = pane.id
  context.value = { pane, entry: null, x: Math.min(event.clientX, window.innerWidth - 260), y: Math.min(event.clientY, window.innerHeight - 330) }
}

const openInOtherPane = async (pane: PaneState, path = pane.path) => {
  context.value = null
  if (otherPane(pane).archiveBusy) return
  otherPane(pane).archive = null
  await loadPane(otherPane(pane), path)
  activePane.value = otherPane(pane).id
}

const swapPaneLocations = async () => {
  const leftPath = left.path
  const rightPath = right.path
  context.value = null
  await Promise.all([loadPane(left, rightPath), loadPane(right, leftPath)])
}

const transferIcon = (pane: PaneState) => pane.id === 'left' ? 'pi pi-arrow-right' : 'pi pi-arrow-left'
const transferDirection = (pane: PaneState) => pane.id === 'left' ? '→' : '←'

const runTransfer = async (operation: 'copy' | 'move', pane: PaneState, entry?: LocalFileEntry) => {
  const entries = operationEntries(pane, entry)
  if (!entries.length || busy.value || !requireDiskDestination(pane)) return
  busy.value = true; context.value = null; notice.value = ''
  try {
    const report = await invoke<any>(`file_manager_${operation}`, { sources: entries.map(item => item.path), destination: otherPane(pane).path })
    notice.value = `${operation === 'copy' ? '复制' : '移动'}完成：${report.processed} 项，${formatBytes(report.bytes)}`
    await Promise.all([loadPane(pane, pane.path, false), loadPane(otherPane(pane), otherPane(pane).path, false)])
  } catch (error) { notice.value = `操作未完成：${error}` }
  finally { busy.value = false }
}

const beginEditor = (type: 'rename' | 'mkdir' | 'delete', pane: PaneState, entry?: LocalFileEntry) => {
  context.value = null
  editor.type = type; editor.pane = pane; editor.entry = entry || null
  editor.value = type === 'rename' ? entry?.name || '' : type === 'mkdir' ? '新建文件夹' : ''
}

const submitEditor = async () => {
  const pane = editor.pane
  if (!pane || busy.value) return
  busy.value = true
  try {
    if (editor.type === 'rename' && editor.entry) await invoke('file_manager_rename', { source: editor.entry.path, newName: editor.value })
    if (editor.type === 'mkdir') await invoke('file_manager_create_directory', { parent: pane.path, name: editor.value })
    if (editor.type === 'delete') {
      const entries = operationEntries(pane, editor.entry)
      await invoke('file_manager_recycle', { paths: entries.map(item => item.path) })
      notice.value = `已将 ${entries.length} 项移入系统回收站`
    }
    editor.type = ''; await loadPane(pane, pane.path, false)
  } catch (error) { notice.value = `操作未完成：${error}` }
  finally { busy.value = false }
}

const showProperties = async (pane: PaneState, entry: LocalFileEntry) => {
  context.value = null; properties.value = null; notice.value = ''; busy.value = true
  try { properties.value = await invoke('file_manager_properties', { path: entry.path }) }
  catch {
    notice.value = '无法读取此项目的属性。它可能是系统保护或特殊文件夹，可尝试在 Windows 文件管理器中查看。'
  }
  finally { busy.value = false }
}

const compressToOther = (pane: PaneState, entry?: LocalFileEntry) => {
  if (!requireDiskDestination(pane)) return
  const entries = operationEntries(pane, entry)
  if (entries.length) emit('compress', entries, otherPane(pane).path)
  context.value = null
}

const extractToOther = (pane: PaneState, entry?: LocalFileEntry) => {
  if (!requireDiskDestination(pane)) return
  const entries = operationEntries(pane, entry).filter(isArchive)
  if (entries.length) emit('extract', entries.map(item => item.path), otherPane(pane).path)
  context.value = null
}

const formatBytes = (bytes: number) => {
  if (!bytes) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']; const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1)
  return `${(bytes / 1024 ** index).toFixed(index ? 1 : 0)} ${units[index]}`
}
const formatDate = (value: any) => {
  if (!value) return '—'
  const date = typeof value === 'number' ? new Date(value) : typeof value === 'string' ? new Date(value) : value.secs_since_epoch ? new Date(value.secs_since_epoch * 1000) : new Date(value)
  return Number.isNaN(date.getTime()) ? '—' : date.toLocaleString()
}
const icon = (entry: LocalFileEntry) => entry.isDir ? 'pi pi-folder' : isArchive(entry) ? 'pi pi-box' : /\.(jpg|jpeg|png|webp|gif)$/i.test(entry.name) ? 'pi pi-image' : /\.pdf$/i.test(entry.name) ? 'pi pi-file-pdf' : 'pi pi-file'
const chooseArchive = async () => {
  const queued = import.meta.env.VITE_DESKTOP_E2E ? window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection() : undefined
  const selected = queued === undefined
    ? await open({ multiple: false, filters: [{ name: '压缩包', extensions: DECOMPRESS_ARCHIVE_PICKER_EXTENSIONS }] })
    : Array.isArray(queued) ? queued[0] : queued
  if (typeof selected === 'string') openInPane(activePaneState(), selected)
}

const activePaneState = () => activePane.value === 'left' ? left : right
const handleGlobalPointer = () => {
  context.value = null
  for (const pane of [left, right]) {
    if (pane.locationMenuOpen) cancelPathEditing(pane)
  }
}
const handleKeydown = (event: KeyboardEvent) => {
  const target = event.target
  const isEditable = target instanceof Element && target.matches('input, textarea, select, [contenteditable="true"]')
  if (event.defaultPrevented || isEditable || previewEntry.value || document.querySelector('[role="dialog"], [role="alertdialog"]')) return
  const pane = activePaneState()
  if (pane.archive) return
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'a') {
    event.preventDefault()
    selectAll(pane)
  } else if (event.key === 'F5') {
    event.preventDefault()
    void loadPane(pane, pane.path, false)
  } else if (event.key === 'Escape' && context.value) {
    event.preventDefault()
    context.value = null
  } else if (event.key === 'Escape' && pane.selectionMode) {
    event.preventDefault()
    toggleSelectionMode(pane)
  }
}
onMounted(async () => {
  window.addEventListener('pointerdown', handleGlobalPointer)
  window.addEventListener('keydown', handleKeydown)
  locations.value = await invoke<Location[]>('file_manager_locations')
  if (unmounted) return
  const defaults = locations.value.filter(item => item.kind === 'drive')
  const first = locations.value.find(item => item.kind === 'home') || defaults[0] || locations.value[0]
  const second = defaults.find(item => item.path !== first?.path) || locations.value.find(item => item.kind === 'known') || first
  if (first && !left.archive && !left.path) await loadPane(left, first.path)
  if (second && !right.archive && !right.path) await loadPane(right, second.path)
  if (import.meta.env.MODE !== 'test') {
    const unlisten = await listen<string[]>('tauri://file-drop', event => { const pane=activePaneState(); if (!pane.archive && event.payload[0]) openInPane(pane,event.payload[0]) })
    if (unmounted) unlisten(); else unlistenDrop=unlisten
  }
})
onUnmounted(() => {
  unmounted = true; unlistenDrop?.()
  paneRequest.left++
  paneRequest.right++
  window.removeEventListener('pointerdown', handleGlobalPointer)
  window.removeEventListener('keydown', handleKeydown)
})

const activeSelectionCount = computed(() => {const pane=activePane.value==='left'?left:right;return pane.archive?pane.archiveSelected:pane.selected.size})
</script>

<template>
  <div ref="managerRoot" class="file-manager h-full min-w-0 flex flex-col gap-3 p-responsive p-6" data-testid="dual-pane-file-manager">
    <header class="manager-header shrink-0 flex items-center justify-between gap-4">
      <div><div class="flex items-center gap-2"><h1 class="text-2xl font-black tracking-tight text-content">双栏文件浏览器</h1><HelpHint label="双栏操作说明">在一栏选择，在另一栏复制、移动、压缩或解压。双击打开文件夹或压缩包，点击眼睛预览文件。不覆盖同名文件，删除的文件进入回收站。</HelpHint></div></div>
      <div class="manager-actions"><slot name="header-actions"/><button data-testid="file-manager-open-archive" :disabled="activePaneState().archiveBusy" @click="chooseArchive"><i class="pi pi-box"></i><span>打开压缩包</span></button></div>
    </header>
    <div v-if="notice" class="notice-bar" data-testid="file-manager-notice"><i class="pi pi-info-circle"></i><span>{{ notice }}</span><button @click="notice = ''"><i class="pi pi-times"></i></button></div>
    <p v-if="savedLocations.error.value" role="alert" class="text-sm text-red-500">{{ savedLocations.error.value }}</p>
    <main class="pane-grid flex-1 min-h-0">
      <section v-for="pane in [left, right]" :key="pane.id" class="file-pane" :data-file-pane="pane.id" :class="{ active: activePane === pane.id }" @pointerdown="activePane = pane.id" @focusin="activePane = pane.id">
        <ArchiveBrowserView v-if="pane.archiveCache" v-show="!!pane.archive" :ref="component=>bindArchiveApi(pane.id,component)" :key="pane.archiveCache.id" embedded :pane-id="pane.id" :locations="locations" :saved-locations="savedLocations" :source="pane.archiveCache" :destination="otherPane(pane).archive ? '' : otherPane(pane).path" :active="activePane===pane.id && !!pane.archive" @leave="leaveArchive(pane)" @navigate-disk="navigateDisk(pane,$event)" @pane-state="state=>{pane.archiveBusy=state.busy;pane.archiveSelected=state.selected}" @extracted="refreshDestination(pane,$event)"/>
        <template v-if="!pane.archive">
        <FilePaneNavigation :pane-id="pane.id" :path="pane.path" :crumbs="browserBreadcrumbs(pane.path)" :folders="pane.entries.filter(item=>item.isDir).length" :files="pane.entries.filter(item=>!item.isDir).length" :locations="locations" :saved="savedLocations" :loading="pane.loading" :selection-mode="pane.selectionMode" :can-back="pane.historyIndex>0" :can-forward="pane.historyIndex<pane.history.length-1 || canResumeArchive(pane)" :can-up="parentPath(pane.path)!==pane.path" @back="navigateHistory(pane,-1)" @forward="navigateHistory(pane,1)" @up="loadPane(pane,parentPath(pane.path))" @refresh="loadPane(pane,pane.path,false)" @create="beginEditor('mkdir',pane)" @selection="toggleSelectionMode(pane)" @tools="openBlankContext(pane,$event)" @navigate="path=>{pane.pathDraft=path;submitPath(pane)}" @breadcrumb="index=>loadPane(pane,browserBreadcrumbs(pane.path)[index].path)"/>
        <FilePaneList :entries="pane.entries.map(entry=>({...entry,icon:icon(entry),preview:!entry.isDir && !!localPreviewKind(entry.name)}))" :selected="pane.selected" :selection-mode="pane.selectionMode" :loading="pane.loading" :error="pane.error" @select="(entry,event)=>toggle(pane,entry,event)" @activate="entry=>activate(pane,entry)" @preview="previewLocal" @context="(entry,event)=>entry ? openContext(pane,entry,event) : openBlankContext(pane,event)">
          <template #error><button v-if="savedLocations.isBookmarked(pane.pathDraft)" :data-testid="`remove-unavailable-bookmark-${pane.id}`" @click="savedLocations.toggleBookmark(pane.pathDraft)">移除此位置的书签（不删除文件）</button></template>
        </FilePaneList>
        <footer><span>{{ pane.selected.size ? `已选择 ${pane.selected.size} 项` : pane.selectionMode ? '多选模式 · 单击切换选择 · Esc 退出' : '单击选择 · Ctrl 多选 · 双击打开 · 右键操作' }}</span><button v-if="pane.selectionMode" @click="toggleSelectionMode(pane)">退出多选</button></footer>
        </template>
      </section>
    </main>
    <footer class="transfer-bar shrink-0">
      <span>当前栏已选 <b>{{ activeSelectionCount }}</b> 项</span>
      <button title="复制到另一栏" :disabled="!activeSelectionCount || busy || activePaneState().archiveBusy || !!otherPane(activePaneState()).archive" @click="activePaneState().archive ? transferArchiveSelection(activePaneState()) : runTransfer('copy', activePaneState())"><i class="pi pi-copy"></i><span>复制到另一栏</span></button>
      <button :title="`移动到另一栏 ${transferDirection(activePaneState())}`" :disabled="!activeSelectionCount || busy || !!activePaneState().archive || !!otherPane(activePaneState()).archive" @click="runTransfer('move', activePaneState())"><i :class="transferIcon(activePaneState())"></i><span>移动到另一栏 {{ transferDirection(activePaneState()) }}</span></button>
      <button title="压缩到另一栏" :disabled="!activeSelectionCount || busy || !!activePaneState().archive || !!otherPane(activePaneState()).archive" @click="compressToOther(activePane === 'left' ? left : right)"><i class="pi pi-box"></i><span>压缩到另一栏</span></button>
    </footer>

    <LocalFilePreview v-if="previewEntry" :entry="previewEntry" @close="previewEntry=null"/>
    <Teleport to="body">
      <section v-if="context" class="file-context" :style="{ left: `${context.x}px`, top: `${context.y}px` }" @pointerdown.stop @contextmenu.prevent>
        <header><i :class="context.entry ? icon(context.entry) : 'pi pi-folder-open'"></i><strong class="truncate">{{ context.entry?.name || context.pane.path }}</strong></header>
        <template v-if="context.entry">
          <button v-if="context.entry.isDir || isArchive(context.entry)" @click="activate(context.pane, context.entry); context = null"><i class="pi pi-folder-open"></i><span>{{ context.entry.isDir ? '在当前栏打开文件夹' : '浏览压缩包' }}</span></button>
          <button v-if="!context.entry.isDir && localPreviewKind(context.entry.name)" data-testid="file-manager-context-preview" @click="previewLocal(context.entry)"><i class="pi pi-eye"/><span>预览文件</span></button>
          <button data-testid="file-manager-open-system" @click="openInSystemManager(context.entry)"><i class="pi pi-external-link"></i><span>{{ context.entry.isDir ? '在文件管理器中打开' : '在文件管理器中定位' }}</span></button>
          <button v-if="context.entry.isDir" data-testid="file-manager-open-folder-other" @click="openInOtherPane(context.pane, context.entry.path)"><i :class="transferIcon(context.pane)"></i><span>在另一栏打开此文件夹 {{ transferDirection(context.pane) }}</span></button>
          <button :disabled="!!otherPane(context.pane).archive" @click="runTransfer('copy', context.pane, context.entry)"><i class="pi pi-copy"></i><span>复制到另一栏 {{ transferDirection(context.pane) }}</span></button>
          <button :disabled="!!otherPane(context.pane).archive" @click="runTransfer('move', context.pane, context.entry)"><i :class="transferIcon(context.pane)"></i><span>移动到另一栏 {{ transferDirection(context.pane) }}</span></button>
          <button :disabled="!!otherPane(context.pane).archive" @click="compressToOther(context.pane, context.entry)"><i class="pi pi-box"></i><span>压缩到另一栏</span></button>
          <button v-if="isArchive(context.entry)" :disabled="!!otherPane(context.pane).archive" @click="extractToOther(context.pane, context.entry)"><i class="pi pi-download"></i><span>解压到另一栏</span></button>
          <div></div>
          <button @click="beginEditor('rename', context.pane, context.entry)"><i class="pi pi-pencil"></i><span>重命名</span></button>
          <button class="danger" @click="beginEditor('delete', context.pane, context.entry)"><i class="pi pi-trash"></i><span>移到回收站</span></button>
          <button @click="showProperties(context.pane, context.entry)"><i class="pi pi-info-circle"></i><span>属性</span></button>
        </template>
        <template v-else>
          <button data-testid="file-manager-open-same-other" @click="openInOtherPane(context.pane)"><i :class="transferIcon(context.pane)"></i><span>另一栏打开相同文件夹 {{ transferDirection(context.pane) }}</span></button>
          <button v-if="!left.archive && !right.archive" @click="swapPaneLocations"><i class="pi pi-arrow-right-arrow-left"></i><span>交换左右栏位置</span></button>
          <button @click="loadPane(context.pane, context.pane.path, false); context = null"><i class="pi pi-refresh"></i><span>刷新当前文件夹</span><kbd>F5</kbd></button>
          <button @click="beginEditor('mkdir', context.pane)"><i class="pi pi-plus"></i><span>新建文件夹</span></button>
          <div></div>
          <button @click="selectAll(context.pane)"><i class="pi pi-check-square"></i><span>全选</span><kbd>Ctrl+A</kbd></button>
          <button @click="toggleSelectionMode(context.pane)"><i :class="context.pane.selectionMode ? 'pi pi-times' : 'pi pi-check-square'"></i><span>{{ context.pane.selectionMode ? '退出多选模式' : '进入多选模式' }}</span></button>
        </template>
      </section>

      <div v-if="editor.type" class="modal-backdrop" @pointerdown.self="editor.type = ''">
        <section class="manager-dialog">
          <h2>{{ editor.type === 'rename' ? '重命名' : editor.type === 'mkdir' ? '新建文件夹' : '移到回收站' }}</h2>
          <p v-if="editor.type === 'delete'">将所选项目移入 Windows 系统回收站，可从回收站恢复。确认继续？</p>
          <input v-else v-model="editor.value" autofocus @keyup.enter="submitEditor">
          <footer><button @click="editor.type = ''">取消</button><button class="primary" :disabled="busy" @click="submitEditor">确认</button></footer>
        </section>
      </div>
      <div v-if="properties" class="modal-backdrop" @pointerdown.self="properties = null">
        <section class="manager-dialog properties-dialog"><h2>属性</h2><dl><div><dt>名称</dt><dd>{{ properties.name }}</dd></div><div><dt>位置</dt><dd>{{ properties.path }}</dd></div><div><dt>类型</dt><dd>{{ properties.isDir ? '文件夹' : '文件' }}</dd></div><div><dt>大小</dt><dd>{{ formatBytes(properties.bytes) }}（{{ properties.bytes }} 字节）</dd></div><div><dt>内容</dt><dd>{{ properties.files }} 个文件，{{ properties.directories }} 个文件夹</dd></div><div><dt>修改时间</dt><dd>{{ formatDate(Number(properties.modifiedUnixMs)) }}</dd></div></dl><footer><button class="primary" @click="properties = null">关闭</button></footer></section>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.manager-header h1 { line-height: 1; }
.file-manager{overflow:hidden}
.status-pill { display:flex; align-items:center; gap:.45rem; padding:.55rem .8rem; border:1px solid var(--border-subtle); border-radius:999px; color:var(--text-muted); background:var(--bg-input); font-size:.66rem; font-weight:800; }
.status-pill i { color:var(--dynamic-accent); }
.manager-actions{display:flex;align-items:center;gap:.55rem}.manager-actions>button{height:2.35rem;padding:0 .75rem;display:flex;align-items:center;gap:.4rem;border-radius:.72rem;background:var(--dynamic-accent);color:white;font-size:.68rem;font-weight:850}
.notice-bar { display:flex; align-items:center; gap:.55rem; padding:.65rem .8rem; border-radius:.8rem; border:1px solid color-mix(in srgb,var(--dynamic-accent) 35%,var(--border-subtle)); background:color-mix(in srgb,var(--dynamic-accent) 8%,var(--bg-card)); color:var(--text-content); font-size:.72rem; font-weight:750; }
.notice-bar span { flex:1; min-width:0; overflow-wrap:anywhere; }.notice-bar i{color:var(--dynamic-accent)}
.pane-grid { display:grid; grid-template-columns:minmax(0,1fr) minmax(0,1fr); gap:.65rem; }
.file-pane { min-width:0; min-height:0; overflow:hidden; display:flex; flex-direction:column; border:1px solid var(--border-subtle); border-radius:1rem; background:color-mix(in srgb,var(--bg-card) 92%,transparent); box-shadow:0 12px 32px rgb(0 0 0 / .08); }
.file-pane.active { border-color:color-mix(in srgb,var(--dynamic-accent) 58%,var(--border-subtle)); box-shadow:0 0 0 1px color-mix(in srgb,var(--dynamic-accent) 18%,transparent),0 12px 32px rgb(0 0 0 / .1); }
.pane-nav { position:relative;display:flex; align-items:center; gap:.3rem; padding:.55rem; border-bottom:1px solid var(--border-subtle); }.pane-nav>button{width:2.1rem;height:2.1rem;border-radius:.6rem;color:var(--text-muted)}.pane-nav>button:hover:not(:disabled){background:var(--bg-input);color:var(--dynamic-accent)}.pane-nav>button:disabled{opacity:.28}.pane-nav>button.selection-mode-button.active{background:color-mix(in srgb,var(--dynamic-accent) 14%,transparent);color:var(--dynamic-accent);box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--dynamic-accent) 38%,transparent)}
.path-combobox{position:relative;display:flex;min-width:5.5rem;height:2.1rem;flex:1;align-items:center;border:1px solid var(--border-subtle);border-radius:.65rem;background:var(--bg-input);transition:border-color .18s ease,box-shadow .18s ease,background-color .18s ease}.path-combobox:focus-within,.path-combobox.open{border-color:color-mix(in srgb,var(--dynamic-accent) 65%,var(--border-subtle));background:color-mix(in srgb,var(--dynamic-accent) 5%,var(--bg-input));box-shadow:0 0 0 3px color-mix(in srgb,var(--dynamic-accent) 10%,transparent)}.path-combobox-icon{flex:0 0 auto;margin-left:.65rem;color:var(--dynamic-accent);font-size:.7rem}.path-combobox input{min-width:0;height:100%;flex:1;border:0;background:transparent;padding:0 .45rem;color:var(--text-content);font-size:.67rem;font-weight:780;outline:none;text-overflow:ellipsis}.path-combobox input::selection{background:color-mix(in srgb,var(--dynamic-accent) 28%,transparent)}.path-combobox .path-combobox-toggle{display:grid;width:2rem;height:100%;flex:0 0 2rem;place-items:center;border-left:1px solid transparent;border-radius:0 .6rem .6rem 0;color:var(--text-muted);font-size:.55rem;transition:color .18s ease,background-color .18s ease}.path-combobox .path-combobox-toggle:hover,.path-combobox .path-combobox-toggle.open{background:color-mix(in srgb,var(--dynamic-accent) 10%,transparent);color:var(--dynamic-accent)}.path-combobox .path-combobox-toggle i{transition:transform .18s ease}.path-combobox .path-combobox-toggle.open i{transform:rotate(180deg)}
.location-menu{position:absolute;z-index:30;top:calc(100% + .45rem);left:0;display:grid;width:max(100%,16rem);max-width:min(23rem,calc(100vw - 5rem));max-height:18rem;gap:.18rem;overflow-x:hidden;overflow-y:auto;border:1px solid color-mix(in srgb,var(--dynamic-accent) 25%,var(--border-subtle));border-radius:.8rem;background:color-mix(in srgb,var(--bg-card) 96%,transparent);padding:.4rem;box-shadow:0 18px 45px rgb(0 0 0 / .28);backdrop-filter:blur(18px)}.location-menu-heading{display:flex;align-items:center;justify-content:space-between;gap:.7rem;padding:.4rem .5rem .5rem;color:var(--text-content);font-size:.6rem;font-weight:900}.location-menu-heading small{color:var(--text-muted);font-size:.52rem;font-weight:700}.location-menu>button{display:grid;width:100%;min-height:2.45rem;grid-template-columns:1.5rem minmax(0,1fr) 1rem;align-items:center;gap:.45rem;border-radius:.58rem;padding:.35rem .45rem;color:var(--text-muted);text-align:left;transition:background-color .15s ease,color .15s ease}.location-menu>button:hover,.location-menu>button.selected{background:color-mix(in srgb,var(--dynamic-accent) 10%,transparent);color:var(--dynamic-accent)}.location-menu>button>i:first-child{display:grid;width:1.5rem;height:1.5rem;place-items:center;border-radius:.45rem;background:color-mix(in srgb,var(--dynamic-accent) 9%,transparent);color:var(--dynamic-accent);font-size:.67rem}.location-menu>button span{display:block;min-width:0}.location-menu strong,.location-menu small{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.location-menu strong{color:var(--text-content);font-size:.65rem}.location-menu small{margin-top:.08rem;color:var(--text-muted);font-size:.54rem;font-weight:650}.location-menu .selected-mark{color:var(--dynamic-accent);font-size:.58rem}
.path-strip { display:grid; grid-template-columns:auto minmax(0,1fr) auto; align-items:center; gap:.45rem; padding:.42rem .75rem; color:var(--text-muted); font-size:.64rem; border-bottom:1px solid color-mix(in srgb,var(--border-subtle) 70%,transparent); }.path-strip>i{color:var(--dynamic-accent)}.path-breadcrumbs{display:flex;min-width:0;align-items:center;gap:.1rem;overflow-x:auto;overflow-y:hidden;scrollbar-width:none}.path-breadcrumbs::-webkit-scrollbar{display:none}.path-breadcrumbs>i{flex:0 0 auto;color:var(--text-muted);font-size:.5rem}.path-breadcrumbs button{flex:0 0 auto;max-width:9rem;overflow:hidden;border-radius:.45rem;padding:.28rem .38rem;color:var(--text-muted);font-size:.63rem;font-weight:800;text-overflow:ellipsis;white-space:nowrap}.path-breadcrumbs button:hover,.path-breadcrumbs button.current{background:color-mix(in srgb,var(--dynamic-accent) 9%,transparent);color:var(--dynamic-accent)}
.file-head,.file-row { box-sizing:border-box;display:grid;grid-template-columns:minmax(9rem,1fr) 5rem 8rem;gap:.5rem;align-items:center; }.file-head{padding:.5rem .75rem;border-bottom:1px solid var(--border-subtle);color:var(--text-muted);font-size:.62rem;font-weight:900}.file-list{flex:1;min-height:0;overflow-y:auto;overflow-x:hidden}.file-row{width:100%;padding:.52rem .75rem;border-bottom:1px solid color-mix(in srgb,var(--border-subtle) 55%,transparent);color:var(--text-muted);font-size:.64rem;text-align:left}.file-row>span{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.file-row:hover{background:color-mix(in srgb,var(--dynamic-accent) 6%,transparent)}.file-row.selected{background:color-mix(in srgb,var(--dynamic-accent) 13%,transparent);outline:1px solid color-mix(in srgb,var(--dynamic-accent) 48%,transparent);outline-offset:-1px}.file-name{min-width:0;display:flex;align-items:center;gap:.6rem}.file-name>i{width:1.2rem;color:var(--dynamic-accent);font-size:1rem}.file-name strong,.file-name small{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.file-name strong{color:var(--text-content);font-size:.72rem}.file-name small{margin-top:.12rem;color:var(--text-muted);font-size:.54rem}.pane-empty{height:100%;display:grid;place-items:center;align-content:center;gap:.6rem;color:var(--text-muted);font-size:.7rem}.pane-empty i{font-size:1.6rem;color:var(--dynamic-accent)}.pane-empty.error{padding:2rem;text-align:center;color:#ef4444}.file-pane>footer{display:flex;min-height:2rem;align-items:center;justify-content:space-between;gap:.5rem;padding:.4rem .7rem;border-top:1px solid var(--border-subtle);color:var(--text-muted);font-size:.6rem;font-weight:700}.file-pane>footer button{flex:0 0 auto;border-radius:.45rem;background:color-mix(in srgb,var(--dynamic-accent) 10%,transparent);padding:.25rem .45rem;color:var(--dynamic-accent);font-weight:850}
.transfer-bar{display:flex;align-items:center;justify-content:flex-end;gap:.5rem}.transfer-bar>span{margin-right:auto;color:var(--text-muted);font-size:.7rem}.transfer-bar b{color:var(--dynamic-accent)}.transfer-bar button{height:2.45rem;padding:0 .8rem;border:1px solid var(--border-subtle);border-radius:.72rem;background:var(--bg-input);color:var(--text-content);font-size:.68rem;font-weight:850}.transfer-bar button:hover:not(:disabled){border-color:var(--dynamic-accent);color:var(--dynamic-accent)}.transfer-bar button:disabled{opacity:.35}
@media(max-width:900px){.file-manager{overflow:hidden;padding:.65rem;gap:.55rem}.manager-header{gap:.65rem}.manager-header h1{font-size:1.15rem}.manager-header p{margin-top:.2rem;font-size:.6rem}.status-pill{display:none}.pane-grid{grid-template-columns:minmax(0,1fr) minmax(0,1fr);grid-template-rows:minmax(0,1fr);gap:.45rem}.pane-nav{overflow:visible}.pane-nav>button{flex:0 0 1.85rem;width:1.85rem;height:1.85rem}.path-combobox{height:1.85rem}.file-head,.file-row{grid-template-columns:minmax(0,1fr) 3.9rem}.file-head span:last-child,.file-row>span:last-child{display:none}.file-pane>footer span{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.transfer-bar{gap:.35rem}.transfer-bar button{height:2.15rem;padding:0 .55rem}}
@media(max-width:620px){.manager-actions>button{width:2.2rem;padding:0;justify-content:center}.manager-actions>button span{display:none}.file-head,.file-row{grid-template-columns:minmax(0,1fr)}.file-head span:nth-child(n+2),.file-row>span:nth-child(n+2){display:none}.path-strip{grid-template-columns:auto minmax(0,1fr)}.path-strip>span{display:none}.transfer-bar>span{display:none}.transfer-bar button{width:2.15rem;padding:0;justify-content:center}.transfer-bar button span{display:none}.transfer-bar button i{font-size:.8rem}.file-pane>footer{padding:.35rem .45rem}.file-pane>footer button{display:none}}
@media(max-width:1100px){.pane-nav{flex-wrap:wrap}.pane-nav>.path-combobox{order:2;flex:1 0 100%;min-width:0}.pane-nav>button{flex-shrink:0}}
@media(min-width:901px) and (max-width:1100px){.file-head,.file-row{grid-template-columns:minmax(0,1fr) 4rem 6rem}}
.file-pane{container:file-pane / inline-size}.file-pane>.archive-pane-browser{flex:1;min-height:0}.file-row{cursor:pointer}.file-row:focus-visible{outline:2px solid var(--dynamic-accent);outline-offset:-2px}.file-preview-trigger{display:grid;place-items:center;flex-shrink:0;width:25px;height:28px;margin-left:auto;border-radius:7px;color:var(--dynamic-accent);background:color-mix(in srgb,var(--dynamic-accent) 9%,transparent)}.file-preview-trigger:hover{background:color-mix(in srgb,var(--dynamic-accent) 18%,transparent)}.file-name>.truncate{min-width:0;flex:1}.file-context button:disabled{opacity:.4;cursor:default}
</style>

<style>
.file-context { position:fixed;z-index:1000;width:15rem;padding:.45rem;border:1px solid var(--border-subtle);border-radius:.9rem;background:var(--bg-card);box-shadow:0 18px 55px rgb(0 0 0 / .25); }.file-context header{display:flex;align-items:center;gap:.55rem;padding:.65rem;color:var(--text-content)}.file-context header i{color:var(--dynamic-accent)}.file-context button{width:100%;display:flex;align-items:center;gap:.65rem;padding:.62rem .7rem;border-radius:.58rem;color:var(--text-content);font-size:.7rem;font-weight:750;text-align:left}.file-context button:hover{background:color-mix(in srgb,var(--dynamic-accent) 10%,transparent);color:var(--dynamic-accent)}.file-context button i{width:1rem}.file-context button span{min-width:0;flex:1}.file-context button kbd{margin-left:auto;color:var(--text-muted);font-size:.56rem}.file-context button.danger{color:#ef4444}.file-context>div{height:1px;margin:.35rem;background:var(--border-subtle)}
.modal-backdrop{position:fixed;inset:0;z-index:1100;display:grid;place-items:center;padding:2rem;background:rgb(0 0 0 / .42);backdrop-filter:blur(6px)}.manager-dialog{width:min(28rem,90vw);padding:1.2rem;border:1px solid var(--border-subtle);border-radius:1rem;background:var(--bg-card);box-shadow:0 22px 70px rgb(0 0 0 / .3)}.manager-dialog h2{color:var(--text-content);font-size:1rem;font-weight:900}.manager-dialog p{margin-top:.8rem;color:var(--text-muted);font-size:.75rem;line-height:1.6}.manager-dialog input{width:100%;height:2.6rem;margin-top:.8rem;padding:0 .75rem;border:1px solid var(--border-subtle);border-radius:.7rem;background:var(--bg-input);color:var(--text-content);outline:none}.manager-dialog input:focus{border-color:var(--dynamic-accent)}.manager-dialog footer{display:flex;justify-content:flex-end;gap:.5rem;margin-top:1rem}.manager-dialog footer button{padding:.6rem .9rem;border-radius:.65rem;background:var(--bg-input);color:var(--text-muted);font-size:.7rem;font-weight:850}.manager-dialog footer .primary{background:var(--dynamic-accent);color:white}.properties-dialog dl{margin-top:.8rem}.properties-dialog dl div{display:grid;grid-template-columns:5rem minmax(0,1fr);gap:.7rem;padding:.5rem 0;border-bottom:1px solid var(--border-subtle);font-size:.7rem}.properties-dialog dt{color:var(--text-muted)}.properties-dialog dd{color:var(--text-content);overflow-wrap:anywhere}
</style>
