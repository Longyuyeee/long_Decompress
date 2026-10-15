import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import FailedExtractionRetry from '../FailedExtractionRetry.vue'
import { useTaskStore } from '@/stores/task'
import { createTaskHistoryRecord } from '@/types/taskHistory'
import { useAppStore } from '@/stores/app'

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), open: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/dialog', () => ({ open: mocks.open }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => vi.fn()) }))
const setup = (errors = ['[archive-inspection:damaged] 文件损坏']) => {
  const pinia = createPinia()
  setActivePinia(pinia)
  const store = useTaskStore()
  const ids = errors.map((error, index) => {
    const id = `original-${index}`
    store.addTask({ id, name: `${id}.zip`, type: 'decompression', sourceFiles: [`C:/${id}.zip`], outputPath: 'C:/output', recycleSourceAfterExtract: true })
    store.failTask(id, error)
    return id
  })
  const wrapper = mount(FailedExtractionRetry, { props: { taskIds: ids }, global: { plugins: [pinia],
    stubs: { ExtractionOutputPreview: true, Modal: { template: '<section><slot /><slot name="footer" /></section>' } } } })
  return { wrapper, store, ids, pinia }
}

beforeEach(() => {
  vi.clearAllMocks()
  mocks.invoke.mockImplementation(async (cmd: string, args: any) => {
    if (cmd === 'detect_split_archive') return { is_split: false }
    if (cmd === 'get_file_info') return { name: 'new.zip', is_dir: !args.path.endsWith('.zip'), size: 100 }
    if (cmd === 'diagnose_archive') return { status: 'healthy', integrityTested: true }
    return undefined
  })
})

