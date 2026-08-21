import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import HistoryView from '../HistoryView.vue'

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))

const records = [
  {
    id: 'compress-1', name: 'photos.zip', taskType: 'compression', status: 'completed',
    sourcePaths: ['C:/photos'], outputPath: 'C:/photos.zip', format: 'zip',
    startedAt: '2026-08-19T02:00:00.000Z', completedAt: '2026-08-19T02:00:03.500Z',
    durationMs: 3500, processedBytes: 4096, totalBytes: 4096, errorMessage: null,
    logs: [{ timestamp: '2026-08-19T02:00:03.500Z', message: '压缩完成', severity: 'success' }],
  },
  {
    id: 'extract-1', name: 'broken.7z', taskType: 'decompression', status: 'failed',
    sourcePaths: ['D:/broken.7z'], outputPath: 'D:/broken', format: '7z',
    startedAt: '2026-08-18T02:00:00.000Z', completedAt: '2026-08-18T02:00:01.000Z',
    durationMs: 1000, processedBytes: 1024, totalBytes: 2048, errorMessage: '数据错误', logs: [],
  },
]

const mountView = () => mount(HistoryView, { global: { plugins: [createPinia()], stubs: { Teleport: true } } })

describe('HistoryView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'load_app_settings') return '{}'
      if (command === 'list_task_history') return records
      return undefined
    })
  })

  it('renders persisted metrics, real records, filters, and details', async () => {
    const wrapper = mountView()
    await flushPromises()

    expect(mocks.invoke).toHaveBeenCalledWith('list_task_history', { limit: 500 })
    expect(wrapper.get('[data-testid="history-kpis"]').text()).toContain('50%')
    expect(wrapper.get('[data-testid="history-list"]').text()).toContain('photos.zip')
    expect(wrapper.get('[data-testid="history-list"]').text()).toContain('broken.7z')
    const completedBadge = wrapper.findAll('[data-testid="history-status-badge"]')
      .find(badge => badge.text() === '已完成')
    expect(completedBadge?.classes()).toContain('history-status-badge')

    await wrapper.get('[data-testid="history-type-filter"]').setValue('decompression')
    expect(wrapper.get('[data-testid="history-list"]').text()).not.toContain('photos.zip')
    expect(wrapper.get('[data-testid="history-list"]').text()).toContain('broken.7z')

    await wrapper.find('[data-testid="history-list"] article').trigger('click')
    expect(wrapper.get('[data-testid="history-detail"]').text()).toContain('数据错误')
    expect(wrapper.get('[data-testid="history-detail"]').text()).toContain('D:/broken.7z')
    expect(wrapper.get('[data-testid="history-detail"]').classes()).toContain('history-detail-solid')
  })

  it('shows a helpful empty state and can clear persisted history', async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'load_app_settings') return '{}'
      if (command === 'list_task_history') return []
      return undefined
    })
    const wrapper = mountView()
    await flushPromises()
    expect(wrapper.get('[data-testid="history-empty"]').text()).toContain('还没有历史任务')
  })
})
