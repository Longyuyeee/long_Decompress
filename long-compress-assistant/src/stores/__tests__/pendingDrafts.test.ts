import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useCompressionStore, type FileObject } from '../compression'
import { useTaskStore } from '../task'
import { useArchiveConversionStore } from '../archiveConversion'
import { PENDING_DRAFTS_KEY, usePendingDraftStore } from '../pendingDrafts'
import { useDecompressionPreferences } from '../decompressionPreferences'
const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => vi.fn()) }))
let session: ReturnType<typeof usePendingDraftStore> | undefined
beforeEach(() => { localStorage.clear(); mocks.invoke.mockReset(); mocks.invoke.mockResolvedValue({ name: 'file', size: 20, is_dir: false }); setActivePinia(createPinia()) })
afterEach(() => { session?.dispose(); session = undefined })
function file(path: string) { return { path, name: path.split('/').pop()!, size: 20, type: 'file', isDirectory: false } }
function reopen() { session?.dispose(); setActivePinia(createPinia()); session = usePendingDraftStore(); session.initialize(); return session }
describe('pending archive draft recovery', () => {
  it('zpaq1537 recovers guarded selections without passwords or merging different ranges of one snapshot',async()=>{
    session=usePendingDraftStore();session.initialize();const tasks=useTaskStore(),snapshot={version:1,date:20261020130000}
    for(const path of ['old.txt','other.txt'])tasks.addTask({id:path,name:path,type:'decompression',sourceFiles:['C:/old.zpaq'],outputPath:'D:/out',zpaqSnapshot:snapshot,zpaqCatalogId:'a'.repeat(64),selectedEntries:[path],password:'secret-1537',configurationMode:'individual'})
    session.flush();expect(localStorage.getItem(PENDING_DRAFTS_KEY)).not.toContain('secret-1537');await reopen().restore();expect(useTaskStore().tasks).toHaveLength(2);expect(useTaskStore().tasks.map(t=>t.selectedEntries)).toEqual([['old.txt'],['other.txt']]);expect(useTaskStore().tasks.every(t=>t.zpaqCatalogId==='a'.repeat(64)&&t.draftPasswordRequired&&t.status==='pending')).toBe(true);expect(mocks.invoke.mock.calls.some(([cmd])=>cmd==='extract_file')).toBe(false)
  })
  it('zpaq1536 restores update mapping alongside conversion, excludes passwords and flags missing additional sources without starting',async()=>{
    mocks.invoke.mockImplementation(async(cmd,args)=>{if(cmd==='detect_split_archive')return{is_split:false,archive_format:'zpaq'};if(cmd==='get_file_info'&&args.path==='C:/missing.txt')throw Error('missing');return{name:'file',size:20,is_dir:false}})
    session=usePendingDraftStore();session.initialize();const store=useArchiveConversionStore()
    store.showUpdate(['C:/old.zpaq']);store.updateItem(store.pending[0].id,{updateSources:['C:/missing.txt'],archivePrefix:'资料',sourcePassword:'native-secret-1536',splitMiB:1});store.show();store.addSources(['C:/old.zpaq']);session.flush()
    expect(localStorage.getItem(PENDING_DRAFTS_KEY)).not.toContain('native-secret-1536')
    await reopen().restore();const restored=useArchiveConversionStore();expect(restored.pending).toHaveLength(2)
    const update=restored.pending.find(i=>i.intent==='zpaq-update')!;expect(restored.effectiveSettings(update)).toMatchObject({format:'zpaq',updateSources:['C:/missing.txt'],archivePrefix:'资料',splitMiB:1,sourcePassword:'',password:''});expect(update.sourceIssue).toContain('missing.txt');expect(restored.passwordRequirements(update)).toEqual({source:true,target:false})
    expect(mocks.invoke.mock.calls.some(([cmd])=>['convert_archive','update_zpaq_archive'].includes(cmd))).toBe(false)
  })
  it('zpaq1530 restores two snapshots of one archive as separate drafts without starting them', async () => {
    session=usePendingDraftStore();session.initialize()
    const one={version:1,date:20261020130000},two={version:2,date:20261020140000}
    for(const snapshot of [one,two])useTaskStore().addTask({id:'snapshot-'+snapshot.version,name:'old.zpaq',type:'decompression',sourceFiles:['C:/old.zpaq'],outputPath:'D:/out',zpaqSnapshot:snapshot})
    const conversion=useArchiveConversionStore();conversion.openSnapshot('C:/old.zpaq',one,'secret');conversion.openSnapshot('C:/old.zpaq',two,'secret');session.flush()
    await reopen().restore()
    expect(useTaskStore().tasks.map(t=>t.zpaqSnapshot)).toEqual([one,two])
    const reopened=useArchiveConversionStore();expect(reopened.pending.map(item=>reopened.effectiveSettings(item).sourceZpaqSnapshot)).toEqual([one,two])
    expect(mocks.invoke.mock.calls.some(([command])=>['extract_file','convert_archive'].includes(command))).toBe(false)
  })
  it('keeps extraction, compression and separate conversion encodings after reopening without secrets', async () => {
    session = usePendingDraftStore(); session.initialize()
    const compression = useCompressionStore(), tasks = useTaskStore(), conversion = useArchiveConversionStore()
    compression.globalSettings.zipPasswordEncoding = 'cp950'
    compression.addFile(file('C:/source.txt'))
    tasks.addTask({ id: 'encoded-input', name: 'old.zip', type: 'decompression', sourceFiles: ['C:/old.zip'],
      outputPath: 'D:/out', zipPasswordEncoding: 'cp932', password: 'input-secret' })
    conversion.addSources(['C:/convert.zip'])
    conversion.updateItem(conversion.pending[0].id, { sourceZipPasswordEncoding: 'cp936', zipPasswordEncoding: 'cp950',
      sourcePassword: 'source-secret', password: 'target-secret' })
    session.flush()
    const saved = JSON.parse(localStorage.getItem(PENDING_DRAFTS_KEY)!)
    expect(saved.extraction.items[0].zipPasswordEncoding).toBe('cp932')
    expect(saved.compression.defaults.zipPasswordEncoding).toBe('cp950')
    expect(saved.conversion.items[0].settings).toMatchObject({ sourceZipPasswordEncoding: 'cp936', zipPasswordEncoding: 'cp950' })
    expect(JSON.stringify(saved)).not.toMatch(/input-secret|source-secret|target-secret/)
    await reopen().restore()
    expect(useTaskStore().tasks[0].zipPasswordEncoding).toBe('cp932')
    expect(useCompressionStore().globalSettings.zipPasswordEncoding).toBe('cp950')
    expect(useArchiveConversionStore().effectiveSettings(useArchiveConversionStore().pending[0])).toMatchObject({ sourceZipPasswordEncoding: 'cp936', zipPasswordEncoding: 'cp950', sourcePassword: '', password: '' })
    expect(mocks.invoke.mock.calls.some(([command]) => ['extract_file', 'compress_files', 'convert_archive'].includes(command))).toBe(false)
  })
  it('retains the new package password requirement after only the source password was reentered', async () => {
    session = usePendingDraftStore(); session.initialize()
    const conversion = useArchiveConversionStore()
    conversion.addSources(['C:/encrypted.zip'])
    conversion.sourcePassword = 'source-secret'; conversion.password = 'target-secret'
    session.flush()
    const restored = reopen(); await restored.restore()
    const recoveredConversion = useArchiveConversionStore(), item = recoveredConversion.pending[0]
    expect(recoveredConversion.passwordRequirements(item)).toEqual({ source: true, target: true })
    recoveredConversion.updateItem(item.id, { sourcePassword: 'source-again' })
    restored.flush()
    const stored = JSON.parse(localStorage.getItem(PENDING_DRAFTS_KEY)!).conversion.items[0]
    expect(stored).toMatchObject({ sourcePasswordRequired: true, targetPasswordRequired: true, passwordRequired: true })
    expect(stored.settings).toMatchObject({ sourcePassword: '', password: '' })
    expect(recoveredConversion.passwordRequirements(item)).toEqual({ source: false, target: true })
    await recoveredConversion.start()
    expect(item.error).toContain('新包密码')
  })
  it('recovers files, groups, extraction and conversion without passwords or automatic execution', async () => {
    session = usePendingDraftStore(); session.initialize()
    const compression = useCompressionStore(), tasks = useTaskStore(), conversion = useArchiveConversionStore()
    compression.globalSettings.level = 3; compression.globalOutputPath = 'D:/默认'
    compression.globalSettings.password = 'global-secret'
    compression.globalSettings.zipSplitMode = 'standard'
    compression.globalSettings.zipMethod = 'zstd'
    compression.addFile(file('C:/first.txt')); compression.addFile(file('C:/group.txt'))
    const group = compression.createGroup(['C:/group.txt'])
    compression.updateGroupSettings(group, { ...compression.globalSettings, format: '7z', password: 'group-secret' })
    tasks.addTask({ id: 'old-extract', name: '解压.zip', type: 'decompression', sourceFiles: ['C:/解压.zip'], outputPath: 'E:/结果', password: 'extract-secret', extractToSubfolder: true, configurationMode: 'individual' })
    conversion.openFromExplorer(['C:/convert.zip'], 'convert'); conversion.password = 'new-secret'; conversion.sourcePassword = 'old-secret'
    session.flush()
    expect(localStorage.getItem(PENDING_DRAFTS_KEY)).not.toMatch(/global-secret|group-secret|extract-secret|new-secret|old-secret/)
    const restored = reopen()
    expect(restored.count).toBe(4)
    expect(useCompressionStore().selectedFiles).toHaveLength(0)
    await restored.restore()
    expect(useCompressionStore().selectedFiles[0]).toMatchObject({ path: 'C:/first.txt', draftPasswordRequired: true })
    expect(useCompressionStore().groups[0].settings).toMatchObject({ format: '7z', password: '' })
    useCompressionStore().initializeArchiveDefaults('zip', 9, 'X:/其他')
    expect(useCompressionStore().globalSettings.level).toBe(3)
    expect(useCompressionStore().globalSettings.zipSplitMode).toBe('standard')
    expect(useCompressionStore().globalSettings.zipMethod).toBe('zstd')
    expect(useTaskStore().tasks[0]).toMatchObject({ status: 'pending', outputPath: 'E:/结果', draftPasswordRequired: true })
    expect(useArchiveConversionStore().pending[0].passwordRequired).toBe(true)
    expect(useCompressionStore().autoStartRequested).toBe(false)
    expect(mocks.invoke.mock.calls.every(([cmd]) => cmd !== 'convert_archive' && cmd !== 'compress_files' && cmd !== 'extract_archive')).toBe(true)
  })
  it('saves only unstarted drafts and marks missing sources in the recovered item', async () => {
    session = usePendingDraftStore(); session.initialize()
    const compression = useCompressionStore(), tasks = useTaskStore(), conversion = useArchiveConversionStore()
    compression.addFile(file('C:/missing.txt')); compression.addFile({ ...file('C:/started.txt'), taskId: 'running' })
    tasks.addTask({ id: 'running-extract', name: 'x', type: 'decompression', sourceFiles: ['C:/x.zip'], outputPath: 'D:/' }); tasks.tasks[0].status = 'running'
    conversion.addSources(['C:/finished.zip']); conversion.items[0].taskId = 'completed'
    session.flush()
    const restored = reopen(); expect(restored.count).toBe(1)
    mocks.invoke.mockRejectedValue('找不到来源')
    await restored.restore()
    expect(useCompressionStore().selectedFiles[0].sourceIssue).toContain('不存在')
    expect(useCompressionStore().selectedFiles[0].taskId).toBeUndefined()
    expect(useTaskStore().tasks).toHaveLength(0)
  })
  it('keeps unopened old work while saving new work and discard only drops the recovery offer', async () => {
    session = usePendingDraftStore(); session.initialize()
    useCompressionStore().addFile(file('C:/old.txt')); session.flush()
    const current = reopen()
    useCompressionStore().addFile(file('C:/new.txt')); current.flush()
    expect(JSON.parse(localStorage.getItem(PENDING_DRAFTS_KEY)!).compression.files).toHaveLength(2)
    current.discard()
    expect(JSON.parse(localStorage.getItem(PENDING_DRAFTS_KEY)!).compression.files.map((item: FileObject) => item.path)).toEqual(['C:/new.txt'])
  })
  it('preserves extraction defaults and avoids adding duplicate sources on recovery', async () => {
    session = usePendingDraftStore(); session.initialize()
    Object.assign(useDecompressionPreferences(), { outputPath: 'D:/我的目录', sameDirectory: false, extractToSubfolder: false })
    useTaskStore().addTask({ id: 'old', name: 'x.zip', type: 'decompression', sourceFiles: ['C:/x.zip'], outputPath: 'D:/我的目录', configurationMode: 'global' })
    session.flush(); const restored = reopen(); await restored.restore()
    expect(useDecompressionPreferences().outputPath).toBe('D:/我的目录')
    expect(useTaskStore().tasks[0].configurationMode).toBe('global')
    restored.flush(); expect(restored.count).toBe(0)
  })
})

