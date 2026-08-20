import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/tauri'
import type { TaskHistoryRecord } from '@/types/taskHistory'

export const useHistoryStore = defineStore('history', () => {
  const records = ref<TaskHistoryRecord[]>([])
  const isLoading = ref(false)
  const isInitialized = ref(false)
  const error = ref<string | null>(null)

  const sortedRecords = computed(() => [...records.value].sort(
    (a, b) => new Date(b.completedAt).getTime() - new Date(a.completedAt).getTime(),
  ))

  const fetchHistory = async () => {
    isLoading.value = true
    error.value = null
    try {
      records.value = await invoke<TaskHistoryRecord[]>('list_task_history', { limit: 500 })
      isInitialized.value = true
    } catch (caught) {
      error.value = String(caught)
      throw caught
    } finally {
      isLoading.value = false
    }
  }

  const deleteRecord = async (id: string) => {
    await invoke('delete_task_history', { id })
    records.value = records.value.filter(record => record.id !== id)
  }

  const clearHistory = async () => {
    await invoke('clear_task_history')
    records.value = []
  }

  return {
    records,
    sortedRecords,
    isLoading,
    isInitialized,
    error,
    fetchHistory,
    deleteRecord,
    clearHistory,
  }
})
