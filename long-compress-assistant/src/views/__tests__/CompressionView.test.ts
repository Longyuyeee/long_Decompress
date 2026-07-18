import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, nextTick } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import CompressionView from '../CompressionView.vue'
import { useAppStore } from '@/stores/app'
import { useCompressionStore } from '@/stores/compression'
import { useTaskStore } from '@/stores/task'

const mocks = vi.hoisted(() => ({
  compressFiles: vi.fn(),
  checkRarCompressionSupport: vi.fn(),
  openRarDownloadPage: vi.fn(),
  installWinRarWithWinget: vi.fn(),
  invoke: vi.fn(),
}))

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => vi.fn()) }))
vi.mock('@/composables/useTauriCommands', () => ({
  useTauriCommands: () => ({
    compressFiles: mocks.compressFiles,
    checkRarCompressionSupport: mocks.checkRarCompressionSupport,
    openRarDownloadPage: mocks.openRarDownloadPage,
    installWinRarWithWinget: mocks.installWinRarWithWinget,
  }),
}))

const DropzoneStub = defineComponent({
  name: 'EnhancedFileDropzone',
  emits: ['files-selected'],
  template: '<button class="dropzone-stub" type="button">add source</button>',
})

const mountView = () => mount(CompressionView, {
  global: {
    plugins: [createPinia()],
    stubs: {
      EnhancedFileDropzone: DropzoneStub,
      CompressionSettingsPanel: true,
      GlobalSettingsModal: true,
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

describe('CompressionView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    mocks.invoke.mockResolvedValue('{}')
    mocks.compressFiles.mockResolvedValue(undefined)
    mocks.checkRarCompressionSupport.mockResolvedValue({ available: true, message: 'ready' })
    mocks.installWinRarWithWinget.mockResolvedValue({ available: true, encoder_path: 'C:/Program Files/WinRAR/Rar.exe', message: 'ready' })
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

    expect(mocks.compressFiles).toHaveBeenCalledWith(
      expect.any(String),
      ['C:/input/sample.txt'],
      'C:/input/sample.zip',
      expect.objectContaining({ format: 'zip', level: 6 }),
    )
    expect(taskStore.tasks).toHaveLength(1)
    expect(taskStore.tasks[0].status).toBe('completed')
    expect(appStore.successMessage).toBeTruthy()
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

    expect(mocks.compressFiles).toHaveBeenCalledTimes(2)
    expect(useTaskStore().tasks.map(task => task.status)).toEqual(['failed', 'completed'])
    expect(appStore.error).toContain('disk full')
    expect(appStore.successMessage).toBeTruthy()
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
})