describe('failed extraction corrections', () => {
  it('keeps a single correction inline and hands the chosen source and fresh password to rescue', async () => {
    const { wrapper, store, ids, pinia } = setup()
    wrapper.unmount()
    const inline = mount(FailedExtractionRetry, { props: { taskIds: ids, inline: true }, global: { plugins: [pinia], stubs: { ExtractionOutputPreview: true } } })
    expect(inline.find('[data-testid="inline-extraction-recovery"]').exists()).toBe(true)
    expect(inline.find('[role="dialog"]').exists()).toBe(false)
    await inline.get('[data-testid="retry-source"]').setValue('C:/complete/reselected.zip')
    await inline.get('[data-testid="retry-password"]').setValue('new rescue password')
    await inline.get('[data-testid="retry-rescue"]').trigger('click')
    expect(useAppStore().pendingArchiveDiagnosis).toMatchObject({ path: 'C:/complete/reselected.zip', taskId: ids[0], password: 'new rescue password', outputPath: 'C:/output' })
    expect(store.tasks[0].sourceFiles).toEqual(['C:/original-0.zip'])
    expect((inline.get('[data-testid="retry-password"]').element as HTMLInputElement).value).toBe('')
    inline.unmount()
  })

  it('shows detected missing volumes and queues a correction after they are supplied, preserving output choices', async () => {
    mocks.invoke.mockImplementation(async (cmd: string, args: any) => {
      if (cmd === 'detect_split_archive') return { is_split: true, first_part: args.path, is_complete: false, missing_parts: ['C:/part.zip.002'] }
      return undefined
    })
    const { wrapper, store, ids, pinia } = setup(['[archive-inspection:missing-volume] 缺失分卷'])
    await flushPromises()
    expect(wrapper.get('[data-testid="retry-missing-volumes"]').text()).toContain('part.zip.002')
    wrapper.unmount()
    store.tasks[0].extractToSubfolder = false
    mocks.invoke.mockImplementation(async (cmd: string, args: any) => {
      if (cmd === 'detect_split_archive') return { is_split: true, first_part: args.path, is_complete: true, missing_parts: [] }
      if (cmd === 'get_file_info') return { name: 'part.zip.001', is_dir: args.path === 'C:/output', size: 100 }
      return undefined
    })
    const inline = mount(FailedExtractionRetry, { props: { taskIds: ids, inline: true }, global: { plugins: [pinia], stubs: { ExtractionOutputPreview: true } } })
    await inline.get('[data-testid="create-retry-drafts"]').trigger('click'); await flushPromises()
    expect(store.tasks[1]).toMatchObject({ status: 'pending', outputPath: 'C:/output', extractToSubfolder: false, retryOfTaskId: ids[0] })
    expect(inline.emitted('created')?.[0]).toEqual([[store.tasks[1].id]])
    expect(mocks.invoke.mock.calls.some(([cmd]) => cmd === 'diagnose_archive' || cmd === 'decompress_file')).toBe(false)
    inline.unmount()
  })

  it('does not silently reuse a failed password when opened from the batch', async () => {
    const { wrapper, store, ids, pinia } = setup(['密码错误'])
    wrapper.unmount()
    store.tasks[0].password = 'previously-failed-secret'
    store.tasks[0].passwordRequired = true
    const reopened = mount(FailedExtractionRetry, { props: { taskIds: ids }, global: { plugins: [pinia],
      stubs: { ExtractionOutputPreview: true, Modal: { template: '<section><slot /><slot name="footer" /></section>' } } } })
    expect((reopened.get('[data-testid="retry-password"]').element as HTMLInputElement).value).toBe('')
    reopened.unmount()
  })

  it('remembers a created retry when the correction window is reopened', async () => {
    const { wrapper, store, ids, pinia } = setup(['unknown'])
    await wrapper.get('[data-testid="create-retry-drafts"]').trigger('click')
    await flushPromises()
    expect(store.tasks).toHaveLength(2)
    wrapper.unmount()
    const reopened = mount(FailedExtractionRetry, { props: { taskIds: ids }, global: { plugins: [pinia],
      stubs: { ExtractionOutputPreview: true, Modal: { template: '<section><slot /><slot name="footer" /></section>' } } } })
    expect(reopened.get('[data-testid="create-retry-drafts"]').attributes('disabled')).toBeDefined()
    expect(reopened.text()).toContain('已创建新任务')
    expect(store.tasks).toHaveLength(2)
    reopened.unmount()
  })

  it('creates new pending tasks for selected failures, preserves the original history and never auto-starts', async () => {
    const { wrapper, store, ids } = setup(['unknown cause', 'unknown cause', 'unknown cause'])
    await flushPromises()
    const old = store.tasks.map(createTaskHistoryRecord)
    await wrapper.findAll('article > label input')[2].setValue(false)
    await wrapper.findAll('[data-testid="retry-password"]')[0].setValue('new-secret')
    await wrapper.get('[data-testid="create-retry-drafts"]').trigger('click')
    await flushPromises()
    const drafts = store.tasks.filter(task => task.status === 'pending')
    expect(drafts).toHaveLength(2)
    expect(drafts.map(task => task.retryOfTaskId)).toEqual(ids.slice(0, 2))
    expect(drafts[0]).toMatchObject({ password: 'new-secret', configurationMode: 'individual', recycleSourceAfterExtract: false })
    expect(drafts.every(task => !ids.includes(task.id))).toBe(true)
    expect(store.tasks.slice(0, 3).map(createTaskHistoryRecord)).toEqual(old)
    expect(mocks.invoke.mock.calls.some(([cmd]) => cmd === 'decompress_file')).toBe(false)
    await wrapper.get('[data-testid="create-retry-drafts"]').trigger('click')
    expect(store.tasks).toHaveLength(5)
    expect(createTaskHistoryRecord(drafts[0]).retryOfTaskId).toBe(ids[0])
    wrapper.unmount()
  })

  it('blocks incomplete volumes and still creates another selected valid item, with per-item feedback', async () => {
    const { wrapper, store } = setup(['missing volume', 'unknown cause'])
    await flushPromises()
    mocks.invoke.mockImplementation(async (cmd: string, args: any) => {
      if (cmd === 'detect_split_archive') return args.path.includes('original-0')
        ? { is_split: true, first_part: args.path, is_complete: false, missing_parts: ['part2.rar'] }
        : { is_split: false }
      if (cmd === 'get_file_info') return { name: 'new.zip', is_dir: args.path === 'C:/output', size: 100 }
    })
    await wrapper.get('[data-testid="create-retry-drafts"]').trigger('click')
    await flushPromises()
    expect(store.tasks.filter(task => task.status === 'pending')).toHaveLength(1)
    expect(wrapper.text()).toContain('part2.rar')
    expect(wrapper.text()).toContain('本次已创建 1 项')
    wrapper.unmount()
  })

  it('requires a healthy integrity result for a damaged source and excludes incomplete rollback', async () => {
    const { wrapper, store } = setup(['[archive-inspection:damaged] 损坏', '[archive-output:Publishing:rollback-incomplete] 恢复目录需核对'])
    mocks.invoke.mockImplementation(async (cmd: string, args: any) => {
      if (cmd === 'detect_split_archive') return null
      if (cmd === 'get_file_info') return { name: 'new.zip', is_dir: args.path === 'C:/output', size: 100 }
      if (cmd === 'diagnose_archive') return { status: 'truncated', integrityTested: true }
    })
    await wrapper.get('[data-testid="create-retry-drafts"]').trigger('click')
    await flushPromises()
    expect(store.tasks).toHaveLength(2)
    expect(wrapper.text()).toContain('尚未通过完整性检查')
    expect(wrapper.text()).toContain('不能直接重试回滚不完整')
    expect(mocks.invoke.mock.calls.filter(([cmd]) => cmd === 'detect_split_archive')).toHaveLength(1)
    wrapper.unmount()
  })

  it('does not enqueue after closing while validation is pending', async () => {
    const { wrapper, store } = setup(['unknown'])
    let finish!: (value: unknown) => void
    await flushPromises()
    mocks.invoke.mockImplementation((cmd: string) => cmd === 'detect_split_archive' ? new Promise(resolve => { finish = resolve }) : Promise.resolve(undefined))
    await wrapper.get('[data-testid="create-retry-drafts"]').trigger('click')
    await flushPromises()
    wrapper.unmount()
    finish(null)
    await flushPromises()
    expect(store.tasks).toHaveLength(1)
  })

  it('requires successful original-history preservation and does not silently lose the failed attempt', async () => {
    const { wrapper, store } = setup(['unknown'])
    await flushPromises()
    store.tasks[0].historySaveError = 'database unavailable'
    mocks.invoke.mockRejectedValue(new Error('database unavailable'))
    await wrapper.get('[data-testid="create-retry-drafts"]').trigger('click')
    await flushPromises()
    expect(store.tasks).toHaveLength(1)
    expect(wrapper.text()).toContain('原失败记录尚未保存成功')
    wrapper.unmount()
  })

  it('stops an expensive integrity check without creating a task after the result arrives', async () => {
    const { wrapper, store } = setup()
    await flushPromises()
    let finish!: (value: unknown) => void
    mocks.invoke.mockImplementation(async (cmd: string, args: any) => {
      if (cmd === 'detect_split_archive') return null
      if (cmd === 'get_file_info') return { name: 'new.zip', is_dir: args.path === 'C:/output', size: 100 }
      if (cmd === 'diagnose_archive') return new Promise(resolve => { finish = resolve })
    })
    await wrapper.get('[data-testid="create-retry-drafts"]').trigger('click')
    await flushPromises()
    await wrapper.findAll('button').find(button => button.text() === '停止检查')!.trigger('click')
    expect(mocks.invoke.mock.calls.some(([cmd]) => cmd === 'cancel_archive_diagnosis')).toBe(true)
    finish({ status: 'healthy', integrityTested: true })
    await flushPromises()
    expect(store.tasks).toHaveLength(1)
    expect(wrapper.text()).toContain('已停止检查')
    wrapper.unmount()
  })

  it('applies a shared output only to selected eligible rows without changing the failed originals', async () => {
    const { wrapper, store } = setup(['unknown', 'unknown', '[archive-output:Publishing:rollback-incomplete] retained backup'])
    await wrapper.findAll('article > label input')[1].setValue(false)
    mocks.open.mockResolvedValue('D:/chosen')
    await wrapper.findAll('button').find(button => button.text() === '选择共用输出目录')!.trigger('click')
    await flushPromises()
    await wrapper.findAll('button').find(button => button.text() === '应用到勾选项')!.trigger('click')
    const outputs = wrapper.findAll('[data-testid="retry-output"]')
    expect((outputs[0].element as HTMLInputElement).value).toBe('D:/chosen')
    expect((outputs[1].element as HTMLInputElement).value).toBe('C:/output')
    expect(store.tasks.every(task => task.outputPath === 'C:/output')).toBe(true)
    wrapper.unmount()
  })
})
