import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import ArchiveBrowserView from '../ArchiveBrowserView.vue'

const mocks = vi.hoisted(() => ({
  selectFiles: vi.fn(),
  selectDirectory: vi.fn(),
  browseArchive: vi.fn(),
  cancelArchiveBrowse: vi.fn(),
  previewArchiveImage: vi.fn(),
  previewArchiveText: vi.fn(),
  materializeNestedArchive: vi.fn(),
  openArchiveEntry: vi.fn(),
  decompressFile: vi.fn(),
  clipboardWrite: vi.fn(),
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
    cancelArchiveBrowse: mocks.cancelArchiveBrowse,
    previewArchiveImage: mocks.previewArchiveImage,
    previewArchiveText: mocks.previewArchiveText,
    materializeNestedArchive: mocks.materializeNestedArchive,
    openArchiveEntry: mocks.openArchiveEntry,
    decompressFile: mocks.decompressFile,
  })
}))

describe('ArchiveBrowserView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: mocks.clipboardWrite },
    })
    mocks.clipboardWrite.mockResolvedValue(undefined)
    mocks.cancelArchiveBrowse.mockResolvedValue(undefined)
    mocks.selectDirectory.mockResolvedValue(null)
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
    mocks.previewArchiveText.mockResolvedValue({
      entryPath: 'docs/readme.txt', content: '你好，归档文本\n第二行', encoding: 'UTF-8',
      byteSize: 28, totalSize: 28, truncated: false, lineCount: 2,
    })
    mocks.openArchiveEntry.mockResolvedValue({
      status: 'opened', entryPath: 'docs/readme.txt', cachePath: 'C:/cache/readme.txt', dangerous: false,
    })
    mocks.materializeNestedArchive.mockResolvedValue({
      entryPath: 'nested.7z', cachePath: 'C:/cache/nested.7z',
      parentSha256: 'parent-hash', contentSha256: 'child-hash', depth: 2,
    })
  })

  it('loads structured entries and extracts only the checked files', async () => {
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    expect(mocks.browseArchive).toHaveBeenCalledWith('C:/archives/demo.zip', '', expect.any(String))
    expect(wrapper.text()).toContain('docs')
    expect(wrapper.text()).toContain('image.png')

    await wrapper.get('[data-entry-path="image.png"] .browser-checkbox').trigger('click')
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

  it('cancels an active archive metadata read and shows a clear idle result', async () => {
    mocks.browseArchive.mockReturnValueOnce(new Promise(() => undefined))
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    expect(wrapper.get('[data-testid="archive-browse-cancel"]').text()).toContain('取消读取')
    await wrapper.get('[data-testid="archive-browse-cancel"]').trigger('click')
    await flushPromises()

    expect(mocks.cancelArchiveBrowse).toHaveBeenCalledWith(expect.any(String))
    expect(wrapper.get('[data-testid="archive-browse-notice"]').text()).toContain('已取消读取压缩包内容')
  })

  it('shows object-specific context actions without changing selection', async () => {
    const wrapper = mount(ArchiveBrowserView, { attachTo: document.body })
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    await wrapper.get('[data-entry-path="image.png"] .browser-checkbox').trigger('click')
    expect(wrapper.find('footer').text()).toContain('已选择 1 / 2 个文件')
    await wrapper.get('[data-entry-path="image.png"]').trigger('contextmenu', { clientX: 80, clientY: 90 })

    const menu = document.querySelector('[data-testid="archive-context-menu"]') as HTMLElement
    expect(menu).not.toBeNull()
    expect(menu.textContent).toContain('image.png')
    expect(menu.querySelector('[data-testid="archive-context-preview"]')).not.toBeNull()
    expect(menu.querySelector('[data-testid="archive-context-default-open"]')).not.toBeNull()
    expect(wrapper.find('footer').text()).toContain('已选择 1 / 2 个文件')

    ;(menu.querySelector('[data-testid="archive-context-copy-path"]') as HTMLButtonElement).click()
    await flushPromises()
    expect(mocks.clipboardWrite).toHaveBeenCalledWith('image.png')
    expect(mocks.setSuccess).toHaveBeenCalledWith('已复制归档内路径')
    wrapper.unmount()
  })

  it('opens ordinary entries with the Windows default application route', async () => {
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    await wrapper.get('[data-entry-path="image.png"]').trigger('dblclick')
    await flushPromises()
    expect(mocks.openArchiveEntry).toHaveBeenCalledWith('C:/archives/demo.zip', 'image.png', '', false)
    expect(mocks.setSuccess).toHaveBeenCalledWith('已使用默认应用打开：image.png')
  })

  it('keeps active content unopened until the explicit warning is confirmed', async () => {
    mocks.browseArchive.mockResolvedValueOnce({
      format: 'ZIP', totalFiles: 1, totalDirectories: 0,
      totalUncompressedSize: 8, totalCompressedSize: 8, encrypted: false,
      entries: [{ path: 'scripts/run.cmd', name: 'run.cmd', size: 8, compressedSize: 8, modified: null, crc: null, encrypted: false, isDir: false }],
    })
    mocks.openArchiveEntry
      .mockResolvedValueOnce({ status: 'confirmationRequired', entryPath: 'scripts/run.cmd', cachePath: null, dangerous: true })
      .mockResolvedValueOnce({ status: 'opened', entryPath: 'scripts/run.cmd', cachePath: 'C:/cache/run.cmd', dangerous: true })
    const wrapper = mount(ArchiveBrowserView, { attachTo: document.body })
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    await wrapper.get('[data-entry-path="scripts/"]').trigger('dblclick')
    await wrapper.get('[data-entry-path="scripts/run.cmd"]').trigger('dblclick')
    await flushPromises()
    expect(mocks.openArchiveEntry).toHaveBeenCalledTimes(1)
    expect(mocks.openArchiveEntry).toHaveBeenLastCalledWith('C:/archives/demo.zip', 'scripts/run.cmd', '', false)
    expect(document.querySelector('[data-testid="archive-dangerous-open-dialog"]')).not.toBeNull()
    expect((document.querySelector('[data-testid="archive-dangerous-cancel"]') as HTMLButtonElement).autofocus).toBe(true)

    ;(document.querySelector('[data-testid="archive-dangerous-confirm"]') as HTMLButtonElement).click()
    await flushPromises()
    expect(mocks.openArchiveEntry).toHaveBeenLastCalledWith('C:/archives/demo.zip', 'scripts/run.cmd', '', true)
    wrapper.unmount()
  })

  it('routes context extraction and details through existing safe entry semantics', async () => {
    mocks.selectDirectory.mockResolvedValue('C:/archives/custom-output')
    const wrapper = mount(ArchiveBrowserView, { attachTo: document.body })
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    await wrapper.get('[data-entry-path="image.png"] .browser-checkbox').trigger('click')
    await wrapper.find('input[placeholder="搜索文件名或路径"]').setValue('readme')
    const readmeRow = wrapper.get('[data-entry-path="docs/readme.txt"]')
    await readmeRow.trigger('contextmenu', { clientX: 100, clientY: 110 })
    ;(document.querySelector('[data-testid="archive-context-details"]') as HTMLButtonElement).click()
    await flushPromises()
    expect((document.querySelector('[data-testid="archive-entry-details"]') as HTMLElement).textContent).toContain('docs/readme.txt')
    ;(document.querySelector('[aria-label="关闭条目详情"]') as HTMLButtonElement).click()

    await readmeRow.trigger('contextmenu', { clientX: 100, clientY: 110 })
    ;(document.querySelector('[data-testid="archive-context-extract-choose"]') as HTMLButtonElement).click()
    await flushPromises()
    expect(mocks.decompressFile).toHaveBeenCalledWith('C:/archives/demo.zip', expect.objectContaining({
      outputPath: 'C:/archives/custom-output',
      selectedEntries: ['docs/readme.txt'],
      conflictPolicy: 'rename',
    }))
    wrapper.unmount()
  })

  it('provides keyboard equivalents for context menu actions', async () => {
    const wrapper = mount(ArchiveBrowserView, { attachTo: document.body })
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    const imageRow = wrapper.get('[data-entry-path="image.png"]')
    await imageRow.get('.browser-checkbox').trigger('click')
    await imageRow.trigger('click')
    await wrapper.get('.browser-page').trigger('keydown', { key: 'F10', shiftKey: true })
    expect(document.querySelector('[data-testid="archive-context-menu"]')).not.toBeNull()
    await wrapper.get('.browser-page').trigger('keydown', { key: 'Escape' })
    expect(document.querySelector('[data-testid="archive-context-menu"]')).toBeNull()

    await wrapper.get('.browser-page').trigger('keydown', { key: 'c', ctrlKey: true, shiftKey: true })
    await flushPromises()
    expect(mocks.clipboardWrite).toHaveBeenCalledWith('image.png')
    await wrapper.get('.browser-page').trigger('keydown', { key: 'Enter', altKey: true })
    expect(document.querySelector('[data-testid="archive-entry-details"]')).not.toBeNull()
    ;(document.querySelector('[aria-label="关闭条目详情"]') as HTMLButtonElement).click()

    await wrapper.get('[data-entry-path="docs/"]').trigger('click')
    await wrapper.get('.browser-page').trigger('keydown', { key: 'c', ctrlKey: true, shiftKey: true })
    await flushPromises()
    expect(mocks.clipboardWrite).toHaveBeenLastCalledWith('docs')
    await wrapper.get('.browser-page').trigger('keydown', { key: 'Enter', altKey: true })
    expect((document.querySelector('[data-testid="archive-entry-details"]') as HTMLElement).textContent).toContain('docs')
    expect((document.querySelector('[data-testid="archive-entry-details"]') as HTMLElement).textContent).not.toContain('docs/readme.txt')
    wrapper.unmount()
  })

  it('uses file-manager focus, multiselect and directory navigation semantics', async () => {
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    const imageRow = wrapper.get('[data-entry-path="image.png"]')
    await imageRow.trigger('click')
    expect(imageRow.classes()).toContain('focused')
    expect(wrapper.find('footer').text()).toContain('已选择 2 / 2 个文件')

    await imageRow.trigger('click', { ctrlKey: true })
    expect(wrapper.find('footer').text()).toContain('已选择 1 / 2 个文件')

    await wrapper.get('[data-entry-path="docs/"]').trigger('dblclick')
    expect(wrapper.get('[data-testid="archive-breadcrumbs"]').text()).toContain('docs')
    expect(wrapper.get('[data-entry-path="docs/readme.txt"]').text()).toContain('readme.txt')
    expect(wrapper.get('[data-testid="archive-nav-back"]').attributes('disabled')).toBeUndefined()

    await wrapper.get('[data-testid="archive-nav-refresh"]').trigger('click')
    await flushPromises()
    expect(mocks.browseArchive).toHaveBeenCalledTimes(2)
    expect(wrapper.get('[data-testid="archive-breadcrumbs"]').text()).toContain('docs')
    expect(wrapper.find('footer').text()).toContain('已选择 1 / 2 个文件')

    await wrapper.get('.browser-page').trigger('keydown', { key: 'Backspace' })
    expect(wrapper.get('[data-testid="archive-breadcrumbs"]').text()).not.toContain('docs')
    expect(wrapper.get('[data-entry-path="docs/"]').exists()).toBe(true)

    await wrapper.get('[data-testid="archive-nav-back"]').trigger('click')
    expect(wrapper.get('[data-testid="archive-breadcrumbs"]').text()).toContain('docs')
    await wrapper.get('[data-testid="archive-nav-forward"]').trigger('click')
    expect(wrapper.get('[data-testid="archive-breadcrumbs"]').text()).not.toContain('docs')
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

    expect(wrapper.find('footer').text()).toContain('已选择 2 / 2 个文件')
    await wrapper.get('.preview-trigger').trigger('click')
    await flushPromises()

    expect(mocks.previewArchiveImage).toHaveBeenCalledWith(
      'C:/archives/demo.zip', 'image.png', '',
    )
    expect(wrapper.get('[data-testid="archive-image-preview"] img').attributes('src'))
      .toBe('data:image/png;base64,c2FmZQ==')
    expect(wrapper.get('[data-testid="archive-entry-preview"]').text()).toContain('只读 · 未写入磁盘')
    expect(wrapper.find('footer').text()).toContain('已选择 2 / 2 个文件')

    await wrapper.get('[aria-label="关闭预览"]').trigger('click')
    expect(wrapper.find('[data-testid="archive-image-preview"]').exists()).toBe(false)
  })

  it('previews bounded decoded text and explains internal versus default opening', async () => {
    const wrapper = mount(ArchiveBrowserView, { attachTo: document.body })
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    await wrapper.get('[data-entry-path="docs/"]').trigger('dblclick')
    await wrapper.get('[data-entry-path="docs/readme.txt"] .preview-trigger').trigger('click')
    await flushPromises()

    expect(mocks.previewArchiveText).toHaveBeenCalledWith(
      'C:/archives/demo.zip', 'docs/readme.txt', '',
    )
    expect(wrapper.get('[data-testid="archive-text-preview"]').text()).toContain('你好，归档文本')
    expect(wrapper.get('[data-testid="archive-entry-preview"]').text()).toContain('UTF-8')
    expect(wrapper.get('[data-testid="archive-entry-preview"]').text()).toContain('完整显示')
    expect(wrapper.get('[data-testid="archive-entry-preview"]').text()).toContain('默认应用打开')
    wrapper.unmount()
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

  it('enters nested archives, preserves outer state, and enforces the three-layer UI boundary', async () => {
    mocks.browseArchive
      .mockResolvedValueOnce({
        format: 'ZIP', totalFiles: 2, totalDirectories: 0,
        totalUncompressedSize: 30, totalCompressedSize: 20, encrypted: false,
        entries: [
          { path: 'middle.7z', name: 'middle.7z', size: 20, compressedSize: 12, modified: null, crc: null, encrypted: false, isDir: false },
          { path: 'outer-note.txt', name: 'outer-note.txt', size: 10, compressedSize: 8, modified: null, crc: null, encrypted: false, isDir: false },
        ],
      })
      .mockResolvedValueOnce({
        format: '7Z', totalFiles: 1, totalDirectories: 0,
        totalUncompressedSize: 12, totalCompressedSize: 8, encrypted: false,
        entries: [{ path: 'inner.zip', name: 'inner.zip', size: 12, compressedSize: 8, modified: null, crc: null, encrypted: false, isDir: false }],
      })
      .mockResolvedValueOnce({
        format: 'ZIP', totalFiles: 1, totalDirectories: 0,
        totalUncompressedSize: 6, totalCompressedSize: 4, encrypted: false,
        entries: [{ path: 'fourth.zip', name: 'fourth.zip', size: 6, compressedSize: 4, modified: null, crc: null, encrypted: false, isDir: false }],
      })
    mocks.materializeNestedArchive
      .mockResolvedValueOnce({ entryPath: 'middle.7z', cachePath: 'C:/cache/middle.7z', parentSha256: 'outer-hash', contentSha256: 'middle-hash', depth: 2 })
      .mockResolvedValueOnce({ entryPath: 'inner.zip', cachePath: 'C:/cache/inner.zip', parentSha256: 'middle-hash', contentSha256: 'inner-hash', depth: 3 })

    const wrapper = mount(ArchiveBrowserView, { attachTo: document.body })
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()
    await wrapper.get('[data-entry-path="outer-note.txt"] .browser-checkbox').trigger('click')
    await wrapper.get('[data-entry-path="middle.7z"]').trigger('dblclick')
    await flushPromises()

    expect(mocks.materializeNestedArchive).toHaveBeenLastCalledWith(
      'C:/archives/demo.zip', 'middle.7z', '', 2, [],
    )
    expect(wrapper.get('[data-testid="archive-chain"]').text()).toContain('demo.zip')
    expect(wrapper.get('[data-testid="archive-chain"]').text()).toContain('middle.7z')
    expect(wrapper.get('input[type="password"]').element.value).toBe('')

    await wrapper.get('input[type="password"]').setValue('middle-only-password')
    await wrapper.get('[data-entry-path="inner.zip"]').trigger('dblclick')
    await flushPromises()
    expect(mocks.materializeNestedArchive).toHaveBeenLastCalledWith(
      'C:/cache/middle.7z', 'inner.zip', 'middle-only-password', 3, ['outer-hash', 'middle-hash'],
    )
    expect(wrapper.get('[data-testid="archive-chain"]').text()).toContain('3 / 3 层')

    await wrapper.get('[data-entry-path="fourth.zip"]').trigger('contextmenu', { clientX: 100, clientY: 100 })
    const nestedButton = document.querySelector('[data-testid="archive-context-enter-nested"]') as HTMLButtonElement
    expect(nestedButton.disabled).toBe(true)
    expect(nestedButton.textContent).toContain('已达到 3 层上限')
    ;(document.querySelector('[data-testid="archive-chain"] button') as HTMLButtonElement).click()
    await flushPromises()
    expect(wrapper.get('[data-entry-path="middle.7z"]').exists()).toBe(true)
    expect(wrapper.find('footer').text()).toContain('已选择 1 / 2 个文件')
    wrapper.unmount()
  })

  it('does not inherit an outer password when an inner archive requires its own password', async () => {
    mocks.browseArchive
      .mockResolvedValueOnce({
        format: 'ZIP', totalFiles: 1, totalDirectories: 0,
        totalUncompressedSize: 10, totalCompressedSize: 8, encrypted: true,
        entries: [{ path: 'locked.7z', name: 'locked.7z', size: 10, compressedSize: 8, modified: null, crc: null, encrypted: false, isDir: false }],
      })
      .mockRejectedValueOnce(new Error('Unable to read 7Z metadata: PasswordRequired'))
      .mockResolvedValueOnce({
        format: '7Z', totalFiles: 1, totalDirectories: 0,
        totalUncompressedSize: 4, totalCompressedSize: 4, encrypted: true,
        entries: [{ path: 'secret.txt', name: 'secret.txt', size: 4, compressedSize: 4, modified: null, crc: null, encrypted: true, isDir: false }],
      })
    mocks.materializeNestedArchive.mockResolvedValueOnce({
      entryPath: 'locked.7z', cachePath: 'C:/cache/locked.7z', parentSha256: 'outer-hash', contentSha256: 'locked-hash', depth: 2,
    })
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()
    await wrapper.get('input[type="password"]').setValue('outer-password')
    await wrapper.get('[data-entry-path="locked.7z"]').trigger('dblclick')
    await flushPromises()

    expect(mocks.materializeNestedArchive).toHaveBeenCalledWith(
      'C:/archives/demo.zip', 'locked.7z', 'outer-password', 2, [],
    )
    expect(wrapper.get('input[type="password"]').element.value).toBe('')
    expect(wrapper.text()).toContain('内层归档尚未打开')
    expect(mocks.setError).not.toHaveBeenCalled()
    await wrapper.get('input[type="password"]').setValue('inner-password')
    await wrapper.get('[data-testid="archive-nested-retry"]').trigger('click')
    await flushPromises()
    expect(mocks.browseArchive).toHaveBeenLastCalledWith('C:/cache/locked.7z', 'inner-password', expect.any(String))
  })

  it('ignores a late inner browse result after the user returns to the outer archive', async () => {
    let resolveNested!: (value: any) => void
    const pendingNested = new Promise(resolve => { resolveNested = resolve })
    mocks.browseArchive
      .mockResolvedValueOnce({
        format: 'ZIP', totalFiles: 1, totalDirectories: 0,
        totalUncompressedSize: 10, totalCompressedSize: 8, encrypted: false,
        entries: [{ path: 'slow.7z', name: 'slow.7z', size: 10, compressedSize: 8, modified: null, crc: null, encrypted: false, isDir: false }],
      })
      .mockReturnValueOnce(pendingNested)
    mocks.materializeNestedArchive.mockResolvedValueOnce({
      entryPath: 'slow.7z', cachePath: 'C:/cache/slow.7z', parentSha256: 'outer-hash', contentSha256: 'slow-hash', depth: 2,
    })
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()
    await wrapper.get('[data-entry-path="slow.7z"]').trigger('dblclick')
    await flushPromises()
    await wrapper.get('[data-testid="archive-chain"] button').trigger('click')
    expect(mocks.cancelArchiveBrowse).toHaveBeenCalledWith(expect.any(String))
    resolveNested({
      format: '7Z', totalFiles: 1, totalDirectories: 0,
      totalUncompressedSize: 4, totalCompressedSize: 4, encrypted: false,
      entries: [{ path: 'late.txt', name: 'late.txt', size: 4, compressedSize: 4, modified: null, crc: null, encrypted: false, isDir: false }],
    })
    await flushPromises()
    expect(wrapper.get('[data-entry-path="slow.7z"]').exists()).toBe(true)
    expect(wrapper.find('[data-entry-path="late.txt"]').exists()).toBe(false)
    expect(wrapper.get('[data-testid="archive-chain"]').text()).toContain('1 / 3 层')
  })
})
