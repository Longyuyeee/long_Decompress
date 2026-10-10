import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import HistoryMediaDraft from '../HistoryMediaDraft.vue'
import { useCompressionStore } from '@/stores/compression'
import { usePdfWorkspaceStore } from '@/stores/pdfWorkspace'
import { useTaskStore } from '@/stores/task'
import { createDefaultImageSettings } from '@/utils/imageCompressionWorkspace'
import type { TaskHistoryRecord } from '@/types/taskHistory'

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
const base = { name: 'source', taskType: 'compression', status: 'failed', outputPath: 'D:/out',
  completedAt: '2026-10-10T00:00:00Z', durationMs: 1, processedBytes: 0, totalBytes: 0, logs: [] } as const
const cases: TaskHistoryRecord[] = [
  { ...base, id: 'old-image', workloadKind: 'image', sourcePaths: ['C:/a.png'], logs: [], executionConfig: {
    schemaVersion: 1, kind: 'image-compression', settings: { ...createDefaultImageSettings(), quality: 61, outputDirectory: 'D:/images' } } },
  { ...base, id: 'old-video', workloadKind: 'video', sourcePaths: ['C:/a.mp4'], logs: [], executionConfig: {
    schemaVersion: 1, kind: 'video-compression', outputDirectory: 'D:/video', settings: { preset: 'small', quality: 40, maxWidth: 1280, maxHeight: 720 } } },
  { ...base, id: 'old-pdf', workloadKind: 'pdf', sourcePaths: ['C:/a.pdf'], logs: [], executionConfig: {
    schemaVersion: 1, kind: 'pdf-optimization', outputDirectory: 'D:/pdf', mode: 'compatible-image-optimization', allowLargerOutput: true } },
]
const allDrafts = () => [...useCompressionStore().imageItems, ...useCompressionStore().videoItems, ...usePdfWorkspaceStore().items]
describe('retry media using the corresponding analysis and confirmation flow', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    mocks.invoke.mockReset()
    mocks.invoke.mockImplementation(async (_cmd, { path }) => ({ name: path.split('/').pop(), size: 500, is_dir: path.startsWith('E:/') }))
  })
  it.each(cases)('restores $workloadKind independently and never starts or changes global settings', async record => {
    const store = useCompressionStore()
    const original = JSON.stringify([store.imageGlobalSettings, store.videoGlobalSettings, store.videoOutputDirectory, usePdfWorkspaceStore().outputDirectory])
    const wrapper = mount(HistoryMediaDraft, { props: { record } })
    expect(wrapper.get('[data-testid="media-restore-notice"]').text()).toContain('上次参数')
    await wrapper.get('[data-testid="media-retry-output"]').setValue('E:/retry')
    await wrapper.get('[data-testid="media-retry-create"]').trigger('click')
    await flushPromises()
    expect(allDrafts()).toHaveLength(1)
    expect(allDrafts()[0]).toMatchObject({ path: record.sourcePaths[0], retryOfTaskId: record.id })
    if (record.workloadKind === 'image') expect(store.imageItems[0]).toMatchObject({ status: 'inspecting', settings: { quality: 61, outputDirectory: 'E:/retry' } })
    if (record.workloadKind === 'video') expect(store.videoItems[0]).toMatchObject({ status: 'planning', outputDirectory: 'E:/retry', settings: { quality: 40, maxWidth: 1280 } })
    if (record.workloadKind === 'pdf') expect(usePdfWorkspaceStore().items[0]).toMatchObject({ status: 'analyzing', outputDirectory: 'E:/retry', mode: 'compatible-image-optimization', frozen: false, riskConfirmed: false, allowLargerOutput: true })
    expect(useTaskStore().tasks).toHaveLength(0)
    expect(JSON.stringify([store.imageGlobalSettings, store.videoGlobalSettings, store.videoOutputDirectory, usePdfWorkspaceStore().outputDirectory])).toBe(original)
    expect(wrapper.get('a').attributes('href')).toBe(`#/special-compression?mode=${record.workloadKind}`)
    wrapper.unmount()
    const reopened = mount(HistoryMediaDraft, { props: { record } })
    await reopened.get('[data-testid="media-retry-create"]').trigger('click')
    await flushPromises()
    expect(allDrafts()).toHaveLength(1)
    reopened.unmount()
  })
  it('a missing source is not replaced with a different file', async () => {
    const wrapper = mount(HistoryMediaDraft, { props: { record: cases[0] } })
    mocks.invoke.mockRejectedValueOnce(new Error('file missing'))
    await wrapper.get('[data-testid="media-retry-create"]').trigger('click')
    await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('file missing')
    expect(allDrafts()).toHaveLength(0)
    wrapper.unmount()
  })
  it('old and incomplete snapshots explicitly request fresh configuration', async () => {
    const wrapper = mount(HistoryMediaDraft, { props: { record: { ...cases[0], executionConfig: null } } })
    expect(wrapper.get('[data-testid="media-restore-notice"]').text()).toContain('默认值')
    await wrapper.setProps({ record: { ...cases[0], id: 'incomplete', executionConfig: { schemaVersion: 1, kind: 'image-compression' } as any } })
    expect(wrapper.get('[data-testid="media-restore-notice"]').text()).toContain('默认值')
    wrapper.unmount()
  })
  it('unmount while checking does not leave an unexpected draft behind', async () => {
    const wrapper = mount(HistoryMediaDraft, { props: { record: cases[1] } })
    let resolve!: (value: unknown) => void
    mocks.invoke.mockImplementationOnce(() => new Promise(done => { resolve = done }))
    await wrapper.get('[data-testid="media-retry-create"]').trigger('click')
    wrapper.unmount()
    resolve({ name: 'a.mp4', size: 50, is_dir: false })
    await flushPromises()
    expect(allDrafts()).toHaveLength(0)
  })
})
