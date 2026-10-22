import { computed, ref, watch } from 'vue'
import { defineStore } from 'pinia'
import { useTaskStore, type TaskStatus } from './task'

interface BatchResult {
  zpaqRetrieval?: import('@/types/zpaqRetrieval').ZpaqRetrievalSelection
  id: string
  name: string
  outputPath: string
  status: TaskStatus | 'removed'
  needsConflictChoice: boolean
}
const terminal = new Set<TaskStatus>(['completed', 'failed', 'cancelled'])

// Keep only the latest launch's small result summary, never passwords or logs.
// Store lifetime follows the app, so navigating away cannot reset the batch.
export const useDecompressionBatchStore = defineStore('decompressionBatch', () => {
  const tasks = useTaskStore()
  const results = ref<BatchResult[]>([])
  const sequence = ref(0)
  const showingFailures = ref(false)
  const currentTasks = computed(() => new Map(tasks.tasks
    .filter(task => task.type === 'decompression' && (!task.workloadKind || task.workloadKind === 'archive'))
    .map(task => [task.id, { zpaqRetrieval:task.zpaqRetrieval, name: task.name, outputPath: task.status === 'completed' ? task.outputPath : '', status: task.status, needsConflictChoice: task.conflicts.length > 0 }])))

  watch(currentTasks, current => {
    for (const result of results.value) {
      const task = current.get(result.id)
      if (task) Object.assign(result, task)
      else if (!terminal.has(result.status as TaskStatus)) {
        result.status = 'removed'
        result.needsConflictChoice = false
      }
    }
  }, { flush: 'sync' })

  const begin = (ids: string[]) => {
    const next = [...new Set(ids)].flatMap(id => {
      const task = currentTasks.value.get(id)
      return task?.status === 'pending' ? [{ id, ...task }] : []
    })
    if (!next.length) return
    results.value = next
    sequence.value++
    showingFailures.value = false
  }
  const counts = computed(() => {
    const count = { total: results.value.length, completed: 0, failed: 0, cancelled: 0, unfinished: 0, conflicts: 0, removed: 0 }
    for (const result of results.value) {
      if (result.status === 'removed') count.removed++
      else if (terminal.has(result.status)) count[result.status as 'completed' | 'failed' | 'cancelled']++
      else {
        count.unfinished++
        if (result.needsConflictChoice) count.conflicts++
      }
    }
    return count
  })
  const visibleFailureIds = computed(() => results.value
    .filter(result => currentTasks.value.get(result.id)?.status === 'failed')
    .map(result => result.id))
  const completedResults = computed(() => results.value.filter(result => result.status === 'completed'))
  return { results, sequence, showingFailures, begin, counts, visibleFailureIds, completedResults }
})