it('wim1531 reopens mixed stored/XPRESS drafts without silently changing the method or starting',async()=>{
  session=usePendingDraftStore();session.initialize();const compression=useCompressionStore(),conversion=useArchiveConversionStore()
  compression.globalSettings.format='wim';compression.globalSettings.wimMethod='xpress';compression.addFile(file('C:/folder.txt'));compression.addFile(file('C:/other.txt'));compression.updateFileSettings('C:/other.txt',{...compression.globalSettings,wimMethod:'store'})
  conversion.addSources(['C:/archive.zip']);conversion.updateItem(conversion.pending[0].id,{format:'wim',wimMethod:'xpress',sourcePassword:'never-persist'});session.flush()
  expect(localStorage.getItem(PENDING_DRAFTS_KEY)).not.toContain('never-persist');await reopen().restore()
  const restored=useCompressionStore();expect(restored.getEffectiveSettings(restored.selectedFiles[0].settings).wimMethod).toBe('xpress');expect(restored.selectedFiles[1].settings?.wimMethod).toBe('store')
  const converted=useArchiveConversionStore();expect(converted.effectiveSettings(converted.pending[0]).wimMethod).toBe('xpress');expect(mocks.invoke.mock.calls.some(([cmd])=>['compress_files','convert_archive'].includes(cmd))).toBe(false)
})

