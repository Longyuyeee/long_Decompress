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
