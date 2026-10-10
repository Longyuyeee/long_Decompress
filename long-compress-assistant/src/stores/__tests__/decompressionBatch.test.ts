import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useTaskStore, type TaskType } from '../task'
import { useDecompressionBatchStore } from '../decompressionBatch'

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: vi.fn(async () => undefined) }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => vi.fn()) }))

const add = (id: string, type: TaskType = 'decompression') => {
  const store = useTaskStore()
  store.addTask({ id, name: id, type, sourceFiles: [`C:/${id}.zip`], outputPath: 'C:/out' })
  return store.tasks.find(task => task.id === id)!
}

describe('latest extraction run summary', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('retains the completed actual output after clearing, without exposing pending destinations as results', () => {
    const batch = useDecompressionBatchStore()
    const task = add('trip')
    batch.begin([task.id])
    expect(batch.completedResults).toEqual([])
    expect(batch.results[0].outputPath).toBe('')
    task.outputPath = 'D:/相册/旅行'
    task.status = 'completed'
    useTaskStore().tasks = []
    expect(batch.completedResults).toMatchObject([{ id: 'trip', name: 'trip', outputPath: 'D:/相册/旅行' }])
    const next = add('next')
    batch.begin([next.id])
    expect(batch.completedResults).toEqual([])
  })

  it('shows my 17 successes and 3 failures without counting older tasks or media', () => {
    const batch = useDecompressionBatchStore()
    const old = add('old')
    old.status = 'failed'
    const media = add('video', 'compression')
    media.workloadKind = 'video'
    const current = Array.from({ length: 20 }, (_, index) => add(`item-${index}`))
    batch.begin([...current.map(task => task.id), old.id, media.id])
    current.forEach((task, index) => { task.status = index < 17 ? 'completed' : 'failed' })
    expect(batch.counts).toMatchObject({ total: 20, completed: 17, failed: 3, cancelled: 0, unfinished: 0 })
    expect(batch.visibleFailureIds).toEqual(['item-17', 'item-18', 'item-19'])
  })

  it('does not call cancelled or conflict-waiting tasks successful', () => {
    const batch = useDecompressionBatchStore()
    const cancelled = add('cancelled')
    const conflict = add('conflict')
    batch.begin([cancelled.id, conflict.id])
    cancelled.status = 'cancelled'
    conflict.conflicts.push({ taskId: conflict.id, fileName: 'a', sourcePath: 'a', destPath: 'b', sourceSize: 1, destSize: 1, sourceModified: 0, destModified: 0 })
    expect(batch.counts).toMatchObject({ cancelled: 1, unfinished: 1, conflicts: 1, completed: 0, failed: 0 })
    conflict.conflicts = []
    conflict.status = 'completed'
    expect(batch.counts).toMatchObject({ unfinished: 0, conflicts: 0, completed: 1 })
  })

  it('retains compact terminal counts after clearing the queue and replaces them on the next run', () => {
    const tasks = useTaskStore()
    const batch = useDecompressionBatchStore()
    const first = add('first')
    batch.begin([first.id])
    first.status = 'failed'
    first.password = 'do-not-retain'
    tasks.tasks = []
    expect(batch.counts.failed).toBe(1)
    expect(batch.visibleFailureIds).toEqual([])
    expect(JSON.stringify(batch.results)).not.toContain('do-not-retain')
    batch.showingFailures = true
    const second = add('second')
    batch.begin([second.id, second.id])
    expect(batch.counts).toMatchObject({ total: 1, failed: 0, unfinished: 1 })
    expect(batch.showingFailures).toBe(false)
    expect(batch.sequence).toBe(2)
  })

  it('keeps the same run across consumers, excludes later imports and accounts for removed waiting tasks', () => {
    const task = add('selected')
    const batch = useDecompressionBatchStore()
    batch.begin([task.id])
    add('later')
    expect(useDecompressionBatchStore().counts.total).toBe(1)
    useTaskStore().tasks = useTaskStore().tasks.filter(item => item.id !== task.id)
    expect(batch.counts).toMatchObject({ total: 1, removed: 1, unfinished: 0 })
    batch.begin([])
    expect(batch.sequence).toBe(1)
  })
})
