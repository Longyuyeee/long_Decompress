import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/tauri'
import type { TaskHistoryRecord } from '@/types/taskHistory'

interface HistoryCursor { completedAt: string; id: string }
interface HistoryPage { records: TaskHistoryRecord[]; nextCursor: HistoryCursor | null; logsOmitted?: boolean }
export const HISTORY_WINDOW_LIMIT = 500
export interface HistoryQuery {
  text: string
  taskType?: string
  status?: string
  since?: string
  failureCategory?: string
}

export const useHistoryStore = defineStore('history', () => {
  const records = ref<TaskHistoryRecord[]>([])
  const isLoading = ref(false)
  const isInitialized = ref(false)
  const error = ref<string | null>(null)
  const nextCursor = ref<HistoryCursor | null>(null)
  const hasMore = computed(() => nextCursor.value !== null)
  const newerCursor = ref<HistoryCursor | null>(null)
  const hasNewer = computed(() => newerCursor.value !== null)
  let activeQuery: HistoryQuery = { text: '' }
  let generation = 0
  const deletedIds = new Set<string>()

  const sortedRecords = computed(() => [...records.value].sort(
    (a, b) => new Date(b.completedAt).getTime() - new Date(a.completedAt).getTime()
      || (a.id < b.id ? 1 : a.id > b.id ? -1 : 0),
  ))

  const readPage = async (append: boolean, newer = false) => {
    if (append && (isLoading.value || !(newer ? hasNewer.value : hasMore.value))) return
    const request = ++generation
    deletedIds.clear()
    isLoading.value = true
    error.value = null
    try {
      const page = await invoke<HistoryPage>('list_task_history_page', { limit: 100, cursor: append ? (newer ? newerCursor.value : nextCursor.value) : null, query: { ...activeQuery }, omitLogs: true, ...(newer ? { newer: true } : {}) })
      if (request !== generation) return
      const merged = new Map((append ? records.value : []).map(record => [record.id, record]))
      for (const record of page.records) if (!deletedIds.has(record.id)) merged.set(record.id, { ...record, logsLoaded: !page.logsOmitted })
      const all = [...merged.values()].sort((a, b) => new Date(b.completedAt).getTime() - new Date(a.completedAt).getTime() || (a.id < b.id ? 1 : a.id > b.id ? -1 : 0))
      const cursorFor = (record: TaskHistoryRecord): HistoryCursor => ({ completedAt: record.completedAt, id: record.id })
      records.value = newer ? all.slice(0, HISTORY_WINDOW_LIMIT) : all.slice(-HISTORY_WINDOW_LIMIT)
      if (newer) {
        newerCursor.value = page.nextCursor
        if (all.length > HISTORY_WINDOW_LIMIT) nextCursor.value = cursorFor(records.value.at(-1)!)
      } else {
        nextCursor.value = page.nextCursor
        if (!append) newerCursor.value = null
        else if (all.length > HISTORY_WINDOW_LIMIT) newerCursor.value = cursorFor(records.value[0])
      }
      isInitialized.value = true
    } catch (caught) {
      if (request === generation) { error.value = String(caught); throw caught }
    } finally {
      if (request === generation) isLoading.value = false
    }
  }
  const fetchHistory = () => readPage(false)
  const loadMore = () => readPage(true)
  const loadNewer = () => readPage(true, true)
  const prepareQuery = (query: HistoryQuery) => {
    activeQuery = { ...query }
    generation++
    records.value = []
    nextCursor.value = null
    newerCursor.value = null
    error.value = null
    isLoading.value = true
    isInitialized.value = false
  }

  const deleteRecord = async (id: string) => {
    await invoke('delete_task_history', { id })
    deletedIds.add(id)
    records.value = records.value.filter(record => record.id !== id)
  }

  const clearHistory = async () => {
    await invoke('clear_task_history')
    generation++
    isLoading.value = false
    error.value = null
    nextCursor.value = null
    newerCursor.value = null
    isInitialized.value = true
    deletedIds.clear()
    records.value = []
  }

  return {
    records,
    sortedRecords,
    isLoading,
    isInitialized,
    error,
    fetchHistory,
    prepareQuery,
    loadMore,
    loadNewer,
    hasNewer,
    hasMore,
    deleteRecord,
    clearHistory,
  }
})
