import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, nextTick } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import DecompressView from '../DecompressView.vue'
import { useAppStore } from '@/stores/app'
import { useTaskStore } from '@/stores/task'
import { useDecompressionBatchStore } from '@/stores/decompressionBatch'

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  decompressFile: vi.fn(),
  resolveExtractionConflict: vi.fn(),
  preflightOperationResources: vi.fn(),
  listArchiveContents: vi.fn(),
  listen: vi.fn(async () => vi.fn()),
}))

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }))
vi.mock('@tauri-apps/api/dialog', () => ({
  open: vi.fn(),
  confirm: vi.fn(async () => false),
}))
vi.mock('@/composables/useTauriCommands', () => ({
  useTauriCommands: () => ({
    invoke: mocks.invoke,
    decompressFile: mocks.decompressFile,
    resolveExtractionConflict: mocks.resolveExtractionConflict,
    preflightOperationResources: mocks.preflightOperationResources,
    listArchiveContents: mocks.listArchiveContents,
    testArchiveIntegrity: vi.fn(),
  }),
}))

const DropzoneStub = defineComponent({
  name: 'EnhancedFileDropzone',
  emits: ['files-selected'],
  template: '<button type="button">add archive</button>',
})

const mountView = (pinia = createPinia()) => mount(DecompressView, {
  global: {
    plugins: [pinia],
    stubs: { Teleport: true,
      EnhancedFileDropzone: DropzoneStub,
      AeroTable: defineComponent({ name: 'AeroTable', props: ['selectedTaskIds', 'statusFilter', 'taskType', 'taskIds'], template: '<div><slot name="before-tasks" /></div>' }),
      ConflictResolutionModal: true,
      RouterLink: true,
      Transition: false,
    },
  },
})

