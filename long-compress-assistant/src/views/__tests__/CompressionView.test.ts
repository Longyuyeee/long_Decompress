import { beforeEach, describe, expect, it, vi } from 'vitest'
import { statSync } from 'node:fs'
import { resolve } from 'node:path'
import { defineComponent, nextTick } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import CompressionView from '../CompressionView.vue'
import SpecialCompressionView from '../SpecialCompressionView.vue'
import { useAppStore } from '@/stores/app'
import { useCompressionStore } from '@/stores/compression'
import { useTaskStore } from '@/stores/task'

const mocks = vi.hoisted(() => ({
  compressFiles: vi.fn(),
  preflightOperationResources: vi.fn(),
  checkRarCompressionSupport: vi.fn(),
  openRarDownloadPage: vi.fn(),
  installWinRarWithWinget: vi.fn(),
  getFileInfo: vi.fn(),
  planImageCompressionDestination: vi.fn(),
  compressImageFile: vi.fn(),
  planVideoCompression: vi.fn(),
  planVideoCompressionDestination: vi.fn(),
  compressVideoFile: vi.fn(),
  openVideoOutputWithDefaultApplication: vi.fn(),
  analyzePdfInput: vi.fn(),
  planPdfOptimizationDestination: vi.fn(),
  compressPdfFile: vi.fn(),
  openPdfOutputWithDefaultApplication: vi.fn(),
  invoke: vi.fn(),
  ask: vi.fn(),
}))

vi.mock('@tauri-apps/api/tauri', () => ({
  invoke: mocks.invoke,
  convertFileSrc: (path: string) => `asset://localhost/${encodeURIComponent(path)}`,
}))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => vi.fn()) }))
vi.mock('@tauri-apps/api/dialog', () => ({ ask: mocks.ask, open: vi.fn() }))
vi.mock('@/composables/useTauriCommands', () => ({
  useTauriCommands: () => ({
    compressFiles: mocks.compressFiles,
    preflightOperationResources: mocks.preflightOperationResources,
    checkRarCompressionSupport: mocks.checkRarCompressionSupport,
    openRarDownloadPage: mocks.openRarDownloadPage,
    installWinRarWithWinget: mocks.installWinRarWithWinget,
    getFileInfo: mocks.getFileInfo,
    planImageCompressionDestination: mocks.planImageCompressionDestination,
    compressImageFile: mocks.compressImageFile,
    planVideoCompression: mocks.planVideoCompression,
    planVideoCompressionDestination: mocks.planVideoCompressionDestination,
    compressVideoFile: mocks.compressVideoFile,
    openVideoOutputWithDefaultApplication: mocks.openVideoOutputWithDefaultApplication,
    analyzePdfInput: mocks.analyzePdfInput,
    planPdfOptimizationDestination: mocks.planPdfOptimizationDestination,
    compressPdfFile: mocks.compressPdfFile,
    openPdfOutputWithDefaultApplication: mocks.openPdfOutputWithDefaultApplication,
  }),
}))

const DropzoneStub = defineComponent({
  name: 'EnhancedFileDropzone',
  emits: ['files-selected'],
  template: '<button class="dropzone-stub" type="button">add source</button>',
})

const mountView = (pinia = createPinia()) => mount(CompressionView, {
  global: {
    plugins: [pinia],
    stubs: {
      EnhancedFileDropzone: DropzoneStub,
      CompressionSettingsPanel: true,
      GlobalSettingsModal: true,
      Transition: false,
      Teleport: true,
    },
  },
})

const mountSpecialView = (pinia = createPinia()) => mount(SpecialCompressionView, {
  global: {
    plugins: [pinia],
    stubs: {
      EnhancedFileDropzone: DropzoneStub,
      Transition: false,
      Teleport: true,
    },
  },
})

const source = (path = 'C:/input/sample.txt') => ({
  name: 'sample.txt',
  path,
  size: 12,
  type: 'text/plain',
  isDirectory: false,
})

const videoPlan = (preset: 'clear' | 'balanced' | 'small' = 'balanced') => ({
  probe: {
    source: 'C:/input/rotated.mp4',
    inputBytes: 22_769,
    container: 'mov,mp4,m4a,3gp,3g2,mj2',
    durationMs: 1_000,
    overallBitRate: 182_152,
    primaryVideo: {
      index: 0,
      codec: 'h264',
      profile: 'Main',
      encodedWidth: 640,
      encodedHeight: 360,
      visibleWidth: 360,
      visibleHeight: 640,
      rotationDegrees: 90,
      pixelFormat: 'yuv420p',
      colorTransfer: null,
      hdr: false,
      nominalFrameRate: '25/6',
      averageFrameRate: '25/8',
      averageFrameRateMilli: 3_125,
      frameRateMode: 'variable',
      bitRate: 80_000,
      default: true,
    },
    videoStreamCount: 1,
    audioStreams: [{ index: 1, codec: 'aac', channels: 1, sampleRate: 48_000, bitRate: 69_000, language: null, default: true }],
    subtitleStreams: [{ index: 2, codec: 'mov_text', language: null, default: true, forced: false }],
    chapterCount: 0,
    attachedPictureCount: 0,
    policy: {
      container: 'output-mp4', video: 'transcode-h264-mf-software', audio: 'preserve-primary-as-aac-when-present',
      additionalAudio: 'drop-with-explicit-warning', subtitles: 'drop-with-explicit-warning', chapters: 'drop-with-explicit-warning',
      attachedPictures: 'drop-with-explicit-warning', rotation: 'normalize-to-visible-pixel-orientation',
      variableFrameRate: 'preserve-input-timestamps', hdr: 'refuse-before-encoding',
    },
    warnings: ['VIDEO_PROBE_SUBTITLES_WILL_BE_DROPPED: explicit confirmation is required before encoding'],
    blockingReasons: [],
  },
  preset: {
    preset, label: preset, videoBitsPerPixelMilli: 75, minimumVideoBitRate: 800_000,
    maximumVideoBitRate: 8_000_000, audioBitRate: 128_000, defaultMaxWidth: 720, defaultMaxHeight: 1_280,
  },
  effectiveMaxWidth: 720,
  effectiveMaxHeight: 1_280,
  outputWidth: 360,
  outputHeight: 640,
  willResize: false,
  willUpscale: false,
  aspectRatioPolicy: 'preserve-within-even-dimension-rounding',
  targetVideoBitRate: 800_000,
  targetAudioBitRate: 128_000,
  estimatedOutput: {
    isEstimate: true,
    lowBytes: 92_800,
    highBytes: 145_000,
    basis: 'duration-output-pixels-average-frame-rate-and-preset-bitrate-envelope',
    disclaimer: 'estimate-only; source complexity, VFR timing and encoder behavior can change the final size',
  },
  streamChanges: [
    'VIDEO_PLAN_ROTATION_NORMALIZED: 90 degree metadata will be applied to visible pixels',
    'VIDEO_PROBE_SUBTITLES_WILL_BE_DROPPED: explicit confirmation is required before encoding',
  ],
  requiresExplicitConfirmation: true,
  canEncode: true,
})

