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
  it('cannot resurrect a task that settles while a pause request is in flight', async () => {
    const store = useTaskStore()
    store.addTask({ id: 'settling', name: 'small.rar', type: 'decompression', sourceFiles: [], outputPath: '' })
    store.updateTaskStatus('settling', 'extracting')
    let finish!: () => void
    mocks.invoke.mockImplementationOnce(() => new Promise<void>(resolve => { finish = resolve }))
    const pending = store.pauseTask('settling')
    store.updateTaskStatus('settling', 'completed')
    finish()
    expect(await pending).toBe(false)
    expect(store.tasks[0].status).toBe('completed')
    expect(store.tasks[0].pausedAt).toBeUndefined()
  })
  it('excludes multiple real pause intervals from history, including terminal while paused', async () => {
    vi.useFakeTimers()
    try {
      const store = useTaskStore()
      store.addTask({ id: 'timed', name: '资料.rar', type: 'decompression', sourceFiles: [], outputPath: '' })
      store.updateTaskStatus('timed', 'extracting')
      vi.advanceTimersByTime(500)
      expect(await store.pauseTask('timed')).toBe(true)
      vi.advanceTimersByTime(5000)
      expect(await store.resumeTask('timed')).toBe(true)
      vi.advanceTimersByTime(500)
      expect(await store.pauseTask('timed')).toBe(true)
      vi.advanceTimersByTime(3000)
      store.updateTaskStatus('timed', 'completed')
      await store.waitForHistoryPersistence('timed')
      expect(mocks.invoke).toHaveBeenCalledWith('save_task_history', { record: expect.objectContaining({ durationMs: 1000 }) })
      expect(store.tasks[0].pausedAt).toBeUndefined()
    } finally { vi.useRealTimers() }
  })

  it('does not restore speed while paused or retain a cleared terminal sample', async () => {
    const store = useTaskStore()
    await store.initListeners()
    store.addTask({ id: 'rate', name: 'large.rar', type: 'decompression', sourceFiles: [], outputPath: '' })
    store.updateTaskStatus('rate', 'extracting')
    const emit = (speed: string | null, eta_seconds: number | null) => mocks.listeners.get('task-progress')!({ payload: {
      task_id: 'rate', progress: 0.5, processed_bytes: 50, total_bytes: 100, speed, eta_seconds,
    } })
    emit('10 MB/s', 3)
    expect(store.tasks[0].speed).toBe('10 MB/s')
    await store.pauseTask('rate')
    emit('10 MB/s', 3)
    expect(store.tasks[0].speed).toBeUndefined()
    expect(store.tasks[0].etaSeconds).toBeUndefined()
    await store.resumeTask('rate')
    emit('10 MB/s', 3)
    emit(null, null)
    expect(store.tasks[0].speed).toBeUndefined()
    expect(store.tasks[0].etaSeconds).toBeUndefined()
  })
  it.each(['completed', 'failed', 'cancelled'] as const)('freezes %s progress against late engine events', async status => {
    const store = useTaskStore()
    await store.initListeners()
    store.addTask({ id: 'terminal', name: 'archive.zip', type: 'compression', sourceFiles: [], outputPath: '' })
    const task = store.tasks[0]
    task.progress = 66
    store.updateTaskStatus('terminal', status)
    expect(task.progress).toBe(status === 'completed' ? 100 : 66)
    const snapshot = JSON.stringify(task)
    mocks.listeners.get('task-progress')!({ payload: { task_id: 'terminal', progress: 0.2, stage: 'Splitting', processed_bytes: 20, total_bytes: 100 } })
    expect(JSON.stringify(task)).toBe(snapshot)
  })
  it('resets byte telemetry on entering splitting without marking 100% complete', async () => {
    const store = useTaskStore()
    await store.initListeners()
    store.addTask({ id: 'split', name: '资料.zip', type: 'compression', sourceFiles: [], outputPath: '' })
    const task = store.tasks[0]
    task.status = 'compressing'
    task.processedBytes = 10000
    task.totalBytes = 10000
    task.speed = 800
    task.etaSeconds = 1
    mocks.listeners.get('task-progress')?.({ payload: {
      task_id: 'split', stage: 'Splitting', progress: 0, processed_bytes: 0, total_bytes: 1000, output_bytes: 0,
    } })
    expect(task).toMatchObject({ stage: 'Splitting', progress: 0, processedBytes: 0, totalBytes: 1000, outputBytes: 0 })
    expect(task.speed).toBeUndefined()
    expect(task.etaSeconds).toBeUndefined()
    expect(task.compressionSourceProgress).toEqual({ processedBytes: 10000, totalBytes: 10000 })
    mocks.listeners.get('task-progress')?.({ payload: {
      task_id: 'split', stage: 'Splitting', progress: 1, processed_bytes: 1000, total_bytes: 1000,
    } })
    expect(task.status).toBe('compressing')
    expect(task.processedBytes).toBe(1000)
    expect(task.compressionSourceProgress).toEqual({ processedBytes: 10000, totalBytes: 10000 })
  })

  beforeEach(() => {
    setActivePinia(createPinia())
    mocks.listeners.clear()
    mocks.invoke.mockReset()
    mocks.invoke.mockResolvedValue(undefined)
  })

  it('keeps the live queue separate from persisted history reads', () => {
    const store = useTaskStore()

    expect(store).not.toHaveProperty('fetchTasks')
    expect(mocks.invoke).not.toHaveBeenCalledWith('list_task_history', expect.anything())
  })

  it('persists every failure with an explicit final reason and terminal log', async () => {
    const store = useTaskStore()
    store.addTask({
      id: 'damaged-rar',
      name: 'damaged.rar',
      type: 'decompression',
      sourceFiles: ['C:/archives/damaged.rar'],
      outputPath: 'C:/output',
    })

    store.failTask('damaged-rar', 'RAR 文件损坏或不完整')
    await store.waitForHistoryPersistence('damaged-rar')

    expect(store.tasks[0]).toMatchObject({
      status: 'failed',
      error: 'RAR 文件损坏或不完整',
    })
    expect(store.tasks[0].logs.at(-1)).toMatchObject({
      message: '最终失败原因：RAR 文件损坏或不完整',
      severity: 'error',
    })
    expect(mocks.invoke).toHaveBeenCalledWith('save_task_history', {
      record: expect.objectContaining({
        status: 'failed',
        errorMessage: 'RAR 文件损坏或不完整',
        logs: expect.arrayContaining([
          expect.objectContaining({ message: '最终失败原因：RAR 文件损坏或不完整' }),
        ]),
      }),
    })
  })

  it('persists the last observed stage without inventing an error category', async () => {
    const store = useTaskStore()
    await store.initListeners()
    store.addTask({ id: 'stage-failure', name: 'test.zip', type: 'decompression', sourceFiles: [], outputPath: '' })
    mocks.listeners.get('task-progress')?.({ payload: {
      task_id: 'stage-failure', stage: 'Verifying', progress: 0.9,
    } })
    store.failTask('stage-failure', 'verification stopped')
    await store.waitForHistoryPersistence('stage-failure')
    expect(mocks.invoke).toHaveBeenCalledWith('save_task_history', {
      record: expect.objectContaining({ failure: {
        schemaVersion: 1, category: 'unknown', stage: 'Verifying', evidence: 'observed-stage',
      } }),
    })
  })

  it('uses a visible fallback when a caller omits the failure reason', async () => {
    const store = useTaskStore()
    store.addTask({
      id: 'unknown-failure',
      name: 'unknown.zip',
      type: 'decompression',
      sourceFiles: ['C:/archives/unknown.zip'],
      outputPath: 'C:/output',
    })

    store.updateTaskStatus('unknown-failure', 'failed')
    await store.waitForHistoryPersistence('unknown-failure')

    expect(store.tasks[0].error).toContain('未返回具体错误原因')
    expect(store.tasks[0].logs.at(-1)?.message).toContain('最终失败原因：')
  })

  it('serializes final-reason snapshots so older writes cannot overwrite newer ones', async () => {
    let finishFirst!: () => void
    mocks.invoke.mockImplementationOnce(() => new Promise<void>(resolve => { finishFirst = resolve }))
    const store = useTaskStore()
    store.addTask({ id: 'ordered', name: 'test.rar', type: 'decompression', sourceFiles: [], outputPath: '' })
    store.failTask('ordered', 'initial failure')
    store.failTask('ordered', 'precise failure')
    expect(mocks.invoke).toHaveBeenCalledTimes(1)
    store.clearFinishedTasks()
    expect(store.tasks).toHaveLength(1)
    finishFirst()
    expect(await store.waitForHistoryPersistence('ordered')).toBe(true)
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, 'save_task_history', {
      record: expect.objectContaining({ errorMessage: 'precise failure' }),
    })
    expect(store.tasks[0].historySaving).toBe(false)
  })

  it('retains an unsaved task and retries persistence without rerunning extraction', async () => {
    const warning = vi.spyOn(console, 'warn').mockImplementation(() => {})
    mocks.invoke.mockRejectedValueOnce(new Error('database unavailable'))
    const store = useTaskStore()
    store.addTask({ id: 'unsaved', name: 'test.rar', type: 'decompression', sourceFiles: [], outputPath: '' })
    store.failTask('unsaved', 'source damaged')
    expect(await store.waitForHistoryPersistence('unsaved')).toBe(false)
    expect(store.tasks[0].historySaveError).toContain('历史记录保存失败')
    store.removeTask('unsaved')
    store.clearFinishedTasks()
    expect(store.tasks).toHaveLength(1)
    expect(await store.retryHistoryPersistence('unsaved')).toBe(true)
    expect(store.tasks[0].historySaveError).toBeUndefined()
    expect(store.tasks[0].error).toBe('source damaged')
    expect(mocks.invoke).toHaveBeenCalledTimes(2)
    store.clearFinishedTasks()
    expect(store.tasks).toHaveLength(0)
    warning.mockRestore()
  })

  it('cancels a queued task locally without calling a backend process', async () => {
    const store = useTaskStore()
    store.addTask({
      id: 'queued-task',
      name: 'queued.zip',
      type: 'decompression',
      sourceFiles: ['C:/archives/queued.zip'],
      outputPath: 'C:/archives',
    })

    await expect(store.cancelTask('queued-task')).resolves.toBe(true)
    expect(store.tasks[0].status).toBe('cancelled')
    expect(mocks.invoke).not.toHaveBeenCalledWith('cancel_compression', expect.anything())
  })

  it('pauses, resumes, and can stop the same active archive task', async () => {
    const store = useTaskStore()
    store.addTask({
      id: 'active-task',
      name: 'active.7z',
      type: 'compression',
      sourceFiles: ['C:/archives/source.bin'],
      outputPath: 'C:/archives/active.7z',
    })
    store.updateTaskStatus('active-task', 'compressing')

    await expect(store.pauseTask('active-task')).resolves.toBe(true)
    expect(store.tasks[0]).toMatchObject({ status: 'paused', pausedFromStatus: 'compressing' })
    expect(mocks.invoke).toHaveBeenCalledWith('pause_archive_task', { taskId: 'active-task' })

    await expect(store.resumeTask('active-task')).resolves.toBe(true)
    expect(store.tasks[0].status).toBe('compressing')
    expect(mocks.invoke).toHaveBeenCalledWith('resume_archive_task', { taskId: 'active-task' })

    await store.pauseTask('active-task')
    await expect(store.cancelTask('active-task')).resolves.toBe(true)
    expect(store.tasks[0].status).toBe('cancelled')
    expect(mocks.invoke).toHaveBeenCalledWith('cancel_compression', { taskId: 'active-task' })
  })

  it('does not advertise archive pause semantics for special-compression tasks', async () => {
    const store = useTaskStore()
    store.addTask({
      id: 'video-task',
      name: 'video.mp4',
      type: 'compression',
      workloadKind: 'video',
      sourceFiles: ['C:/video.mp4'],
      outputPath: 'C:/video-compressed.mp4',
    })
    store.updateTaskStatus('video-task', 'compressing')

    await expect(store.pauseTask('video-task')).resolves.toBe(false)
    expect(store.tasks[0].status).toBe('compressing')
    expect(mocks.invoke).not.toHaveBeenCalledWith('pause_archive_task', expect.anything())
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

  it('keeps video heartbeats attached to the last measured progress snapshot', async () => {
    const store = useTaskStore()
    await store.initListeners()
    store.addTask({
      id: 'video-task',
      name: 'sample.mp4',
      type: 'compression',
      workloadKind: 'video',
      sourceFiles: ['C:/videos/sample.mov'],
      outputPath: 'C:/videos/sample-compressed.mp4',
    })
    store.updateTaskStatus('video-task', 'compressing')

    mocks.listeners.get('task-progress')?.({
      payload: {
        task_id: 'video-task',
        stage: 'Encoding',
        progress: 0.425,
        speed: '1.25x',
        eta_seconds: 12,
        output_bytes: 8192,
        output_bytes_estimated: true,
        current_time_ms: 1250,
        speed_multiple: 1.25,
        output_to_input_ratio: 0.4,
      },
    })
    mocks.listeners.get('task-progress')?.({
      payload: {
        task_id: 'video-task',
        stage: 'still-encoding',
        progress: 0.425,
        output_bytes: 8192,
        output_bytes_estimated: true,
        current_time_ms: 1250,
        speed_multiple: 1.25,
        output_to_input_ratio: 0.4,
        heartbeat_seconds_since_progress: 5,
        heartbeat_at: '2026-08-29T00:00:00Z',
      },
    })

    expect(store.tasks[0]).toMatchObject({
      progress: 42.5,
      stage: 'still-encoding',
      currentTimeMs: 1250,
      speedMultiple: 1.25,
      outputBytes: 8192,
      outputBytesEstimated: true,
      outputToInputRatio: 0.4,
      heartbeatSecondsSinceProgress: 5,
      heartbeatAt: '2026-08-29T00:00:00Z',
    })
  })
})
