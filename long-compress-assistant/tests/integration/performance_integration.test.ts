import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useTaskStore } from '@/stores/task'

const mocks = vi.hoisted(() => ({
  listeners: new Map<string, (event: { payload: any }) => void>(),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (event: string, callback: (event: { payload: any }) => void) => {
    mocks.listeners.set(event, callback)
    return vi.fn()
  }),
}))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: vi.fn() }))

describe('Task progress event performance', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    mocks.listeners.clear()
  })

  it('processes a burst of progress events without losing the final state', async () => {
    const tasks = useTaskStore()
    await tasks.initListeners()
    tasks.addTask({
      id: 'perf-task', name: 'large.zip', type: 'decompression',
      sourceFiles: ['large.zip'], outputPath: 'output',
    })
    const listener = mocks.listeners.get('task-progress')!

    const started = performance.now()
    for (let index = 1; index <= 1_000; index++) {
      listener({ payload: { task_id: 'perf-task', progress: index / 1_000 } })
    }
    const elapsed = performance.now() - started

    expect(tasks.tasks[0].progress).toBe(100)
    expect(elapsed).toBeLessThan(250)
  })
})