describe('DecompressView', () => {
  it('executes restored ZIP input encoding without changing ordinary automatic requests', async () => {
    const wrapper = mountView()
    const store = useTaskStore()
    store.addTask({ id: 'encoded', name: 'old.zip', type: 'decompression', sourceFiles: ['C:/old.zip'], outputPath: 'D:/encoded', zipPasswordEncoding: 'cp932', configurationMode: 'individual' })
    store.addTask({ id: 'auto', name: 'ordinary.zip', type: 'decompression', sourceFiles: ['C:/ordinary.zip'], outputPath: 'D:/auto' })
    await nextTick()
    await wrapper.get('[data-testid="start-decompression-queue"]').trigger('click'); await flushPromises()
    expect(mocks.decompressFile).toHaveBeenCalledWith('C:/old.zip', expect.objectContaining({ zipPasswordEncoding: 'cp932' }), 'encoded')
    expect(mocks.decompressFile).toHaveBeenCalledWith('C:/ordinary.zip', expect.objectContaining({ zipPasswordEncoding: undefined }), 'auto')
    wrapper.unmount()
  })
  it('re-detects reopened renamed ZIPs before execution instead of discarding the saved input choice', async () => {
    const wrapper = mountView()
    useTaskStore().addTask({ id: 'renamed-draft', name: 'old.7z', type: 'decompression', sourceFiles: ['C:/old.7z'], outputPath: 'D:/out', zipPasswordEncoding: 'cp950', configurationMode: 'individual' })
    mocks.invoke.mockImplementation(async (cmd) => cmd === 'detect_split_archive' ? { is_split: false, archive_format: 'ZIP' } : undefined)
    await nextTick()
    await wrapper.get('[data-testid="start-decompression-queue"]').trigger('click'); await flushPromises()
    expect(mocks.decompressFile).toHaveBeenCalledWith('C:/old.7z', expect.objectContaining({ zipPasswordEncoding: 'cp950' }), 'renamed-draft')
    wrapper.unmount()
  })
  it('does not start a task cancelled while detecting its restored ZIP encoding', async () => {
    const wrapper = mountView()
    const store = useTaskStore()
    store.addTask({ id: 'cancel-probe', name: 'a.zip', type: 'decompression', sourceFiles: ['C:/a.zip'], outputPath: 'D:/out', zipPasswordEncoding: 'cp932' })
    let finish!: (value: unknown) => void
    mocks.invoke.mockImplementation((command: string) => command === 'detect_split_archive' ? new Promise(resolve=>{finish=resolve}) : Promise.resolve([]))
    await nextTick()
    await wrapper.get('[data-testid="start-decompression-queue"]').trigger('click'); await flushPromises()
    expect(finish).toBeTypeOf('function')
    expect(mocks.decompressFile).not.toHaveBeenCalled()
    store.tasks[0].status = 'cancelled'
    finish({ is_split: false, archive_format: 'ZIP' }); await flushPromises()
    expect(mocks.decompressFile).not.toHaveBeenCalled()
    expect(mocks.preflightOperationResources).not.toHaveBeenCalled()
    expect(store.tasks[0].status).toBe('cancelled')
    wrapper.unmount()
  })
  it.each(['EGG','ALZ'])('applies the detected %s batch source encoding and preserves unrelated settings', async format => {
    const wrapper=mountView(),store=useTaskStore()
    for(const id of ['legacy','rar'])store.addTask({id,name:id+'.data',type:'decompression',sourceFiles:['C:/'+id+'.data'],outputPath:'D:/out',password:'keep',configurationMode:'global'})
    mocks.invoke.mockImplementation(async(cmd,args)=>cmd==='detect_split_archive'?{is_split:false,archive_format:args.path.includes('legacy')?format:'RAR'}:undefined)
    await nextTick();const table=wrapper.findComponent({name:'AeroTable'});table.vm.$emit('toggle-task','legacy');table.vm.$emit('toggle-task','rar');await nextTick()
    const bar=wrapper.findComponent({name:'BatchConfigBar'});bar.vm.$emit('apply',{zipPasswordEncoding:'cp936'});await flushPromises()
    expect(store.tasks[0]).toMatchObject({format,zipPasswordEncoding:'cp936',configurationMode:'global',outputPath:'D:/out',password:'keep'})
    expect(store.tasks[1].zipPasswordEncoding).toBeUndefined();bar.vm.$emit('apply',{zipPasswordEncoding:'auto'});await flushPromises();expect(store.tasks[0].zipPasswordEncoding).toBeUndefined();wrapper.unmount()
  })
  it('batch encoding uses detected ZIP content without detaching global output or touching non-ZIP tasks', async () => {
    const wrapper = mountView()
    const store = useTaskStore()
    for (const id of ['renamed', 'rar', 'other']) store.addTask({ id, name: `${id}.rar`, type: 'decompression', sourceFiles: [`C:/${id}.rar`], outputPath: 'D:/out', password: 'keep', configurationMode: 'global' })
    mocks.invoke.mockImplementation(async (_cmd, args) => ({ is_split: false, archive_format: args.path.includes('renamed') ? 'ZIP' : 'RAR' }))
    await nextTick()
    const table = wrapper.findComponent({ name: 'AeroTable' })
    table.vm.$emit('toggle-task', 'renamed'); table.vm.$emit('toggle-task', 'rar')
    await nextTick()
    const bar = wrapper.findComponent({ name: 'BatchConfigBar' })
    bar.vm.$emit('apply', { zipPasswordEncoding: 'cp950' }); await flushPromises()
    expect(store.tasks[0]).toMatchObject({ zipPasswordEncoding: 'cp950', configurationMode: 'global', outputPath: 'D:/out', password: 'keep' })
    expect(store.tasks[1].zipPasswordEncoding).toBeUndefined()
    expect(store.tasks[2].zipPasswordEncoding).toBeUndefined()
    bar.vm.$emit('apply', { zipPasswordEncoding: 'auto' }); await flushPromises()
    expect(store.tasks[0].zipPasswordEncoding).toBeUndefined()
    wrapper.unmount()
  })
  it.each(['start', 'replace', 'unmount'])('does not apply a late encoding detection after I %s the task', async action => {
    const wrapper = mountView()
    const store = useTaskStore()
    store.addTask({ id: 'late', name: 'a.zip', type: 'decompression', sourceFiles: ['C:/a.zip'], outputPath: 'D:/out', configurationMode: 'global' })
    await nextTick()
    wrapper.findComponent({ name: 'AeroTable' }).vm.$emit('toggle-task', 'late'); await nextTick()
    let finish!: (value: unknown) => void
    mocks.invoke.mockImplementationOnce(() => new Promise(resolve=>{finish=resolve}))
    wrapper.findComponent({ name: 'BatchConfigBar' }).vm.$emit('apply', { zipPasswordEncoding: 'cp932' })
    if (action === 'start') store.tasks[0].status = 'extracting'
    else if (action === 'replace') store.tasks[0].sourceFiles[0] = 'C:/replacement.zip'
    else wrapper.unmount()
    finish({ is_split: false, archive_format: 'ZIP' }); await flushPromises()
    expect(store.tasks[0].zipPasswordEncoding).toBeUndefined()
    expect(store.tasks[0].configurationMode).toBe('global')
    if (action !== 'unmount') wrapper.unmount()
  })
  it('does not downgrade an unconfirmed forum volume to an ordinary ZIP', async () => {
    const wrapper = mountView()
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'detect_split_archive') throw new Error('[archive-inspection:incomplete] 未找到可读的终卷目录；总卷数无法确认')
      return []
    })
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{name:'资料.haozip01.zip',path:'C:/资料.haozip01.zip'}])
    await flushPromises()
    expect(useTaskStore().tasks).toHaveLength(0)
    expect(useAppStore().error).toContain('终卷目录')
    expect(useAppStore().error).not.toContain('[archive-inspection:')
    mocks.invoke.mockImplementation(async () => ({is_split:false}))
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{name:'资料.haozip02.zip',path:'C:/资料.haozip02.zip'}])
    await flushPromises()
    expect(useTaskStore().tasks).toHaveLength(0)
    wrapper.unmount()
  })

  it('deduplicates genuine forum member names using the detector canonical source', async () => {
    const parts = ['C:/资料.haozip01.zip', 'C:/资料.haozip02.zip']
    const wrapper = mountView()
    mocks.invoke.mockImplementation(async (command: string) => command === 'detect_split_archive' ? {
      is_split:true, format:'ZipSplit', base_name:'资料', first_part:parts[0], parts,
      total_parts:2, is_complete:true, missing_parts:[],
    } : [])
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [parts[1],parts[0]].map(path=>({name:path.split('/').pop(),path})))
    await flushPromises()
    expect(useTaskStore().tasks).toHaveLength(1)
    expect(useTaskStore().tasks[0]).toMatchObject({sourceFiles:[parts[0]],name:'资料 (2 个分卷)'})
    wrapper.unmount()
  })

  it('imports only verified SFX and never queues an ordinary executable', async () => {
    const wrapper = mountView()
    mocks.invoke.mockImplementation(async (command: string, args: any) => {
      if (command === 'detect_split_archive') {
        if (args.path.endsWith('program.exe')) throw new Error('[archive-inspection:unsupported] 不是可读取的 ZIP、7Z 或 RAR 自解压归档')
        return { is_split: false, archive_format: '7z' }
      }
      return []
    })
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [
      {name:'收到的资料.exe',path:'C:/收到的资料.exe'},
      {name:'program.exe',path:'C:/program.exe'},
    ])
    await flushPromises()
    expect(useTaskStore().tasks.map(task=>task.sourceFiles[0])).toEqual(['C:/收到的资料.exe'])
    expect(useAppStore().error).toContain('不是可读取')
    expect(useAppStore().error).not.toContain('[archive-inspection:')
    wrapper.unmount()
  })

  it('deduplicates original EXE first and external RAR members after native canonicalization', async () => {
    const wrapper = mountView(), parts = ['C:/资料.part1.exe', 'C:/资料.part2.rar']
    mocks.invoke.mockImplementation(async (cmd: string) => cmd === 'detect_split_archive' ? { is_split: true, format: 'RarSplit', archive_format: 'rar', base_name: '资料', first_part: parts[0], parts, total_parts: 2, is_complete: true, missing_parts: [] } : [])
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [parts[1], parts[0]].map(path => ({ name: path.split('/').pop(), path })))
    await flushPromises()
    expect(useTaskStore().tasks).toHaveLength(1)
    expect(useTaskStore().tasks[0]).toMatchObject({ sourceFiles: [parts[0]], format: 'rar' })
    wrapper.unmount()
  })

  it('explains the original EXE alternative when the RAR first member is absent', async () => {
    const wrapper = mountView(), first = 'C:/资料.part1.rar'
    mocks.invoke.mockImplementation(async (cmd: string) => cmd === 'detect_split_archive' ? { is_split: true, format: 'RarSplit', first_part: first, parts: ['C:/资料.part2.rar'], is_complete: false, missing_parts: [first] } : [])
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{ name: '资料.part2.rar', path: 'C:/资料.part2.rar' }])
    await flushPromises()
    expect(useTaskStore().tasks).toHaveLength(0)
    expect(useAppStore().error).toContain('EXE')
    expect(useAppStore().error).toContain('不要改名')
    wrapper.unmount()
  })

  it('changes selected pending configuration without rewriting passwords, defaults or running tasks', async () => {
    const wrapper = mountView()
    const store = useTaskStore()
    for (const id of ['first', 'second', 'other', 'running']) store.addTask({ id, name: `${id}.zip`, type: 'decompression', sourceFiles: [`C:/${id}.zip`], outputPath: `D:/${id}`, extractToSubfolder: true, password: `${id}-secret`, configurationMode: 'global' })
    store.tasks.find(task => task.id === 'running')!.status = 'extracting'
    await nextTick()
    const table = wrapper.findComponent({ name: 'AeroTable' })
    table.vm.$emit('toggle-task', 'first'); table.vm.$emit('toggle-task', 'second'); table.vm.$emit('toggle-task', 'running')
    await nextTick()
    const bar = wrapper.findComponent({ name: 'BatchConfigBar' })
    expect(bar.props('count')).toBe(2)
    bar.vm.$emit('apply', { outputDirectory: '', extractToSubfolder: false })
    await nextTick()
    expect(store.tasks[0]).toMatchObject({ outputPath: 'C:/', extractToSubfolder: false, password: 'first-secret', configurationMode: 'individual' })
    expect(store.tasks[1]).toMatchObject({ outputPath: 'C:/', extractToSubfolder: false, password: 'second-secret' })
    expect(store.tasks[2]).toMatchObject({ outputPath: 'D:/other', extractToSubfolder: true, configurationMode: 'global' })
    expect(store.tasks[3]).toMatchObject({ status: 'extracting', outputPath: 'D:/running', extractToSubfolder: true })
    await wrapper.get('[data-testid="global-subfolder"]').trigger('click')
    expect(store.tasks[0].extractToSubfolder).toBe(false)
    wrapper.unmount()
  })
  const addConflictTask = (id: string) => {
    const store = useTaskStore()
    store.addTask({ id, name: `${id}.zip`, type: 'decompression', sourceFiles: [`C:/${id}.zip`], outputPath: 'C:/out' })
    const task = store.tasks.find(item => item.id === id)!
    task.conflicts = ['first.txt', 'second.txt'].map(fileName => ({
      taskId: id, fileName, sourcePath: `C:/${id}.zip`, destPath: `C:/out/${fileName}`,
      sourceSize: 2, destSize: 1, sourceModified: 0, destModified: 0,
    }))
    return task
  }

  it('collects my different per-file choices and commits the staged result only once', async () => {
    const wrapper = mountView()
    const task = addConflictTask('mixed')
    await flushPromises()
    let dialog = wrapper.findComponent({ name: 'ConflictResolutionModal' })
    dialog.vm.$emit('resolve', 'skip', false)
    await flushPromises()
    expect(task.conflicts.map(item => item.fileName)).toEqual(['second.txt'])
    expect(mocks.resolveExtractionConflict).not.toHaveBeenCalled()
    dialog = wrapper.findComponent({ name: 'ConflictResolutionModal' })
    dialog.vm.$emit('resolve', 'overwrite', false)
    await flushPromises()
    expect(mocks.resolveExtractionConflict).toHaveBeenCalledExactlyOnceWith('mixed', [
      { destPath: 'C:/out/first.txt', action: 'skip' },
      { destPath: 'C:/out/second.txt', action: 'overwrite' },
    ], undefined)
    expect(mocks.decompressFile).not.toHaveBeenCalled()
    expect(task.conflicts).toEqual([])
    wrapper.unmount()
  })

  it('remembers my first choice after leaving the page and publishes it with the remaining choice', async () => {
    const pinia = createPinia()
    let wrapper = mountView(pinia)
    addConflictTask('navigation')
    await flushPromises()
    wrapper.findComponent({ name: 'ConflictResolutionModal' }).vm.$emit('resolve', 'skip', false)
    await flushPromises()
    wrapper.unmount()
    wrapper = mountView(pinia)
    await flushPromises()
    const dialog = wrapper.findComponent({ name: 'ConflictResolutionModal' })
    expect(dialog.props('taskId')).toBe('navigation')
    dialog.vm.$emit('resolve', 'rename', false)
    await flushPromises()
    expect(mocks.resolveExtractionConflict).toHaveBeenCalledExactlyOnceWith('navigation', [
      { destPath: 'C:/out/first.txt', action: 'skip' },
      { destPath: 'C:/out/second.txt', action: 'rename' },
    ], undefined)
    wrapper.unmount()
  })

  it('does not submit twice after navigating away during publication', async () => {
    const pinia = createPinia()
    let wrapper = mountView(pinia)
    const task = addConflictTask('publishing')
    await flushPromises()
    let finish!: () => void
    mocks.resolveExtractionConflict.mockImplementationOnce(() => new Promise<void>(resolve => { finish = resolve }))
    wrapper.findComponent({ name: 'ConflictResolutionModal' }).vm.$emit('resolve', 'overwrite', true)
    await flushPromises()
    wrapper.unmount()
    wrapper = mountView(pinia)
    await flushPromises()
    expect(wrapper.findComponent({ name: 'ConflictResolutionModal' }).exists()).toBe(false)
    expect(task.conflictResolutionPending).toBe(true)
    finish()
    await flushPromises()
    expect(task.conflicts).toEqual([])
    expect(task.conflictResolutionPending).toBe(false)
    expect(mocks.resolveExtractionConflict).toHaveBeenCalledTimes(1)
    wrapper.unmount()
  })

  it('does not start extraction again while staged conflicts are waiting for my choices', async () => {
    const wrapper = mountView()
    addConflictTask('waiting')
    await flushPromises()
    const start = wrapper.findAll('button').find(button => button.text().includes(useAppStore().t('decompress.start_queue')))
    await start!.trigger('click')
    await flushPromises()
    expect(mocks.decompressFile).not.toHaveBeenCalled()
    expect(mocks.preflightOperationResources).not.toHaveBeenCalled()
    wrapper.unmount()
  })

  it('applies overwrite only to this task and waits for publication before showing the next task', async () => {
    const wrapper = mountView()
    useAppStore().settings.conflictPolicy = 'ask'
    const first = addConflictTask('first')
    addConflictTask('next')
    await flushPromises()
    let complete!: () => void
    mocks.resolveExtractionConflict.mockImplementationOnce(() => new Promise<void>(resolve => { complete = resolve }))
    const dialog = wrapper.findComponent({ name: 'ConflictResolutionModal' })
    dialog.vm.$emit('resolve', 'overwrite', true)
    dialog.vm.$emit('resolve', 'overwrite', true)
    await flushPromises()
    expect(mocks.resolveExtractionConflict).toHaveBeenCalledTimes(1)
    expect(mocks.resolveExtractionConflict).toHaveBeenCalledWith('first', [{ destPath: 'C:/out/first.txt', action: 'overwrite' }], 'overwrite')
    expect(useAppStore().settings.conflictPolicy).toBe('ask')
    expect(first.conflicts).toHaveLength(2)
    expect(dialog.props('busy')).toBe(true)
    complete()
    await flushPromises()
    expect(first.conflicts).toEqual([])
    const next = wrapper.findComponent({ name: 'ConflictResolutionModal' })
    expect(next.props('taskId')).toBe('next')
    expect(next.props('busy')).toBe(false)
    wrapper.unmount()
  })

  it('retains failed publication details without automatically asking the same failed task again', async () => {
    const wrapper = mountView()
    const task = addConflictTask('failure')
    await flushPromises()
    mocks.resolveExtractionConflict.mockImplementationOnce(async () => {
      useTaskStore().failTask(task.id, '目标文件被占用，无法保存')
      throw new Error('目标文件被占用，无法保存')
    })
    wrapper.findComponent({ name: 'ConflictResolutionModal' }).vm.$emit('resolve', 'rename', true)
    await flushPromises()
    expect(task.error).toContain('目标文件被占用')
    expect(task.conflicts).toHaveLength(2)
    expect(wrapper.findComponent({ name: 'ConflictResolutionModal' }).exists()).toBe(false)
    expect(mocks.resolveExtractionConflict).toHaveBeenCalledTimes(1)
    wrapper.unmount()
  })

  it('does not publish from a stale dialog after I cancel the task', async () => {
    const wrapper = mountView()
    const task = addConflictTask('cancelled')
    await flushPromises()
    task.status = 'cancelled'
    wrapper.findComponent({ name: 'ConflictResolutionModal' }).vm.$emit('resolve', 'overwrite', true)
    await flushPromises()
    expect(mocks.resolveExtractionConflict).not.toHaveBeenCalled()
    wrapper.unmount()
  })

  it('lets me inspect only this run failures and return to the full queue without starting hidden tasks', async () => {
    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{ name: 'new.zip', path: 'C:/new.zip' }])
    await flushPromises()
    const store = useTaskStore()
    const task = store.tasks[0]
    const start = wrapper.findAll('button').find(button => button.text().includes(useAppStore().t('decompress.start_queue')))
    await start!.trigger('click')
    await flushPromises()
    store.failTask(task.id, '文件损坏')
    store.addTask({ id: 'later', name: 'later.zip', type: 'decompression', sourceFiles: ['C:/later.zip'], outputPath: 'C:/out' })
    await flushPromises()
    expect(useDecompressionBatchStore().counts.total).toBe(1)
    await wrapper.get('[data-testid="show-batch-failures"]').trigger('click')
    expect(wrapper.findComponent({ name: 'AeroTable' }).props('taskIds')).toEqual([task.id])
    expect(wrapper.find('[data-testid="decompression-batch-config-mode"]').exists()).toBe(false)
    expect(wrapper.findAll('button').some(button => button.text().includes(useAppStore().t('decompress.clear_finished')))).toBe(false)
    expect(wrapper.findAll('button').some(button => button.text().includes(useAppStore().t('decompress.start_queue')))).toBe(false)
    expect(mocks.decompressFile).toHaveBeenCalledTimes(1)
    await wrapper.get('[data-testid="show-all-decompression-tasks"]').trigger('click')
    expect(wrapper.findComponent({ name: 'AeroTable' }).props('taskIds')).toBeUndefined()
    wrapper.unmount()
  })

  beforeEach(() => {
    localStorage.clear()
    vi.clearAllMocks()
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'detect_split_archive') return null
      if (command === 'load_app_settings') return '{}'
      return []
    })
    mocks.listArchiveContents.mockResolvedValue(['one.txt', 'two.txt'])
    mocks.decompressFile.mockResolvedValue(undefined)
    mocks.resolveExtractionConflict.mockResolvedValue(undefined)
    mocks.preflightOperationResources.mockResolvedValue({
      operation: 'decompression',
      outputPath: 'C:/archives',
      probePath: 'C:/archives',
      mountPoint: 'C:/',
      fileSystem: 'NTFS',
      location: 'local',
      medium: 'ssd',
      totalBytes: 1_000_000_000,
      availableBytes: 900_000_000,
      estimatedOutputBytes: 20,
      requiredBytes: 134_217_748,
      reserveBytes: 134_217_728,
      estimateSource: 'archive_metadata',
      estimateReliable: true,
      status: 'ready',
      canStart: true,
      summary: '空间充足',
      warnings: [],
    })
  })

  it('adds an archive task and enables smart subfolder extraction for multiple roots', async () => {
    const wrapper = mountView()
    useAppStore().updateSettings({ smartExtractionDirectory: true })
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{
      name: 'bundle.zip',
      path: 'C:/archives/bundle.zip',
    }])
    await nextTick()
    await nextTick()

    const task = useTaskStore().tasks[0]
    expect(task).toMatchObject({
      name: 'bundle.zip',
      sourceFiles: ['C:/archives/bundle.zip'],
      outputPath: 'C:/archives',
      extractToSubfolder: true,
    })
    expect(useAppStore().recentFiles).toContain('C:/archives/bundle.zip')
    wrapper.unmount()
  })

  it.each(['individual', 'global', 'source'] as const)('does not let a late archive listing undo my %s change', async change => {
    let finish!: (files: string[]) => void
    mocks.listArchiveContents.mockImplementationOnce(() => new Promise<string[]>(resolve => { finish = resolve }))
    const wrapper = mountView()
    useAppStore().updateSettings({ smartExtractionDirectory: true })
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{ name: 'slow.zip', path: 'C:/slow.zip' }])
    await flushPromises()
    const task = useTaskStore().tasks[0]
    if (change === 'individual') {
      task.extractToSubfolder = false
      task.configurationMode = 'individual'
    } else if (change === 'global') {
      // Even toggling away and back is an explicit user choice.
      await wrapper.get('[data-testid="global-subfolder"]').trigger('click')
      await wrapper.get('[data-testid="global-subfolder"]').trigger('click')
    } else {
      task.sourceFiles = ['C:/replacement.zip']
    }
    const chosen = task.extractToSubfolder
    finish(chosen ? ['single.txt'] : ['one/a.txt', 'two/b.txt'])
    await flushPromises()
    expect(task.extractToSubfolder).toBe(chosen)
    wrapper.unmount()
  })

  it('recognizes two top-level folders even when ZIP lists only their files', async () => {
    mocks.listArchiveContents.mockResolvedValueOnce(['甲/a.txt', '乙/b.txt'])
    const wrapper = mountView()
    useAppStore().updateSettings({ smartExtractionDirectory: true })
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{ name: 'folders.zip', path: 'C:/folders.zip' }])
    await flushPromises()
    expect(useTaskStore().tasks[0].extractToSubfolder).toBe(true)
    wrapper.unmount()
  })

  it('does not inspect archive structure unless I enable the setting', async () => {
    const wrapper = mountView()
    expect(useAppStore().settings.smartExtractionDirectory).toBe(false)
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{ name: 'bundle.zip', path: 'C:/bundle.zip' }])
    await flushPromises()
    expect(mocks.listArchiveContents).not.toHaveBeenCalled()
    expect(useTaskStore().tasks[0].extractToSubfolder).toBe(false)
    wrapper.unmount()
  })

  it.each([false, true])('ignores outstanding analysis after disabling it, even if enabled again: %s', async reenable => {
    let finish!: (files: string[]) => void
    mocks.listArchiveContents.mockImplementationOnce(() => new Promise<string[]>(resolve => { finish = resolve }))
    const wrapper = mountView()
    const app = useAppStore()
    app.updateSettings({ smartExtractionDirectory: true })
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{ name: 'slow.zip', path: 'C:/slow.zip' }])
    await flushPromises()
    app.updateSettings({ smartExtractionDirectory: false })
    if (reenable) app.updateSettings({ smartExtractionDirectory: true })
    finish(['one.txt', 'two.txt'])
    await flushPromises()
    expect(useTaskStore().tasks[0].extractToSubfolder).toBe(false)
    wrapper.unmount()
  })

  it('rejects Office document containers before creating extraction tasks', async () => {
    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{
      name: 'report.docx',
      path: 'C:/documents/report.docx',
    }, {
      name: 'budget.xlsx',
      path: 'C:/documents/budget.xlsx',
    }])
    await nextTick()

    expect(useTaskStore().tasks).toHaveLength(0)
    expect(useAppStore().error).toContain('2')
    expect(mocks.invoke).not.toHaveBeenCalledWith('detect_split_archive', expect.anything())
    wrapper.unmount()
  })

  it('groups unordered numeric split volumes into one task using the first volume', async () => {
    const parts = [1, 2, 3, 4, 5].map(index => `C:/archives/project.zip.${String(index).padStart(3, '0')}`)
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'detect_split_archive') return {
        is_split: true,
        format: 'GenericNumeric',
        base_name: 'project.zip',
        parts,
        first_part: parts[0],
        total_parts: 5,
        total_size: 500,
        is_complete: true,
        missing_parts: [],
      }
      return []
    })
    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [5, 2, 1, 4, 3, 3].map(index => ({
      name: `project.zip.${String(index).padStart(3, '0')}`,
      path: parts[index - 1],
    })))
    await flushPromises()

    expect(useTaskStore().tasks).toHaveLength(1)
    expect(useTaskStore().tasks[0]).toMatchObject({
      name: 'project.zip (5 个分卷)',
      sourceFiles: [parts[0]],
    })
    expect(mocks.invoke.mock.calls.filter(([command]) => command === 'detect_split_archive')).toHaveLength(1)
    wrapper.unmount()
  })

  it('rejects an incomplete split group with the missing volume name', async () => {
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'detect_split_archive') return {
        is_split: true,
        format: 'GenericNumeric',
        base_name: 'project.zip',
        parts: ['C:/archives/project.zip.001', 'C:/archives/project.zip.003'],
        first_part: 'C:/archives/project.zip.001',
        total_parts: 3,
        total_size: 200,
        is_complete: false,
        missing_parts: ['C:/archives/project.zip.002'],
      }
      return []
    })
    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{
      name: 'project.zip.003',
      path: 'C:/archives/project.zip.003',
    }])
    await flushPromises()

    expect(useTaskStore().tasks).toHaveLength(0)
    expect(useAppStore().error).toContain('project.zip.002')
    wrapper.unmount()
  })

  it('starts pending archive tasks with the configured extraction options', async () => {
    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{
      name: 'bundle.zip',
      path: 'C:/archives/bundle.zip',
    }])
    await nextTick()
    await nextTick()

    // Explicit user choice after importing, independent of the opt-in analysis.
    await wrapper.get('[data-testid="global-subfolder"]').trigger('click')
    const appStore = useAppStore()
    const startButton = wrapper.findAll('button').find(button => button.text().includes(appStore.t('decompress.start_queue')))
    await startButton!.trigger('click')

    expect(mocks.decompressFile).toHaveBeenCalledWith(
      'C:/archives/bundle.zip',
      expect.objectContaining({
        outputPath: 'C:/archives',
        createSubdirectory: true,
        conflictPolicy: appStore.settings.conflictPolicy,
      }),
      expect.any(String),
    )
    wrapper.unmount()
  })

  it('applies the global recycle-bin default to pending and newly added tasks', async () => {
    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{
      name: 'bundle.zip',
      path: 'C:/archives/bundle.zip',
    }])
    await flushPromises()

    const globalRecycleSwitch = wrapper.get('[data-testid="global-recycle-source-switch"]')
    expect(globalRecycleSwitch.attributes('aria-checked')).toBe('false')
    await globalRecycleSwitch.trigger('click')

    const appStore = useAppStore()
    const taskStore = useTaskStore()
    expect(appStore.settings.autoDeleteSource).toBe(true)
    expect(taskStore.tasks[0].recycleSourceAfterExtract).toBe(true)

    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{
      name: 'second.zip',
      path: 'C:/archives/second.zip',
    }])
    await flushPromises()
    expect(taskStore.tasks[1].recycleSourceAfterExtract).toBe(true)

    const startButton = wrapper.findAll('button').find(
      button => button.text().includes(appStore.t('decompress.start_queue')),
    )
    await startButton!.trigger('click')
    await flushPromises()

    expect(mocks.decompressFile).toHaveBeenCalledWith(
      'C:/archives/bundle.zip',
      expect.objectContaining({ deleteAfter: true }),
      expect.any(String),
    )
    wrapper.unmount()
  })

  it('keeps a file-conflict task resumable without showing a false failure', async () => {
    mocks.decompressFile.mockRejectedValueOnce(
      new Error('Extraction failed: File conflict requires resolution'),
    )
    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{
      name: 'bundle.zip',
      path: 'C:/archives/bundle.zip',
    }])
    await nextTick()
    await nextTick()

    const appStore = useAppStore()
    const taskStore = useTaskStore()
    const startButton = wrapper.findAll('button').find(
      button => button.text().includes(appStore.t('decompress.start_queue')),
    )
    await startButton!.trigger('click')
    await flushPromises()

    expect(taskStore.tasks[0]).toMatchObject({
      status: 'pending',
      error: undefined,
    })
    expect(taskStore.tasks[0].logs.at(-1)).toMatchObject({
      message: appStore.t('decompress.conflict_waiting'),
      severity: 'warning',
    })
    expect(appStore.error).toBeNull()
    wrapper.unmount()
  })

  it('blocks decompression before the engine when expanded contents cannot fit', async () => {
    mocks.preflightOperationResources.mockResolvedValueOnce({
      operation: 'decompression',
      outputPath: 'C:/archives',
      probePath: 'C:/archives',
      mountPoint: 'C:/',
      fileSystem: 'NTFS',
      location: 'local',
      medium: 'ssd',
      totalBytes: 1_000,
      availableBytes: 100,
      estimatedOutputBytes: 900,
      requiredBytes: 134_218_628,
      reserveBytes: 134_217_728,
      estimateSource: 'archive_metadata',
      estimateReliable: true,
      status: 'blocked',
      canStart: false,
      summary: '解压空间不足',
      warnings: [],
    })
    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{
      name: 'large.zip',
      path: 'C:/archives/large.zip',
    }])
    await flushPromises()
    const startButton = wrapper.findAll('button').find(
      button => button.text().includes(useAppStore().t('decompress.start_queue')),
    )
    await startButton!.trigger('click')
    await flushPromises()

    expect(mocks.decompressFile).not.toHaveBeenCalled()
    expect(useTaskStore().tasks[0]).toMatchObject({
      status: 'failed',
      error: '解压空间不足',
      resourcePreflight: { status: 'blocked', canStart: false },
    })
    wrapper.unmount()
  })

  it('never displays or starts pending compression tasks from the decompression workspace', async () => {
    const wrapper = mountView()
    const taskStore = useTaskStore()
    const compressionTaskId = taskStore.addTask({
      id: 'pending-compression',
      name: 'source-folder',
      type: 'compression',
      sourceFiles: ['C:/input/source-folder'],
      outputPath: 'C:/input/source-folder.7z',
    })
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{
      name: 'bundle.zip',
      path: 'C:/archives/bundle.zip',
    }])
    await nextTick()
    await nextTick()

    const table = wrapper.findComponent({ name: 'AeroTable' })
    expect(table.props('taskType')).toBe('decompression')

    const startButton = wrapper.findAll('button').find(
      button => button.text().includes(useAppStore().t('decompress.start_queue')),
    )
    await startButton!.trigger('click')

    expect(mocks.decompressFile).toHaveBeenCalledTimes(1)
    expect(mocks.decompressFile).toHaveBeenCalledWith(
      'C:/archives/bundle.zip',
      expect.anything(),
      expect.any(String),
    )
    expect(taskStore.tasks.find(task => task.id === compressionTaskId)?.status).toBe('pending')
    wrapper.unmount()
  })

  it('clears only finished decompression tasks', async () => {
    const wrapper = mountView()
    const taskStore = useTaskStore()
    for (const [id, type] of [
      ['finished-compression', 'compression'],
      ['finished-decompression', 'decompression'],
    ] as const) {
      taskStore.addTask({
        id,
        name: id,
        type,
        sourceFiles: [`C:/input/${id}`],
        outputPath: 'C:/output',
      })
      taskStore.updateTaskStatus(id, 'completed')
      await taskStore.waitForHistoryPersistence(id)
    }
    await nextTick()

    const clearButton = wrapper.findAll('button').find(
      button => button.text().includes(useAppStore().t('decompress.clear_finished')),
    )
    await clearButton!.trigger('click')

    expect(taskStore.tasks.map(task => task.id)).toEqual(['finished-compression'])
    wrapper.unmount()
  })

  it('runs a queued one-click extraction in a safe same-name folder', async () => {
    const wrapper = mountView()
    const taskStore = useTaskStore()
    const existingTaskId = taskStore.addTask({
      id: 'existing-pending-task',
      name: 'existing.zip',
      type: 'decompression',
      sourceFiles: ['C:/archives/existing.zip'],
      outputPath: 'C:/archives',
      extractToSubfolder: false,
    })
    useAppStore().enqueueContextAction({
      action: 'context-quick-extract',
      files: ['C:/archives/quick.zip'],
    })
    await flushPromises()
    await nextTick()

    expect(mocks.decompressFile).toHaveBeenCalledWith(
      'C:/archives/quick.zip',
      expect.objectContaining({
        outputPath: 'C:/archives',
        createSubdirectory: true,
      }),
      expect.any(String),
    )
    expect(mocks.decompressFile).not.toHaveBeenCalledWith(
      'C:/archives/existing.zip',
      expect.anything(),
      existingTaskId,
    )
    expect(taskStore.tasks.find(task => task.id === existingTaskId)?.status).toBe('pending')
    wrapper.unmount()
  })

  it('serially consumes every queued context action without dropping files', async () => {
    const wrapper = mountView()
    const appStore = useAppStore()
    appStore.enqueueContextAction({ action: 'context-quick-extract', files: ['C:/archives/first.zip'] })
    appStore.enqueueContextAction({ action: 'context-quick-extract', files: ['C:/archives/second.zip'] })
    await flushPromises()
    await nextTick()

    expect(mocks.decompressFile).toHaveBeenCalledTimes(2)
    expect(mocks.decompressFile.mock.calls.map(call => call[0])).toEqual([
      'C:/archives/first.zip',
      'C:/archives/second.zip',
    ])
    wrapper.unmount()
  })

  it('opens a new-task confirmation for a supplied password without rewriting the failed task', async () => {
    const wrapper = mountView()
    const taskStore = useTaskStore()
    const taskId = taskStore.addTask({
      id: 'encrypted-task',
      name: 'encrypted.7z',
      type: 'decompression',
      sourceFiles: ['C:/archives/encrypted.7z'],
      outputPath: 'C:/archives/output',
      extractToSubfolder: true,
      password: 'correct-password',
      passwordRequired: true,
    })
    taskStore.updateTaskStatus(taskId, 'failed')
    await nextTick()

    wrapper.findComponent({ name: 'AeroTable' }).vm.$emit('retry-with-password', taskId)
    await flushPromises()

    expect(mocks.decompressFile).not.toHaveBeenCalled()
    expect(taskStore.tasks.find(task => task.id === taskId)?.status).toBe('failed')
    expect(wrapper.findComponent({ name: 'FailedExtractionRetry' }).props('taskIds')).toEqual([taskId])
    expect(wrapper.findComponent({ name: 'FailedExtractionRetry' }).props('suppliedPassword')).toBe('correct-password')
    wrapper.unmount()
  })

  it('keeps the original password-required state while the correction dialog is open', async () => {
    const wrapper = mountView()
    const taskStore = useTaskStore()
    const taskId = taskStore.addTask({
      id: 'wrong-password-task',
      name: 'encrypted.7z',
      type: 'decompression',
      sourceFiles: ['C:/archives/encrypted.7z'],
      outputPath: 'C:/archives/output',
      password: 'wrong-password',
      passwordRequired: true,
    })
    taskStore.updateTaskStatus(taskId, 'failed')
    await nextTick()

    wrapper.findComponent({ name: 'AeroTable' }).vm.$emit('retry-with-password', taskId)
    await flushPromises()

    expect(taskStore.tasks.find(task => task.id === taskId)).toMatchObject({
      status: 'failed',
      passwordRequired: true,
    })
    wrapper.unmount()
  })

  it('leaves password discovery to the backend and exposes one manual retry state', async () => {
    mocks.decompressFile.mockRejectedValueOnce('PasswordRequired')
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'detect_split_archive') return null
      if (command === 'load_app_settings') return '{}'
      return []
    })

    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{
      name: 'plain.zip',
      path: 'C:/archives/plain.zip',
    }])
    await flushPromises()

    const appStore = useAppStore()
    const startButton = wrapper.findAll('button').find(button => button.text().includes(appStore.t('decompress.start_queue')))
    await startButton!.trigger('click')
    await flushPromises()

    expect(mocks.invoke).not.toHaveBeenCalledWith('get_dictionary_passwords', expect.anything())
    expect(mocks.invoke).not.toHaveBeenCalledWith('verify_archive_password', expect.anything())
    expect(mocks.decompressFile).toHaveBeenCalledTimes(1)
    expect(useTaskStore().tasks[0]).toMatchObject({
      status: 'failed',
      passwordRequired: true,
    })
    wrapper.unmount()
  })

  it('does not rewrite a password failure already persisted by the command owner', async () => {
    let originalLogs: unknown
    mocks.decompressFile.mockImplementationOnce(async () => {
      const store = useTaskStore()
      const task = store.tasks[0]
      store.failTask(task.id, '需要输入密码才能解压')
      await store.waitForHistoryPersistence(task.id)
      originalLogs = JSON.parse(JSON.stringify(task.logs))
      throw '需要输入密码才能解压'
    })
    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{ name: 'encrypted.zip', path: 'C:/encrypted.zip' }])
    await flushPromises()
    await wrapper.findAll('button').find(button => button.text().includes(useAppStore().t('decompress.start_queue')))!.trigger('click')
    await flushPromises()
    expect(useTaskStore().tasks[0]).toMatchObject({ status: 'failed', passwordRequired: true, error: '需要输入密码才能解压' })
    expect(useTaskStore().tasks[0].logs).toEqual(originalLogs)
    expect(mocks.invoke.mock.calls.filter(([command]) => command === 'save_task_history')).toHaveLength(1)
    wrapper.unmount()
  })

  it('keeps a non-password backend failure visible on both the task and workspace', async () => {
    mocks.decompressFile.mockRejectedValueOnce({ message: 'disk full' })
    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{
      name: 'large.zip',
      path: 'C:/archives/large.zip',
    }])
    await flushPromises()

    const appStore = useAppStore()
    const startButton = wrapper.findAll('button').find(
      button => button.text().includes(appStore.t('decompress.start_queue')),
    )
    await startButton!.trigger('click')
    await flushPromises()

    expect(useTaskStore().tasks[0]).toMatchObject({
      status: 'failed',
      error: 'disk full',
      passwordRequired: false,
    })
    expect(appStore.error).toContain('disk full')
    wrapper.unmount()
  })

  it('keeps inspection evidence on the failed task but hides routing markers in its toast', async () => {
    const reason = '[archive-inspection:incomplete] 缺少分卷：收到的密码资料.z02'
    mocks.decompressFile.mockRejectedValueOnce({ message: reason })
    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{ name: '资料.zip', path: 'C:/资料.zip' }])
    await flushPromises()
    await wrapper.get('[data-testid="start-decompression-queue"]').trigger('click')
    await flushPromises()
    expect(useTaskStore().tasks[0]).toMatchObject({ status: 'failed', error: reason, passwordRequired: false })
    expect(useAppStore().error).toContain('缺少分卷：收到的密码资料.z02')
    expect(useAppStore().error).not.toContain('[archive-inspection:')
    wrapper.unmount()
  })

  it('reports partial cancellation when one active task cannot be stopped', async () => {
    const wrapper = mountView()
    const taskStore = useTaskStore()
    for (const [id, status] of [['running-task', 'extracting'], ['blocked-task', 'preparing']] as const) {
      taskStore.addTask({
        id,
        name: `${id}.zip`,
        type: 'decompression',
        sourceFiles: [`C:/archives/${id}.zip`],
        outputPath: 'C:/archives',
      })
      taskStore.updateTaskStatus(id, status)
    }
    const cancelTask = vi.spyOn(taskStore, 'cancelTask')
      .mockResolvedValueOnce(true)
      .mockResolvedValueOnce(false)
    await nextTick()

    const appStore = useAppStore()
    const cancelButton = wrapper.findAll('button').find(
      button => button.text().includes(appStore.t('decompress.stop_all')),
    )
    await cancelButton!.trigger('click')
    await flushPromises()

    expect(cancelTask).toHaveBeenNthCalledWith(1, 'running-task')
    expect(cancelTask).toHaveBeenNthCalledWith(2, 'blocked-task')
    expect(appStore.error).toContain('1')
    wrapper.unmount()
  })

  it('keeps global controls on the page without a required mode switch', async () => {
    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{ name: 'configurable.zip', path: 'C:/archives/configurable.zip' }])
    await flushPromises()
    const task = useTaskStore().tasks[0]
    expect(task.configurationMode).toBe('global')
    expect(wrapper.find('[data-testid="decompression-config-mode-individual"]').exists()).toBe(false)
    task.configurationMode = 'individual'
    task.recycleSourceAfterExtract = false
    await wrapper.get('[data-testid="global-recycle-source-switch"]').trigger('click')
    expect(task.recycleSourceAfterExtract).toBe(false)
    wrapper.unmount()
  })

  it('runs the queue in the same natural-name order shown by the task table', async () => {
    const wrapper = mountView()
    const taskStore = useTaskStore()
    const appStore = useAppStore()
    appStore.updateSettings({ maxConcurrentTasks: 1 })
    for (const [id, name] of [
      ['third', '1 (3)-3.rar'],
      ['first', '1 (1)-1.rar'],
      ['second', '1 (2)-2.rar'],
    ]) {
      taskStore.addTask({
        id,
        name,
        type: 'decompression',
        sourceFiles: [`C:/archives/${name}`],
        outputPath: `C:/outputs/${id}`,
      })
    }
    await nextTick()

    const startButton = wrapper.findAll('button').find(
      button => button.text().includes(appStore.t('decompress.start_queue')),
    )
    await startButton!.trigger('click')
    await flushPromises()

    expect(mocks.decompressFile.mock.calls.map(([path]) => path)).toEqual([
      'C:/archives/1 (1)-1.rar',
      'C:/archives/1 (2)-2.rar',
      'C:/archives/1 (3)-3.rar',
    ])
    wrapper.unmount()
  })
})
