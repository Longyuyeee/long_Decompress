import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
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
})
