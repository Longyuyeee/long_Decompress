import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
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

const page = (records: any[], query: any = {}) => ({ records: records.filter(record =>
  (!query.text || [record.name, record.outputPath, record.errorMessage, ...record.sourcePaths].some(value => value?.toLowerCase().includes(query.text.toLowerCase())))
  && (!query.taskType || record.taskType === query.taskType)
  && (!query.status || record.status === query.status)
  && (!query.since || Date.parse(record.completedAt) >= Date.parse(query.since))
  && (!query.failureCategory || (record.status === 'failed' && (record.failure?.category || 'unknown') === query.failureCategory))), nextCursor: null })
const settleSearch = async () => { await new Promise(resolve => setTimeout(resolve, 275)); await flushPromises() }
const mountView = () => mount(HistoryView, { global: { plugins: [createPinia()], stubs: { Teleport: true } } })

describe('HistoryView', () => {
  it('搜索直接返回更早的匹配记录，刷新失败仍保留当前结果', async () => {
    mocks.invoke.mockImplementation(async (command, args) => command === 'list_task_history_page'
      ? args.query.text ? page([records[1]], args.query) : { records: [records[0]], nextCursor: { completedAt: records[0].completedAt, id: records[0].id } } : undefined)
    const wrapper = mountView()
    await flushPromises()
    await wrapper.get('[data-testid="history-search"]').setValue('broken')
    await settleSearch()
    expect(wrapper.get('[data-testid="history-retention-policy"]').text()).toContain('覆盖全部')
    expect(wrapper.get('[data-testid="history-list"]').text()).toContain('broken.7z')
    expect(wrapper.find('[data-testid="history-load-more"]').exists()).toBe(false)
    mocks.invoke.mockRejectedValueOnce(new Error('temporary database error'))
    await wrapper.findAll('button').find(button => button.text().includes('刷新'))!.trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('temporary database error')
    expect(wrapper.get('[data-testid="history-list"]').text()).toContain('broken.7z')
    wrapper.unmount()
  })
  it.each([
    ['damaged', '重新获取完整文件'],
    ['missing-volume', '不猜测缺失分卷名称'],
    ['permission', '来源读取'],
    ['output-conflict', '不要覆盖'],
  ])('我看到 %s 后，可以按指引确认来源和输出，而不是自动重跑', async (category, expected) => {
    const failed = { ...records[1], errorMessage: '已记录的原始详情', failure: {
      schemaVersion: 1, category, stage: category === 'output-conflict' ? 'Publishing' : 'inspection', evidence: 'recorded-marker',
    } }
    mocks.invoke.mockImplementation(async command => command === 'list_task_history_page' ? page([failed]) : undefined)
    const wrapper = mountView()
    await flushPromises()
    const callsBeforeOpening = [...mocks.invoke.mock.calls]
    await wrapper.get('[data-testid="history-record-row"]').trigger('click')
    const guidance = wrapper.get('[data-testid="history-recovery-guidance"]')
    expect(guidance.text()).toContain(expected)
    expect(guidance.attributes('aria-label')).toBe(guidance.get('h3').text())
    const draft = wrapper.get('[data-testid="history-extraction-draft"]')
    expect(draft.text()).toContain('重新选择文件')
    expect(draft.text()).toContain('选择输出目录')
    expect(draft.text()).toContain('不会自动开始')
    expect(guidance.element.compareDocumentPosition(draft.element) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(mocks.invoke.mock.calls).toEqual(callsBeforeOpening)
    mocks.save.mockResolvedValue('C:/reports/next-step.json')
    await wrapper.get('[data-testid="history-export"]').trigger('click')
    await flushPromises()
    const report = JSON.parse(mocks.invoke.mock.calls.find(([command]) => command === 'write_text_file')![1].content)
    expect(report.recoveryGuidance.steps.join(' ')).toContain(expected)
    expect(report.task.errorMessage).toBe(failed.errorMessage)
    wrapper.unmount()
  })
  beforeEach(() => {
    vi.spyOn(Date, 'now').mockReturnValue(Date.parse('2026-09-12T00:00:00Z'))
    vi.clearAllMocks()
    mocks.save.mockResolvedValue(null)
    localStorage.clear()
    mocks.invoke.mockImplementation(async (command: string, args: any) => {
      if (command === 'load_app_settings') return '{}'
      if (command === 'list_task_history_page') return page(records, args.query)
      return undefined
    })
  })

  afterEach(() => { vi.restoreAllMocks() })

  it('默认展示全部时间，清除条件后重新查找全部历史', async () => {
    vi.mocked(Date.now).mockReturnValue(Date.parse('2026-09-22T00:00:00Z'))
    const wrapper = mountView()
    await flushPromises()
    expect(wrapper.get('[data-testid="history-list"]').text()).toContain('photos.zip')
    await wrapper.get('[data-testid="history-search"]').setValue('不存在的名字')
    await settleSearch()
    expect(wrapper.get('[data-testid="history-empty"]').text()).toContain('没有符合条件')
    await wrapper.get('[data-testid="history-reset-filters"]').trigger('click')
    await settleSearch()
    expect(wrapper.get('[data-testid="history-list"]').text()).toContain('photos.zip')
    expect(wrapper.get('[data-testid="history-list"]').text()).toContain('broken.7z')
    wrapper.unmount()
  })

  it('选择最近30天时把时间边界提交到数据库查询', async () => {
    const wrapper = mountView()
    await flushPromises()
    await wrapper.get('[data-testid="history-range-filter"]').setValue('30d')
    await settleSearch()
    expect(mocks.invoke).toHaveBeenLastCalledWith('list_task_history_page', expect.objectContaining({
      cursor: null, query: expect.objectContaining({ since: new Date(Date.now() - 30 * 86_400_000).toISOString() }),
    }))
    wrapper.unmount()
  })

  it('renders persisted metrics, real records, filters, and details', async () => {
    const wrapper = mountView()
    await flushPromises()

    expect(mocks.invoke).toHaveBeenCalledWith('list_task_history_page', { limit: 100, cursor: null, query: { text: '' } })
    expect(wrapper.get('[data-testid="history-kpis"]').text()).toContain('50%')
    expect(wrapper.get('[data-testid="history-list"]').text()).toContain('photos.zip')
    expect(wrapper.get('[data-testid="history-list"]').text()).toContain('broken.7z')
    const completedBadge = wrapper.findAll('[data-testid="history-status-badge"]')
      .find(badge => badge.text() === '已完成')
    expect(completedBadge?.classes()).toContain('history-status-badge')

    await wrapper.get('[data-testid="history-type-filter"]').setValue('decompression')
    await settleSearch()
    expect(wrapper.get('[data-testid="history-list"]').text()).not.toContain('photos.zip')
    expect(wrapper.get('[data-testid="history-list"]').text()).toContain('broken.7z')

    await wrapper.find('[data-testid="history-list"] article').trigger('click')
    expect(wrapper.get('[data-testid="history-detail"]').text()).toContain('数据错误')
    expect(wrapper.get('[data-testid="history-recovery-guidance"]').text()).toContain('原因尚未确定')
    expect(wrapper.get('[data-testid="history-detail"]').text()).toContain('D:/broken.7z')
    expect(wrapper.get('[data-testid="history-detail"]').classes()).toContain('history-detail-solid')
  })

  it('shows a helpful empty state and can clear persisted history', async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'load_app_settings') return '{}'
      if (command === 'list_task_history_page') return page([])
      return undefined
    })
    const wrapper = mountView()
    await flushPromises()
    expect(wrapper.get('[data-testid="history-empty"]').text()).toContain('还没有历史任务')
    expect(wrapper.find('[data-testid="history-reset-filters"]').exists()).toBe(false)
  })

  it('我点开失败记录，先看最终原因，再决定是否重新解压', async () => {
    const wrapper = mountView()
    await flushPromises()
    await wrapper.get('[data-testid="history-type-filter"]').setValue('decompression')
    await settleSearch()
    await wrapper.find('[data-testid="history-list"] article').trigger('click')
    const detail = wrapper.get('[data-testid="history-detail"]')
    const reason = detail.findAll('section').find(section => section.text().includes('未分类 · 阶段未知'))!
    const draft = detail.get('[data-testid="history-extraction-draft"]')
    expect(reason.element.compareDocumentPosition(draft.element) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(reason.text()).toContain('最终失败原因')
    expect(reason.text()).toContain('数据错误')
    expect(reason.element.parentElement?.children[1]).toBe(reason.element)
    const report = detail.get('[data-testid="history-export"]')
    expect(report.element.compareDocumentPosition(draft.element) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(mocks.invoke.mock.calls.every(([command]) => !['decompress_archive', 'save_task_history'].includes(command))).toBe(true)
  })

  it.each(['completed', 'cancelled'])('任务是 %s 时，不把旧错误当作本次最终失败', async status => {
    mocks.invoke.mockImplementation(async command => command === 'list_task_history_page'
      ? page([{ ...records[1], status }]) : undefined)
    const wrapper = mountView()
    await flushPromises()
    await wrapper.find('[data-testid="history-list"] article').trigger('click')
    expect(wrapper.find('[data-testid="history-final-failure"]').exists()).toBe(false)
  })

  it('searches error text and filters legacy failures without inventing their category', async () => {
    const wrapper = mountView()
    await flushPromises()
    await wrapper.get('[data-testid="history-search"]').setValue('数据错误')
    await settleSearch()
    expect(wrapper.get('[data-testid="history-list"]').text()).toContain('broken.7z')
    expect(wrapper.get('[data-testid="history-list"]').text()).not.toContain('photos.zip')
    await wrapper.get('[data-testid="history-failure-filter"]').setValue('unknown')
    await settleSearch()
    await wrapper.find('[data-testid="history-list"] article').trigger('click')
    expect(wrapper.get('[data-testid="history-detail"]').text()).toContain('未分类 · 阶段未知')
  })

  it('exports only the selected persisted task and does not write when the dialog is cancelled', async () => {
    const wrapper = mountView()
    await flushPromises()
    await wrapper.get('[data-testid="history-type-filter"]').setValue('decompression')
    await settleSearch()
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

  it('shows and exports read-only rollback guidance without changing history', async () => {
    const recoveryRecord = { ...records[1], errorMessage: '[archive-output:Publishing:rollback-incomplete] 恢复目录：D:/recovery',
      failure: { schemaVersion: 1, category: 'rollback-incomplete', stage: 'Publishing', evidence: 'recorded-marker' } }
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'load_app_settings') return '{}'
      if (command === 'list_task_history_page') return page([recoveryRecord])
      return undefined
    })
    const wrapper = mountView()
    await flushPromises()
    await wrapper.find('[data-testid="history-list"] article').trigger('click')
    expect(wrapper.get('[data-testid="history-recovery-guidance"]').text()).toContain('请选择新的空目录')
    const reason = wrapper.get('[data-testid="history-final-failure"]')
    const guidance = wrapper.get('[data-testid="history-recovery-guidance"]')
    expect(reason.element.compareDocumentPosition(guidance.element) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(wrapper.get('[data-testid="history-detail"]').text()).toContain('D:/recovery')
    expect(mocks.invoke).not.toHaveBeenCalledWith('save_task_history', expect.anything())
    expect(mocks.invoke).not.toHaveBeenCalledWith('open_in_explorer', expect.anything())
    mocks.save.mockResolvedValue('C:/reports/recovery.json')
    await wrapper.get('[data-testid="history-export"]').trigger('click')
    await flushPromises()
    const call = mocks.invoke.mock.calls.find(([command]) => command === 'write_text_file')!
    const report = JSON.parse(call[1].content)
    expect(report.recoveryGuidance.steps).toHaveLength(4)
    expect(report.task.errorMessage).toContain('D:/recovery')
  })
})
