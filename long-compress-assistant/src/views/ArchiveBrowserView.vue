<script setup lang="ts">
import { computed, ref } from 'vue'
import { useAppStore } from '@/stores/app'
import { useTauriCommands, type ArchiveBrowseResult, type ArchiveEntryInfo } from '@/composables/useTauriCommands'

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
const loading = ref(false)
const extracting = ref(false)

const extensionGroups: Record<string, Set<string>> = {
  image: new Set(['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'svg', 'ico', 'avif']),
  document: new Set(['txt', 'md', 'pdf', 'doc', 'docx', 'xls', 'xlsx', 'ppt', 'pptx', 'csv', 'json', 'xml']),
  archive: new Set(['zip', '7z', 'rar', 'tar', 'gz', 'bz2', 'xz', 'zst', 'iso', 'cab'])
}

const files = computed(() => result.value?.entries.filter(entry => !entry.isDir) ?? [])
const directories = computed(() => {
  const values = new Set<string>()
  files.value.forEach(entry => {
    const parts = entry.path.replace(/\\/g, '/').split('/')
    parts.pop()
    for (let index = 1; index <= parts.length; index++) values.add(parts.slice(0, index).join('/'))
  })
  return [...values].sort((a, b) => a.localeCompare(b))
})

const filteredEntries = computed(() => {
  const search = query.value.trim().toLocaleLowerCase()
  return files.value.filter(entry => {
    const normalized = entry.path.replace(/\\/g, '/')
    const parent = normalized.includes('/') ? normalized.slice(0, normalized.lastIndexOf('/')) : ''
    if (activeDirectory.value && parent !== activeDirectory.value) return false
    if (search && !normalized.toLocaleLowerCase().includes(search)) return false
    if (typeFilter.value === 'all') return true
    const extension = entry.name.includes('.') ? entry.name.split('.').pop()!.toLocaleLowerCase() : ''
    if (typeFilter.value === 'other') {
      return !Object.values(extensionGroups).some(group => group.has(extension))
    }
    return extensionGroups[typeFilter.value]?.has(extension) ?? false
  })
})

const visibleSelected = computed(() => filteredEntries.value.length > 0 && filteredEntries.value.every(entry => selected.value.has(entry.path)))

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

const chooseArchive = async () => {
  const picked = await commands.selectFiles(false)
  if (!picked[0]) return
  archivePath.value = picked[0].path
  outputPath.value = parentDirectory(picked[0].path)
  await loadArchive()
}

const loadArchive = async () => {
  if (!archivePath.value || loading.value) return
  loading.value = true
  result.value = null
  selected.value = new Set()
  activeDirectory.value = ''
  try {
    result.value = await commands.browseArchive(archivePath.value, password.value)
    selected.value = new Set(result.value.entries.filter(entry => !entry.isDir).map(entry => entry.path))
  } catch (error) {
    appStore.setError(String(error))
  } finally {
    loading.value = false
  }
}

const chooseOutput = async () => {
  const picked = await commands.selectDirectory(outputPath.value || undefined)
  if (typeof picked === 'string') outputPath.value = picked
}

const toggleEntry = (entry: ArchiveEntryInfo) => {
  const next = new Set(selected.value)
  next.has(entry.path) ? next.delete(entry.path) : next.add(entry.path)
  selected.value = next
}

const toggleVisible = () => {
  const next = new Set(selected.value)
  filteredEntries.value.forEach(entry => visibleSelected.value ? next.delete(entry.path) : next.add(entry.path))
  selected.value = next
}

const extractSelected = async () => {
  if (!archivePath.value || !outputPath.value || selected.value.size === 0 || extracting.value) return
  extracting.value = true
  try {
    await commands.decompressFile(archivePath.value, {
      outputPath: outputPath.value,
      password: password.value || undefined,
      keepStructure: true,
      overwrite: false,
      deleteAfter: false,
      preserveTimestamps: true,
      selectedEntries: [...selected.value],
      conflictPolicy: 'rename'
    })
    appStore.setSuccess(`已解压 ${selected.value.size} 个所选文件`)
  } catch (error) {
    appStore.setError(String(error))
  } finally {
    extracting.value = false
  }
}
</script>

<template>
  <div class="browser-page h-full min-w-0 overflow-hidden p-responsive p-8 flex flex-col gap-5">
    <header class="shrink-0 flex flex-wrap items-end justify-between gap-4">
      <div class="min-w-0">
        <h1 class="text-4xl font-black text-content tracking-tighter">压缩包浏览中心</h1>
        <p class="text-muted text-sm font-bold mt-2">查看目录、搜索筛选，并只解压需要的文件</p>
      </div>
      <button class="browser-primary" type="button" @click="chooseArchive">
        <i class="pi pi-folder-open"></i><span>打开压缩包</span>
      </button>
    </header>

    <section class="aero-card shrink-0 p-4 grid gap-3 browser-toolbar">
      <div class="min-w-0 browser-field">
        <span class="browser-label">压缩包</span>
        <button class="browser-input text-left truncate" type="button" @click="chooseArchive">{{ archivePath || '选择需要浏览的压缩包' }}</button>
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
      <div class="min-w-0 browser-field">
        <span class="browser-label">解压到</span>
        <button class="browser-input text-left truncate" type="button" @click="chooseOutput">{{ outputPath || '选择输出目录' }}</button>
      </div>
    </section>

    <div v-if="loading" class="aero-card flex-1 min-h-0 grid place-items-center">
      <div class="text-center text-muted"><i class="pi pi-spin pi-spinner text-primary text-3xl"></i><p class="mt-4 font-bold">正在读取压缩包结构…</p></div>
    </div>

    <div v-else-if="!result" class="aero-card flex-1 min-h-0 grid place-items-center border-dashed">
      <div class="max-w-sm text-center px-6"><i class="pi pi-list text-primary text-5xl"></i><h2 class="mt-5 text-xl font-black text-content">先打开一个压缩包</h2><p class="mt-2 text-sm text-muted leading-6">内容只在本机读取。密码 ZIP、7Z、RAR 的元数据由进程内引擎处理，不写入命令行。</p></div>
    </div>

    <template v-else>
      <section class="shrink-0 grid grid-cols-2 md:grid-cols-4 gap-3">
        <div class="browser-stat"><span>格式</span><strong>{{ result.format }}</strong></div>
        <div class="browser-stat"><span>文件</span><strong>{{ result.totalFiles }}</strong></div>
        <div class="browser-stat"><span>展开大小</span><strong>{{ formatBytes(result.totalUncompressedSize) }}</strong></div>
        <div class="browser-stat"><span>安全状态</span><strong>{{ result.encrypted ? '已加密' : '未加密' }}</strong></div>
      </section>

      <section class="aero-card flex-1 min-h-0 min-w-0 overflow-hidden browser-workspace">
        <aside class="min-h-0 min-w-0 overflow-y-auto overflow-x-hidden custom-scrollbar border-r border-subtle/70 p-3">
          <button class="directory-entry" :class="{ active: activeDirectory === '' }" type="button" @click="activeDirectory = ''"><i class="pi pi-home"></i><span>全部文件</span></button>
          <button v-for="directory in directories" :key="directory" class="directory-entry" :class="{ active: activeDirectory === directory }" type="button" @click="activeDirectory = directory">
            <i class="pi pi-folder"></i><span class="truncate">{{ directory }}</span>
          </button>
        </aside>

        <div class="min-h-0 min-w-0 overflow-hidden flex flex-col">
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
          <div class="flex-1 min-h-0 overflow-y-auto overflow-x-hidden custom-scrollbar">
            <button v-for="entry in filteredEntries" :key="entry.path" type="button" class="browser-row" @click="toggleEntry(entry)">
              <span class="browser-checkbox" :class="{ checked: selected.has(entry.path) }"><i v-if="selected.has(entry.path)" class="pi pi-check"></i></span>
              <span class="min-w-0 text-left"><strong class="block truncate text-content">{{ entry.name }}</strong><small class="block truncate text-muted mt-0.5">{{ entry.path }}</small></span>
              <span class="hidden md:block text-left text-muted">{{ formatBytes(entry.size) }}</span>
              <span class="hidden lg:block text-left text-muted truncate">{{ entry.modified || '—' }}</span>
              <span class="hidden xl:block text-left font-mono text-muted truncate">{{ entry.crc || '—' }}</span>
            </button>
            <div v-if="filteredEntries.length === 0" class="h-full grid place-items-center text-muted text-sm">没有符合条件的文件</div>
          </div>
        </div>
      </section>

      <footer class="shrink-0 flex flex-wrap items-center justify-between gap-3">
        <span class="text-sm font-bold text-muted">已选择 <strong class="text-primary">{{ selected.size }}</strong> / {{ result.totalFiles }} 个文件</span>
        <button class="browser-primary" type="button" :disabled="selected.size === 0 || !outputPath || extracting" @click="extractSelected">
          <i :class="extracting ? 'pi pi-spin pi-spinner' : 'pi pi-download'"></i><span>{{ extracting ? '正在解压' : '解压所选文件' }}</span>
        </button>
      </footer>
    </template>
  </div>
</template>

<style scoped>
.browser-page { max-width: 100%; }
.browser-toolbar { grid-template-columns: minmax(0, 1.45fr) minmax(12rem, .7fr) minmax(0, 1fr); }
.browser-field { display: flex; flex-direction: column; gap: .4rem; }
.browser-label { color: var(--text-muted); font-size: .7rem; font-weight: 900; letter-spacing: .12em; }
.browser-input, .browser-search, .browser-select { height: 2.75rem; border: 1px solid var(--border-subtle); border-radius: .85rem; background: var(--bg-input); color: var(--text-content); padding: 0 .9rem; outline: none; }
.browser-input:focus, .browser-search:focus-within, .browser-select:focus { border-color: var(--dynamic-accent); }
.browser-icon-button { width: 2.75rem; height: 2.75rem; border-radius: .85rem; border: 1px solid var(--border-subtle); color: var(--dynamic-accent); background: var(--bg-input); }
.browser-primary { min-height: 2.85rem; padding: 0 1.15rem; border-radius: .9rem; display: inline-flex; align-items: center; justify-content: center; gap: .55rem; color: white; font-size: .8rem; font-weight: 900; background: var(--dynamic-accent); box-shadow: 0 8px 24px color-mix(in srgb, var(--dynamic-accent) 25%, transparent); }
.browser-primary:disabled, .browser-icon-button:disabled { opacity: .45; cursor: not-allowed; }
.browser-stat { min-width: 0; padding: .85rem 1rem; border-radius: 1rem; border: 1px solid var(--border-subtle); background: color-mix(in srgb, var(--bg-card) 85%, transparent); display: flex; flex-direction: column; gap: .25rem; }
.browser-stat span { color: var(--text-muted); font-size: .68rem; font-weight: 800; }
.browser-stat strong { color: var(--text-content); font-size: 1rem; overflow: hidden; text-overflow: ellipsis; }
.browser-workspace { display: grid; grid-template-columns: minmax(10rem, 15rem) minmax(0, 1fr); }
.directory-entry { width: 100%; min-width: 0; display: flex; align-items: center; gap: .65rem; border-radius: .75rem; padding: .68rem .75rem; color: var(--text-muted); font-size: .76rem; font-weight: 800; text-align: left; }
.directory-entry:hover, .directory-entry.active { background: color-mix(in srgb, var(--dynamic-accent) 13%, transparent); color: var(--dynamic-accent); }
.browser-search { display: flex; align-items: center; gap: .6rem; padding-inline: .8rem; }
.browser-search input { width: 100%; min-width: 0; background: transparent; outline: none; }
.browser-select { min-width: 8.5rem; }
.browser-table-head, .browser-row { min-width: 0; display: grid; grid-template-columns: 1.5rem minmax(10rem, 1fr) minmax(5rem, .25fr) minmax(8rem, .42fr) minmax(5rem, .25fr); align-items: center; gap: .75rem; padding: .7rem 1rem; }
.browser-table-head { color: var(--text-muted); font-size: .68rem; font-weight: 900; border-bottom: 1px solid var(--border-subtle); }
.browser-row { width: 100%; border-bottom: 1px solid color-mix(in srgb, var(--border-subtle) 60%, transparent); font-size: .75rem; }
.browser-row:hover { background: color-mix(in srgb, var(--dynamic-accent) 7%, transparent); }
.browser-checkbox { width: 1.15rem; height: 1.15rem; border: 1px solid var(--border-subtle); border-radius: .35rem; display: inline-grid; place-items: center; color: white; font-size: .55rem; }
.browser-checkbox.checked { background: var(--dynamic-accent); border-color: var(--dynamic-accent); }
@media (max-width: 1050px) { .browser-toolbar { grid-template-columns: minmax(0, 1fr) minmax(12rem, .65fr); } .browser-field:last-child { grid-column: 1 / -1; } .browser-table-head, .browser-row { grid-template-columns: 1.5rem minmax(8rem, 1fr) minmax(5rem, .28fr) minmax(7rem, .4fr); } }
@media (max-width: 760px) { .browser-page { padding: 1rem; } .browser-workspace { grid-template-columns: 1fr; grid-template-rows: minmax(5rem, 8rem) minmax(0, 1fr); } .browser-workspace aside { border-right: 0; border-bottom: 1px solid var(--border-subtle); } .browser-toolbar { grid-template-columns: 1fr; } .browser-field:last-child { grid-column: auto; } .browser-table-head, .browser-row { grid-template-columns: 1.5rem minmax(0, 1fr); } }
</style>
