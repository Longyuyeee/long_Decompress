import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import type { LibrarySource, LibraryHit, LibraryScan, LibraryResults, LibraryCredentials, LibraryOrganizationState, LibrarySearchFilter } from '@/types/archiveLibrary'

export const useArchiveLibraryStore = defineStore('archiveLibrary', () => {
  const sources = ref<LibrarySource[]>([])
  const hits = ref<LibraryHit[]>([])
  const organization = ref<LibraryOrganizationState>({ sources: [], searches: [] })
  const tag = ref(''), favoritesOnly = ref(false)
  const workspace = ref<{ expanded: string; sourcePage: number; sourceSearch: string; scrollTop: number; view?: 'search' | 'sources' }>({ expanded: '', sourcePage: 1, sourceSearch: '', scrollTop: 0 })
  const tags = computed(() => [...new Set(organization.value.sources.flatMap(row => row.tags))].sort((a,b) => a.localeCompare(b)))
  const query = ref(''), sourceId = ref(''), category = ref('all'), page = ref(1), total = ref(0)
  const error = ref(''), searchError = ref(''), notice = ref('')
  const loading = ref(false), searching = ref(false), indexing = ref(false), scanning = ref(false)
  const credentials = ref<Record<string, LibraryCredentials>>({})
  const paused = ref(false), checking = ref(false), skipped = ref(0)
  let resumeWait: (() => void) | undefined
  const activeSource = ref(''), completed = ref(0), planned = ref(0), stopRequested = ref(false)
  const scanResult = ref<LibraryScan | null>(null)
  const ready = computed(() => sources.value.filter(row => !!row.indexedAt || row.status === 'ready'))
  const pending = computed(() => sources.value.filter(row => row.status !== 'ready'))
  let initialized = false, initialization: Promise<void> | undefined, searchSequence = 0, scanId = ''
  const explain = (reason: unknown) => String(reason).replace(/^Error:\s*/, '')

  const workspaceKey = 'long-decompress-library-search-v1'
  try {
    const saved = JSON.parse(localStorage.getItem(workspaceKey) || 'null')
    if (saved && typeof saved.query === 'string' && saved.query.length <= 512) {
      query.value=saved.query; sourceId.value=typeof saved.sourceId==='string'?saved.sourceId:''; category.value=typeof saved.category==='string'?saved.category:'all'; tag.value=typeof saved.tag==='string'?saved.tag:''; favoritesOnly.value=saved.favoritesOnly===true; page.value=Number.isInteger(saved.page)&&saved.page>0?saved.page:1
      if(saved.workspace) { const w=saved.workspace; workspace.value={view:w.view==='sources'?'sources':'search',expanded:typeof w.expanded==='string'?w.expanded:'',sourceSearch:typeof w.sourceSearch==='string'?w.sourceSearch:'',sourcePage:Number.isInteger(w.sourcePage)&&w.sourcePage>0?w.sourcePage:1,scrollTop:w.view&&typeof w.scrollTop==='number'?Math.max(0,w.scrollTop):0} }
    }
  } catch { /* A missing or old view preference does not block the library. */ }
  function filters(): LibrarySearchFilter { return { query:query.value,sourceId:sourceId.value,category:category.value,tag:tag.value,favoritesOnly:favoritesOnly.value,page:page.value } }
  watch(() => JSON.stringify({ ...filters(), workspace:workspace.value }), value => { try { localStorage.setItem(workspaceKey,value) } catch { /* View position is optional. */ } })
  async function loadOrganization() { const value=await invoke<LibraryOrganizationState>('library_organization'); organization.value=value || {sources:[],searches:[]} }
  const labels = (id:string) => organization.value.sources.find(row=>row.sourceId===id) || {sourceId:id,favorite:false,tags:[]}
  async function organize(id:string,values:{favorite:boolean;tags:string[]}) {
    try { organization.value=await invoke('library_organize',{value:{sourceId:id,...values}}); await search(); return true }
    catch(reason) {error.value=explain(reason);return false}
  }
  async function saveSearch(name:string) {
    try {organization.value=await invoke('library_save_search',{name,filters:{...filters(),page:1}});notice.value='常用搜索已保存，同名条件会更新';return true}
    catch(reason){error.value=explain(reason);return false}
  }
  async function deleteSearch(id:string) {try{organization.value=await invoke('library_delete_search',{id});return true}catch(reason){error.value=explain(reason);return false}}
  function applySearch(id:string) {
    const saved=organization.value.searches.find(row=>row.id===id);if(!saved)return
    if(saved.filters.sourceId&&!sources.value.some(row=>row.id===saved.filters.sourceId)){error.value='这份常用搜索的来源已移出资料库，请重新选择来源后保存条件';return}
    query.value=saved.filters.query;sourceId.value=saved.filters.sourceId;category.value=saved.filters.category;tag.value=saved.filters.tag||'';favoritesOnly.value=saved.filters.favoritesOnly===true;page.value=1
    void search()
  }
  async function search() {
    const sequence = ++searchSequence
    searching.value = true; searchError.value = ''
    try {
      const value = await invoke<LibraryResults>('library_search', { request: filters() })
      if (sequence !== searchSequence) return
      hits.value = value.items; total.value = value.total; page.value = value.page
    } catch (reason) {
      if (sequence === searchSequence) { hits.value = []; total.value = 0; searchError.value = explain(reason) }
    } finally { if (sequence === searchSequence) searching.value = false }
  }
  async function reload() {
    sources.value = await invoke<LibrarySource[]>('library_sources')
    if (sourceId.value && !sources.value.some(row => row.id === sourceId.value)) sourceId.value = ''
    await search()
  }
  async function initialize() {
    if (initialized) return
    if (initialization) return initialization
    initialization = (async () => {
      loading.value = true; error.value = ''
      try { await Promise.all([reload(),loadOrganization()]); initialized = true } catch (reason) { error.value = explain(reason) }
      finally { loading.value = false; initialization = undefined }
    })()
    return initialization
  }
  async function add(paths: string[]) {
    if (!paths.length) return false
    error.value = ''; loading.value = true
    try { sources.value = await invoke<LibrarySource[]>('library_add', { paths }); notice.value = '来源已加入，点击建立索引后即可跨包查找'; return true }
    catch (reason) { error.value = explain(reason); return false }
    finally { loading.value = false }
  }
  async function remove(id: string) {
    error.value = ''
    try { await invoke('library_remove', { sourceId: id }); delete credentials.value[id]; await Promise.all([reload(),loadOrganization()]); notice.value = '已移出资料库，原包保留'; return true }
    catch (reason) { error.value = explain(reason); return false }
  }
  async function index(ids: string[], force = false) {
    if (indexing.value || checking.value || scanning.value || !ids.length) return
    indexing.value = true; paused.value = false; skipped.value = 0; stopRequested.value = false; completed.value = 0; planned.value = ids.length; error.value = ''; notice.value = ''
    let failures = 0
    try {
      for (const id of ids) {
        if (paused.value) await new Promise<void>(resolve => { resumeWait = resolve })
        if (stopRequested.value) break
        if (!sources.value.some(row => row.id === id)) continue
        activeSource.value = id
        const row = sources.value.find(row => row.id === id); if (row) { row.status = 'indexing'; row.note = '' }
        try { const result = await invoke<{ skipped: boolean }>('library_index', { sourceId: id, credentials: credentials.value[id] || { password: '', encoding: sources.value.find(row => row.id === id)?.sourceEncoding || undefined }, force }); if (result?.skipped) skipped.value++ } catch { if (!stopRequested.value) failures++ }
        completed.value++; activeSource.value = ''
        await reload()
      }
      notice.value = stopRequested.value ? '已停止，已完成来源可搜索，其他来源可稍后继续' : failures ? `本次处理 ${completed.value} 份，${failures} 份需要处理，请查看来源中的原因` : `本次处理 ${completed.value} 份，${skipped.value} 份未变化并沿用索引`
    } catch (reason) { error.value = explain(reason) }
    finally { activeSource.value = ''; indexing.value = false; paused.value = false; resumeWait = undefined }
  }
  async function scan(directory: string) {
    if (scanning.value || indexing.value || checking.value) return
    scanning.value = true; stopRequested.value = false; error.value = ''; scanResult.value = null
    scanId = `library-scan-${crypto.randomUUID()}`
    try { scanResult.value = await invoke<LibraryScan>('library_scan', { directory, jobId: scanId }) }
    catch (reason) { error.value = explain(reason) }
    finally { scanning.value = false; scanId = '' }
  }
  async function stop() {
    stopRequested.value = true; paused.value = false; resumeWait?.(); resumeWait = undefined
    const id = activeSource.value || scanId
    if (id) await invoke('library_cancel', { jobId: id }).catch(reason => { error.value = explain(reason) })
  }
  async function check(hit: LibraryHit) {
    try { const row = await invoke<LibrarySource>('library_check', { sourceId: hit.sourceId, entryPath: hit.path }); if(row) sources.value = sources.value.map(old => old.id === row.id ? row : old); return true }
    catch (reason) { error.value = explain(reason); sourceId.value = hit.sourceId; await reload().catch(() => undefined); return false }
  }
  function pause() { paused.value = true }
  function resume() { paused.value = false; resumeWait?.(); resumeWait = undefined }
  function setCredentials(id: string, value: LibraryCredentials) { credentials.value[id] = { ...value } }
  async function audit(ids: string[]) {
    if (checking.value || indexing.value || scanning.value) return
    checking.value = true; paused.value = false; stopRequested.value = false; error.value = ''; completed.value = 0; planned.value = ids.length
    try {
      for (const id of ids) {
        if (paused.value) await new Promise<void>(resolve => { resumeWait = resolve })
        if (stopRequested.value) break
        activeSource.value = id
        const value = await invoke<LibrarySource>('library_audit', { sourceId: id })
        sources.value = sources.value.map(row => row.id === id ? value : row); completed.value++; activeSource.value = ''
      }
      await search(); notice.value = stopRequested.value ? '已停止检查，其他来源可稍后继续' : '来源状态已更新；离线或变化资料仍可搜索上次索引，使用前请先处理'
    } catch (reason) { error.value = explain(reason) }
    finally { checking.value = false; activeSource.value = ''; paused.value = false; resumeWait = undefined }
  }
  async function relocate(id: string, path: string) {
    error.value = ''
    try {
      const value = await invoke<LibrarySource>('library_relocate', { sourceId: id, path })
      sources.value = sources.value.map(row => row.id === id ? value : row); delete credentials.value[id]; await search()
      notice.value = '已重新定位，保留上次索引；请刷新确认当前来源后再查看或取回'; return true
    } catch (reason) { error.value = explain(reason); return false }
  }
  return { organization, tag, favoritesOnly, tags, workspace, labels, organize, saveSearch, deleteSearch, applySearch, sources, hits, query, sourceId, category, page, total, error, searchError, notice, loading, searching, indexing, scanning, activeSource, completed, planned, stopRequested, scanResult, ready, pending, initialize, reload, search, add, remove, index, scan, stop, check, credentials, paused, checking, skipped, pause, resume, setCredentials, audit, relocate }
})
