import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import type { LibrarySource, LibraryHit, LibraryScan, LibraryResults } from '@/types/archiveLibrary'

export const useArchiveLibraryStore = defineStore('archiveLibrary', () => {
  const sources = ref<LibrarySource[]>([])
  const hits = ref<LibraryHit[]>([])
  const query = ref(''), sourceId = ref(''), category = ref('all'), page = ref(1), total = ref(0)
  const error = ref(''), searchError = ref(''), notice = ref('')
  const loading = ref(false), searching = ref(false), indexing = ref(false), scanning = ref(false)
  const activeSource = ref(''), completed = ref(0), planned = ref(0), stopRequested = ref(false)
  const scanResult = ref<LibraryScan | null>(null)
  const ready = computed(() => sources.value.filter(row => row.status === 'ready'))
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
    try { await invoke('library_remove', { sourceId: id }); await reload(); notice.value = '已移出资料库，原包保留'; return true }
    catch (reason) { error.value = explain(reason); return false }
  }
  async function index(ids: string[]) {
    if (indexing.value || scanning.value || !ids.length) return
    indexing.value = true; stopRequested.value = false; completed.value = 0; planned.value = ids.length; error.value = ''; notice.value = ''
    let failures = 0
    try {
      for (const id of ids) {
        if (stopRequested.value) break
        if (!sources.value.some(row => row.id === id)) continue
        activeSource.value = id
        const row = sources.value.find(row => row.id === id); if (row) { row.status = 'indexing'; row.note = '' }
        try { await invoke('library_index', { sourceId: id }) } catch { if (!stopRequested.value) failures++ }
        completed.value++
        await reload()
      }
      notice.value = stopRequested.value ? '已停止，已完成来源可搜索，其他来源可稍后继续' : failures ? `本次处理 ${completed.value} 份，${failures} 份需要处理，请查看来源中的原因` : `已完成 ${completed.value} 份来源的索引`
    } catch (reason) { error.value = explain(reason) }
    finally { activeSource.value = ''; indexing.value = false }
  }
  async function scan(directory: string) {
    if (scanning.value || indexing.value) return
    scanning.value = true; stopRequested.value = false; error.value = ''; scanResult.value = null
    scanId = `library-scan-${crypto.randomUUID()}`
    try { scanResult.value = await invoke<LibraryScan>('library_scan', { directory, jobId: scanId }) }
    catch (reason) { error.value = explain(reason) }
    finally { scanning.value = false; scanId = '' }
  }
  async function stop() {
    stopRequested.value = true
    const id = activeSource.value || scanId
    if (id) await invoke('library_cancel', { jobId: id }).catch(reason => { error.value = explain(reason) })
  }
  async function check(hit: LibraryHit) {
    try { await invoke('library_check', { sourceId: hit.sourceId, entryPath: hit.path }); return true }
    catch (reason) { error.value = explain(reason); sourceId.value = hit.sourceId; return false }
  }
  return { sources, hits, query, sourceId, category, page, total, error, searchError, notice, loading, searching, indexing, scanning, activeSource, completed, planned, stopRequested, scanResult, ready, pending, initialize, reload, search, add, remove, index, scan, stop, check }
})
