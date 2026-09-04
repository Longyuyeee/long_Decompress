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
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('initializes with current default state', () => {
    const store = useCompressionStore()

    expect(store.selectedFiles).toEqual([])
    expect(store.groups).toEqual([])
    expect(store.globalSettings).toEqual(defaultOptions())
    expect(store.globalOutputPath).toBe('')
    expect(store.totalOriginalSize).toBe(0)
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
