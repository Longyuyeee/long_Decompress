<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { open } from '@tauri-apps/api/dialog'

export interface LocalFileEntry {
  path: string
  name: string
  size: number
  isDir: boolean
  extension?: string | null
  modified?: string | number | null
}

interface Location { name: string; path: string; kind: string }
interface PaneState {
  id: 'left' | 'right'
  path: string
  entries: LocalFileEntry[]
  selected: Set<string>
  history: string[]
  historyIndex: number
  selectionMode: boolean
  loading: boolean
  error: string
}

const emit = defineEmits<{
  openArchive: [path: string]
  compress: [entries: LocalFileEntry[], destination: string]
  extract: [paths: string[], destination: string]
}>()

const locations = ref<Location[]>([])
const activePane = ref<'left' | 'right'>('left')
const busy = ref(false)
const notice = ref('')
const context = ref<{ pane: PaneState; entry: LocalFileEntry | null; x: number; y: number } | null>(null)
const properties = ref<any | null>(null)
const editor = reactive<{ type: '' | 'rename' | 'mkdir' | 'delete'; pane: PaneState | null; entry: LocalFileEntry | null; value: string }>({ type: '', pane: null, entry: null, value: '' })

const createPane = (id: 'left' | 'right'): PaneState => reactive({ id, path: '', entries: [], selected: new Set<string>(), history: [], historyIndex: -1, selectionMode: false, loading: false, error: '' })
const left = createPane('left')
const right = createPane('right')
const otherPane = (pane: PaneState) => pane.id === 'left' ? right : left
const isArchive = (entry: LocalFileEntry) => /\.(zip|7z|rar|tar|gz|tgz|bz2|xz|zst|cab|iso|wim)$/i.test(entry.name)
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
  pane.loading = true
  pane.error = ''
  try {
    const raw = await invoke<any[]>('list_files', { path })
    pane.entries = sortEntries(raw.map(normalizeEntry))
    pane.path = path
    pane.selected = new Set()
    if (pushHistory && pane.history[pane.historyIndex] !== path) {
      pane.history = pane.history.slice(0, pane.historyIndex + 1)
      pane.history.push(path)
      pane.historyIndex = pane.history.length - 1
    }
  } catch (error) {
    pane.error = String(error)
  } finally { pane.loading = false }
}

const navigateHistory = (pane: PaneState, delta: number) => {
  const index = pane.historyIndex + delta
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

const activate = (pane: PaneState, entry: LocalFileEntry) => {
  activePane.value = pane.id
  if (entry.isDir) void loadPane(pane, entry.path)
  else if (isArchive(entry)) emit('openArchive', entry.path)
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
  if (!entries.length || busy.value) return
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
  const entries = operationEntries(pane, entry)
  if (entries.length) emit('compress', entries, otherPane(pane).path)
  context.value = null
}

const extractToOther = (pane: PaneState, entry?: LocalFileEntry) => {
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
    ? await open({ multiple: false, filters: [{ name: '压缩包', extensions: ['zip', '7z', 'rar', 'tar', 'gz', 'tgz', 'bz2', 'xz', 'zst', 'cab', 'iso', 'wim'] }] })
    : Array.isArray(queued) ? queued[0] : queued
  if (typeof selected === 'string') emit('openArchive', selected)
}

const activePaneState = () => activePane.value === 'left' ? left : right
const handleGlobalPointer = () => { context.value = null }
const handleKeydown = (event: KeyboardEvent) => {
  const target = event.target
  const isEditable = target instanceof Element && target.matches('input, textarea, select, [contenteditable="true"]')
  if (isEditable) return
  const pane = activePaneState()
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
  const defaults = locations.value.filter(item => item.kind === 'drive')
  const first = locations.value.find(item => item.kind === 'home') || defaults[0] || locations.value[0]
  const second = defaults.find(item => item.path !== first?.path) || locations.value.find(item => item.kind === 'known') || first
  if (first) await loadPane(left, first.path)
  if (second) await loadPane(right, second.path)
})
onUnmounted(() => {
  window.removeEventListener('pointerdown', handleGlobalPointer)
  window.removeEventListener('keydown', handleKeydown)
})

const activeSelectionCount = computed(() => (activePane.value === 'left' ? left : right).selected.size)
</script>

