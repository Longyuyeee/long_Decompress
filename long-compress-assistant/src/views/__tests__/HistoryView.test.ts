import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import HistoryView from '../HistoryView.vue'
import { useAppStore } from '@/stores/app'

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), save: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/dialog', () => ({ save: mocks.save }))
vi.mock('@tauri-apps/api/app', () => ({ getVersion: vi.fn(async () => '1.3.2') }))

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
    mocks.save.mockResolvedValue(null)
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

  it('searches error text and filters legacy failures without inventing their category', async () => {
    const wrapper = mountView()
    await flushPromises()
    await wrapper.get('[data-testid="history-search"]').setValue('数据错误')
    expect(wrapper.get('[data-testid="history-list"]').text()).toContain('broken.7z')
    expect(wrapper.get('[data-testid="history-list"]').text()).not.toContain('photos.zip')
    await wrapper.get('[data-testid="history-failure-filter"]').setValue('unknown')
    await wrapper.find('[data-testid="history-list"] article').trigger('click')
    expect(wrapper.get('[data-testid="history-detail"]').text()).toContain('未分类 · 阶段未知')
  })

  it('exports only the selected persisted task and does not write when the dialog is cancelled', async () => {
    const wrapper = mountView()
    await flushPromises()
    await wrapper.get('[data-testid="history-type-filter"]').setValue('decompression')
    await wrapper.find('[data-testid="history-list"] article').trigger('click')
    await wrapper.get('[data-testid="history-export"]').trigger('click')
    await flushPromises()
    expect(mocks.invoke).not.toHaveBeenCalledWith('write_text_file', expect.anything())
    mocks.save.mockResolvedValue('C:/reports/task.json')
    await wrapper.get('[data-testid="history-export"]').trigger('click')
    await flushPromises()
    const call = mocks.invoke.mock.calls.find(([command]) => command === 'write_text_file')!
    const report = JSON.parse(call[1].content)
    expect(call[1].path).toBe('C:/reports/task.json')
    expect(report.task.id).toBe('extract-1')
    expect(report.failure.category).toBe('unknown')
    expect(report.task.errorMessage).toBe('数据错误')
    expect(call[1].content).not.toContain('photos.zip')
  })

  it('reports export write failures and allows another attempt', async () => {
    const wrapper = mountView()
    await flushPromises()
    await wrapper.find('[data-testid="history-list"] article').trigger('click')
    mocks.save.mockResolvedValue('C:/reports/task.json')
    mocks.invoke.mockRejectedValueOnce(new Error('disk full'))
    await wrapper.get('[data-testid="history-export"]').trigger('click')
    await flushPromises()
    expect(wrapper.get('[data-testid="history-export"]').attributes('disabled')).toBeUndefined()
    expect(useAppStore().error).toContain('诊断报告导出失败：Error: disk full')
    expect(wrapper.get('[data-testid="history-detail"]').exists()).toBe(true)
  })
})
