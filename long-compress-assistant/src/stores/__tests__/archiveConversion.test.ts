import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useArchiveConversionStore } from '../archiveConversion'
import { useTaskStore } from '../task'
import { createTaskHistoryRecord } from '@/types/taskHistory'
const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => vi.fn()) }))
beforeEach(() => { setActivePinia(createPinia()); mocks.invoke.mockReset() })
function draft() {
  const store = useArchiveConversionStore()
  store.sourcePath = 'C:\\原包.zip'; store.outputPath = 'C:\\新包.7z'
  store.sourcePassword = 'input-secret'; store.password = 'output-secret'
  store.encryptFilenames = true; store.splitMiB = 1
  return store
}
describe('archive conversion lifecycle', () => {
  it('does not revive a terminal task when cancellation acknowledgement arrives during history persistence', async () => {
    let rejectConversion!: (reason: string) => void
    let acknowledgeCancel!: () => void
    let saveHistory!: () => void
    mocks.invoke.mockImplementation(command => {
      if (command === 'convert_archive') return new Promise((_, reject) => { rejectConversion = reject })
      if (command === 'cancel_compression') return new Promise<void>(resolve => { acknowledgeCancel = resolve })
      if (command === 'save_task_history') return new Promise<void>(resolve => { saveHistory = resolve })
      return Promise.resolve()
    })
    const store = draft(); const pending = store.start()
    await vi.waitFor(() => expect(rejectConversion).toBeTypeOf('function'))
    const cancellation = store.cancel()
    rejectConversion('转换已取消，原包保留')
    await vi.waitFor(() => expect(saveHistory).toBeTypeOf('function'))
    expect(store.task?.status).toBe('cancelled')
    acknowledgeCancel(); await cancellation
    expect(store.task?.status).toBe('cancelled')
    saveHistory(); await pending
  })
  it('keeps input and output passwords separate and persists only conversion settings', async () => {
    mocks.invoke.mockImplementation(async command => command === 'convert_archive'
      ? { primaryPath: 'C:\\新包.7z.001', files: [], totalBytes: 25, split: true } : undefined)
    const store = draft()
    await store.start()
    expect(mocks.invoke).toHaveBeenCalledWith('convert_archive', expect.objectContaining({ sourcePassword: 'input-secret', options: expect.objectContaining({ password: 'output-secret', verify_after: true, delete_after: false, split_size: 1048576 }) }))
    const record = mocks.invoke.mock.calls.find(([command]) => command === 'save_task_history')![1].record
    expect(record.executionConfig.kind).toBe('archive-conversion')
    expect(JSON.stringify(record)).not.toMatch(/input-secret|output-secret/)
    expect(record.status).toBe('completed')
    expect(store.password).toBe(''); expect(store.sourcePassword).toBe('')
    store.restore(record)
    expect(store.outputPath).toBe(''); expect(store.sourcePath).toBe('C:\\原包.zip')
    expect(store.splitMiB).toBe(1)
  })
  it('blocks incomplete configuration without creating a task', async () => {
    const store = draft(); store.password = ''
    await store.start()
    expect(store.error).toContain('新包密码')
    expect(useTaskStore().tasks).toHaveLength(0)
    expect(mocks.invoke).not.toHaveBeenCalled()
  })
  it('persists the real final failure and no output', async () => {
    mocks.invoke.mockImplementation(async command => { if (command === 'convert_archive') throw '转换读取原包失败：文件损坏' })
    const store = draft(); await store.start()
    const record = createTaskHistoryRecord(useTaskStore().tasks[0])
    expect(record.status).toBe('failed'); expect(record.errorMessage).toContain('文件损坏')
    expect(record.archiveOutput).toBeNull()
    expect(store.busy).toBe(false)
  })
  it('does not declare cancellation complete before the engine settles', async () => {
    let rejectConversion!: (reason: string) => void
    mocks.invoke.mockImplementation(async command => command === 'convert_archive'
      ? new Promise((_, reject) => { rejectConversion = reject }) : undefined)
    const store = draft(); const pending = store.start()
    await vi.waitFor(() => expect(rejectConversion).toBeTypeOf('function'))
    await store.cancel()
    expect(store.busy).toBe(true); expect(store.task?.status).toBe('cancelling')
    rejectConversion('转换已取消，原包保留'); await pending
    expect(store.busy).toBe(false); expect(store.task?.status).toBe('cancelled')
  })
})
