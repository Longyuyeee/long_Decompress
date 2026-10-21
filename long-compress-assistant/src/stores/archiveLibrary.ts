import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import type { LibrarySource, LibraryHit, LibraryScan, LibraryResults, LibraryCredentials } from '@/types/archiveLibrary'

export const useArchiveLibraryStore = defineStore('archiveLibrary', () => {
  const sources = ref<LibrarySource[]>([])
  const hits = ref<LibraryHit[]>([])
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

  async function search() {
    const sequence = ++searchSequence
    searching.value = true; searchError.value = ''
    try {
      const value = await invoke<LibraryResults>('library_search', { request: { query: query.value, sourceId: sourceId.value, category: category.value, page: page.value } })
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
      try { await reload(); initialized = true } catch (reason) { error.value = explain(reason) }
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
    try { await invoke('library_remove', { sourceId: id }); delete credentials.value[id]; await reload(); notice.value = '已移出资料库，原包保留'; return true }
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
  return { sources, hits, query, sourceId, category, page, total, error, searchError, notice, loading, searching, indexing, scanning, activeSource, completed, planned, stopRequested, scanResult, ready, pending, initialize, reload, search, add, remove, index, scan, stop, check, credentials, paused, checking, skipped, pause, resume, setCredentials, audit, relocate }
})
