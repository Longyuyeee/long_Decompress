import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/tauri'
import type { TaskHistoryRecord } from '@/types/taskHistory'

interface HistoryCursor { completedAt: string; id: string }
interface HistoryPage { records: TaskHistoryRecord[]; nextCursor: HistoryCursor | null }

export const useHistoryStore = defineStore('history', () => {
  const records = ref<TaskHistoryRecord[]>([])
  const isLoading = ref(false)
  const isInitialized = ref(false)
  const error = ref<string | null>(null)
  const nextCursor = ref<HistoryCursor | null>(null)
  const hasMore = computed(() => nextCursor.value !== null)
  let generation = 0
  const deletedIds = new Set<string>()

  const sortedRecords = computed(() => [...records.value].sort(
    (a, b) => new Date(b.completedAt).getTime() - new Date(a.completedAt).getTime()
      || (a.id < b.id ? 1 : a.id > b.id ? -1 : 0),
  ))

  const readPage = async (append: boolean) => {
    if (append && (isLoading.value || !hasMore.value)) return
    const request = ++generation
    if (!append) deletedIds.clear()
    isLoading.value = true
    error.value = null
    try {
      const page = await invoke<HistoryPage>('list_task_history_page', { limit: 100, cursor: append ? nextCursor.value : null })
      if (request !== generation) return
      const merged = new Map((append ? records.value : []).map(record => [record.id, record]))
      for (const record of page.records) if (!deletedIds.has(record.id)) merged.set(record.id, record)
      records.value = [...merged.values()]
      nextCursor.value = page.nextCursor
      isInitialized.value = true
    } catch (caught) {
      if (request === generation) { error.value = String(caught); throw caught }
    } finally {
      if (request === generation) isLoading.value = false
    }
  }
  const fetchHistory = () => readPage(false)
  const loadMore = () => readPage(true)

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
    loadMore,
    hasMore,
    deleteRecord,
    clearHistory,
  }
})
