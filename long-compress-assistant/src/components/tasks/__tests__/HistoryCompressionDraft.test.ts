import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import HistoryCompressionDraft from '../HistoryCompressionDraft.vue'
import { useCompressionStore } from '@/stores/compression'
import type { TaskHistoryRecord } from '@/types/taskHistory'

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
const record: TaskHistoryRecord = {
  id: 'old-compression', name: '照片合集', taskType: 'compression', workloadKind: 'archive', status: 'failed',
  sourcePaths: ['C:/a.jpg', 'C:/b.jpg'], outputPath: 'D:/out/照片合集.zip',
  completedAt: '2026-10-10T00:00:00Z', durationMs: 2, processedBytes: 0, totalBytes: 0, logs: [],
  executionConfig: { schemaVersion: 1, kind: 'archive-compression', outputDirectory: 'D:/out', passwordRequired: false,
    settings: { format: 'zip', level: 8, filename: '照片合集', splitArchive: false, splitSize: '100',
      keepStructure: true, verifyAfter: true, createSolidArchive: false, fastPack: true } },
}
const setup = (input = record) => {
  const pinia = createPinia()
  setActivePinia(pinia)
  const options = { props: { record: input }, global: { plugins: [pinia], stubs: { CompressionSettingsPanel: true } } }
  return { wrapper: mount(HistoryCompressionDraft, options), options, store: useCompressionStore() }
}
describe('create another archive without repeating the failed task', () => {
  beforeEach(() => {
    vi.resetAllMocks()
    mocks.invoke.mockImplementation(async (_command, args) => ({ name: args.path.split('/').pop(), size: 42, is_dir: !args.path.endsWith('.jpg') }))
  })
  it('restores actual settings, checks every source and creates only a draft, including after reopening details', async () => {
    const { wrapper, options, store } = setup()
    expect(wrapper.get('[data-testid="compression-restore-notice"]').text()).toContain('已带入')
    await wrapper.get('[data-testid="compression-retry-output"]').setValue('E:/new')
    await wrapper.get('[data-testid="compression-retry-create"]').trigger('click')
    await flushPromises()
    expect(store.groups).toHaveLength(1)
    expect(store.groups[0]).toMatchObject({ retryOfTaskId: record.id, outputPath: 'E:/new', settings: {
      level: 8, fastPack: false, password: '', deleteAfter: false, filename: '照片合集',
    } })
    expect(store.groups[0].files.map(file => file.path)).toEqual(record.sourcePaths)
    expect(store.groups[0].taskId).toBeUndefined()
    expect(store.autoStartRequested).toBe(false)
    expect(mocks.invoke.mock.calls.map(([cmd]) => cmd)).toEqual(['get_file_info', 'get_file_info', 'get_file_info'])
    expect(record.status).toBe('failed')
    wrapper.unmount()
    const reopened = mount(HistoryCompressionDraft, options)
    await reopened.get('[data-testid="compression-retry-create"]').trigger('click')
    await flushPromises()
    expect(store.groups).toHaveLength(1)
    reopened.unmount()
  })
  it('does not silently omit a missing source from a multi-file archive', async () => {
    const { wrapper, store } = setup()
    mocks.invoke.mockRejectedValueOnce(new Error('来源文件不存在'))
    await wrapper.get('[data-testid="compression-retry-create"]').trigger('click')
    await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('来源文件不存在')
    expect(store.groups).toHaveLength(0)
    wrapper.unmount()
  })
  it('rejects a non-directory output and leaves settings available to correct', async () => {
    const { wrapper, store } = setup()
    mocks.invoke.mockResolvedValue({ name: 'file', size: 42, is_dir: false })
    await wrapper.get('[data-testid="compression-retry-create"]').trigger('click')
    await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('不是目录')
    expect(store.groups).toHaveLength(0)
    wrapper.unmount()
  })
  it('never turns a password-protected retry into an unencrypted archive silently', async () => {
    const input = structuredClone(record)
    if (input.executionConfig?.kind === 'archive-compression') input.executionConfig.passwordRequired = true
    const { wrapper, store } = setup(input)
    await wrapper.get('[data-testid="compression-retry-create"]').trigger('click')
    await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('重新填写')
    expect(store.groups).toHaveLength(0)
    expect(mocks.invoke).not.toHaveBeenCalled()
    wrapper.unmount()
  })
  it('old records say the settings are defaults, not restored parameters', () => {
    const { wrapper } = setup({ ...record, executionConfig: null })
    expect(wrapper.get('[data-testid="compression-restore-notice"]').text()).toContain('不是上次配置')
    expect((wrapper.get('[data-testid="compression-retry-output"]').element as HTMLInputElement).value).toBe('')
    wrapper.unmount()
  })
  it('changing the selected record while checks are pending never creates the previous draft', async () => {
    const { wrapper, store } = setup()
    let resolve!: (value: unknown) => void
    mocks.invoke.mockImplementationOnce(() => new Promise(done => { resolve = done }))
    await wrapper.get('[data-testid="compression-retry-create"]').trigger('click')
    await wrapper.setProps({ record: { ...record, id: 'other' } })
    resolve({ name: 'a.jpg', size: 42, is_dir: false })
    await flushPromises()
    expect(store.groups).toHaveLength(0)
    wrapper.unmount()
  })
})
