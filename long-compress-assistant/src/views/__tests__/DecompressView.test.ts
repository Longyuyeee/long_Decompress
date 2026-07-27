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
    await nextTick()
    await nextTick()

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

  it('does not report a dictionary password when backend validation rejects it', async () => {
    mocks.decompressFile.mockRejectedValueOnce('PasswordRequired')
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'detect_split_archive') return null
      if (command === 'load_app_settings') return '{}'
      if (command === 'get_dictionary_passwords') return ['!@#$%^&*']
      if (command === 'verify_archive_password') return false
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

    expect(mocks.invoke).toHaveBeenCalledWith('verify_archive_password', {
      taskId: useTaskStore().tasks[0].id,
      filePath: 'C:/archives/plain.zip',
      password: '!@#$%^&*',
    })
    expect(mocks.decompressFile).toHaveBeenCalledTimes(1)
    expect(useTaskStore().tasks[0].logs.some(log => log.message.includes('密码破解成功'))).toBe(false)
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
      button => button.text().includes(appStore.t('common.cancel')),
    )
    await cancelButton!.trigger('click')
    await flushPromises()

    expect(cancelTask).toHaveBeenNthCalledWith(1, 'running-task')
    expect(cancelTask).toHaveBeenNthCalledWith(2, 'blocked-task')
    expect(appStore.error).toContain('1')
    wrapper.unmount()
  })
})
