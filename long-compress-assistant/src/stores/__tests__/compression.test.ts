import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useCompressionStore, type CompressionOptions, type FileObject } from '../compression'

const defaultOptions = (): CompressionOptions => ({
  format: 'zip',
  level: 6,
  password: '',
  filename: '',
  splitArchive: false,
  splitSize: '1024',
  keepStructure: true,
  deleteAfter: false,
  verifyAfter: true,
  createSolidArchive: false
})

const mockFile = (name: string, size: number): FileObject => ({
  name,
  path: `C:/archives/${name}`,
  size,
  type: 'file',
  isDirectory: false
})

describe('Compression Store', () => {
  it('keeps a selected standard volume type when changing ZIP to ZIPX in batch', () => {
    const store = useCompressionStore(), file = mockFile('share.bin', 1700000)
    store.addFile(file)
    store.updateFileSettings(file.path, { ...defaultOptions(), splitArchive: true, zipSplitMode: 'standard' })
    store.applyBatchConfiguration([file.path], { format: 'zipx' })
    expect(store.selectedFiles[0].settings).toMatchObject({ format: 'zipx', splitArchive: true, zipSplitMode: 'standard' })
    expect(store.globalSettings.zipSplitMode).toBeUndefined()
  })
  it('only applies target encodings to ZIP/ZIPX and does not detach inherited 7Z drafts', () => {
    const store=useCompressionStore(); store.globalSettings.format='7z'
    const zip=mockFile('zip.txt',10), zipx=mockFile('zipx.txt',10), other=mockFile('other.txt',10)
    ;[zip,zipx,other].forEach(f=>store.addFile(f))
    store.updateFileSettings(zip.path,defaultOptions())
    store.updateFileSettings(zipx.path,{...defaultOptions(),format:'zipx'})
    expect(store.applyBatchConfiguration([zip.path,zipx.path,other.path],{zipPasswordEncoding:'cp950'})).toBe(2)
    expect(store.selectedFiles[0].settings?.zipPasswordEncoding).toBe('cp950')
    expect(store.selectedFiles[1].settings?.zipPasswordEncoding).toBe('cp950')
    expect(store.selectedFiles[2].settings).toBeUndefined()
    expect(store.globalSettings.zipPasswordEncoding).toBeUndefined()
    expect(()=>store.applyBatchConfiguration([other.path],{zipPasswordEncoding:'cp932'})).toThrow('没有适用')
    expect(store.applyBatchConfiguration([other.path],{format:'zip',zipPasswordEncoding:'cp932'})).toBe(1)
    expect(store.selectedFiles[2].settings).toMatchObject({format:'zip',zipPasswordEncoding:'cp932'})
  })
  it('does not detach inherited non-ZIP drafts when only selected ZIP methods change', () => {
    const store = useCompressionStore(), first = mockFile('one', 1), other = mockFile('two', 1)
    store.globalSettings.format = '7z'; store.addFile(first); store.addFile(other)
    store.updateFileSettings(first.path, defaultOptions())
    expect(store.applyBatchConfiguration([first.path, other.path], { zipMethod: 'zstd' })).toBe(1)
    expect(store.selectedFiles[0].settings?.zipMethod).toBe('zstd')
    expect(store.selectedFiles[1].settings).toBeUndefined()
    expect(() => store.applyBatchConfiguration([other.path], { zipMethod: 'zstd' })).toThrow('没有 ZIP')
  })
  it('changes methods only for selected ZIP drafts and creates ZIPX without touching defaults', () => {
    const store = useCompressionStore()
    const zip = mockFile('zip.txt', 10), zipx = mockFile('zipx.txt', 10), untouched = mockFile('untouched.txt', 10)
    ;[zip, zipx, untouched].forEach(file => store.addFile(file))
    store.updateFileSettings(zipx.path, { ...defaultOptions(), format: 'zipx', password: 'keep', splitArchive: true })
    store.applyBatchConfiguration([zip.path, zipx.path], { zipMethod: 'zstd' })
    expect(store.selectedFiles[0].settings).toMatchObject({ format: 'zip', zipMethod: 'zstd' })
    expect(store.selectedFiles[1].settings).toMatchObject({ format: 'zipx', password: 'keep', splitArchive: true })
    expect(store.selectedFiles[1].settings?.zipMethod).toBeUndefined()
    expect(store.selectedFiles[2].settings).toBeUndefined()
    expect(store.globalSettings.zipMethod).toBeUndefined()
    store.applyBatchConfiguration([zip.path], { format: 'zipx' })
    expect(store.selectedFiles[0].settings?.format).toBe('zipx')
  })
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('applies TAR.LZIP to selected file/group drafts without changing unselected or global settings', () => {
    const store = useCompressionStore()
    const first = mockFile('first.txt', 10), second = mockFile('second.txt', 20), untouched = mockFile('untouched.txt', 30)
    ;[first, second, untouched].forEach(file => store.addFile(file))
    const groupId = store.createGroup([second.path])
    store.globalSettings = { ...defaultOptions(), format: '7z', password: 'global-secret', splitArchive: true, createSolidArchive: true, encryptFilenames: true }
    expect(store.applyBatchConfiguration([first.path, groupId], { format: 'tar.lz' })).toBe(2)
    const expected = { format: 'tar.lz', password: '', splitArchive: true, createSolidArchive: false, encryptFilenames: false, rarRecoveryPercent: 0, rarRecoveryVolumes: 0 }
    expect(store.selectedFiles[0].settings).toMatchObject(expected)
    expect(store.groups[0].settings).toMatchObject(expected)
    expect(store.selectedFiles[1].settings).toBeUndefined()
    expect(store.globalSettings).toMatchObject({ format: '7z', password: 'global-secret', splitArchive: true })
  })

  it('clears saved password prompts when restored file/group drafts switch to LZIP', () => {
    const store = useCompressionStore()
    const first = mockFile('restored.txt', 10), second = mockFile('restored-group.txt', 20)
    store.addFile(first); store.addFile(second)
    const groupId = store.createGroup([second.path])
    store.selectedFiles[0].draftPasswordRequired = true
    store.groups[0].draftPasswordRequired = true
    store.updateFileSettings(first.path, { ...defaultOptions(), format: 'lz' })
    store.updateGroupSettings(groupId, { ...defaultOptions(), format: 'tar.lz' })
    expect(store.selectedFiles[0].draftPasswordRequired).toBe(false)
    expect(store.groups[0].draftPasswordRequired).toBe(false)
    store.groups[0].draftPasswordRequired = true
    store.applyBatchConfiguration([groupId], { format: 'tar.lz' })
    expect(store.groups[0].draftPasswordRequired).toBe(false)
  })

  it('initializes with current default state', () => {
    const store = useCompressionStore()

    expect(store.selectedFiles).toEqual([])
    expect(store.groups).toEqual([])
    expect(store.globalSettings).toEqual(defaultOptions())
    expect(store.globalOutputPath).toBe('')
    expect(store.totalOriginalSize).toBe(0)
  })

  it('changes only selected fields on file/group drafts and freezes their other settings', () => {
    const store = useCompressionStore()
    const first = mockFile('first.txt', 10), second = mockFile('second.txt', 20), untouched = mockFile('untouched.txt', 30)
    ;[first, second, untouched].forEach(file => store.addFile(file))
    const groupId = store.createGroup([second.path])
    store.updateFileSettings(first.path, { ...defaultOptions(), password: 'first-secret', filename: 'custom', level: 2 })
    store.updateGroupSettings(groupId, { ...defaultOptions(), password: 'group-secret', level: 4 })
    store.updateFileOutputPath(first.path, 'E:/first')
    store.globalOutputPath = 'D:/default'
    expect(store.applyBatchConfiguration([first.path, groupId], { level: 9 })).toBe(2)
    expect(store.selectedFiles[0]).toMatchObject({ outputPath: 'E:/first', settings: { level: 9, password: 'first-secret', filename: 'custom' } })
    expect(store.groups[0]).toMatchObject({ outputPath: '', settings: { level: 9, password: 'group-secret' } })
    expect(store.selectedFiles[1].settings).toBeUndefined()
    store.globalSettings.level = 1
    store.globalOutputPath = 'F:/later'
    expect(store.selectedFiles[0].settings?.level).toBe(9)
    expect(store.groups[0].settings?.level).toBe(9)
    expect(store.globalSettings.password).toBe('')
  })

  it('keeps explicit source-directory output and excludes submitted tasks and empty patches', () => {
    const store = useCompressionStore()
    const first = mockFile('first.txt', 10), submitted = mockFile('submitted.txt', 20)
    store.addFile(first); store.addFile(submitted)
    store.globalOutputPath = 'D:/default'
    store.bindJobTask(submitted.path, 'running', defaultOptions(), 'E:/running.zip')
    expect(store.applyBatchConfiguration([first.path, submitted.path], {})).toBe(0)
    expect(store.selectedFiles[0].settings).toBeUndefined()
    expect(store.applyBatchConfiguration([first.path, submitted.path], { outputDirectory: '' })).toBe(1)
    expect(store.selectedFiles[0].outputPath).toBe('')
    expect(store.selectedFiles[1].outputPath).toBe('E:/running.zip')
    store.globalOutputPath = 'F:/later'
    expect(store.getEffectiveOutputPath(store.selectedFiles[0].outputPath)).toBe('')
  })

  it('explains incompatible filename encryption before changing any selected draft', () => {
    const store = useCompressionStore()
    const first = mockFile('first.txt', 10), encrypted = mockFile('encrypted.txt', 20)
    store.addFile(first); store.addFile(encrypted)
    store.updateFileSettings(encrypted.path, { ...defaultOptions(), format: '7z', password: 'secret', encryptFilenames: true })
    expect(() => store.applyBatchConfiguration([first.path, encrypted.path], { format: 'zip', level: 9 })).toThrow('文件名加密')
    expect(store.selectedFiles[0].settings).toBeUndefined()
    expect(store.selectedFiles[1].settings).toMatchObject({ format: '7z', password: 'secret', level: 6, encryptFilenames: true })
  })

  it('adds files and ignores duplicate paths', () => {
    const store = useCompressionStore()
    const file = mockFile('sample.zip', 1024)

    expect(store.addFile(file)).toBe(true)
    expect(store.addFile(file)).toBe(false)

    expect(store.selectedFiles).toHaveLength(1)
    expect(store.selectedFiles[0]).toMatchObject({
      name: 'sample.zip',
      path: 'C:/archives/sample.zip',
      expanded: false
    })
  })

  it('calculates total size across loose files and groups', () => {
    const store = useCompressionStore()

    store.addFile(mockFile('one.txt', 100))
    store.addFile(mockFile('two.txt', 200))
    store.addFile(mockFile('three.txt', 300))

    store.createGroup(['C:/archives/one.txt', 'C:/archives/two.txt'])

    expect(store.selectedFiles.map(file => file.name)).toEqual(['three.txt'])
    expect(store.groups).toHaveLength(1)
    expect(store.totalOriginalSize).toBe(600)
  })

  it('stores per-file settings and output path', () => {
    const store = useCompressionStore()
    const options = { ...defaultOptions(), format: '7z' as const, password: 'archive-pass' }

    store.addFile(mockFile('private.7z', 512))
    store.updateFileSettings('C:/archives/private.7z', options)
    store.updateFileOutputPath('C:/archives/private.7z', 'D:/output')

    expect(store.selectedFiles[0].settings).toEqual(options)
    expect(store.selectedFiles[0].outputPath).toBe('D:/output')
    expect(store.getEffectiveSettings(store.selectedFiles[0].settings)).toEqual(options)
    expect(store.getEffectiveOutputPath(store.selectedFiles[0].outputPath)).toBe('D:/output')
  })

  it('applies persisted archive defaults once without resetting later edits', () => {
    const store = useCompressionStore()

    store.initializeArchiveDefaults('7z', 8)
    expect(store.globalSettings.format).toBe('7z')
    expect(store.globalSettings.level).toBe(8)

    store.globalSettings.level = 3
    store.initializeArchiveDefaults('zip', 6)
    expect(store.globalSettings.format).toBe('7z')
    expect(store.globalSettings.level).toBe(3)
  })

  it('freezes group defaults on edit and preserves an explicit same-directory destination', () => {
    const store = useCompressionStore()
    store.addFile(mockFile('grouped.txt', 100))
    const id = store.createGroup(['C:/archives/grouped.txt'])
    store.globalOutputPath = 'D:/default'
    const level = store.globalSettings.level
    store.updateGroupOutputPath(id, '')
    store.globalOutputPath = 'E:/changed'
    store.globalSettings.level = 1
    expect(store.getEffectiveOutputPath(store.groups[0].outputPath)).toBe('')
    expect(store.getEffectiveSettings(store.groups[0].settings).level).toBe(level)
  })

  it('switches files and groups explicitly between global and individual settings', () => {
    const store = useCompressionStore()
    const individual = { ...defaultOptions(), format: '7z' as const, level: 9 }
    store.addFile(mockFile('loose.txt', 100))
    store.updateFileSettings('C:/archives/loose.txt', individual)
    expect(store.getEffectiveSettings(store.selectedFiles[0].settings)).toEqual(individual)

    store.useGlobalFileSettings('C:/archives/loose.txt')
    expect(store.selectedFiles[0].settings).toBeUndefined()
    expect(store.getEffectiveSettings(store.selectedFiles[0].settings)).toEqual(store.globalSettings)

    const groupId = store.createGroup(['C:/archives/loose.txt'])
    store.updateGroupSettings(groupId, individual)
    expect(store.groups[0].settings).toEqual(individual)
    store.useGlobalGroupSettings(groupId)
    expect(store.groups[0].settings).toBeUndefined()
    expect(store.getEffectiveSettings(store.groups[0].settings)).toEqual(store.globalSettings)
  })

  it('creates, updates, dissolves, and removes groups', () => {
    const store = useCompressionStore()
    const groupOptions = { ...defaultOptions(), format: 'tar.gz' as const, level: 9 }

    store.addFile(mockFile('a.txt', 100))
    store.addFile(mockFile('b.txt', 200))

    const groupId = store.createGroup(['C:/archives/a.txt', 'C:/archives/b.txt'])
    store.updateGroupSettings(groupId, groupOptions)
    store.updateGroupOutputPath(groupId, 'D:/group-output')

    expect(store.selectedFiles).toHaveLength(0)
    expect(store.groups[0]).toMatchObject({
      id: groupId,
      files: expect.any(Array),
      settings: groupOptions,
      outputPath: 'D:/group-output'
    })

    store.removeFileFromGroup(groupId, 'C:/archives/a.txt')
    expect(store.groups[0].files.map(file => file.name)).toEqual(['b.txt'])

    store.dissolveGroup(groupId)
    expect(store.groups).toHaveLength(0)
    expect(store.selectedFiles.map(file => file.name)).toEqual(['b.txt'])
  })

  it('does not add a loose duplicate of a file that is already grouped', () => {
    const store = useCompressionStore()
    const file = mockFile('grouped-once.txt', 100)

    store.addFile(file)
    store.createGroup([file.path])

    expect(store.addFile(file)).toBe(false)
    expect(store.selectedFiles).toHaveLength(0)
    expect(store.groups[0].files).toHaveLength(1)
  })

  it('falls back to global settings and output path', () => {
    const store = useCompressionStore()
    store.globalOutputPath = 'D:/global-output'
    store.globalSettings = { ...defaultOptions(), format: 'xz', level: 3 }

    expect(store.getEffectiveSettings()).toEqual(store.globalSettings)
    expect(store.getEffectiveOutputPath()).toBe('D:/global-output')
  })

  it('creates a grouped template draft without inheriting auto start or duplicate sources', () => {
    const store = useCompressionStore()
    const existing = mockFile('existing.log', 100)
    const fresh = mockFile('fresh.log', 200)
    store.addFile(existing)
    store.requestAutoStart()

    const result = store.addTemplateDraft(
      [existing, fresh, fresh],
      '日志归档',
      { ...defaultOptions(), format: '7z', filename: '日志归档-2026-08-10' },
    )

    expect(result).toEqual(expect.objectContaining({ addedCount: 1, skippedCount: 2 }))
    expect(store.groups).toHaveLength(1)
    expect(store.groups[0]).toMatchObject({
      name: '日志归档',
      files: [{ name: 'fresh.log' }],
      settings: { format: '7z', filename: '日志归档-2026-08-10' },
    })
    expect(store.autoStartRequested).toBe(false)
  })

  it('publishes completed estimates and invalidates them when job sources change', () => {
    const store = useCompressionStore()
    const file = mockFile('analysis.txt', 8 * 1024 * 1024)
    store.addFile(file)
    store.setAnalysisState(file.path, {
      status: 'completed',
      result: {
        totalSize: file.size,
        fileCount: 1,
        sampledFiles: 1,
        sampledBytes: 64 * 1024,
        estimatedSize: 2 * 1024 * 1024,
        estimatedRatio: 0.25,
        estimatedSecondsLow: 1,
        estimatedSecondsHigh: 3,
        confidence: 'medium',
        recommendedFormat: '7z',
        recommendedLevel: 7,
        recommendedSolid: false,
        lowValueBytes: 0,
        lowValueFileCount: 0,
        reasons: ['text compresses well'],
      },
      format: 'zip',
      level: 6,
    })

    expect(store.estimatedSize[file.path]).toBe(2 * 1024 * 1024)
    store.recordActualSize(file.path, 2_500_000)
    expect(store.compressionAnalysis[file.path].actualSize).toBe(2_500_000)
    expect(store.compressionAnalysis[file.path].predictionErrorPercent).toBe(16)

    store.createGroup([file.path])
    expect(store.estimatedSize[file.path]).toBeUndefined()

    const groupId = store.groups[0].id
    store.setAnalysisState(groupId, { status: 'running', analysisId: 'analysis-1' })
    store.prepareQuickPacks()
    expect(store.compressionAnalysis).toEqual({})
  })

  it('binds submitted files and groups to immutable task snapshots and removes them together', () => {
    const store = useCompressionStore()
    const submittedOptions = { ...defaultOptions(), format: '7z' as const, level: 8 }

    store.addFile(mockFile('loose.txt', 100))
    store.addFile(mockFile('grouped.txt', 200))
    const groupId = store.createGroup(['C:/archives/grouped.txt'])

    store.bindJobTask('C:/archives/loose.txt', 'file-task', submittedOptions, 'D:/loose.7z')
    store.bindJobTask(groupId, 'group-task', submittedOptions, 'D:/group.7z')
    submittedOptions.level = 1

    expect(store.selectedFiles[0]).toMatchObject({
      taskId: 'file-task',
      outputPath: 'D:/loose.7z',
      settings: { format: '7z', level: 8 },
    })
    expect(store.groups[0]).toMatchObject({
      taskId: 'group-task',
      outputPath: 'D:/group.7z',
      settings: { format: '7z', level: 8 },
    })

    store.removeJobsByTaskIds(['file-task'])
    expect(store.selectedFiles).toHaveLength(0)
    expect(store.groups).toHaveLength(1)

    store.removeJobsByTaskIds(['group-task'])
    expect(store.groups).toHaveLength(0)
  })
})
