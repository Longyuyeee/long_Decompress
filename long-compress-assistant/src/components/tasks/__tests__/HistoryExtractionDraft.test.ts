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
  const wrapper = mount(HistoryExtractionDraft, { props: { record }, global: { plugins: [pinia], stubs: { ExtractionOutputPreview: true } } })
  await wrapper.get('[data-testid="history-draft-select-output"]').trigger('click')
  await flushPromises()
  return { wrapper, store: useTaskStore() }
}
describe('history extraction draft', () => {
  it('comparison1554 history recreates an exact folder selection without trying whole archive detection',async()=>{
    setActivePinia(createPinia());const comparisonSelection={source:{path:'C:/left',kind:'directory' as const,revision:'a'.repeat(64)},paths:['docs/合同.txt'],keepStructure:false,conflictPolicy:'skip' as const};const saved:TaskHistoryRecord={...record,sourcePaths:['C:/left'],executionConfig:{schemaVersion:1,kind:'archive-extraction',outputDirectory:'D:/out',extractToSubfolder:false,comparisonSelection}};mocks.invoke.mockResolvedValue(undefined);const view=mount(HistoryExtractionDraft,{props:{record:saved},global:{stubs:{ExtractionOutputPreview:true}}});expect(view.get('[data-testid="history-comparison-selection"]').text()).toContain('1 项');expect(view.find('[data-testid="history-draft-password"]').exists()).toBe(false);await view.get('[data-testid="history-draft-create"]').trigger('click');await flushPromises();expect(useTaskStore().tasks[0]).toMatchObject({comparisonSelection,selectedEntries:['docs/合同.txt'],status:'pending'});expect(mocks.invoke.mock.calls.some(call=>call[0]==='detect_split_archive')).toBe(false);view.unmount()
  })
  it('library1549 recreates only the saved selected range after confirming the current source',async()=>{
    const pinia=createPinia();setActivePinia(pinia)
    const saved:TaskHistoryRecord={...record,sourcePaths:['C:/source.zip'],executionConfig:{schemaVersion:1,kind:'archive-extraction',outputDirectory:'D:/out',extractToSubfolder:false,librarySourceId:'source',libraryRevision:'revision',libraryKeepStructure:false,libraryConflictPolicy:'skip',selectedEntries:['docs/合同.txt']}}
    mocks.invoke.mockImplementation(async(command,args)=>command==='library_selection'?{path:'C:/source.zip'}:command==='detect_split_archive'?{is_split:false,archive_format:'ZIP'}:{name:'source.zip',size:100,is_dir:args.path==='D:/out'})
    const view=mount(HistoryExtractionDraft,{props:{record:saved},global:{plugins:[pinia],stubs:{ExtractionOutputPreview:true}}});expect(view.get('[data-testid="history-library-selection"]').text()).toContain('1 个文件');await view.get('[data-testid="history-draft-create"]').trigger('click');await flushPromises();expect(useTaskStore().tasks[0]).toMatchObject({status:'pending',librarySourceId:'source',libraryRevision:'revision',libraryKeepStructure:false,libraryConflictPolicy:'skip',selectedEntries:['docs/合同.txt']});view.unmount()
  })

  it('restores EGG key encoding without retaining the password', () => {
    const pinia=createPinia();setActivePinia(pinia)
    const saved:TaskHistoryRecord={...record,sourcePaths:['C:/原卷.vol3.egg'],format:'EGG',executionConfig:{schemaVersion:1,kind:'archive-extraction',outputDirectory:'D:/out',extractToSubfolder:false,zipPasswordEncoding:'cp936'}}
    const wrapper=mount(HistoryExtractionDraft,{props:{record:saved},global:{plugins:[pinia],stubs:{ExtractionOutputPreview:true}}})
    expect(wrapper.text()).toContain('EGG 密码编码')
    expect(wrapper.get('[data-testid="history-draft-zip-password-encoding"]').text()).toContain('CP936')
    expect(wrapper.get('[data-testid="history-draft-password"]').element.value).toBe('')
    wrapper.unmount()
  })
  it('describes ALZ filename and password codepage in a restored native-member task', () => {
    const pinia=createPinia();setActivePinia(pinia)
    const saved: TaskHistoryRecord={...record,sourcePaths:['C:/原卷.a01'],format:'ALZ',executionConfig:{schemaVersion:1,kind:'archive-extraction',outputDirectory:'D:/out',extractToSubfolder:false,zipPasswordEncoding:'cp936'}}
    const wrapper=mount(HistoryExtractionDraft,{props:{record:saved},global:{plugins:[pinia],stubs:{ExtractionOutputPreview:true}}})
    expect(wrapper.text()).toContain('ALZ 文件名与密码编码')
    expect(wrapper.get('[data-testid="history-draft-zip-password-encoding"]').text()).toContain('简体中文（CP936）')
    expect(wrapper.get('[data-testid="history-draft-password"]').element.value).toBe('')
    wrapper.unmount()
  })
  it('restores the ZIP encoding into a pending draft, then clears it when opening an old history entry', async () => {
    const pinia = createPinia(); setActivePinia(pinia)
    const saved: TaskHistoryRecord = { ...record, sourcePaths: ['C:/old.zip'], format: 'ZIP', executionConfig: { schemaVersion: 1, kind: 'archive-extraction', outputDirectory: 'D:/output', extractToSubfolder: false, zipPasswordEncoding: 'cp950' } }
    mocks.invoke.mockImplementation(async (cmd, args) => cmd === 'detect_split_archive' ? { is_split: false, archive_format: 'ZIP' } : { name: 'old.zip', size: 100, is_dir: args.path === 'D:/output' })
    const wrapper = mount(HistoryExtractionDraft, { props: { record: saved }, global: { plugins: [pinia], stubs: { ExtractionOutputPreview: true } } })
    expect(wrapper.get('[data-testid="history-draft-zip-password-encoding"]').text()).toContain('繁体中文')
    await wrapper.get('[data-testid="history-draft-create"]').trigger('click'); await flushPromises()
    expect(useTaskStore().tasks[0]).toMatchObject({ zipPasswordEncoding: 'cp950', status: 'pending' })
    expect(useTaskStore().tasks[0].password).toBeUndefined()
    await wrapper.setProps({ record: { ...saved, id: 'legacy', executionConfig: undefined } })
    expect(wrapper.get('[data-testid="history-draft-zip-password-encoding"]').text()).toContain('自动兼容')
    expect(saved.executionConfig?.zipPasswordEncoding).toBe('cp950')
    wrapper.unmount()
  })
  it('rejects an unconfirmed forum member instead of creating a standalone ZIP draft', async () => {
    const {wrapper,store} = await setup()
    await wrapper.get('[data-testid="history-draft-source"]').setValue('C:/资料.haozip02.zip')
    mocks.invoke.mockResolvedValue({is_split:false})
    await wrapper.get('[data-testid="history-draft-create"]').trigger('click')
    await flushPromises()
    expect(store.tasks).toHaveLength(0)
    expect(wrapper.text()).toContain('无法确认分卷组')
    wrapper.unmount()
  })
  it('带入已保存的目录和子目录选择，允许修改后创建，再切到旧记录时清空设置', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const saved: TaskHistoryRecord = { ...record, executionConfig: {
      schemaVersion: 1, kind: 'archive-extraction', outputDirectory: 'D:/上次目录', extractToSubfolder: false,
    } }
    const wrapper = mount(HistoryExtractionDraft, { props: { record: saved }, global: { plugins: [pinia], stubs: { ExtractionOutputPreview: true } } })
    expect(wrapper.get('[data-testid="history-config-restored"]').text()).toContain('已带入')
    expect((wrapper.get('[data-testid="history-draft-output"]').element as HTMLInputElement).value).toBe('D:/上次目录')
    expect((wrapper.get('[data-testid="history-draft-subfolder"]').element as HTMLInputElement).checked).toBe(false)
    await wrapper.get('[data-testid="history-draft-output"]').setValue('D:/这次目录')
    await wrapper.get('[data-testid="history-draft-password"]').setValue('this-run-only')
    await wrapper.get('[data-testid="history-draft-create"]').trigger('click')
    await flushPromises()
    expect(useTaskStore().tasks[0]).toMatchObject({ outputPath: 'D:/这次目录', extractToSubfolder: false, password: 'this-run-only', status: 'pending' })
    expect(saved.executionConfig?.outputDirectory).toBe('D:/上次目录')
    await wrapper.setProps({ record: { ...record, id: 'legacy' } })
    expect(wrapper.find('[data-testid="history-config-restored"]').exists()).toBe(false)
    expect((wrapper.get('[data-testid="history-draft-output"]').element as HTMLInputElement).value).toBe('')
    expect((wrapper.get('[data-testid="history-draft-password"]').element as HTMLInputElement).value).toBe('')
    wrapper.unmount()
  })
  beforeEach(() => {
    vi.resetAllMocks()
    mocks.open.mockResolvedValue('D:/output')
    mocks.invoke.mockImplementation(async (command, args) => command === 'detect_split_archive'
      ? { is_split: false } : { name: 'old.rar', size: 100, is_dir: !args.path.endsWith('.rar') })
  })
  it('creates a new pending individual task without executing or changing history', async () => {
    const { wrapper, store } = await setup()
    await wrapper.get('[data-testid="history-draft-create"]').trigger('click')
    await flushPromises()
    expect(store.tasks).toHaveLength(1)
    expect(store.tasks[0]).toMatchObject({ status: 'pending', configurationMode: 'individual', outputPath: 'D:/output', recycleSourceAfterExtract: false, extractToSubfolder: true })
    expect(store.tasks[0].id).not.toBe(record.id)
    expect(store.tasks[0].retryOfTaskId).toBe(record.id)
    expect(store.tasks[0].password).toBeUndefined()
    expect(store.tasks[0].logs[0].message).toContain(record.id)
    expect(record.status).toBe('failed')
    expect(mocks.invoke.mock.calls.map(call => call[0])).toEqual(['detect_split_archive', 'get_file_info', 'get_file_info'])
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
