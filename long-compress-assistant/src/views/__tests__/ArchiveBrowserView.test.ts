import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import ArchiveBrowserView from '../ArchiveBrowserView.vue'

const mocks = vi.hoisted(() => ({
  selectFiles: vi.fn(),
  selectDirectory: vi.fn(),
  browseArchive: vi.fn(),
  previewArchiveImage: vi.fn(),
  decompressFile: vi.fn(),
  setError: vi.fn(),
  setSuccess: vi.fn(),
}))

vi.mock('@/stores/app', () => ({
  useAppStore: () => ({ setError: mocks.setError, setSuccess: mocks.setSuccess })
}))

vi.mock('@/composables/useTauriCommands', () => ({
  useTauriCommands: () => ({
    selectFiles: mocks.selectFiles,
    selectDirectory: mocks.selectDirectory,
    browseArchive: mocks.browseArchive,
    previewArchiveImage: mocks.previewArchiveImage,
    decompressFile: mocks.decompressFile,
  })
}))

describe('ArchiveBrowserView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.selectFiles.mockResolvedValue([{ path: 'C:/archives/demo.zip', name: 'demo.zip', size: 12, isDir: false, modified: 0 }])
    mocks.browseArchive.mockResolvedValue({
      format: 'ZIP', totalFiles: 2, totalDirectories: 1,
      totalUncompressedSize: 30, totalCompressedSize: 20, encrypted: false,
      entries: [
        { path: 'docs/', name: 'docs', size: 0, compressedSize: 0, modified: null, crc: null, encrypted: false, isDir: true },
        { path: 'docs/readme.txt', name: 'readme.txt', size: 10, compressedSize: 8, modified: null, crc: '11111111', encrypted: false, isDir: false },
        { path: 'image.png', name: 'image.png', size: 20, compressedSize: 12, modified: null, crc: '22222222', encrypted: false, isDir: false },
      ]
    })
    mocks.decompressFile.mockResolvedValue('C:/archives/demo')
    mocks.previewArchiveImage.mockResolvedValue({
      entryPath: 'image.png', mimeType: 'image/png',
      dataUrl: 'data:image/png;base64,c2FmZQ==', byteSize: 4, width: 1, height: 1,
    })
  })

  it('loads structured entries and extracts only the checked files', async () => {
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    expect(mocks.browseArchive).toHaveBeenCalledWith('C:/archives/demo.zip', '')
    expect(wrapper.text()).toContain('readme.txt')
    expect(wrapper.text()).toContain('image.png')

    const rows = wrapper.findAll('.browser-row')
    await rows[1].trigger('click')
    await wrapper.find('footer .browser-primary').trigger('click')
    await flushPromises()

    expect(mocks.decompressFile).toHaveBeenCalledWith('C:/archives/demo.zip', expect.objectContaining({
      outputPath: 'C:/archives',
      keepStructure: true,
      selectedEntries: ['docs/readme.txt'],
      conflictPolicy: 'rename',
    }))
    expect(mocks.setSuccess).toHaveBeenCalledWith('已解压 1 个所选文件')
  })

  it('filters entries by directory, search text and type without horizontal overflow containers', async () => {
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    await wrapper.find('input[placeholder="搜索文件名或路径"]').setValue('readme')
    expect(wrapper.findAll('.browser-row')).toHaveLength(1)
    expect(wrapper.find('.browser-row').text()).toContain('readme.txt')
    expect(wrapper.find('.browser-workspace').classes()).toContain('overflow-hidden')
    expect(wrapper.find('.browser-workspace [class*="overflow-y-auto"]').exists()).toBe(true)
  })

  it('renders archive folders as a collapsible hierarchy instead of full-path rows', async () => {
    mocks.browseArchive.mockResolvedValueOnce({
      format: 'ZIP', totalFiles: 1, totalDirectories: 2,
      totalUncompressedSize: 10, totalCompressedSize: 8, encrypted: false,
      entries: [
        { path: 'docs/guides/readme.txt', name: 'readme.txt', size: 10, compressedSize: 8, modified: null, crc: null, encrypted: false, isDir: false },
      ],
    })
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    expect(wrapper.find('.directory-pane').text()).toContain('docs')
    expect(wrapper.find('.directory-pane').text()).not.toContain('guides')
    await wrapper.get('.directory-toggle').trigger('click')
    expect(wrapper.find('.directory-pane').text()).toContain('guides')
    expect(wrapper.find('.directory-pane').text()).not.toContain('docs/guides')
  })

  it('previews a bounded raster entry without changing its selection', async () => {
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    expect(wrapper.findAll('.browser-checkbox.checked')).toHaveLength(3)
    await wrapper.get('.preview-trigger').trigger('click')
    await flushPromises()

    expect(mocks.previewArchiveImage).toHaveBeenCalledWith(
      'C:/archives/demo.zip', 'image.png', '',
    )
    expect(wrapper.get('[data-testid="archive-image-preview"] img').attributes('src'))
      .toBe('data:image/png;base64,c2FmZQ==')
    expect(wrapper.get('[data-testid="archive-image-preview"]').text()).toContain('只读 · 未写入磁盘')
    expect(wrapper.findAll('.browser-checkbox.checked')).toHaveLength(3)

    await wrapper.get('[aria-label="关闭预览"]').trigger('click')
    expect(wrapper.find('[data-testid="archive-image-preview"]').exists()).toBe(false)
  })

  it('keeps preview disabled when the archive route cannot prove bounded reading', async () => {
    mocks.browseArchive.mockResolvedValueOnce({
      format: '7Z', totalFiles: 1, totalDirectories: 0,
      totalUncompressedSize: 20, totalCompressedSize: 12, encrypted: false,
      entries: [{ path: 'image.png', name: 'image.png', size: 20, compressedSize: 12, modified: null, crc: null, encrypted: false, isDir: false }],
    })
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    expect(wrapper.get('.preview-trigger').attributes('disabled')).toBeDefined()
    expect(wrapper.get('.preview-trigger').attributes('title')).toContain('ZIP 与 TAR')
    expect(mocks.previewArchiveImage).not.toHaveBeenCalled()
  })
})