it('wim1532 reopens mixed stored/LZX drafts without silently changing the method or starting',async()=>{
  session=usePendingDraftStore();session.initialize();const compression=useCompressionStore(),conversion=useArchiveConversionStore()
  compression.globalSettings.format='wim';compression.globalSettings.wimMethod='lzx';compression.addFile(file('C:/folder.txt'));compression.addFile(file('C:/other.txt'));compression.updateFileSettings('C:/other.txt',{...compression.globalSettings,wimMethod:'store'})
  conversion.addSources(['C:/archive.zip']);conversion.updateItem(conversion.pending[0].id,{format:'wim',wimMethod:'lzx',sourcePassword:'never-persist'});session.flush()
  expect(localStorage.getItem(PENDING_DRAFTS_KEY)).not.toContain('never-persist');await reopen().restore()
  const restored=useCompressionStore();expect(restored.getEffectiveSettings(restored.selectedFiles[0].settings).wimMethod).toBe('lzx');expect(restored.selectedFiles[1].settings?.wimMethod).toBe('store')
  const converted=useArchiveConversionStore();expect(converted.effectiveSettings(converted.pending[0]).wimMethod).toBe('lzx');expect(mocks.invoke.mock.calls.some(([cmd])=>['compress_files','convert_archive'].includes(cmd))).toBe(false)
})

