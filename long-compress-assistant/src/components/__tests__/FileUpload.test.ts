import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import EnhancedFileDropzone from '../ui/EnhancedFileDropzone.vue'

const mocks = vi.hoisted(() => ({
  open: vi.fn(),
  invoke: vi.fn(),
  listen: vi.fn(async () => vi.fn()),
  setError: vi.fn(),
  t: vi.fn((key: string) => key)
}))

vi.mock('@tauri-apps/api/dialog', () => ({
  open: mocks.open
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: mocks.listen
}))

vi.mock('@tauri-apps/api/tauri', () => ({
  invoke: mocks.invoke
}))

vi.mock('@/stores/app', () => ({
  useAppStore: () => ({
    setError: mocks.setError,
    t: mocks.t
  })
}))

const fileWithPath = (name: string, path: string, size = 1024, type = 'text/plain') => {
  const file = new File(['content'], name, { type })
  Object.defineProperty(file, 'size', { value: size })
  Object.defineProperty(file, 'path', { value: path })
  return file
}

describe('EnhancedFileDropzone', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.invoke.mockResolvedValue({ is_dir: false, size: 0 })
  })

  it('renders as an accessible file dropzone', () => {
    const wrapper = mount(EnhancedFileDropzone)

    expect(wrapper.attributes('role')).toBe('button')
    expect(wrapper.attributes('tabindex')).toBe('0')
    expect(wrapper.find('input[type="file"]').exists()).toBe(true)
    expect(wrapper.find('.pi-cloud-upload').exists()).toBe(true)
  })

  it('gives file and folder dropzones distinct accessible names', () => {
    const fileDropzone = mount(EnhancedFileDropzone)
    const folderDropzone = mount(EnhancedFileDropzone, { props: { mode: 'folder' } })

    expect(fileDropzone.attributes('aria-label')).toBe('compress.add_files: decompress.drop_hint')
    expect(folderDropzone.attributes('aria-label')).toBe('compress.add_folders: compress.drop_folder_hint')
  })

  it('emits selected files from the hidden file input', async () => {
    const wrapper = mount(EnhancedFileDropzone)
    const input = wrapper.find('input[type="file"]')

    Object.defineProperty(input.element, 'files', {
      value: [fileWithPath('archive.zip', 'C:/archives/archive.zip', 2048, 'application/zip')]
    })

    await input.trigger('change')

    expect(wrapper.emitted('files-selected')?.[0][0]).toEqual([
      {
        name: 'archive.zip',
        path: 'C:/archives/archive.zip',
        size: 2048,
        type: 'application/zip',
        isDirectory: false
      }
    ])
  })

  it('shows an error when browser files do not expose a native path', async () => {
    const wrapper = mount(EnhancedFileDropzone)
    const input = wrapper.find('input[type="file"]')

    Object.defineProperty(input.element, 'files', {
      value: [new File(['content'], 'browser-only.zip', { type: 'application/zip' })]
    })

    await input.trigger('change')

    expect(wrapper.emitted('files-selected')).toBeUndefined()
    expect(mocks.setError).toHaveBeenCalledOnce()
  })

  it('emits dropped files with native paths', async () => {
    const wrapper = mount(EnhancedFileDropzone)
    const dropEvent = new Event('drop') as DragEvent

    Object.defineProperty(dropEvent, 'dataTransfer', {
      value: {
        files: [fileWithPath('drop.7z', 'C:/archives/drop.7z', 4096, 'application/x-7z-compressed')]
      }
    })

    await wrapper.element.dispatchEvent(dropEvent)

    expect(wrapper.emitted('files-selected')?.[0][0]).toEqual([
      {
        name: 'drop.7z',
        path: 'C:/archives/drop.7z',
        size: 4096,
        type: 'application/x-7z-compressed',
        isDirectory: false
      }
    ])
  })

  it('uses the native folder picker in folder mode', async () => {
    mocks.open.mockResolvedValueOnce(['C:/work/source', 'D:/assets'])
    mocks.invoke.mockResolvedValue({ is_dir: true, size: 0 })
    const wrapper = mount(EnhancedFileDropzone, {
      props: { mode: 'folder' }
    })

    await wrapper.trigger('click')
    await flushPromises()

    expect(mocks.open).toHaveBeenCalledWith({
      directory: true,
      multiple: true,
      title: 'compress.add_folders'
    })
    expect(wrapper.emitted('files-selected')?.[0][0]).toEqual([
      {
        name: 'source',
        path: 'C:/work/source',
        size: 0,
        type: 'directory',
        isDirectory: true
      },
      {
        name: 'assets',
        path: 'D:/assets',
        size: 0,
        type: 'directory',
        isDirectory: true
      }
    ])
  })

  it('uses an unfiltered native picker and reads real filesystem metadata', async () => {
    mocks.open.mockResolvedValueOnce(['C:/images/photo.jpg', 'C:/images/animated.gif'])
    mocks.invoke.mockImplementation(async (_command: string, { path }: { path: string }) => ({
      is_dir: false,
      size: path.endsWith('.jpg') ? 1536 : 3072,
    }))
    const wrapper = mount(EnhancedFileDropzone, {
      props: { accept: 'jpg,jpeg,png,webp', unfilteredPicker: true, pickerTitle: '选择图片文件' }
    })

    await wrapper.trigger('click')
    await flushPromises()

    expect(mocks.open).toHaveBeenCalledWith({
      directory: false,
      multiple: true,
      title: '选择图片文件',
      filters: []
    })
    expect(mocks.invoke).toHaveBeenCalledTimes(2)
    expect(wrapper.emitted('files-selected')?.[0][0]).toEqual([
      { name: 'photo.jpg', path: 'C:/images/photo.jpg', size: 1536, type: 'file', isDirectory: false },
      { name: 'animated.gif', path: 'C:/images/animated.gif', size: 3072, type: 'file', isDirectory: false }
    ])
  })

  it('passes supported extensions to the native file picker by default', async () => {
    mocks.open.mockResolvedValueOnce(null)
    const wrapper = mount(EnhancedFileDropzone, {
      props: {
        accept: '.zip,.7z,.rar,.001,.z01,.r00',
        pickerTitle: '选择压缩包',
      },
    })

    await wrapper.trigger('click')
    await flushPromises()

    expect(mocks.open).toHaveBeenCalledWith({
      directory: false,
      multiple: true,
      title: '选择压缩包',
      filters: [{
        name: 'Archives',
        extensions: ['zip', '7z', 'rar', '001', 'z01', 'r00'],
      }],
    })
  })
})
