import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import CompressionAnalysisCard from '../CompressionAnalysisCard.vue'
import { useCompressionStore, type CompressionOptions } from '@/stores/compression'

const mocks = vi.hoisted(() => ({ analyze: vi.fn(), cancel: vi.fn() }))
vi.mock('@/composables/useTauriCommands', () => ({
  useTauriCommands: () => ({
    analyzeCompressionSources: mocks.analyze,
    cancelCompressionAnalysis: mocks.cancel,
  })
}))

const options: CompressionOptions = {
  format: 'zip', level: 6, password: '', filename: '', splitArchive: false,
  splitSize: '1024', keepStructure: true, deleteAfter: false,
  verifyAfter: true, createSolidArchive: false,
}
const result = {
  totalSize: 10_000_000, fileCount: 4, sampledFiles: 4, sampledBytes: 400_000,
  estimatedSize: 3_000_000, estimatedRatio: 0.3,
  estimatedSecondsLow: 2, estimatedSecondsHigh: 7, confidence: 'medium' as const,
  recommendedFormat: '7z', recommendedLevel: 7, recommendedSolid: true,
  lowValueBytes: 0, lowValueFileCount: 0,
  reasons: ['文本可压缩性较高'],
}

const mountCard = (modelValue = options, compact = false) => mount(CompressionAnalysisCard, {
  props: { jobId: 'job-1', paths: ['C:/input'], modelValue, compact },
  global: { plugins: [createPinia()] },
})

describe('CompressionAnalysisCard', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    mocks.analyze.mockResolvedValue(result)
    mocks.cancel.mockResolvedValue(undefined)
  })

  it('renders a shorter but explicit explanation in compact task details', () => {
    const wrapper = mountCard(options, true)

    expect(wrapper.get('[data-testid="compression-analysis"]').classes()).toContain('is-compact')
    expect(wrapper.get('.analysis-description').text()).toBe('抽样估算，不自动修改设置')
    expect(wrapper.get('.analysis-description').attributes('title')).toContain('最多 2 MiB')
  })

  it('runs bounded analysis, exposes the real estimate and applies existing settings fields', async () => {
    const wrapper = mountCard()
    await wrapper.get('.analysis-button').trigger('click')
    await flushPromises()

    expect(mocks.analyze).toHaveBeenCalledWith(expect.stringMatching(/^analysis-/), ['C:/input'], 'zip', 6)
    expect(wrapper.text()).toContain('2.86 MB')
    expect(wrapper.text()).toContain('建议 7Z · L7')
    expect(useCompressionStore().estimatedSize['job-1']).toBe(3_000_000)
    useCompressionStore().recordActualSize('job-1', 3_200_000)
    await wrapper.vm.$nextTick()
    expect(wrapper.text()).toContain('实际体积')
    expect(wrapper.text()).toContain('预测误差')

    await wrapper.get('.analysis-apply').trigger('click')
    expect(wrapper.emitted('update:modelValue')?.[0]?.[0]).toMatchObject({
      format: '7z', level: 7, createSolidArchive: true,
    })
  })

  it('marks results stale after settings change and blocks adopting them', async () => {
    const wrapper = mountCard()
    await wrapper.get('.analysis-button').trigger('click')
    await flushPromises()
    await wrapper.setProps({ modelValue: { ...options, level: 9 } })

    expect(wrapper.text()).toContain('请重新分析')
    expect(wrapper.get('.analysis-apply').attributes('disabled')).toBeDefined()
  })

  it('cancels an active analysis without converting it to a failure', async () => {
    let rejectAnalysis!: (error: Error) => void
    mocks.analyze.mockImplementation(() => new Promise((_, reject) => { rejectAnalysis = reject }))
    const wrapper = mountCard()
    await wrapper.get('.analysis-button').trigger('click')
    await wrapper.get('.analysis-button.is-cancel').trigger('click')
    rejectAnalysis(new Error('Compression analysis cancelled'))
    await flushPromises()

    expect(mocks.cancel).toHaveBeenCalledOnce()
    expect(useCompressionStore().compressionAnalysis['job-1'].status).toBe('cancelled')
    expect(wrapper.text()).toContain('分析已取消')
  })

  it('ignores a stale failure after a replacement analysis starts', async () => {
    let rejectFirst!: (error: Error) => void
    let resolveSecond!: (value: typeof result) => void
    mocks.analyze
      .mockImplementationOnce(() => new Promise((_, reject) => { rejectFirst = reject }))
      .mockImplementationOnce(() => new Promise(resolve => { resolveSecond = resolve }))

    const wrapper = mountCard()
    const store = useCompressionStore()
    await wrapper.get('.analysis-button').trigger('click')
    const firstId = store.compressionAnalysis['job-1'].analysisId
    await wrapper.get('.analysis-button.is-cancel').trigger('click')
    await wrapper.get('.analysis-button').trigger('click')
    const secondId = store.compressionAnalysis['job-1'].analysisId

    expect(secondId).not.toBe(firstId)
    rejectFirst(new Error('stale analysis failed'))
    await flushPromises()
    expect(store.compressionAnalysis['job-1']).toMatchObject({
      status: 'running',
      analysisId: secondId,
    })

    resolveSecond(result)
    await flushPromises()
    expect(store.compressionAnalysis['job-1']).toMatchObject({
      status: 'completed',
      analysisId: secondId,
    })
  })
})