it('wim1533 reopens old Store, XPRESS, LZX and new LZMS without changing methods or starting',async()=>{
  session=usePendingDraftStore();session.initialize();const compression=useCompressionStore(),conversion=useArchiveConversionStore()
  compression.globalSettings.format='wim';compression.globalSettings.wimMethod='lzms'
  const names=['C:/new.txt','C:/old-store.txt','C:/old-xpress.txt','C:/old-lzx.txt'];names.forEach(name=>compression.addFile(file(name)))
  compression.updateFileSettings(names[1],{...compression.globalSettings,wimMethod:undefined})
  compression.updateFileSettings(names[2],{...compression.globalSettings,wimMethod:'xpress'})
  compression.updateFileSettings(names[3],{...compression.globalSettings,wimMethod:'lzx'})
  conversion.addSources(['C:/archive.zip']);conversion.updateItem(conversion.pending[0].id,{format:'wim',wimMethod:'lzms',sourcePassword:'never-persist'});session.flush()
  expect(localStorage.getItem(PENDING_DRAFTS_KEY)).not.toContain('never-persist');await reopen().restore()
  const restored=useCompressionStore();expect(restored.selectedFiles.map(item=>restored.getEffectiveSettings(item.settings).wimMethod??'store')).toEqual(['lzms','store','xpress','lzx'])
  expect(useArchiveConversionStore().effectiveSettings(useArchiveConversionStore().pending[0]).wimMethod).toBe('lzms');expect(mocks.invoke.mock.calls.some(([cmd])=>['compress_files','convert_archive'].includes(cmd))).toBe(false)
})

