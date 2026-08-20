import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { useTaskStore } from '@/stores/task'

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  listeners: new Map<string, (event: { payload: any }) => void>(),
}))

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/event', () => ({
  listen: mocks.listen.mockImplementation(async (event: string, callback: (event: { payload: any }) => void) => {
    mocks.listeners.set(event, callback)
    return vi.fn()
  }),
}))
vi.mock('@tauri-apps/api/dialog', () => ({
  open: vi.fn(), save: vi.fn(), message: vi.fn(), ask: vi.fn(),
}))

describe('Tauri compression integration', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    mocks.invoke.mockReset()
    mocks.invoke.mockImplementation(async (command: string) => command === 'load_app_settings' ? '{}' : undefined)
    mocks.listen.mockClear()
    mocks.listeners.clear()
  })

  it('completes a decompression task through the command bridge', async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'load_app_settings') return '{}'
      if (command === 'extract_file') return 'C:/output'
      return undefined
    })
    const commands = useTauriCommands()
    const tasks = useTaskStore()

    const result = await commands.decompressFile('C:/archives/demo.zip', {
      outputPath: 'C:/output',
      keepStructure: true,
      overwrite: false,
      deleteAfter: false,
      conflictPolicy: 'rename',
    })

    expect(result).toBe('C:/output')
    expect(mocks.invoke).toHaveBeenCalledWith('extract_file', expect.objectContaining({
      filePath: 'C:/archives/demo.zip',
      outputPath: 'C:/output',
      options: expect.objectContaining({ conflict_policy: 'rename' }),
    }))
    expect(tasks.tasks[0]).toMatchObject({ status: 'completed', progress: 100 })
  })

  it('forwards complete compression settings', async () => {
    const commands = useTauriCommands()

    await commands.compressFiles('task-1', ['C:/data/a.txt'], 'C:/data/a.zip', {
      format: 'zip',
      level: 9,
      password: undefined,
      split_size: 1024,
      preserve_paths: true,
      delete_after: false,
    })

    expect(mocks.invoke).toHaveBeenCalledWith('compress_files', {
      taskId: 'task-1',
      files: ['C:/data/a.txt'],
      outputPath: 'C:/data/a.zip',
      options: expect.objectContaining({ format: 'zip', level: 9, split_size: 1024 }),
    })
  })

  it('registers task listeners only once and applies progress events', async () => {
    const tasks = useTaskStore()
    await tasks.initListeners()
    await tasks.initListeners()
    expect(mocks.listen).toHaveBeenCalledTimes(4)

    tasks.addTask({
      id: 'task-2', name: 'demo.zip', type: 'decompression',
      sourceFiles: ['demo.zip'], outputPath: 'output',
    })
    mocks.listeners.get('task-progress')?.({ payload: { task_id: 'task-2', progress: 0.42 } })
    expect(tasks.tasks[0].progress).toBe(42)
  })

  it('preserves real byte totals when a final stage event carries zero placeholders', async () => {
    const tasks = useTaskStore()
    await tasks.initListeners()
    tasks.addTask({
      id: 'zip-telemetry', name: 'large.zip', type: 'compression',
      sourceFiles: ['large.bin'], outputPath: 'large.zip',
    })
    const listener = mocks.listeners.get('task-progress')!

    listener({ payload: {
      task_id: 'zip-telemetry', progress: 1, processed_bytes: 67_108_864,
      total_bytes: 67_108_864, speed: '24.0 MB/s', eta_seconds: 0,
    } })
    listener({ payload: {
      task_id: 'zip-telemetry', progress: 1, stage: 'Finalizing',
      processed_bytes: 0, total_bytes: 0,
    } })

    expect(tasks.tasks[0]).toMatchObject({
      progress: 100,
      processedBytes: 67_108_864,
      totalBytes: 67_108_864,
      speed: '24.0 MB/s',
      etaSeconds: 0,
    })
  })

  it('returns a task to pending when a password is required', async () => {
    const tasks = useTaskStore()
    await tasks.initListeners()
    tasks.addTask({
      id: 'task-3', name: 'secure.7z', type: 'decompression',
      sourceFiles: ['secure.7z'], outputPath: 'output',
    })
    tasks.updateTaskStatus('task-3', 'extracting')

    mocks.listeners.get('password-required')?.({ payload: {
      task_id: 'task-3', file_path: 'secure.7z', file_name: 'secure.7z', format: '7z',
    } })
    expect(tasks.tasks[0]).toMatchObject({ status: 'pending', passwordRequired: true })
  })
})