const pdfReport = (overrides: Record<string, unknown> = {}) => ({
  source: 'C:/input/form.pdf', inputBytes: 4096, analysisComplete: true, pageCount: 1,
  encrypted: false, passwordState: 'not-required', hasDigitalSignature: false,
  signatureFieldNames: [], hasFormFields: true, formFieldNames: ['full_name'],
  hasAttachments: false, attachmentNames: [], outlineCount: 0, warnings: [], blockingReasons: [],
  ...overrides,
})

describe('CompressionView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    mocks.invoke.mockResolvedValue('{}')
    mocks.compressFiles.mockResolvedValue(undefined)
    mocks.planPdfOptimizationDestination.mockResolvedValue({ destination: 'C:/output/form.organized.pdf' })
    mocks.compressPdfFile.mockResolvedValue({
      path: 'C:/output/form.organized.pdf', inputBytes: 4096, outputBytes: 3072,
      savingsRatio: 0.25, outputSha256: 'a'.repeat(64), markOfTheWeb: 'not-present',
      verified: {
        outputBytes: 3072, outputSha256: 'a'.repeat(64),
        sourceFacts: { pageCount: 1, encrypted: false, pageMediaBoxes: [['0', '0', '612', '792']], formFields: [], annotations: [], outlines: [], attachments: [] },
        outputFacts: { pageCount: 1, encrypted: false, pageMediaBoxes: [['0', '0', '612', '792']], formFields: [], annotations: [], outlines: [], attachments: [] },
      },
    })
    mocks.preflightOperationResources.mockResolvedValue({
      operation: 'compression',
      outputPath: 'C:/input/sample.zip',
      probePath: 'C:/input/sample.zip',
      mountPoint: 'C:/',
      fileSystem: 'NTFS',
      location: 'local',
      medium: 'ssd',
      totalBytes: 1_000_000_000,
      availableBytes: 900_000_000,
      estimatedOutputBytes: 13,
      requiredBytes: 134_217_741,
      reserveBytes: 134_217_728,
      estimateSource: 'provided_estimate',
      estimateReliable: false,
      status: 'ready',
      canStart: true,
      summary: '空间充足',
      warnings: [],
    })
    mocks.checkRarCompressionSupport.mockResolvedValue({ available: true, message: 'ready' })
    mocks.installWinRarWithWinget.mockResolvedValue({ available: true, encoder_path: 'C:/Program Files/WinRAR/Rar.exe', message: 'ready' })
    mocks.getFileInfo.mockResolvedValue(null)
    mocks.planImageCompressionDestination.mockResolvedValue({
      status: 'ready',
      destination: 'C:/output/transparent.compressed.png',
    })
    mocks.compressImageFile.mockResolvedValue(undefined)
    mocks.planVideoCompression.mockImplementation(async request => videoPlan(request.preset))
    mocks.planVideoCompressionDestination.mockResolvedValue({ destination: 'C:/output/rotated.compressed.mp4' })
    mocks.compressVideoFile.mockResolvedValue({
      path: 'C:/output/rotated.compressed.mp4',
      inputBytes: 22_769,
      outputBytes: 12_000,
      savingsRatio: 0.4729,
      markOfTheWeb: 'not-present',
      verified: {
        encodedBytes: 12_000,
        container: 'mp4',
        durationMs: 1_000,
        durationDifferenceMs: 0,
        durationToleranceMs: 250,
        videoCodec: 'h264',
        audioCodec: 'aac',
        encodedWidth: 360,
        encodedHeight: 640,
        visibleWidth: 360,
        visibleHeight: 640,
        rotationDegrees: 0,
        decodedVideoFrames: 4,
      },
    })
    mocks.analyzePdfInput.mockResolvedValue(pdfReport())
    mocks.ask.mockResolvedValue(true)
  })

  it('accepts a selected file and runs a complete compression job', async () => {
    const wrapper = mountView()
    const dropzone = wrapper.findComponent(DropzoneStub)
    dropzone.vm.$emit('files-selected', [source()])
    await nextTick()

    const appStore = useAppStore()
    const compressionStore = useCompressionStore()
    const taskStore = useTaskStore()
    expect(compressionStore.selectedFiles).toHaveLength(1)

    const startButton = wrapper.findAll('button').find(button => button.text().includes(appStore.t('compress.start')))
    expect(startButton).toBeTruthy()
    await startButton!.trigger('click')
    await flushPromises()

    expect(mocks.compressFiles).toHaveBeenCalledWith(
      expect.any(String),
      ['C:/input/sample.txt'],
      'C:/input/sample.zip',
      expect.objectContaining({ format: 'zip', level: 6 }),
    )
    expect(mocks.preflightOperationResources).toHaveBeenCalledWith(expect.objectContaining({
      operation: 'compression',
      outputPath: 'C:/input/sample.zip',
      sourcePaths: ['C:/input/sample.txt'],
      estimatedOutputBytes: 13,
      estimateReliable: true,
    }))
    expect(taskStore.tasks).toHaveLength(1)
    expect(taskStore.tasks[0].status).toBe('completed')
    expect(compressionStore.selectedFiles).toHaveLength(1)
    expect(compressionStore.selectedFiles[0].taskId).toBe(taskStore.tasks[0].id)
    expect(wrapper.text()).toContain('sample.txt')
    expect(wrapper.text()).toContain(appStore.t('tasks.status.completed'))
    expect(appStore.successMessage).toBeTruthy()

    const clear = wrapper.findAll('button').find(button => button.text().includes('清除已结束'))
    expect(clear).toBeTruthy()
    await clear!.trigger('click')
    expect(compressionStore.selectedFiles).toHaveLength(0)
    expect(taskStore.tasks).toHaveLength(0)
  })

  it('passes the selected 7z solid mode to the native compression command', async () => {
    const wrapper = mountView()
    const compressionStore = useCompressionStore()
    compressionStore.globalSettings.format = '7z'
    compressionStore.globalSettings.createSolidArchive = true
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [source()])
    await nextTick()

    const startButton = wrapper.findAll('button').find(
      button => button.text().includes(useAppStore().t('compress.start')),
    )
    await startButton!.trigger('click')
    await flushPromises()

    expect(mocks.compressFiles).toHaveBeenCalledWith(
      expect.any(String),
      ['C:/input/sample.txt'],
      'C:/input/sample.7z',
      expect.objectContaining({ format: '7z', create_solid_archive: true }),
    )
  })

  it('confirms non-native password formats before creating an encrypted 7z', async () => {
    const wrapper = mountView()
    const compressionStore = useCompressionStore()
    compressionStore.globalSettings.format = 'tar.gz'
    compressionStore.globalSettings.password = 'secret'
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [source()])
    await nextTick()

    const startButton = wrapper.findAll('button').find(
      button => button.text().includes(useAppStore().t('compress.start')),
    )
    await startButton!.trigger('click')
    await flushPromises()

    expect(mocks.ask).toHaveBeenCalledWith(
      expect.stringContaining('TAR.GZ'),
      expect.objectContaining({ type: 'warning' }),
    )
    expect(mocks.compressFiles).toHaveBeenCalledWith(
      expect.any(String),
      ['C:/input/sample.txt'],
      'C:/input/sample.7z',
      expect.objectContaining({ format: '7z', password: 'secret' }),
    )
  })

  it('does not create a task when encrypted 7z conversion is declined', async () => {
    mocks.ask.mockResolvedValueOnce(false)
    const wrapper = mountView()
    const compressionStore = useCompressionStore()
    compressionStore.globalSettings.format = 'tar.gz'
    compressionStore.globalSettings.password = 'secret'
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [source()])
    await nextTick()

    const startButton = wrapper.findAll('button').find(
      button => button.text().includes(useAppStore().t('compress.start')),
    )
    await startButton!.trigger('click')
    await flushPromises()

    expect(mocks.compressFiles).not.toHaveBeenCalled()
    expect(useTaskStore().tasks).toHaveLength(0)
  })

  it('aligns each draft row into archive name, source path, and status-progress columns', async () => {
    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [source()])
    await nextTick()

    const header = wrapper.get('[data-testid="compression-table-header"]')
    expect(header.get('[data-testid="compression-name-header"]').text()).toBe('压缩包名称')
    expect(header.get('[data-testid="compression-source-header"]').text()).toBe('源文件路径')
    expect(header.get('[data-testid="compression-status-header"]').text()).toBe('压缩状态与进度')

    const row = wrapper.get('[data-testid="compression-draft-row"]')
    expect(row.get('[data-testid="compression-archive-name"]').text()).toContain('sample.zip')
    expect(row.get('[data-testid="compression-source-path"]').text()).toContain('C:/input/sample.txt')
    expect(row.get('[data-testid="compression-status-progress"]').text()).toContain(
      useAppStore().t('compress.status.pending'),
    )
  })

  it('blocks compression before the engine when resource capacity is insufficient', async () => {
    mocks.preflightOperationResources.mockResolvedValueOnce({
      operation: 'compression',
      outputPath: 'C:/input/sample.zip',
      probePath: 'C:/input/sample.zip',
      mountPoint: 'C:/',
      fileSystem: 'NTFS',
      location: 'local',
      medium: 'ssd',
      totalBytes: 1_000,
      availableBytes: 100,
      estimatedOutputBytes: 13,
      requiredBytes: 134_217_741,
      reserveBytes: 134_217_728,
      estimateSource: 'provided_estimate',
      estimateReliable: false,
      status: 'blocked',
      canStart: false,
      summary: '目标盘空间不足',
      warnings: [],
    })
    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [source()])
    await nextTick()
    const startButton = wrapper.findAll('button').find(
      button => button.text().includes(useAppStore().t('compress.start')),
    )
    await startButton!.trigger('click')
    await flushPromises()

    expect(mocks.compressFiles).not.toHaveBeenCalled()
    expect(useTaskStore().tasks[0]).toMatchObject({
      status: 'failed',
      error: '目标盘空间不足',
      resourcePreflight: { status: 'blocked', canStart: false },
    })
    expect(useTaskStore().tasks[0].logs.some(log => log.message.includes('资源预检'))).toBe(true)
    wrapper.unmount()
  })

  it('keeps one row and updates real progress and logs in place for the full lifecycle', async () => {
    let resolveCompression!: () => void
    mocks.compressFiles.mockImplementationOnce(() => new Promise<void>(resolve => {
      resolveCompression = resolve
    }))
    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [source()])
    await nextTick()
    await wrapper.get('[data-testid="compression-draft-row"]').trigger('click')

    const startButton = wrapper.findAll('button').find(
      button => button.text().includes(useAppStore().t('compress.start')),
    )
    await startButton!.trigger('click')
    await flushPromises()

    const compressionStore = useCompressionStore()
    const taskStore = useTaskStore()
    expect(compressionStore.selectedFiles).toHaveLength(1)
    expect(wrapper.findAll('[data-testid="compression-draft-row"]')).toHaveLength(1)
    expect(wrapper.text()).not.toContain('压缩任务')
    expect(taskStore.tasks[0].status).toBe('compressing')

    taskStore.tasks[0].progress = 42
    taskStore.tasks[0].stage = 'Compressing' as any
    taskStore.tasks[0].logs.push({
      task_id: taskStore.tasks[0].id,
      timestamp: new Date().toISOString(),
      message: '正在写入压缩数据',
      severity: 'info',
    })
    await nextTick()

    const execution = wrapper.get('[data-testid="compression-draft-execution"]')
    expect(execution.text()).toContain('压缩中')
    expect(execution.text()).toContain('42.00%')
    expect(execution.text()).toContain('正在写入压缩数据')

    resolveCompression()
    await flushPromises()
    expect(taskStore.tasks[0].status).toBe('completed')
    expect(wrapper.get('[data-testid="compression-draft-row"]').text()).toContain('已完成')
  })

  it('opens file details from the row while the leading checkbox only selects grouping', async () => {
    const wrapper = mountView()
    const compressionStore = useCompressionStore()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [source()])
    await nextTick()

    expect(compressionStore.selectedFiles[0].expanded).toBe(false)
    await wrapper.get('[data-testid="compression-draft-row"]').trigger('click')
    expect(compressionStore.selectedFiles[0].expanded).toBe(true)

    await wrapper.get('[data-testid="compression-group-checkbox"]').trigger('click')
    expect(compressionStore.selectedFiles[0].expanded).toBe(true)
    expect(wrapper.get('[data-testid="compression-grouping-actions"]').text()).toContain('磁吸成组（1）')
    expect(wrapper.get('[data-testid="compression-top-actions"]').text()).not.toContain('磁吸成组')
    expect(wrapper.text()).not.toContain('Stacking')
  })

  it('shows configuration and waiting execution details side by side before compression starts', async () => {
    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [source()])
    await nextTick()

    await wrapper.get('[data-testid="compression-draft-row"]').trigger('click')

    const details = wrapper.get('[data-testid="compression-draft-details"]')
    expect(wrapper.get('[data-testid="compression-draft-row"]').classes()).toContain('compression-job-row')
    expect(details.classes()).toContain('compression-detail-card')
    expect(details.find('[data-testid="compression-draft-config"]').exists()).toBe(true)
    const execution = details.get('[data-testid="compression-draft-execution"]')
    expect(execution.text()).toContain('阶段')
    expect(execution.text()).toContain('等待中')
    expect(execution.text()).toContain('进度')
    expect(execution.text()).toContain('0%')
    expect(execution.text()).toContain('实时执行日志')
    expect(execution.text()).toContain('等待开始压缩')
  })

  it('keeps a magnetic group as one task row through compression and cleanup', async () => {
    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [
      source('C:/input/one.txt'),
      source('C:/input/two.txt'),
    ])
    await nextTick()

    const checkboxes = wrapper.findAll('[data-testid="compression-group-checkbox"]')
    expect(checkboxes).toHaveLength(2)
    await checkboxes[0].trigger('click')
    await checkboxes[1].trigger('click')
    const groupButton = wrapper.get('[data-testid="compression-grouping-actions"]').find('button')
    await groupButton.trigger('click')

    const compressionStore = useCompressionStore()
    expect(compressionStore.selectedFiles).toHaveLength(0)
    expect(compressionStore.groups).toHaveLength(1)
    expect(wrapper.findAll('[data-testid="compression-group-row"]')).toHaveLength(1)
    const groupRow = wrapper.get('[data-testid="compression-group-row"]')
    expect(groupRow.get('[data-testid="compression-archive-name"]').text()).toContain('.zip')
    expect(groupRow.get('[data-testid="compression-source-path"]').text()).toContain('C:/input/one.txt')
    expect(groupRow.get('[data-testid="compression-status-progress"]').text()).toContain(
      useAppStore().t('compress.status.pending'),
    )

    const startButton = wrapper.findAll('button').find(
      button => button.text().includes(useAppStore().t('compress.start')),
    )
    await startButton!.trigger('click')
    await flushPromises()

    expect(mocks.compressFiles).toHaveBeenCalledTimes(1)
    expect(mocks.compressFiles).toHaveBeenCalledWith(
      expect.any(String),
      ['C:/input/one.txt', 'C:/input/two.txt'],
      expect.stringMatching(/新建压缩组 1\.zip$/),
      expect.objectContaining({ format: 'zip' }),
    )
    expect(compressionStore.groups).toHaveLength(1)
    expect(compressionStore.groups[0].taskId).toBe(useTaskStore().tasks[0].id)
    expect(wrapper.get('[data-testid="compression-group-row"]').text()).toContain('已完成')

    const clear = wrapper.findAll('button').find(button => button.text().includes('清除已结束'))
    await clear!.trigger('click')
    expect(compressionStore.groups).toHaveLength(0)
    expect(useTaskStore().tasks).toHaveLength(0)
  })

  it('clears only finished compression tasks from the compression center', async () => {
    const wrapper = mountView()
    const taskStore = useTaskStore()
    taskStore.addTask({
      id: 'finished-compression',
      name: 'done.zip',
      type: 'compression',
      sourceFiles: ['C:/done.txt'],
      outputPath: 'C:/done.zip',
    })
    taskStore.updateTaskStatus('finished-compression', 'completed')
    taskStore.addTask({
      id: 'finished-decompression',
      name: 'other.zip',
      type: 'decompression',
      sourceFiles: ['C:/other.zip'],
      outputPath: 'C:/other',
    })
    taskStore.updateTaskStatus('finished-decompression', 'completed')
    await nextTick()

    const clear = wrapper.findAll('button').find(button => button.text().includes('清除已结束'))
    expect(clear).toBeTruthy()
    await clear!.trigger('click')

    expect(taskStore.tasks.map(task => task.id)).toEqual(['finished-decompression'])
  })

  it('continues the batch after one compression fails', async () => {
    mocks.compressFiles
      .mockRejectedValueOnce(new Error('disk full'))
      .mockResolvedValueOnce(undefined)
    const wrapper = mountView()
    const dropzone = wrapper.findComponent(DropzoneStub)
    dropzone.vm.$emit('files-selected', [source('C:/one/sample.txt'), source('D:/two/sample.txt')])
    await nextTick()

    const appStore = useAppStore()
    const startButton = wrapper.findAll('button').find(button => button.text().includes(appStore.t('compress.start')))
    await startButton!.trigger('click')
    await flushPromises()

    expect(mocks.compressFiles).toHaveBeenCalledTimes(2)
    expect(useTaskStore().tasks.map(task => task.status)).toEqual(['failed', 'completed'])
    expect(appStore.error).toContain('disk full')
    expect(appStore.successMessage).toBeTruthy()
  })

  it('cancels the active job and queued jobs when concurrency is one', async () => {
    let rejectActive!: (error: Error) => void
    mocks.compressFiles.mockImplementationOnce(() => new Promise((_, reject) => {
      rejectActive = reject
    }))
    const wrapper = mountView()
    useAppStore().updateSettings({ maxConcurrentTasks: 1 })
    wrapper.findComponent(DropzoneStub).vm.$emit(
      'files-selected',
      [source('C:/one/first.txt'), source('C:/two/second.txt')],
    )
    await nextTick()

    const startButton = wrapper.findAll('button').find(
      button => button.text().includes(useAppStore().t('compress.start')),
    )
    await startButton!.trigger('click')
    await flushPromises()
    expect(useTaskStore().tasks.map(task => task.status)).toEqual(['compressing', 'pending'])

    const cancelAll = wrapper.findAll('button').find(button => button.text().includes('取消进行中'))
    expect(cancelAll).toBeTruthy()
    await cancelAll!.trigger('click')
    await flushPromises()
    rejectActive(new Error('compression cancelled'))
    await flushPromises()

    expect(mocks.compressFiles).toHaveBeenCalledTimes(1)
    expect(useTaskStore().tasks.map(task => task.status)).toEqual(['cancelled', 'cancelled'])
  })

  it('keeps a row cancelled when the compression command resolves during cancellation', async () => {
    let resolveCompression!: () => void
    mocks.compressFiles.mockImplementationOnce(() => new Promise<void>(resolve => {
      resolveCompression = resolve
    }))
    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [source()])
    await nextTick()

    const startButton = wrapper.findAll('button').find(
      button => button.text().includes(useAppStore().t('compress.start')),
    )
    await startButton!.trigger('click')
    await flushPromises()
    expect(useTaskStore().tasks[0].status).toBe('compressing')

    await wrapper.get('[data-testid="compression-job-cancel"]').trigger('click')
    await flushPromises()
    expect(useTaskStore().tasks[0].status).toBe('cancelled')

    resolveCompression()
    await flushPromises()
    expect(useTaskStore().tasks[0].status).toBe('cancelled')
    expect(wrapper.get('[data-testid="compression-draft-row"]').text()).toContain('已取消')
  })

  it('does not start another active compression for the same output path', async () => {
    const wrapper = mountView()
    const taskStore = useTaskStore()
    taskStore.addTask({
      id: 'existing-compression',
      name: 'sample.txt',
      type: 'compression',
      sourceFiles: ['C:/input/sample.txt'],
      outputPath: 'C:\\input\\sample.zip',
    })
    taskStore.updateTaskStatus('existing-compression', 'compressing')

    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [source()])
    await nextTick()
    const startButton = wrapper.findAll('button').find(
      button => button.text().includes(useAppStore().t('compress.start')),
    )
    await startButton!.trigger('click')

    expect(mocks.compressFiles).not.toHaveBeenCalled()
    expect(taskStore.tasks).toHaveLength(1)
    expect(useAppStore().error).toContain('already writing this output')
  })

  it('starts a compression request that arrives after the view is already mounted', async () => {
    vi.useFakeTimers()
    try {
      const wrapper = mountView()
      const compressionStore = useCompressionStore()
      wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [source()])
      await nextTick()

      compressionStore.requestAutoStart()
      await nextTick()
      await vi.runAllTimersAsync()

      expect(mocks.compressFiles).toHaveBeenCalledTimes(1)
      expect(compressionStore.autoStartRequested).toBe(false)
    } finally {
      vi.useRealTimers()
    }
  })

  it('packs a multi-selection into one ZIP in its current directory', async () => {
    vi.useFakeTimers()
    try {
      mountView()
      const compressionStore = useCompressionStore()
      compressionStore.replaceWithQuickPack(
        [source('C:/work/one.txt'), source('C:/work/two.txt')],
        'work',
        'C:/work',
      )
      await nextTick()
      await vi.runAllTimersAsync()

      expect(mocks.compressFiles).toHaveBeenCalledTimes(1)
      expect(mocks.compressFiles).toHaveBeenCalledWith(
        expect.any(String),
        ['C:/work/one.txt', 'C:/work/two.txt'],
        'C:/work/work.zip',
        expect.objectContaining({ format: 'zip' }),
      )
    } finally {
      vi.useRealTimers()
    }
  })

  it('keeps quick packs from different Explorer directories as separate archives', async () => {
    vi.useFakeTimers()
    try {
      mountView()
      const compressionStore = useCompressionStore()
      compressionStore.prepareQuickPacks()
      compressionStore.addQuickPack([source('C:/work/one.txt')], 'one', 'C:/work')
      compressionStore.addQuickPack([source('D:/other/two.txt')], 'two', 'D:/other')
      await nextTick()
      await vi.runAllTimersAsync()

      expect(mocks.compressFiles).toHaveBeenCalledTimes(2)
      expect(mocks.compressFiles.mock.calls.map(call => call[2])).toEqual([
        'C:/work/one.zip',
        'D:/other/two.zip',
      ])
    } finally {
      vi.useRealTimers()
    }
  })

  it('pauses missing RAR creation and can resume the same job as 7Z', async () => {
    mocks.checkRarCompressionSupport.mockResolvedValue({ available: false, message: 'RAR encoder missing' })
    const wrapper = mountView()
    const compressionStore = useCompressionStore()
    compressionStore.globalSettings.format = 'rar'
    compressionStore.globalSettings.password = 'keep-this-password'
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [source()])
    await nextTick()

    const startButton = wrapper.findAll('button').find(button => button.text().includes(useAppStore().t('compress.start')))
    const pendingStart = startButton!.trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('创建 RAR 需要编码器')
    expect(wrapper.text()).toContain('RAR encoder missing')
    const use7z = wrapper.findAll('button').find(button => button.text().includes('改用 7Z'))
    expect(use7z).toBeTruthy()
    await use7z!.trigger('click')
    await pendingStart
    await flushPromises()

    expect(mocks.compressFiles).toHaveBeenCalledWith(
      expect.any(String),
      ['C:/input/sample.txt'],
      'C:/input/sample.7z',
      expect.objectContaining({ format: '7z', password: 'keep-this-password' }),
    )
  })

  it('keeps the compression center archive-only and exposes three media workspaces in special compression', async () => {
    const pinia = createPinia()
    const archiveWrapper = mountView(pinia)
    archiveWrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [source()])
    await nextTick()
    expect(archiveWrapper.findAll('[data-testid="compression-draft-row"]')).toHaveLength(1)
    expect(archiveWrapper.find('[data-testid="compression-mode-switch"]').exists()).toBe(false)
    expect(archiveWrapper.find('[data-testid="image-compression-workspace"]').exists()).toBe(false)

    const wrapper = mountSpecialView(pinia)
    expect(wrapper.get('[data-testid="image-compression-workspace"]').text()).toContain('统一任务队列')
    expect(wrapper.find('[data-testid="compression-mode-archive"]').exists()).toBe(false)
    await wrapper.get('[data-testid="compression-mode-image"]').trigger('click')
    expect(wrapper.get('[data-testid="image-compression-workspace"]').text()).toContain('统一任务队列')
    expect(wrapper.get('[data-testid="image-compression-workspace"] .primary-action').attributes('disabled')).toBeDefined()
    expect(useCompressionStore().imageItems).toHaveLength(0)
    expect(useCompressionStore().selectedFiles).toHaveLength(1)

    await wrapper.get('[data-testid="compression-mode-video"]').trigger('click')
    expect(wrapper.get('[data-testid="video-compression-workspace"]').text()).toContain('统一任务')
    expect(wrapper.get('[data-testid="video-compression-workspace"] .primary-action').attributes('disabled')).toBeDefined()
    expect(useTaskStore().tasks).toHaveLength(0)

    await wrapper.get('[data-testid="compression-mode-pdf"]').trigger('click')
    expect(wrapper.get('[data-testid="pdf-compression-workspace"]').text()).toContain('统一任务')
    expect(useTaskStore().tasks).toHaveLength(0)

    expect(useCompressionStore().selectedFiles).toHaveLength(1)
  })

  it('analyzes PDF facts and requires explicit lossy confirmation before freezing a local draft', async () => {
    const wrapper = mountSpecialView()
    await wrapper.get('[data-testid="compression-mode-pdf"]').trigger('click')
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{
      name: 'form.pdf', path: 'C:/input/form.pdf', size: 4096, type: 'file', isDirectory: false,
    }])
    await flushPromises()

    await vi.waitFor(() => expect(wrapper.text()).toContain('表单字段'))

    expect(mocks.analyzePdfInput).toHaveBeenCalledWith({ path: 'C:/input/form.pdf', password: null })
    const workspace = wrapper.get('[data-testid="pdf-compression-workspace"]')
    expect(workspace.text()).toContain('表单字段')
    expect(workspace.text()).toContain('4.0 KB')
    expect(workspace.get('[data-testid="pdf-output-preview"]').text()).toContain('form.organized.pdf')

    await workspace.get('[data-testid="pdf-mode-image"]').trigger('click')
    expect(workspace.get('[data-testid="pdf-freeze-configuration"]').attributes('disabled')).toBeDefined()
    await workspace.get('[data-testid="pdf-risk-confirmation"]').setValue(true)
    expect(workspace.get('[data-testid="pdf-freeze-configuration"]').attributes('disabled')).toBeUndefined()
    await workspace.get('[data-testid="pdf-freeze-configuration"]').trigger('click')
    expect(workspace.text()).toContain('配置已锁定')
    expect(workspace.get('[data-testid="pdf-output-preview"]').text()).toContain('form.optimized.pdf')
    expect(useTaskStore().tasks).toHaveLength(0)
  })

  it('keeps signed PDFs analysis-only and unlocks encrypted facts only after password analysis', async () => {
    mocks.analyzePdfInput
      .mockResolvedValueOnce(pdfReport({
        source: 'C:/input/signed.pdf', hasDigitalSignature: true, signatureFieldNames: ['Signature1'],
        blockingReasons: ['PDF_DIGITAL_SIGNATURE_EXECUTION_BLOCKED'],
      }))
      .mockResolvedValueOnce(pdfReport({
        source: 'C:/input/encrypted.pdf', analysisComplete: false, pageCount: null, encrypted: true,
        passwordState: 'required', hasDigitalSignature: null, hasFormFields: null, hasAttachments: null,
        outlineCount: null, blockingReasons: ['PDF_PASSWORD_REQUIRED'],
      }))
      .mockResolvedValueOnce(pdfReport({
        source: 'C:/input/encrypted.pdf', encrypted: true, passwordState: 'accepted', hasFormFields: false,
        formFieldNames: [],
      }))
    const wrapper = mountSpecialView()
    await wrapper.get('[data-testid="compression-mode-pdf"]').trigger('click')
    const dropzone = wrapper.findComponent(DropzoneStub)
    dropzone.vm.$emit('files-selected', [
      { name: 'signed.pdf', path: 'C:/input/signed.pdf', size: 100, type: 'file', isDirectory: false },
      { name: 'encrypted.pdf', path: 'C:/input/encrypted.pdf', size: 100, type: 'file', isDirectory: false },
    ])
    await flushPromises()

    await vi.waitFor(() => expect(wrapper.findAll('[data-testid="pdf-draft-card"]')[1].find('[data-testid="pdf-password-input"]').exists()).toBe(true))

    const cards = wrapper.findAll('[data-testid="pdf-draft-card"]')
    expect(cards[0].text()).toContain('当前仅可分析')
    expect(cards[0].get('[data-testid="pdf-freeze-configuration"]').attributes('disabled')).toBeDefined()
    await cards[1].get('[data-testid="pdf-password-input"]').setValue('fixture-user')
    await cards[1].get('[data-testid="pdf-password-analyze"]').trigger('click')
    await flushPromises()
    expect(mocks.analyzePdfInput).toHaveBeenLastCalledWith({ path: 'C:/input/encrypted.pdf', password: 'fixture-user' })
    expect(cards[1].text()).toContain('密码已验证')
    expect(cards[1].find('[data-testid="pdf-password-input"]').exists()).toBe(false)
    expect(cards[1].text()).toContain('PDF_ENCRYPTED_EXECUTION_UNSUPPORTED')
    expect(cards[1].get('[data-testid="pdf-freeze-configuration"]').attributes('disabled')).toBeDefined()
    expect(useTaskStore().tasks).toHaveLength(0)
  })

  it('runs a frozen PDF through unified tasks, persists measured facts, and opens the published result', async () => {
    const wrapper = mountSpecialView()
    await wrapper.get('[data-testid="compression-mode-pdf"]').trigger('click')
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{
      name: 'form.pdf', path: 'C:/input/form.pdf', size: 4096, type: 'file', isDirectory: false,
    }])
    await flushPromises()

    const workspace = wrapper.get('[data-testid="pdf-compression-workspace"]')
    expect((workspace.get('[data-testid="pdf-allow-larger-output"]').element as HTMLInputElement).checked).toBe(false)
    await workspace.get('[data-testid="pdf-allow-larger-output"]').setValue(true)
    await workspace.get('[data-testid="pdf-freeze-configuration"]').trigger('click')
    await workspace.get('[data-testid="pdf-start-batch"]').trigger('click')
    await flushPromises()

    expect(mocks.planPdfOptimizationDestination).toHaveBeenCalledWith(
      'C:/input/form.pdf', 'lossless-organization', null, [],
    )
    expect(mocks.compressPdfFile).toHaveBeenCalledWith(expect.stringMatching(/^pdf-/), {
      source: 'C:/input/form.pdf', destination: 'C:/output/form.organized.pdf',
      mode: 'lossless-organization', confirmedLossyImageChanges: false,
      preserveMarkOfWeb: true, allowLargerOutput: true,
    })
    const task = useTaskStore().tasks[0]
    expect(task).toMatchObject({
      workloadKind: 'pdf', status: 'completed', outputPath: 'C:/output/form.organized.pdf',
      outputBytes: 3072, outputBytesEstimated: false,
    })
    expect(task.metrics?.media?.pageCount).toBe(1)
    expect(mocks.invoke).toHaveBeenCalledWith('save_task_history', expect.objectContaining({
      record: expect.objectContaining({ workloadKind: 'pdf', status: 'completed' }),
    }))

    await workspace.get('[data-testid="pdf-open-default-app"]').trigger('click')
    expect(mocks.openPdfOutputWithDefaultApplication).toHaveBeenCalledWith('C:/output/form.organized.pdf')
  })

  it('plans a real video candidate, labels estimates, and replans preset changes without creating tasks', async () => {
    const wrapper = mountSpecialView()
    await wrapper.get('[data-testid="compression-mode-video"]').trigger('click')
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{
      name: 'rotated.mp4',
      path: 'C:/input/rotated.mp4',
      size: 22_769,
      type: 'video/mp4',
      isDirectory: false,
    }])
    await flushPromises()

    expect(mocks.planVideoCompression).toHaveBeenCalledWith({
      path: 'C:/input/rotated.mp4',
      preset: 'balanced',
      maxWidth: null,
      maxHeight: null,
    })
    const workspace = wrapper.get('[data-testid="video-compression-workspace"]')
    expect(workspace.get('[data-testid="video-draft-card"]').text()).toContain('360×640')
    expect(workspace.text()).toContain('预计输出 · 估算')
    expect(workspace.text()).toContain('后续执行前必须显式确认')
    expect(useTaskStore().tasks).toHaveLength(0)

    expect(workspace.find('[data-testid="video-preset-small"]').exists()).toBe(false)
    await workspace.get('[data-testid="video-toggle-global-settings"]').trigger('click')
    await workspace.get('[data-testid="video-preset-small"]').trigger('click')
    await flushPromises()
    expect(mocks.planVideoCompression).toHaveBeenLastCalledWith(expect.objectContaining({ preset: 'small' }))
    expect(useCompressionStore().selectedFiles).toHaveLength(0)
    expect(useCompressionStore().imageItems).toHaveLength(0)
    expect(useTaskStore().tasks).toHaveLength(0)
  })

  it('confirms exact video stream changes then persists only verified publication facts', async () => {
    const wrapper = mountSpecialView()
    await wrapper.get('[data-testid="compression-mode-video"]').trigger('click')
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{
      name: 'rotated.mp4',
      path: 'C:/input/rotated.mp4',
      size: 22_769,
      type: 'video/mp4',
      isDirectory: false,
    }])
    await flushPromises()

    const workspace = wrapper.get('[data-testid="video-compression-workspace"]')
    const start = workspace.get('.primary-action')
    expect(start.attributes('disabled')).toBeUndefined()
    await start.trigger('click')
    await flushPromises()

    expect(mocks.ask).toHaveBeenCalledWith(
      expect.stringContaining('字幕流将被移除'),
      expect.objectContaining({ type: 'warning' }),
    )
    expect(mocks.planVideoCompressionDestination).toHaveBeenCalledWith(
      'C:/input/rotated.mp4',
      null,
      [],
    )
    expect(mocks.compressVideoFile).toHaveBeenCalledWith(
      expect.stringContaining('video-'),
      expect.objectContaining({
        destination: 'C:/output/rotated.compressed.mp4',
        confirmedStreamChanges: videoPlan().streamChanges,
        preserveMarkOfWeb: true,
      }),
    )
    expect(useTaskStore().tasks[0]).toMatchObject({
      workloadKind: 'video',
      status: 'completed',
      stage: undefined,
      outputPath: 'C:/output/rotated.compressed.mp4',
      outputBytes: 12_000,
      outputBytesEstimated: false,
      metrics: {
        inputBytes: 22_769,
        outputBytes: 12_000,
        media: { durationMs: 1_000, videoCodec: 'h264', audioCodec: 'aac', container: 'mp4' },
      },
    })
    expect(workspace.text()).toContain('最终输出')
    expect(workspace.text()).toContain('11.7 KiB')
    await workspace.get('[data-testid="video-open-default-app"]').trigger('click')
    expect(mocks.openVideoOutputWithDefaultApplication).toHaveBeenCalledWith('C:/output/rotated.compressed.mp4')
  })

  it('creates no video task or output plan when stream-change confirmation is declined', async () => {
    mocks.ask.mockResolvedValueOnce(false)
    const wrapper = mountSpecialView()
    await wrapper.get('[data-testid="compression-mode-video"]').trigger('click')
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{
      name: 'rotated.mp4', path: 'C:/input/rotated.mp4', size: 22_769,
      type: 'video/mp4', isDirectory: false,
    }])
    await flushPromises()

    await wrapper.get('[data-testid="video-compression-workspace"] .primary-action').trigger('click')
    await flushPromises()

    expect(mocks.planVideoCompressionDestination).not.toHaveBeenCalled()
    expect(mocks.compressVideoFile).not.toHaveBeenCalled()
    expect(useTaskStore().tasks).toHaveLength(0)
  })

  it('cancels an active video through the unified task cancellation command', async () => {
    let rejectEncoding!: (error: Error) => void
    mocks.compressVideoFile.mockImplementationOnce(() => new Promise((_, reject) => {
      rejectEncoding = reject
    }))
    const wrapper = mountSpecialView()
    await wrapper.get('[data-testid="compression-mode-video"]').trigger('click')
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{
      name: 'rotated.mp4', path: 'C:/input/rotated.mp4', size: 22_769,
      type: 'video/mp4', isDirectory: false,
    }])
    await flushPromises()

    void wrapper.get('[data-testid="video-compression-workspace"] .primary-action').trigger('click')
    await flushPromises()
    expect(useTaskStore().tasks[0].status).toBe('compressing')
    await wrapper.get('[data-testid="video-compression-workspace"] .danger-action').trigger('click')
    rejectEncoding(new Error('VIDEO_COMPRESSION_CANCELLED'))
    await flushPromises()

    expect(mocks.invoke).toHaveBeenCalledWith('cancel_compression', expect.objectContaining({ taskId: useTaskStore().tasks[0].id }))
    expect(useTaskStore().tasks[0].status).toBe('cancelled')
  })

  it('does not start video encoding when cancellation lands during destination planning', async () => {
    let resolveDestination!: (value: { destination: string }) => void
    mocks.planVideoCompressionDestination.mockImplementationOnce(() => new Promise(resolve => {
      resolveDestination = resolve
    }))
    const wrapper = mountSpecialView()
    await wrapper.get('[data-testid="compression-mode-video"]').trigger('click')
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{
      name: 'rotated.mp4', path: 'C:/input/rotated.mp4', size: 22_769,
      type: 'video/mp4', isDirectory: false,
    }])
    await flushPromises()

    void wrapper.get('[data-testid="video-compression-workspace"] .primary-action').trigger('click')
    await flushPromises()
    await wrapper.get('[data-testid="video-compression-workspace"] .danger-action').trigger('click')
    resolveDestination({ destination: 'C:/output/rotated.compressed.mp4' })
    await flushPromises()

    expect(mocks.compressVideoFile).not.toHaveBeenCalled()
    expect(useTaskStore().tasks[0].status).toBe('cancelled')
  })

  it('runs a ready real-fixture image through the unified batch and renders verified result facts', async () => {
    const fixturePath = resolve(process.cwd(), 'test-results/media-fixture-audit/fixtures/images/transparent.png')
    const inputBytes = statSync(fixturePath).size
    const outputBytes = 1_000
    const imageFacts = (encodedBytes: number) => ({
      format: 'png' as const,
      encodedBytes,
      encodedWidth: 256,
      encodedHeight: 256,
      visibleWidth: 256,
      visibleHeight: 256,
      orientation: 1,
      frameCount: 1,
      hasAlpha: true,
    })
    mocks.compressImageFile.mockResolvedValue({
      status: 'published',
      input: imageFacts(inputBytes),
      output: imageFacts(outputBytes),
    })
    const wrapper = mountSpecialView()
    await wrapper.get('[data-testid="compression-mode-image"]').trigger('click')
    const compressionStore = useCompressionStore()
    const { accepted } = compressionStore.addImageCandidates([{
      name: 'transparent.png',
      path: fixturePath,
      size: inputBytes,
      isDirectory: false,
    }])
    compressionStore.completeImageInspection(accepted[0].id, {
      width: 256,
      height: 256,
      previewUrl: 'asset://localhost/source',
    })
    await nextTick()

    const start = wrapper.get('[data-testid="image-compression-workspace"] .primary-action')
    expect(start.attributes('disabled')).toBeUndefined()
    await start.trigger('click')
    await flushPromises()

    expect(mocks.planImageCompressionDestination).toHaveBeenCalledWith(expect.objectContaining({
      source: fixturePath,
      targetFormat: 'png',
    }))
    expect(mocks.compressImageFile).toHaveBeenCalledWith(
      expect.stringContaining('image-'),
      expect.objectContaining({ source: fixturePath, destination: 'C:/output/transparent.compressed.png' }),
    )
    expect(useTaskStore().tasks[0]).toMatchObject({
      workloadKind: 'image',
      status: 'completed',
      outputPath: 'C:/output/transparent.compressed.png',
      metrics: { inputBytes, outputBytes },
    })
    const workspace = wrapper.get('[data-testid="image-compression-workspace"]')
    expect(workspace.text()).toContain('C:/output/transparent.compressed.png')
    expect(workspace.text()).toContain('节省 546 B')
    expect(workspace.text()).not.toContain('B-03')
    expect(compressionStore.imageItems[0].resultPreviewUrl).toContain('transparent.compressed.png')
  })

  it('reports a real image failure as incomplete instead of showing a success summary', async () => {
    const fixturePath = resolve(process.cwd(), 'test-results/media-fixture-audit/fixtures/images/transparent.png')
    mocks.compressImageFile.mockRejectedValueOnce(new Error('真实编码失败'))
    const wrapper = mountSpecialView()
    await wrapper.get('[data-testid="compression-mode-image"]').trigger('click')
    const compressionStore = useCompressionStore()
    const { accepted } = compressionStore.addImageCandidates([{
      name: 'transparent.png',
      path: fixturePath,
      size: statSync(fixturePath).size,
      isDirectory: false,
    }])
    compressionStore.completeImageInspection(accepted[0].id, {
      width: 256,
      height: 256,
      previewUrl: 'asset://localhost/source',
    })
    await nextTick()

    await wrapper.get('[data-testid="image-compression-workspace"] .primary-action').trigger('click')
    await flushPromises()

    expect(useTaskStore().tasks[0]).toMatchObject({ status: 'failed', error: 'Error: 真实编码失败' })
    expect(useAppStore().error).toContain('1 个失败')
    expect(useAppStore().successMessage).toBeNull()
  })
})