it('zip1535 restores old Deflate, Zstandard, new PPMd and ZIPX without automatic execution',async()=>{
  session=usePendingDraftStore();session.initialize();const compression=useCompressionStore(),conversion=useArchiveConversionStore();compression.globalSettings.format='zip';compression.globalSettings.zipMethod='ppmd'
  const names=['C:/new.txt','C:/old.txt','C:/zstd.txt','C:/zipx.txt'];names.forEach(n=>compression.addFile(file(n)))
  compression.updateFileSettings(names[1],{...compression.globalSettings,zipMethod:undefined});compression.updateFileSettings(names[2],{...compression.globalSettings,zipMethod:'zstd'});compression.updateFileSettings(names[3],{...compression.globalSettings,format:'zipx',zipMethod:undefined})
  conversion.addSources(['C:/source.7z']);conversion.updateItem(conversion.pending[0].id,{format:'zip',zipMethod:'ppmd',sourcePassword:'never-persist'});session.flush();expect(localStorage.getItem(PENDING_DRAFTS_KEY)).not.toContain('never-persist');await reopen().restore()
  const restored=useCompressionStore();expect(restored.selectedFiles.map(f=>restored.getEffectiveSettings(f.settings).zipMethod??'deflate')).toEqual(['ppmd','deflate','zstd','deflate']);expect(restored.selectedFiles[3].settings?.format).toBe('zipx');expect(useArchiveConversionStore().effectiveSettings(useArchiveConversionStore().pending[0]).zipMethod).toBe('ppmd');expect(mocks.invoke.mock.calls.some(([cmd])=>['compress_files','convert_archive'].includes(cmd))).toBe(false)
})

it('zip1534 restores old Deflate, Zstandard, new BZip2 and ZIPX without automatic execution',async()=>{
  session=usePendingDraftStore();session.initialize();const compression=useCompressionStore(),conversion=useArchiveConversionStore();compression.globalSettings.format='zip';compression.globalSettings.zipMethod='bzip2'
  const names=['C:/new.txt','C:/old.txt','C:/zstd.txt','C:/zipx.txt'];names.forEach(n=>compression.addFile(file(n)))
  compression.updateFileSettings(names[1],{...compression.globalSettings,zipMethod:undefined});compression.updateFileSettings(names[2],{...compression.globalSettings,zipMethod:'zstd'});compression.updateFileSettings(names[3],{...compression.globalSettings,format:'zipx',zipMethod:undefined})
  conversion.addSources(['C:/source.7z']);conversion.updateItem(conversion.pending[0].id,{format:'zip',zipMethod:'bzip2',sourcePassword:'never-persist'});session.flush();expect(localStorage.getItem(PENDING_DRAFTS_KEY)).not.toContain('never-persist');await reopen().restore()
  const restored=useCompressionStore();expect(restored.selectedFiles.map(f=>restored.getEffectiveSettings(f.settings).zipMethod??'deflate')).toEqual(['bzip2','deflate','zstd','deflate']);expect(restored.selectedFiles[3].settings?.format).toBe('zipx');expect(useArchiveConversionStore().effectiveSettings(useArchiveConversionStore().pending[0]).zipMethod).toBe('bzip2');expect(mocks.invoke.mock.calls.some(([cmd])=>['compress_files','convert_archive'].includes(cmd))).toBe(false)
})

it('zip1538 restores old Deflate, Zstandard, new LZMA and ZIPX without automatic execution',async()=>{
  session=usePendingDraftStore();session.initialize();const compression=useCompressionStore(),conversion=useArchiveConversionStore();compression.globalSettings.format='zip';compression.globalSettings.zipMethod='lzma'
  const names=['C:/new.txt','C:/old.txt','C:/zstd.txt','C:/zipx.txt'];names.forEach(n=>compression.addFile(file(n)))
  compression.updateFileSettings(names[1],{...compression.globalSettings,zipMethod:undefined});compression.updateFileSettings(names[2],{...compression.globalSettings,zipMethod:'zstd'});compression.updateFileSettings(names[3],{...compression.globalSettings,format:'zipx',zipMethod:undefined})
  conversion.addSources(['C:/source.7z']);conversion.updateItem(conversion.pending[0].id,{format:'zip',zipMethod:'lzma',sourcePassword:'never-persist'});session.flush();expect(localStorage.getItem(PENDING_DRAFTS_KEY)).not.toContain('never-persist');await reopen().restore()
  const restored=useCompressionStore();expect(restored.selectedFiles.map(f=>restored.getEffectiveSettings(f.settings).zipMethod??'deflate')).toEqual(['lzma','deflate','zstd','deflate']);expect(restored.selectedFiles[3].settings?.format).toBe('zipx');expect(useArchiveConversionStore().effectiveSettings(useArchiveConversionStore().pending[0]).zipMethod).toBe('lzma');expect(mocks.invoke.mock.calls.some(([cmd])=>['compress_files','convert_archive'].includes(cmd))).toBe(false)
})
