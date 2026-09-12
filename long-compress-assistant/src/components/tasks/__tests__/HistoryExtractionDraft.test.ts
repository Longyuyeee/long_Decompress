import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import HistoryExtractionDraft from '../HistoryExtractionDraft.vue'
import { useTaskStore } from '@/stores/task'
import type { TaskHistoryRecord } from '@/types/taskHistory'

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), open: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/dialog', () => ({ open: mocks.open }))
const record: TaskHistoryRecord = {
  id: 'old-task', name: 'old.rar', taskType: 'decompression', workloadKind: 'archive',
  status: 'failed', sourcePaths: ['C:/old.rar'], outputPath: 'C:/old-output',
  completedAt: '2026-09-12T00:00:00Z', durationMs: 0, processedBytes: 0, totalBytes: 0,
  errorMessage: 'source missing', logs: [],
}
const setup = async () => {
  const pinia = createPinia()
  setActivePinia(pinia)
  const wrapper = mount(HistoryExtractionDraft, { props: { record }, global: { plugins: [pinia] } })
  await wrapper.get('[data-testid="history-draft-select-output"]').trigger('click')
  await flushPromises()
  return { wrapper, store: useTaskStore() }
}
describe('history extraction draft', () => {
  beforeEach(() => {
    vi.resetAllMocks()
    mocks.open.mockResolvedValue('D:/output')
    mocks.invoke.mockImplementation(async command => command === 'detect_split_archive'
      ? { is_split: false } : { name: 'old.rar', size: 100, is_dir: false })
  })
  it('creates a new pending individual task without executing or changing history', async () => {
    const { wrapper, store } = await setup()
    await wrapper.get('[data-testid="history-draft-create"]').trigger('click')
    await flushPromises()
    expect(store.tasks).toHaveLength(1)
    expect(store.tasks[0]).toMatchObject({ status: 'pending', configurationMode: 'individual', outputPath: 'D:/output', recycleSourceAfterExtract: false, extractToSubfolder: true })
    expect(store.tasks[0].id).not.toBe(record.id)
    expect(store.tasks[0].password).toBeUndefined()
    expect(store.tasks[0].logs[0].message).toContain(record.id)
    expect(record.status).toBe('failed')
    expect(mocks.invoke.mock.calls.map(call => call[0])).toEqual(['detect_split_archive', 'get_file_info'])
    await wrapper.get('[data-testid="history-draft-create"]').trigger('click')
    expect(store.tasks).toHaveLength(1)
  })
  it('rejects missing sources and allows replacing the source', async () => {
    const { wrapper, store } = await setup()
    mocks.invoke.mockRejectedValueOnce(new Error('file missing'))
    await wrapper.get('[data-testid="history-draft-create"]').trigger('click')
    await flushPromises()
    expect(store.tasks).toHaveLength(0)
    expect(wrapper.text()).toContain('file missing')
    await wrapper.get('[data-testid="history-draft-source"]').setValue('C:/replacement.rar')
    await wrapper.get('[data-testid="history-draft-create"]').trigger('click')
    await flushPromises()
    expect(store.tasks[0].sourceFiles).toEqual(['C:/replacement.rar'])
  })
  it('normalizes a complete split group to its first volume', async () => {
    const { wrapper, store } = await setup()
    mocks.invoke.mockResolvedValueOnce({ is_split: true, first_part: 'C:/set.part1.rar', is_complete: true })
    await wrapper.get('[data-testid="history-draft-create"]').trigger('click')
    await flushPromises()
    expect(store.tasks[0].sourceFiles).toEqual(['C:/set.part1.rar'])
  })
  it('blocks incomplete volumes before creating a task', async () => {
    const { wrapper, store } = await setup()
    mocks.invoke.mockResolvedValueOnce({ is_split: true, is_complete: false, missing_parts: ['part2.rar'] })
    await wrapper.get('[data-testid="history-draft-create"]').trigger('click')
    await flushPromises()
    expect(store.tasks).toHaveLength(0)
    expect(wrapper.text()).toContain('part2.rar')
  })
  it.each([{ size: 0, is_dir: false }, { size: 10, is_dir: true }])('rejects unsuitable source metadata: %j', async metadata => {
    const { wrapper, store } = await setup()
    mocks.invoke.mockResolvedValueOnce({ is_split: false }).mockResolvedValueOnce({ name: 'old.rar', ...metadata })
    await wrapper.get('[data-testid="history-draft-create"]').trigger('click')
    await flushPromises()
    expect(store.tasks).toHaveLength(0)
    expect(wrapper.text()).toContain('来源必须是非空归档文件')
  })
  it('requires an explicitly selected output directory', async () => {
    mocks.open.mockResolvedValue(null)
    const { wrapper, store } = await setup()
    await wrapper.get('[data-testid="history-draft-create"]').trigger('click')
    await flushPromises()
    expect(store.tasks).toHaveLength(0)
    expect(mocks.invoke).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('选择本次输出目录')
  })
  it('does not create a task after the panel is closed during inspection', async () => {
    const { wrapper, store } = await setup()
    let resolve!: (value: unknown) => void
    mocks.invoke.mockImplementationOnce(() => new Promise(done => { resolve = done }))
    await wrapper.get('[data-testid="history-draft-create"]').trigger('click')
    wrapper.unmount()
    resolve({ is_split: false })
    await flushPromises()
    expect(store.tasks).toHaveLength(0)
  })
})
