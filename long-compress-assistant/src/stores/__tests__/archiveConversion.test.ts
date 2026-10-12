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
  it('runs selected items sequentially, keeps individual settings and continues after a failure', async () => {
    mocks.invoke.mockImplementation(async (command, args) => {
      if (command !== 'convert_archive') return
      if (args.sourcePath.endsWith('first.zip')) throw '原包损坏'
      return { primaryPath: args.outputPath, files: [], totalBytes: 123, split: false }
    })
    const store = useArchiveConversionStore()
    store.addSources(['C:/first.zip', 'C:/second.zip', 'C:/third.zip'])
    store.updateItem(store.items[1].id, { format: 'zip', level: 2, password: 'personal-secret' })
    store.items[2].selected = false
    store.applySelected({ outputDirectory: 'D:/结果' })
    await store.start()
    const calls = mocks.invoke.mock.calls.filter(([command]) => command === 'convert_archive')
    expect(calls.map(([, args]) => args.sourcePath)).toEqual(['C:/first.zip', 'C:/second.zip'])
    expect(calls[1][1]).toMatchObject({ outputPath: 'D:/结果/second_转换.zip', options: { format: 'zip', level: 2, password: 'personal-secret' } })
    expect(store.taskFor(store.items[0])?.status).toBe('failed')
    expect(store.taskFor(store.items[1])?.status).toBe('completed')
    expect(store.pending.map(item => item.sourcePath)).toEqual(['C:/third.zip'])
    expect(JSON.stringify(mocks.invoke.mock.calls.filter(([cmd]) => cmd === 'save_task_history'))).not.toContain('personal-secret')
  })
  it('stops the batch after cancellation and retains unstarted settings', async () => {
    let settle!: (error: string) => void
    mocks.invoke.mockImplementation(async command => command === 'convert_archive' ? new Promise((_, reject) => { settle = reject }) : undefined)
    const store = useArchiveConversionStore()
    store.defaults.password = 'shared-secret'
    store.addSources(['C:/first.zip', 'C:/second.zip'])
    const run = store.start()
    await vi.waitFor(() => expect(settle).toBeTypeOf('function'))
    store.openFromExplorer(['C:/new.zip'], 'convert')
    await store.cancel(); settle('转换已取消'); await run
    expect(mocks.invoke.mock.calls.filter(([cmd]) => cmd === 'convert_archive')).toHaveLength(1)
    expect(store.pending).toHaveLength(2)
    expect(store.effectiveSettings(store.pending[0]).password).toBe('shared-secret')
    expect(store.taskFor(store.items[0])?.status).toBe('cancelled')
  })
  it('shows duplicate output errors before starting any item and retains settings when switching rows', async () => {
    const store = useArchiveConversionStore()
    store.addSources(['C:/first.zip', 'D:/first.zip'])
    store.applySelected({ outputDirectory: 'E:/结果' })
    store.chooseContextSource('C:/first.zip'); store.sourcePassword = 'original'
    store.chooseContextSource('D:/first.zip'); store.chooseContextSource('C:/first.zip')
    expect(store.sourcePassword).toBe('original')
    await store.start()
    expect(store.items[1].error).toContain('重复')
    expect(useTaskStore().tasks).toHaveLength(0)
  })
  it('keeps each custom filename when changing directories and lets defaults follow actual edits', () => {
    const store = useArchiveConversionStore()
    store.openFromExplorer(['C:/a.zip', 'C:/b.7z'], 'reconfigure')
    store.updateItem(store.items[0].id, { outputPath: 'D:/自定义.zip', level: 2 })
    store.applySelected({ outputDirectory: 'E:/统一', format: '7z' })
    expect(store.outputFor(store.items[0])).toBe('E:/统一/自定义.7z')
    store.addSources(['C:/c.zip'], 'reconfigure')
    store.updateDefaults({ format: '7z', level: 9 })
    expect(store.effectiveSettings(store.items[0]).level).toBe(2)
    expect(store.effectiveSettings(store.items[2])).toMatchObject({ format: '7z', level: 9 })
  })
  it('opens Explorer sources as explicit drafts and preserves ZIP for password/split changes', () => {
    const store = useArchiveConversionStore()
    store.openFromExplorer(['C:/旧包.zip.001', 'C:/another.7z', 'C:/旧包.zip.001'], 'reconfigure')
    expect(store.visible).toBe(true)
    expect(store.sourcePath).toBe('C:/旧包.zip.001')
    expect(store.format).toBe('zip')
    expect(store.contextSources).toHaveLength(2)
    expect(store.password).toBe(''); expect(store.sourcePassword).toBe('')
    store.outputPath = 'D:/新包.zip'; store.password = 'new-secret'
    store.chooseContextSource(store.sourcePath)
    expect(store.password).toBe('new-secret'); expect(store.outputPath).toBe('D:/新包.zip')
    store.selectSource('E:/手动选择.zip')
    expect(store.sourcePath).toBe('E:/手动选择.zip')
    expect(store.contextSources).toContain('E:/手动选择.zip')
    expect(useTaskStore().tasks).toHaveLength(0)
    expect(mocks.invoke).not.toHaveBeenCalled()
  })
  it('keeps an open or running conversion when a new Explorer request arrives', () => {
    const store = draft()
    store.openFromExplorer(['D:/new.zip'], 'reconfigure')
    expect(store.sourcePath).toBe('C:\\原包.zip')
    expect(store.password).toBe('output-secret')
    expect(store.contextSources).toEqual(['C:\\原包.zip', 'D:/new.zip'])
    store.busy = true
    store.chooseContextSource('D:/new.zip')
    expect(store.sourcePath).toBe('C:\\原包.zip')
    store.busy = false
    store.chooseContextSource('D:/new.zip')
    expect(store.sourcePath).toBe('D:/new.zip')
    expect(store.format).toBe('zip')
    expect(store.password).toBe(''); expect(store.outputPath).toBe('D:/new_调整.zip')
  })
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
    expect(store.outputPath).toBe('C:/原包_转换.7z'); expect(store.sourcePath).toBe('C:\\原包.zip')
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
  it('also waits for cleanup when cancelled from the global task controls', async () => {
    let rejectConversion!: (reason: string) => void
    mocks.invoke.mockImplementation(async command => command === 'convert_archive'
      ? new Promise((_, reject) => { rejectConversion = reject }) : undefined)
    const store = draft(); const pending = store.start()
    await vi.waitFor(() => expect(rejectConversion).toBeTypeOf('function'))
    await useTaskStore().cancelTask(store.task!.id)
    expect(store.task?.status).toBe('cancelling')
    expect(mocks.invoke.mock.calls.some(([command]) => command === 'save_task_history')).toBe(false)
    rejectConversion('转换已取消，原包保留'); await pending
    expect(store.task?.status).toBe('cancelled')
  })
})
