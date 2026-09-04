import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, nextTick } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import DecompressView from '../DecompressView.vue'
import { useAppStore } from '@/stores/app'
import { useTaskStore } from '@/stores/task'

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

const mountView = () => mount(DecompressView, {
  global: {
    plugins: [createPinia()],
    stubs: {
      EnhancedFileDropzone: DropzoneStub,
      AeroTable: true,
      ConflictResolutionModal: true,
      Transition: false,
    },
  },
})

describe('DecompressView', () => {
  beforeEach(() => {
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
    expect(table.attributes('tasktype')).toBe('decompression')

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

  it('retries a failed encrypted archive with the password entered by the user', async () => {
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

    expect(mocks.decompressFile).toHaveBeenCalledWith(
      'C:/archives/encrypted.7z',
      expect.objectContaining({
        password: 'correct-password',
        outputPath: 'C:/archives/output',
        createSubdirectory: true,
      }),
      taskId,
    )
    wrapper.unmount()
  })

  it('keeps password entry available after a localized wrong-password failure', async () => {
    mocks.decompressFile.mockRejectedValueOnce('提供的密码不正确')
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

  it('switches the pending extraction batch between global and individual configuration at the footer', async () => {
    const wrapper = mountView()
    wrapper.findComponent(DropzoneStub).vm.$emit('files-selected', [{
      name: 'configurable.zip',
      path: 'C:/archives/configurable.zip',
    }])
    await flushPromises()

    const task = useTaskStore().tasks[0]
    expect(task.configurationMode).toBe('global')
    expect(wrapper.findComponent({ name: 'AeroTable' }).attributes('onSet-config-mode')).toBeUndefined()
    await wrapper.get('[data-testid="decompression-config-mode-individual"]').trigger('click')
    await nextTick()
    expect(task.configurationMode).toBe('individual')
    await new Promise(resolve => setTimeout(resolve, 220))
    expect(wrapper.find('[data-testid="global-recycle-source-switch"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('展开任务详情后分别调整配置')

    await wrapper.get('[data-testid="decompression-config-mode-global"]').trigger('click')
    await nextTick()
    await new Promise(resolve => setTimeout(resolve, 220))
    expect(task.configurationMode).toBe('global')
    expect(task.outputPath).toBe('C:/archives')
    expect(wrapper.find('[data-testid="global-recycle-source-switch"]').exists()).toBe(true)
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
