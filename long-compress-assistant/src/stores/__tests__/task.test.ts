import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useTaskStore } from '../task'

const mocks = vi.hoisted(() => ({
  listeners: new Map<string, (event: { payload: any }) => void>(),
  invoke: vi.fn(async () => undefined),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (name: string, callback: (event: { payload: any }) => void) => {
    mocks.listeners.set(name, callback)
    return vi.fn()
  }),
}))

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))

describe('task progress state machine', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    mocks.listeners.clear()
    mocks.invoke.mockClear()
  })

  it('keeps the live queue separate from persisted history reads', () => {
    const store = useTaskStore()

    expect(store).not.toHaveProperty('fetchTasks')
    expect(mocks.invoke).not.toHaveBeenCalledWith('list_task_history', expect.anything())
  })

  it('keeps extraction progress at zero while password candidates are being verified', async () => {
    const store = useTaskStore()
    await store.initListeners()
    store.addTask({
      id: 'encrypted-rar',
      name: 'encrypted.rar',
      type: 'decompression',
      sourceFiles: ['C:/archives/encrypted.rar'],
      outputPath: 'C:/archives/encrypted',
    })
    store.updateTaskStatus('encrypted-rar', 'extracting')

    mocks.listeners.get('task-progress')?.({
      payload: {
        task_id: 'encrypted-rar',
        stage: 'password-attempt',
        progress: 1,
        current_password: '保险箱候选',
        password_attempt_current: 2,
        password_attempt_total: 2,
        processed_bytes: 0,
        total_bytes: 0,
      },
    })

    expect(store.tasks[0]).toMatchObject({
      progress: 0,
      stage: 'password-attempt',
      currentPassword: '保险箱候选',
      passwordAttemptCurrent: 2,
      passwordAttemptTotal: 2,
      processedBytes: 0,
      totalBytes: 0,
    })

    mocks.listeners.get('task-progress')?.({
      payload: {
        task_id: 'encrypted-rar',
        progress: 0.25,
        processed_bytes: 512,
        total_bytes: 2048,
      },
    })

    expect(store.tasks[0]).toMatchObject({
      progress: 25,
      stage: 'Extracting',
      processedBytes: 512,
      totalBytes: 2048,
    })
  })

  it('stores the backend-detected archive format for history', async () => {
    const store = useTaskStore()
    await store.initListeners()
    store.addTask({
      id: 'format-task',
      name: 'misleading.zip',
      type: 'decompression',
      sourceFiles: ['C:/archives/misleading.zip'],
      outputPath: 'C:/archives/output',
    })

    mocks.listeners.get('archive-format-detected')?.({
      payload: { taskId: 'format-task', format: '7z' },
    })

    expect(store.tasks[0].format).toBe('7z')
  })
})
