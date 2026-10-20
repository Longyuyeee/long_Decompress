import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useArchiveConversionStore, conversionFormats } from '../archiveConversion'
import { useTaskStore } from '../task'
import { createTaskHistoryRecord } from '@/types/taskHistory'
const mocks = vi.hoisted(() => ({ invoke: vi.fn(), ask: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => vi.fn()) }))
vi.mock('@tauri-apps/api/dialog', () => ({ ask: mocks.ask }))
beforeEach(() => { setActivePinia(createPinia()); mocks.invoke.mockReset(); mocks.ask.mockReset(); mocks.ask.mockResolvedValue(true) })
it('zpaq1526 changes selected targets while keeping each native password and volume size',async()=>{
  mocks.invoke.mockImplementation(async command=>command==='detect_split_archive'?{is_split:false,archive_format:'zip'}:undefined)
  const store=useArchiveConversionStore();store.addSources(['C:/甲.zip','C:/乙.zip'])
  store.items.forEach((item,index)=>store.updateItem(item.id,{password:`密码-${index}`,sourcePassword:`原包-${index}`,splitMiB:index+1}))
  expect(await store.applySelected({format:'zpaq'})).toBe('')
  store.items.forEach((item,index)=>{
    expect(item.settings).toMatchObject({format:'zpaq',password:`密码-${index}`,sourcePassword:`原包-${index}`,splitMiB:index+1})
    expect(store.outputFor(item)).toMatch(/\.zpaq$/)
  })
})
function draft() {
  const store = useArchiveConversionStore()
  store.sourcePath = 'C:\\原包.zip'; store.outputPath = 'C:\\新包.7z'
  store.sourcePassword = 'input-secret'; store.password = 'output-secret'
  store.encryptFilenames = true; store.splitMiB = 1
  return store
}
describe('archive conversion lifecycle', () => {
  it('filters input by detected content and output by target without detaching unrelated drafts', async () => {
    mocks.invoke.mockImplementation(async (command,args)=>command==='detect_split_archive' ? {is_split:false,archive_format:args.path==='C:/renamed.7z'?'ZIP':'7Z'} : undefined)
    const store=useArchiveConversionStore(); store.addSources(['C:/renamed.7z','C:/renamed.zip','C:/other.7z'])
    store.updateItem(store.items[1].id,{format:'zipx'})
    expect(await store.applySelected({sourceZipPasswordEncoding:'cp932',zipPasswordEncoding:'cp950'})).toBe('')
    expect(store.items[0].settings).toMatchObject({format:'7z',sourceZipPasswordEncoding:'cp932',zipPasswordEncoding:'utf8'})
    expect(store.items[1].settings).toMatchObject({format:'zipx',zipPasswordEncoding:'cp950'})
    expect(store.items[1].settings?.sourceZipPasswordEncoding).toBeUndefined()
    expect(store.items[2].settings).toBeUndefined()
    expect(store.batchNotice).toContain('2 项原包')
    store.items[1].selected=store.items[2].selected=false
    expect(await store.applySelected({sourceZipPasswordEncoding:undefined,zipMethod:'zstd'})).toBe('')
    expect(store.items[0].settings?.sourceZipPasswordEncoding).toBeUndefined()
    expect(store.defaults.sourceZipPasswordEncoding).toBeUndefined()
  })
  it('does not let a late source probe overwrite a replacement source format', async () => {
    let finish!:(value:unknown)=>void
    mocks.invoke.mockImplementation(async (command,args)=>command==='detect_split_archive' ? args.path==='C:/old.zip' ? new Promise(resolve=>{finish=resolve}) : {is_split:false,archive_format:'7z'} : undefined)
    const store=useArchiveConversionStore(); store.addSources(['C:/old.zip'])
    await vi.waitFor(()=>expect(finish).toBeTypeOf('function'))
    store.replaceSource(store.items[0].id,'C:/replacement.7z')
    finish({is_split:false,archive_format:'zip'})
    await vi.waitFor(()=>expect(store.items[0].sourceFormat).toBe('7z'))
    expect(await store.applySelected({sourceZipPasswordEncoding:'cp932'})).toContain('没有适用')
    expect(store.items[0].settings).toBeUndefined()
  })
  it('omits a ZIP input override from detected non-ZIP execution and keeps non-ZIP output UTF-8 defaults', async () => {
    mocks.invoke.mockImplementation(async (command,args)=>command==='detect_split_archive' ? {is_split:false,archive_format:'7z'} : command==='convert_archive' ? {primaryPath:args.outputPath,files:[],totalBytes:10,split:false} : undefined)
    const store=useArchiveConversionStore();store.addSources(['C:/renamed.zip'])
    store.updateItem(store.items[0].id,{sourceZipPasswordEncoding:'cp950',zipPasswordEncoding:'cp932'})
    await store.start()
    const args=mocks.invoke.mock.calls.find(([c])=>c==='convert_archive')![1]
    expect(args).not.toHaveProperty('sourceZipPasswordEncoding')
    expect(args.options.zip_password_encoding).toBe('utf8')
    expect(useTaskStore().tasks[0].executionConfig).toMatchObject({zipPasswordEncoding:'utf8'})
  })
  it.each([['cp932', 'cp950'], ['cp950', 'cp936'], ['cp936', 'cp932']] as const)('keeps input %s separate from target %s through execution and history retry', async (sourceEncoding, targetEncoding) => {
    mocks.invoke.mockImplementation(async (command, args) => command === 'convert_archive' ? { primaryPath: args.outputPath, files: [], totalBytes: 20, split: false } : undefined)
    const store = useArchiveConversionStore()
    store.addSources(['C:/old.zip'])
    store.updateItem(store.pending[0].id, { format: 'zip', outputPath: 'C:/new.zip',
      sourcePassword: 'input-secret', password: 'output-secret', sourceZipPasswordEncoding: sourceEncoding, zipPasswordEncoding: targetEncoding })
    await store.start()
    expect(mocks.invoke.mock.calls.find(([command]) => command === 'convert_archive')?.[1]).toMatchObject({
      sourceZipPasswordEncoding: sourceEncoding, options: { zip_password_encoding: targetEncoding },
    })
    const task = useTaskStore().tasks[0]
    expect(task.executionConfig).toMatchObject({ sourceZipPasswordEncoding: sourceEncoding, zipPasswordEncoding: targetEncoding })
    expect(JSON.stringify(task.executionConfig)).not.toMatch(/input-secret|output-secret/)
    store.restore(createTaskHistoryRecord(task))
    expect(store.pending.at(-1)?.settings).toMatchObject({ sourceZipPasswordEncoding: sourceEncoding, zipPasswordEncoding: targetEncoding, sourcePassword: '', password: '' })
  })
  it('does not detach inherited non-ZIP conversion drafts when only ZIP methods change', async () => {
    const store = useArchiveConversionStore(); store.addSources(['C:/one.zip', 'C:/two.7z'])
    store.updateItem(store.pending[0].id, { format: 'zip' })
    expect(await store.applySelected({ zipMethod: 'zstd' })).toBe('')
    expect(store.pending[0].settings).toMatchObject({ format: 'zip', zipMethod: 'zstd' })
    expect(store.pending[1].settings).toBeUndefined()
    store.pending[0].selected = false
    expect(await store.applySelected({ zipMethod: 'zstd' })).toContain('没有 ZIP')
  })
  it.each(['zip', 'zipx'])('keeps %s method/password/standard volumes through conversion and history retry', async format => {
    mocks.invoke.mockImplementation(async (command, args) => command === 'convert_archive' ? { primaryPath: args.outputPath, files: [], totalBytes: 20, split: true } : undefined)
    const store = useArchiveConversionStore()
    store.addSources(['C:/原包.7z'])
    store.updateItem(store.pending[0].id, { format, zipMethod: 'zstd', password: 'target-secret', sourcePassword: 'old-secret', splitMiB: 1, zipSplitMode: 'standard', zipEncryption: 'zipcrypto' })
    await store.start()
    const task = useTaskStore().tasks[0]
    expect(mocks.invoke.mock.calls.find(([cmd]) => cmd === 'convert_archive')?.[1]).toMatchObject({ sourcePassword: 'old-secret', options: { format, zip_method: format === 'zip' ? 'zstd' : 'deflate', zip_encryption: 'zipcrypto', zip_split_mode: 'standard', split_size: 1048576 } })
    expect(task.executionConfig).toMatchObject({ format, zipMethod: format === 'zip' ? 'zstd' : 'deflate', passwordRequired: true })
    expect(JSON.stringify(task.executionConfig)).not.toMatch(/target-secret|old-secret/)
    store.restore(createTaskHistoryRecord(task))
    expect(store.pending.at(-1)?.settings).toMatchObject({ format, zipMethod: format === 'zip' ? 'zstd' : 'deflate', zipSplitMode: 'standard', password: '' })
    expect(store.pending.at(-1)?.targetPasswordRequired).toBe(true)
  })
  it('executes and records the explicitly selected ZIP disk layout', async () => {
    mocks.invoke.mockImplementation(async (command, args) => command === 'convert_archive' ? { primaryPath: args.outputPath, files: [], totalBytes: 20, split: true } : undefined)
    const store = useArchiveConversionStore()
    store.addSources(['C:/原包.zip'])
    store.updateItem(store.pending[0].id, { format: 'zip', splitMiB: 1, zipSplitMode: 'standard' })
    await store.start()
    const call = mocks.invoke.mock.calls.find(([command]) => command === 'convert_archive')
    expect(call?.[1].options).toMatchObject({ zip_split_mode: 'standard', split_size: 1048576 })
    expect(useTaskStore().tasks[0].executionConfig).toMatchObject({ zipSplitMode: 'standard' })
  })
  it('keeps my changed defaults when metadata is refreshed and resumes a restored source', async () => {
    mocks.invoke.mockImplementation(async (command, args) => {
      if (command === 'detect_split_archive') return { is_split: false, archive_format: 'zip' }
      if (command === 'convert_archive') return { primaryPath: args.outputPath, files: [], totalBytes: 20, split: false }
    })
    const store = useArchiveConversionStore()
    store.addSources(['C:/资料.zip'], 'reconfigure')
    store.updateDefaults({ format: 'tar.br', level: 3 })
    store.items[0].sourceIssue = '来源曾经不存在'
    await store.start()
    expect(mocks.invoke.mock.calls.find(([cmd]) => cmd === 'convert_archive')?.[1].options).toMatchObject({ format: 'tar.br', level: 3 })
    expect(store.items[0].sourceIssue).toBeUndefined()
  })
  it('detects a standard ZIP volume group even when only its .zip member was selected', async () => {
    mocks.invoke.mockImplementation(async (command, args) => {
      if (command === 'detect_split_archive') return { is_split: true, archive_format: 'zip', first_part: 'C:/照片.zip', parts: ['C:/照片.z01', 'C:/照片.zip'], is_complete: true, missing_parts: [] }
      if (command === 'convert_archive') return { primaryPath: args.outputPath, files: [], totalBytes: 20, split: false }
    })
    const store = useArchiveConversionStore()
    store.addSources(['C:/照片.zip'], 'reconfigure')
    await store.start()
    expect(store.items).toHaveLength(1)
    expect(useTaskStore().tasks[0].sourceFiles).toEqual(['C:/照片.zip', 'C:/照片.z01'])
    expect(useTaskStore().tasks[0].format).toBe('zip')
  })
  it.each(['tar.lz4', 'tar.br', 'tar.gz'])('converts to plain numeric TAR volumes without silently changing the requested format: %s', async format => {
    mocks.invoke.mockImplementation(async (command, args) => command === 'convert_archive' ? { primaryPath: args.outputPath + '.001', files: [], totalBytes: 20, split: true } : undefined)
    const store = useArchiveConversionStore()
    store.addSources(['C:/原包.zip']); store.format = format; store.splitMiB = 1
    await store.start()
    expect(mocks.invoke.mock.calls.find(([cmd]) => cmd === 'convert_archive')?.[1].options).toMatchObject({ format, split_size: 1048576, password: null })
    expect(useTaskStore().tasks[0].archiveOutput?.primaryPath).toBe(`C:/原包_转换.${format}.001`)
  })
  it('asks for an AES wrapper password before starting conversion and retains the configuration', async () => {
    const store = useArchiveConversionStore()
    store.addSources(['C:/原包.zip']); store.format = 'tar.gz.aes'
    await store.start()
    expect(store.error).toContain('需要填写新包密码')
    expect(mocks.invoke.mock.calls.some(([cmd]) => cmd === 'convert_archive')).toBe(false)
    expect(store.format).toBe('tar.gz.aes'); expect(store.pending).toHaveLength(1)
    mocks.invoke.mockImplementation(async (command, args) => command === 'convert_archive' ? { primaryPath: args.outputPath, files: [], totalBytes: 20, split: false } : undefined)
    store.password = 'new-password'; await store.start()
    expect(mocks.invoke.mock.calls.find(([cmd]) => cmd === 'convert_archive')?.[1].options).toMatchObject({ format: 'tar.gz.aes', password: 'new-password' })
  })
  it('converts to password-protected RAR volumes and records reusable settings without transient consent', async () => {
    mocks.invoke.mockImplementation(async (command, args) => {
      if (command === 'check_rar_compression_support') return { available: true }
      if (command === 'convert_archive') return { primaryPath: args.outputPath.replace(/\.rar$/, '.part1.rar'), files: [], totalBytes: 20, split: true }
    })
    const store = useArchiveConversionStore()
    store.addSources(['C:/原包.zip'])
    store.updateItem(store.items[0].id, { format: 'rar', password: 'new-password', splitMiB: 1, encryptFilenames: true })
    await store.start()
    expect(mocks.ask).toHaveBeenCalledTimes(1)
    expect(mocks.invoke.mock.calls.find(([cmd]) => cmd === 'convert_archive')?.[1].options).toMatchObject({
      format: 'rar', password: 'new-password', split_size: 1048576, encrypt_filenames: true, allow_insecure_password_cli: true })
    const record = mocks.invoke.mock.calls.find(([cmd]) => cmd === 'save_task_history')?.[1].record
    expect(record.executionConfig).toMatchObject({ format: 'rar', splitSize: 1048576, encryptFilenames: true, passwordRequired: true })
    expect(JSON.stringify(record)).not.toMatch(/new-password|allow_insecure/)
  })
  it('retains my RAR configuration when the encoder is missing or I cancel the existing password notice', async () => {
    mocks.invoke.mockImplementation(async command => command === 'check_rar_compression_support' ? { available: false } : undefined)
    const store = useArchiveConversionStore()
    store.addSources(['C:/原包.rar'], 'reconfigure')
    expect(store.format).toBe('rar')
    store.password = 'new-password'; store.splitMiB = 1
    await store.start()
    expect(store.error).toContain('WinRAR')
    expect(store.pending).toHaveLength(1)
    mocks.invoke.mockImplementation(async command => command === 'check_rar_compression_support' ? { available: true } : undefined)
    mocks.ask.mockResolvedValue(false)
    await store.start()
    expect(mocks.invoke.mock.calls.some(([cmd]) => cmd === 'convert_archive')).toBe(false)
    expect(store.password).toBe('new-password')
  })
  it('treats several selected numeric volumes as one archive and executes from its real first volume', async () => {
    mocks.invoke.mockImplementation(async (command, args) => {
      if (command === 'detect_split_archive') return { is_split: true, first_part: 'C:/旅行.zip.001',
        parts: ['C:/旅行.zip.001', 'C:/旅行.zip.002'], total_parts: 2, is_complete: true, missing_parts: [] }
      if (command === 'convert_archive') return { primaryPath: args.outputPath, files: [], totalBytes: 20, split: false }
    })
    const store = useArchiveConversionStore()
    store.addSources(['C:/旅行.zip.002', 'C:/旅行.zip.001', 'C:/旅行.zip.002'], 'reconfigure')
    expect(store.items).toHaveLength(1)
    expect(store.outputFor(store.items[0])).toBe('C:/旅行_调整.zip')
    store.updateItem(store.items[0].id, { level: 2 })
    await store.start()
    const calls = mocks.invoke.mock.calls.filter(([command]) => command === 'convert_archive')
    expect(calls).toHaveLength(1)
    expect(calls[0][1]).toMatchObject({ sourcePath: 'C:/旅行.zip.001', options: { format: 'zip', level: 2 } })
    expect(store.taskFor(store.items[0])?.sourceFiles).toEqual(['C:/旅行.zip.001', 'C:/旅行.zip.002'])
  })
  it('keeps ZIP and a clean package name when adjusting a standard ZIP volume', async () => {
    mocks.invoke.mockImplementation(async (command, args) => {
      if (command === 'detect_split_archive') return { is_split: true, first_part: 'C:/照片.zip',
        parts: ['C:/照片.z01', 'C:/照片.zip'], total_parts: 2, is_complete: true, missing_parts: [] }
      if (command === 'convert_archive') return { primaryPath: args.outputPath, files: [], totalBytes: 20, split: false }
    })
    const store = useArchiveConversionStore()
    store.addSources(['C:/照片.z01', 'C:/照片.zip'], 'reconfigure')
    expect(store.items).toHaveLength(1)
    expect(store.format).toBe('zip')
    expect(store.outputFor(store.items[0])).toBe('C:/照片_调整.zip')
    await store.start()
    expect(mocks.invoke.mock.calls.find(([cmd]) => cmd === 'convert_archive')?.[1].sourcePath).toBe('C:/照片.zip')
  })
  it('lets me add a missing volume and retry the same edited task', async () => {
    let complete = false
    mocks.invoke.mockImplementation(async (command, args) => {
      if (command === 'detect_split_archive') return { is_split: true, first_part: 'C:/资料.7z.001',
        parts: ['C:/资料.7z.001', 'C:/资料.7z.003'], total_parts: 3, is_complete: complete,
        missing_parts: complete ? [] : ['C:/资料.7z.002'] }
      if (command === 'convert_archive') return { primaryPath: args.outputPath, files: [], totalBytes: 20, split: false }
    })
    const store = useArchiveConversionStore()
    store.addSources(['C:/资料.7z.003'])
    const id = store.items[0].id
    store.updateItem(id, { level: 3, outputPath: 'D:/我的资料.7z' })
    await store.start()
    expect(store.items[0].sourceIssue).toContain('资料.7z.002')
    expect(mocks.invoke.mock.calls.some(([cmd]) => cmd === 'convert_archive')).toBe(false)
    complete = true
    await store.start()
    expect(store.items[0].id).toBe(id)
    expect(store.items[0].sourceIssue).toBeUndefined()
    expect(mocks.invoke.mock.calls.find(([cmd]) => cmd === 'convert_archive')?.[1]).toMatchObject({
      outputPath: 'D:/我的资料.7z', options: { level: 3 } })
  })
  it('does not include new Explorer work arriving while the selected volume group is being read', async () => {
    let finish!: (group: unknown) => void
    mocks.invoke.mockImplementation(async (command, args) => {
      if (command === 'detect_split_archive' && args.path.includes('原包')) return new Promise(resolve => { finish = resolve })
      if (command === 'detect_split_archive') return { is_split: false }
      if (command === 'convert_archive') return { primaryPath: args.outputPath, files: [], totalBytes: 20, split: false }
    })
    const store = useArchiveConversionStore()
    store.addSources(['C:/原包.zip.002'])
    const run = store.start()
    await vi.waitFor(() => expect(finish).toBeTypeOf('function'))
    store.openFromExplorer(['C:/新收到.zip'], 'convert')
    finish({ is_split: true, first_part: 'C:/原包.zip.001', parts: ['C:/原包.zip.001', 'C:/原包.zip.002'],
      is_complete: true, missing_parts: [] })
    await vi.waitFor(() => expect(mocks.invoke.mock.calls.filter(([cmd, args]) => cmd === 'detect_split_archive' && args.path.includes('原包'))).toHaveLength(2))
    finish({ is_split: true, first_part: 'C:/原包.zip.001', parts: ['C:/原包.zip.001', 'C:/原包.zip.002'],
      is_complete: true, missing_parts: [] })
    await run
    expect(mocks.invoke.mock.calls.filter(([cmd]) => cmd === 'convert_archive')).toHaveLength(1)
    expect(store.pending.map(item => item.sourcePath)).toEqual(['C:/新收到.zip'])
  })
  it('requires both restored passwords separately and respects an explicit unencrypted output choice', async () => {
    const store = useArchiveConversionStore()
    store.addSources(['C:/加密原包.zip'])
    const item = store.items[0]
    item.passwordRequired = true
    store.updateItem(item.id, { sourcePassword: 'original-again' })
    expect(store.passwordRequirements(item)).toEqual({ source: false, target: true })
    await store.start()
    expect(item.error).toContain('新包密码')
    expect(mocks.invoke.mock.calls.some(([command]) => command === 'convert_archive')).toBe(false)
    store.updateItem(item.id, { password: 'new-again' })
    expect(store.passwordRequirements(item)).toEqual({ source: false, target: false })
    item.targetPasswordRequired = item.passwordRequired = true
    store.updateItem(item.id, { password: '', encryptFilenames: false })
    expect(store.passwordRequirements(item)).toEqual({ source: false, target: false })
  })
  it('adds history replay without overwriting an edited draft and reuses that replay on a repeated click', () => {
    const store = draft()
    const original = store.items[0]
    const originalSettings = { ...original.settings! }
    const tasks = useTaskStore()
    tasks.addTask({ id: 'previous-conversion', name: '上次转换', type: 'compression', workloadKind: 'archive',
      sourceFiles: [original.sourcePath], outputPath: store.outputFor(original),
      executionConfig: { schemaVersion: 1, kind: 'archive-conversion', outputPath: store.outputFor(original),
        format: 'zip', level: 1, splitSize: 0, encryptFilenames: false, passwordRequired: true } })
    tasks.tasks[0].status = 'failed'
    const record = createTaskHistoryRecord(tasks.tasks[0])
    store.restore(record)
    expect(store.items).toHaveLength(2)
    expect(original.settings).toEqual(originalSettings)
    const replay = store.items[1]
    expect(store.effectiveSettings(replay)).toMatchObject({ format: 'zip', level: 1, password: '' })
    store.updateItem(replay.id, { level: 4, password: 'replay-secret' })
    store.restore(record)
    expect(store.items).toHaveLength(2)
    expect(store.effectiveSettings(replay)).toMatchObject({ level: 4, password: 'replay-secret' })
  })
  it('runs selected items sequentially, keeps individual settings and continues after a failure', async () => {
    mocks.invoke.mockImplementation(async (command, args) => {
      if (command !== 'convert_archive') return
      if (args.sourcePath.endsWith('first.zip')) throw '原包损坏'
      return { primaryPath: args.outputPath, files: [], totalBytes: 123, split: false }
    })
    const store = useArchiveConversionStore()
    store.addSources(['C:/first.zip', 'C:/second.zip', 'C:/third.zip'])
    store.updateItem(store.items[1].id, { format: 'zip', level: 2, password: 'personal-secret' })
    store.items[2].selected = false
    store.applySelected({ outputDirectory: 'D:/结果' })
    await store.start()
    const calls = mocks.invoke.mock.calls.filter(([command]) => command === 'convert_archive')
    expect(calls.map(([, args]) => args.sourcePath)).toEqual(['C:/first.zip', 'C:/second.zip'])
    expect(calls[1][1]).toMatchObject({ outputPath: 'D:/结果/second_转换.zip', options: { format: 'zip', level: 2, password: 'personal-secret' } })
    expect(store.taskFor(store.items[0])?.status).toBe('failed')
    expect(store.taskFor(store.items[1])?.status).toBe('completed')
    expect(store.pending.map(item => item.sourcePath)).toEqual(['C:/third.zip'])
    expect(JSON.stringify(mocks.invoke.mock.calls.filter(([cmd]) => cmd === 'save_task_history'))).not.toContain('personal-secret')
  })
  it('stops the batch after cancellation and retains unstarted settings', async () => {
    let settle!: (error: string) => void
    mocks.invoke.mockImplementation(async command => command === 'convert_archive' ? new Promise((_, reject) => { settle = reject }) : undefined)
    const store = useArchiveConversionStore()
    store.defaults.password = 'shared-secret'
    store.addSources(['C:/first.zip', 'C:/second.zip'])
    const run = store.start()
    await vi.waitFor(() => expect(settle).toBeTypeOf('function'))
    store.openFromExplorer(['C:/new.zip'], 'convert')
    await store.cancel(); settle('转换已取消'); await run
    expect(mocks.invoke.mock.calls.filter(([cmd]) => cmd === 'convert_archive')).toHaveLength(1)
    expect(store.pending).toHaveLength(2)
    expect(store.effectiveSettings(store.pending[0]).password).toBe('shared-secret')
    expect(store.taskFor(store.items[0])?.status).toBe('cancelled')
  })
  it('shows duplicate output errors before starting any item and retains settings when switching rows', async () => {
    const store = useArchiveConversionStore()
    store.addSources(['C:/first.zip', 'D:/first.zip'])
    store.applySelected({ outputDirectory: 'E:/结果' })
    store.chooseContextSource('C:/first.zip'); store.sourcePassword = 'original'
    store.chooseContextSource('D:/first.zip'); store.chooseContextSource('C:/first.zip')
    expect(store.sourcePassword).toBe('original')
    await store.start()
    expect(store.items[1].error).toContain('重复')
    expect(useTaskStore().tasks).toHaveLength(0)
  })
  it('keeps each custom filename when changing directories and lets defaults follow actual edits', () => {
    const store = useArchiveConversionStore()
    store.openFromExplorer(['C:/a.zip', 'C:/b.7z'], 'reconfigure')
    store.updateItem(store.items[0].id, { outputPath: 'D:/自定义.zip', level: 2 })
    store.applySelected({ outputDirectory: 'E:/统一', format: '7z' })
    expect(store.outputFor(store.items[0])).toBe('E:/统一/自定义.7z')
    store.addSources(['C:/c.zip'], 'reconfigure')
    store.updateDefaults({ format: '7z', level: 9 })
    expect(store.effectiveSettings(store.items[0]).level).toBe(2)
    expect(store.effectiveSettings(store.items[2])).toMatchObject({ format: '7z', level: 9 })
  })
  it('opens Explorer sources as explicit drafts and preserves ZIP for password/split changes', () => {
    const store = useArchiveConversionStore()
    store.openFromExplorer(['C:/旧包.zip.001', 'C:/another.7z', 'C:/旧包.zip.001'], 'reconfigure')
    expect(store.visible).toBe(true)
    expect(store.sourcePath).toBe('C:/旧包.zip.001')
    expect(store.format).toBe('zip')
    expect(store.contextSources).toHaveLength(2)
    expect(store.password).toBe(''); expect(store.sourcePassword).toBe('')
    store.outputPath = 'D:/新包.zip'; store.password = 'new-secret'
    store.chooseContextSource(store.sourcePath)
    expect(store.password).toBe('new-secret'); expect(store.outputPath).toBe('D:/新包.zip')
    store.selectSource('E:/手动选择.zip')
    expect(store.sourcePath).toBe('E:/手动选择.zip')
    expect(store.contextSources).toContain('E:/手动选择.zip')
    expect(useTaskStore().tasks).toHaveLength(0)
    expect(mocks.invoke.mock.calls.some(([command]) => command === 'convert_archive')).toBe(false)
  })
  it('keeps an open or running conversion when a new Explorer request arrives', () => {
    const store = draft()
    store.openFromExplorer(['D:/new.zip'], 'reconfigure')
    expect(store.sourcePath).toBe('C:\\原包.zip')
    expect(store.password).toBe('output-secret')
    expect(store.contextSources).toEqual(['C:\\原包.zip', 'D:/new.zip'])
    store.busy = true
    store.chooseContextSource('D:/new.zip')
    expect(store.sourcePath).toBe('C:\\原包.zip')
    store.busy = false
    store.chooseContextSource('D:/new.zip')
    expect(store.sourcePath).toBe('D:/new.zip')
    expect(store.format).toBe('zip')
    expect(store.password).toBe(''); expect(store.outputPath).toBe('D:/new_调整.zip')
  })
  it('does not revive a terminal task when cancellation acknowledgement arrives during history persistence', async () => {
    let rejectConversion!: (reason: string) => void
    let acknowledgeCancel!: () => void
    let saveHistory!: () => void
    mocks.invoke.mockImplementation(command => {
      if (command === 'convert_archive') return new Promise((_, reject) => { rejectConversion = reject })
      if (command === 'cancel_compression') return new Promise<void>(resolve => { acknowledgeCancel = resolve })
      if (command === 'save_task_history') return new Promise<void>(resolve => { saveHistory = resolve })
      return Promise.resolve()
    })
    const store = draft(); const pending = store.start()
    await vi.waitFor(() => expect(rejectConversion).toBeTypeOf('function'))
    const cancellation = store.cancel()
    rejectConversion('转换已取消，原包保留')
    await vi.waitFor(() => expect(saveHistory).toBeTypeOf('function'))
    expect(store.task?.status).toBe('cancelled')
    acknowledgeCancel(); await cancellation
    expect(store.task?.status).toBe('cancelled')
    saveHistory(); await pending
  })
  it('keeps input and output passwords separate and persists only conversion settings', async () => {
    mocks.invoke.mockImplementation(async command => command === 'convert_archive'
      ? { primaryPath: 'C:\\新包.7z.001', files: [], totalBytes: 25, split: true } : undefined)
    const store = draft()
    await store.start()
    expect(mocks.invoke).toHaveBeenCalledWith('convert_archive', expect.objectContaining({ sourcePassword: 'input-secret', options: expect.objectContaining({ password: 'output-secret', verify_after: true, delete_after: false, split_size: 1048576 }) }))
    const record = mocks.invoke.mock.calls.find(([command]) => command === 'save_task_history')![1].record
    expect(record.executionConfig.kind).toBe('archive-conversion')
    expect(JSON.stringify(record)).not.toMatch(/input-secret|output-secret/)
    expect(record.status).toBe('completed')
    expect(store.password).toBe(''); expect(store.sourcePassword).toBe('')
    store.restore(record)
    expect(store.outputPath).toBe('C:/原包_转换.7z'); expect(store.sourcePath).toBe('C:\\原包.zip')
    expect(store.splitMiB).toBe(1)
  })
  it('blocks incomplete configuration without creating a task', async () => {
    const store = draft(); store.password = ''
    await store.start()
    expect(store.error).toContain('新包密码')
    expect(useTaskStore().tasks).toHaveLength(0)
    expect(mocks.invoke.mock.calls.some(([command]) => command === 'convert_archive')).toBe(false)
  })
  it('persists the real final failure and no output', async () => {
    mocks.invoke.mockImplementation(async command => { if (command === 'convert_archive') throw '转换读取原包失败：文件损坏' })
    const store = draft(); await store.start()
    const record = createTaskHistoryRecord(useTaskStore().tasks[0])
    expect(record.status).toBe('failed'); expect(record.errorMessage).toContain('文件损坏')
    expect(record.archiveOutput).toBeNull()
    expect(store.busy).toBe(false)
  })
  it('does not declare cancellation complete before the engine settles', async () => {
    let rejectConversion!: (reason: string) => void
    mocks.invoke.mockImplementation(async command => command === 'convert_archive'
      ? new Promise((_, reject) => { rejectConversion = reject }) : undefined)
    const store = draft(); const pending = store.start()
    await vi.waitFor(() => expect(rejectConversion).toBeTypeOf('function'))
    await store.cancel()
    expect(store.busy).toBe(true); expect(store.task?.status).toBe('cancelling')
    rejectConversion('转换已取消，原包保留'); await pending
    expect(store.busy).toBe(false); expect(store.task?.status).toBe('cancelled')
  })
  it('also waits for cleanup when cancelled from the global task controls', async () => {
    let rejectConversion!: (reason: string) => void
    mocks.invoke.mockImplementation(async command => command === 'convert_archive'
      ? new Promise((_, reject) => { rejectConversion = reject }) : undefined)
    const store = draft(); const pending = store.start()
    await vi.waitFor(() => expect(rejectConversion).toBeTypeOf('function'))
    await useTaskStore().cancelTask(store.task!.id)
    expect(store.task?.status).toBe('cancelling')
    expect(mocks.invoke.mock.calls.some(([command]) => command === 'save_task_history')).toBe(false)
    rejectConversion('转换已取消，原包保留'); await pending
    expect(store.task?.status).toBe('cancelled')
  })
})

it('lzma1523 offers single-file conversion and keeps splitting while clearing unsupported target passwords', async () => {
  const store=useArchiveConversionStore();store.addSources(['C:/原包.zip'])
  store.updateItem(store.items[0].id,{splitMiB:64,password:'secret'})
  store.updateItem(store.items[0].id,{format:'lzma'})
  expect(conversionFormats.some(f=>f.value==='lzma')).toBe(true)
  expect(store.items[0].settings).toMatchObject({format:'lzma',splitMiB:64,password:''})
})

it.each(['tar.lzma','lzma.aes','tar.lzma.aes'])('lzma1524 conversion retains splits and applicable target passwords: %s',format=>{
  const store=useArchiveConversionStore();store.addSources(['C:/source.zip'])
  store.updateItem(store.items[0].id,{password:'secret',sourcePassword:'old',splitMiB:64})
  store.updateItem(store.items[0].id,{format})
  expect(conversionFormats.some(f=>f.value===format)).toBe(true)
  expect(store.items[0].settings).toMatchObject({format,splitMiB:64,sourcePassword:'old',password:format==='tar.lzma'?'':'secret'})
})