<template>
  <div class="file-manager h-full min-w-0 flex flex-col gap-3 p-responsive p-6" data-testid="dual-pane-file-manager">
    <header class="manager-header shrink-0 flex items-center justify-between gap-4">
      <div><h1 class="text-2xl font-black tracking-tight text-content">双栏文件浏览器</h1><p class="mt-1 text-xs font-bold text-muted">浏览电脑中的所有文件；在一栏选择，在另一栏落地</p></div>
      <div class="manager-actions"><div class="status-pill"><i class="pi pi-shield"></i><span>不覆盖同名项 · 复制后哈希校验 · 删除进入回收站</span></div><button data-testid="file-manager-open-archive" @click="chooseArchive"><i class="pi pi-box"></i><span>打开压缩包</span></button></div>
    </header>
    <div v-if="notice" class="notice-bar" data-testid="file-manager-notice"><i class="pi pi-info-circle"></i><span>{{ notice }}</span><button @click="notice = ''"><i class="pi pi-times"></i></button></div>
    <main class="pane-grid flex-1 min-h-0">
      <section v-for="pane in [left, right]" :key="pane.id" class="file-pane" :class="{ active: activePane === pane.id }" @pointerdown="activePane = pane.id">
        <nav class="pane-nav">
          <button :disabled="pane.historyIndex <= 0" title="后退" @click="navigateHistory(pane, -1)"><i class="pi pi-arrow-left"></i></button>
          <button :disabled="pane.historyIndex >= pane.history.length - 1" title="前进" @click="navigateHistory(pane, 1)"><i class="pi pi-arrow-right"></i></button>
          <button :disabled="parentPath(pane.path) === pane.path" title="上一级" @click="loadPane(pane, parentPath(pane.path))"><i class="pi pi-arrow-up"></i></button>
          <select :value="pane.path" title="常用位置与磁盘" @change="loadPane(pane, ($event.target as HTMLSelectElement).value)">
            <option v-for="location in locations" :key="`${pane.id}-${location.path}`" :value="location.path">{{ location.name }} · {{ location.path }}</option>
            <option v-if="!locations.some(item => item.path === pane.path)" :value="pane.path">{{ pane.path }}</option>
          </select>
          <button title="刷新" @click="loadPane(pane, pane.path, false)"><i :class="pane.loading ? 'pi pi-spin pi-spinner' : 'pi pi-refresh'"></i></button>
          <button title="新建文件夹" @click="beginEditor('mkdir', pane)"><i class="pi pi-plus"></i></button>
          <button class="selection-mode-button" :class="{ active: pane.selectionMode }" :title="pane.selectionMode ? '退出多选模式（Esc）' : '进入多选模式'" :aria-pressed="pane.selectionMode" :data-testid="`file-manager-selection-mode-${pane.id}`" @click="toggleSelectionMode(pane)"><i :class="pane.selectionMode ? 'pi pi-times' : 'pi pi-check-square'"></i></button>
        </nav>
        <div class="path-strip" :title="pane.path">
          <i class="pi pi-folder-open"></i>
          <nav class="path-breadcrumbs" :aria-label="`${pane.id === 'left' ? '左' : '右'}栏路径`" :data-testid="`file-manager-breadcrumbs-${pane.id}`">
            <template v-for="(crumb, index) in breadcrumbs(pane.path)" :key="crumb.path">
              <i v-if="index" class="pi pi-angle-right"></i>
              <button :class="{ current: index === breadcrumbs(pane.path).length - 1 }" :title="crumb.path" @click="loadPane(pane, crumb.path)">{{ crumb.name }}</button>
            </template>
          </nav>
          <span>{{ pane.entries.filter(item => item.isDir).length }} 文件夹 · {{ pane.entries.filter(item => !item.isDir).length }} 文件</span>
        </div>
        <div class="file-head"><span>名称</span><span>大小</span><span>修改时间</span></div>
        <div class="file-list custom-scrollbar" @contextmenu="openBlankContext(pane, $event)">
          <button v-for="entry in pane.entries" :key="entry.path" class="file-row" :class="{ selected: pane.selected.has(entry.path) }" :data-path="entry.path" @click="toggle(pane, entry, $event)" @dblclick="activate(pane, entry)" @contextmenu.stop="openContext(pane, entry, $event)">
            <span class="file-name"><i :class="icon(entry)"></i><span class="truncate"><strong>{{ entry.name }}</strong><small>{{ entry.isDir ? '文件夹' : (entry.extension || '文件').toUpperCase() }}</small></span></span>
            <span>{{ entry.isDir ? '—' : formatBytes(entry.size) }}</span><span>{{ formatDate(entry.modified) }}</span>
          </button>
          <div v-if="pane.loading" class="pane-empty"><i class="pi pi-spin pi-spinner"></i><span>正在读取…</span></div>
          <div v-else-if="pane.error" class="pane-empty error"><i class="pi pi-exclamation-triangle"></i><span>{{ pane.error }}</span></div>
          <div v-else-if="!pane.entries.length" class="pane-empty"><i class="pi pi-folder"></i><span>此文件夹为空</span></div>
        </div>
        <footer><span>{{ pane.selected.size ? `已选择 ${pane.selected.size} 项` : pane.selectionMode ? '多选模式 · 单击切换选择 · Esc 退出' : '单击选择 · Ctrl 多选 · 双击打开 · 右键操作' }}</span><button v-if="pane.selectionMode" @click="toggleSelectionMode(pane)">退出多选</button></footer>
      </section>
    </main>
    <footer class="transfer-bar shrink-0">
      <span>当前栏已选 <b>{{ activeSelectionCount }}</b> 项</span>
      <button :disabled="!activeSelectionCount || busy" @click="runTransfer('copy', activePane === 'left' ? left : right)"><i class="pi pi-copy"></i>复制到另一栏</button>
      <button :disabled="!activeSelectionCount || busy" @click="runTransfer('move', activePaneState())"><i :class="transferIcon(activePaneState())"></i>移动到另一栏 {{ transferDirection(activePaneState()) }}</button>
      <button :disabled="!activeSelectionCount || busy" @click="compressToOther(activePane === 'left' ? left : right)"><i class="pi pi-box"></i>压缩到另一栏</button>
    </footer>

    <Teleport to="body">
      <section v-if="context" class="file-context" :style="{ left: `${context.x}px`, top: `${context.y}px` }" @pointerdown.stop @contextmenu.prevent>
        <header><i :class="context.entry ? icon(context.entry) : 'pi pi-folder-open'"></i><strong class="truncate">{{ context.entry?.name || context.pane.path }}</strong></header>
        <template v-if="context.entry">
          <button v-if="context.entry.isDir || isArchive(context.entry)" @click="activate(context.pane, context.entry); context = null"><i class="pi pi-folder-open"></i><span>{{ context.entry.isDir ? '在当前栏打开文件夹' : '浏览压缩包' }}</span></button>
          <button data-testid="file-manager-open-system" @click="openInSystemManager(context.entry)"><i class="pi pi-external-link"></i><span>{{ context.entry.isDir ? '在文件管理器中打开' : '在文件管理器中定位' }}</span></button>
          <button v-if="context.entry.isDir" data-testid="file-manager-open-folder-other" @click="openInOtherPane(context.pane, context.entry.path)"><i :class="transferIcon(context.pane)"></i><span>在另一栏打开此文件夹 {{ transferDirection(context.pane) }}</span></button>
          <button @click="runTransfer('copy', context.pane, context.entry)"><i class="pi pi-copy"></i><span>复制到另一栏 {{ transferDirection(context.pane) }}</span></button>
          <button @click="runTransfer('move', context.pane, context.entry)"><i :class="transferIcon(context.pane)"></i><span>移动到另一栏 {{ transferDirection(context.pane) }}</span></button>
          <button @click="compressToOther(context.pane, context.entry)"><i class="pi pi-box"></i><span>压缩到另一栏</span></button>
          <button v-if="isArchive(context.entry)" @click="extractToOther(context.pane, context.entry)"><i class="pi pi-download"></i><span>解压到另一栏</span></button>
          <div></div>
          <button @click="beginEditor('rename', context.pane, context.entry)"><i class="pi pi-pencil"></i><span>重命名</span></button>
          <button class="danger" @click="beginEditor('delete', context.pane, context.entry)"><i class="pi pi-trash"></i><span>移到回收站</span></button>
          <button @click="showProperties(context.pane, context.entry)"><i class="pi pi-info-circle"></i><span>属性</span></button>
        </template>
        <template v-else>
          <button data-testid="file-manager-open-same-other" @click="openInOtherPane(context.pane)"><i :class="transferIcon(context.pane)"></i><span>另一栏打开相同文件夹 {{ transferDirection(context.pane) }}</span></button>
          <button @click="swapPaneLocations"><i class="pi pi-arrow-right-arrow-left"></i><span>交换左右栏位置</span></button>
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
.pane-nav { display:flex; align-items:center; gap:.3rem; padding:.55rem; border-bottom:1px solid var(--border-subtle); }.pane-nav button{width:2.1rem;height:2.1rem;border-radius:.6rem;color:var(--text-muted)}.pane-nav button:hover:not(:disabled){background:var(--bg-input);color:var(--dynamic-accent)}.pane-nav button:disabled{opacity:.28}.pane-nav button.selection-mode-button.active{background:color-mix(in srgb,var(--dynamic-accent) 14%,transparent);color:var(--dynamic-accent);box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--dynamic-accent) 38%,transparent)}.pane-nav select{min-width:0;flex:1;height:2.1rem;padding:0 .55rem;border:1px solid var(--border-subtle);border-radius:.6rem;background:var(--bg-input);color:var(--text-content);font-size:.68rem;font-weight:750}
.path-strip { display:grid; grid-template-columns:auto minmax(0,1fr) auto; align-items:center; gap:.45rem; padding:.42rem .75rem; color:var(--text-muted); font-size:.64rem; border-bottom:1px solid color-mix(in srgb,var(--border-subtle) 70%,transparent); }.path-strip>i{color:var(--dynamic-accent)}.path-breadcrumbs{display:flex;min-width:0;align-items:center;gap:.1rem;overflow-x:auto;overflow-y:hidden;scrollbar-width:none}.path-breadcrumbs::-webkit-scrollbar{display:none}.path-breadcrumbs>i{flex:0 0 auto;color:var(--text-muted);font-size:.5rem}.path-breadcrumbs button{flex:0 0 auto;max-width:9rem;overflow:hidden;border-radius:.45rem;padding:.28rem .38rem;color:var(--text-muted);font-size:.63rem;font-weight:800;text-overflow:ellipsis;white-space:nowrap}.path-breadcrumbs button:hover,.path-breadcrumbs button.current{background:color-mix(in srgb,var(--dynamic-accent) 9%,transparent);color:var(--dynamic-accent)}
.file-head,.file-row { box-sizing:border-box;display:grid;grid-template-columns:minmax(9rem,1fr) 5rem 8rem;gap:.5rem;align-items:center; }.file-head{padding:.5rem .75rem;border-bottom:1px solid var(--border-subtle);color:var(--text-muted);font-size:.62rem;font-weight:900}.file-list{flex:1;min-height:0;overflow-y:auto;overflow-x:hidden}.file-row{width:100%;padding:.52rem .75rem;border-bottom:1px solid color-mix(in srgb,var(--border-subtle) 55%,transparent);color:var(--text-muted);font-size:.64rem;text-align:left}.file-row>span{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.file-row:hover{background:color-mix(in srgb,var(--dynamic-accent) 6%,transparent)}.file-row.selected{background:color-mix(in srgb,var(--dynamic-accent) 13%,transparent);outline:1px solid color-mix(in srgb,var(--dynamic-accent) 48%,transparent);outline-offset:-1px}.file-name{min-width:0;display:flex;align-items:center;gap:.6rem}.file-name>i{width:1.2rem;color:var(--dynamic-accent);font-size:1rem}.file-name strong,.file-name small{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.file-name strong{color:var(--text-content);font-size:.72rem}.file-name small{margin-top:.12rem;color:var(--text-muted);font-size:.54rem}.pane-empty{height:100%;display:grid;place-items:center;align-content:center;gap:.6rem;color:var(--text-muted);font-size:.7rem}.pane-empty i{font-size:1.6rem;color:var(--dynamic-accent)}.pane-empty.error{padding:2rem;text-align:center;color:#ef4444}.file-pane>footer{display:flex;min-height:2rem;align-items:center;justify-content:space-between;gap:.5rem;padding:.4rem .7rem;border-top:1px solid var(--border-subtle);color:var(--text-muted);font-size:.6rem;font-weight:700}.file-pane>footer button{flex:0 0 auto;border-radius:.45rem;background:color-mix(in srgb,var(--dynamic-accent) 10%,transparent);padding:.25rem .45rem;color:var(--dynamic-accent);font-weight:850}
.transfer-bar{display:flex;align-items:center;justify-content:flex-end;gap:.5rem}.transfer-bar>span{margin-right:auto;color:var(--text-muted);font-size:.7rem}.transfer-bar b{color:var(--dynamic-accent)}.transfer-bar button{height:2.45rem;padding:0 .8rem;border:1px solid var(--border-subtle);border-radius:.72rem;background:var(--bg-input);color:var(--text-content);font-size:.68rem;font-weight:850}.transfer-bar button:hover:not(:disabled){border-color:var(--dynamic-accent);color:var(--dynamic-accent)}.transfer-bar button:disabled{opacity:.35}
@media(max-width:900px){.file-manager{overflow:auto}.pane-grid{grid-template-columns:1fr;grid-template-rows:30rem 30rem}.status-pill{display:none}.transfer-bar{flex-wrap:wrap}.file-head,.file-row{grid-template-columns:minmax(8rem,1fr) 4.5rem}.file-head span:last-child,.file-row>span:last-child{display:none}}
</style>

<style>
.file-context { position:fixed;z-index:1000;width:15rem;padding:.45rem;border:1px solid var(--border-subtle);border-radius:.9rem;background:var(--bg-card);box-shadow:0 18px 55px rgb(0 0 0 / .25); }.file-context header{display:flex;align-items:center;gap:.55rem;padding:.65rem;color:var(--text-content)}.file-context header i{color:var(--dynamic-accent)}.file-context button{width:100%;display:flex;align-items:center;gap:.65rem;padding:.62rem .7rem;border-radius:.58rem;color:var(--text-content);font-size:.7rem;font-weight:750;text-align:left}.file-context button:hover{background:color-mix(in srgb,var(--dynamic-accent) 10%,transparent);color:var(--dynamic-accent)}.file-context button i{width:1rem}.file-context button span{min-width:0;flex:1}.file-context button kbd{margin-left:auto;color:var(--text-muted);font-size:.56rem}.file-context button.danger{color:#ef4444}.file-context>div{height:1px;margin:.35rem;background:var(--border-subtle)}
.modal-backdrop{position:fixed;inset:0;z-index:1100;display:grid;place-items:center;padding:2rem;background:rgb(0 0 0 / .42);backdrop-filter:blur(6px)}.manager-dialog{width:min(28rem,90vw);padding:1.2rem;border:1px solid var(--border-subtle);border-radius:1rem;background:var(--bg-card);box-shadow:0 22px 70px rgb(0 0 0 / .3)}.manager-dialog h2{color:var(--text-content);font-size:1rem;font-weight:900}.manager-dialog p{margin-top:.8rem;color:var(--text-muted);font-size:.75rem;line-height:1.6}.manager-dialog input{width:100%;height:2.6rem;margin-top:.8rem;padding:0 .75rem;border:1px solid var(--border-subtle);border-radius:.7rem;background:var(--bg-input);color:var(--text-content);outline:none}.manager-dialog input:focus{border-color:var(--dynamic-accent)}.manager-dialog footer{display:flex;justify-content:flex-end;gap:.5rem;margin-top:1rem}.manager-dialog footer button{padding:.6rem .9rem;border-radius:.65rem;background:var(--bg-input);color:var(--text-muted);font-size:.7rem;font-weight:850}.manager-dialog footer .primary{background:var(--dynamic-accent);color:white}.properties-dialog dl{margin-top:.8rem}.properties-dialog dl div{display:grid;grid-template-columns:5rem minmax(0,1fr);gap:.7rem;padding:.5rem 0;border-bottom:1px solid var(--border-subtle);font-size:.7rem}.properties-dialog dt{color:var(--text-muted)}.properties-dialog dd{color:var(--text-content);overflow-wrap:anywhere}
</style>
