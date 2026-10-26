import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { open } from '@tauri-apps/api/dialog'
import DualPaneFileManager from '../DualPaneFileManager.vue'

const invoke = vi.fn()
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: (...args: any[]) => invoke(...args) }))
vi.mock('@tauri-apps/api/dialog', () => ({ open: vi.fn() }))
vi.mock('@/views/ArchiveBrowserView.vue', () => ({ default: { props: ['source', 'destination', 'active'], emits: ['leave'], template: `<div data-testid="pane-archive-stub"><strong>{{ source.path }}</strong><button data-testid="pane-leave-stub" @click="$emit('leave')">返回文件夹</button></div>` } }))

const leftEntries = [
  { path: 'C:\\left\\Folder', name: 'Folder', size: 0, is_dir: true, extension: null },
  { path: 'C:\\left\\photos.zip', name: 'photos.zip', size: 2048, is_dir: false, extension: 'zip' },
  { path: 'C:\\left\\notes.txt', name: 'notes.txt', size: 128, is_dir: false, extension: 'txt' },
]
const rightEntries = [
  { path: 'D:\\Target', name: 'Target', size: 0, is_dir: true, extension: null },
]

describe('DualPaneFileManager', () => {
  it.each(['ARJ', 'Z', 'UDF', 'ZX02', 'ZX100', 'R02', '001'])('opens %s by double click and exposes cross-pane extraction', async extension => {
    const original = invoke.getMockImplementation()!
    const path = 'C:\\left\\资料.' + extension
    invoke.mockImplementation((command, payload) => command === 'list_files' && payload.path === 'C:\\left'
      ? Promise.resolve([{ path, name: '资料.' + extension, size: 128, is_dir: false }]) : original(command, payload))
    const wrapper = mount(DualPaneFileManager, { attachTo: document.body })
    await flushPromises()
    const row = wrapper.findAll('.file-row').find(row => row.text().includes('资料.'))!
    await row.trigger('contextmenu', { clientX: 20, clientY: 20 }); await flushPromises()
    const extract = Array.from(document.body.querySelectorAll('.file-context button')).find(button => button.textContent?.includes('解压到另一栏')) as HTMLButtonElement
    expect(extract).toBeTruthy(); extract.click(); await flushPromises()
    expect(wrapper.emitted('extract')?.[0]).toEqual([[path], 'D:\\'])
    await row.trigger('dblclick'); expect(wrapper.emitted('openArchive')?.[0]).toEqual([path])
    wrapper.unmount()
  })

  it('uses the shared archive filters when choosing a package from disk', async () => {
    vi.mocked(open).mockResolvedValueOnce('C:/资料.udf')
    const wrapper = mount(DualPaneFileManager); await flushPromises()
    await wrapper.get('[data-testid="file-manager-open-archive"]').trigger('click'); await flushPromises()
    expect(open).toHaveBeenCalledWith(expect.objectContaining({ filters: [expect.objectContaining({
      extensions: expect.arrayContaining(['arj', 'z', 'udf']),
    })] }))
    expect(wrapper.emitted('openArchive')?.[0]).toEqual(['C:/资料.udf'])
    wrapper.unmount()
  })

  it('keeps a bookmarked folder after reopening and opens it in the chosen pane only', async () => {
    let wrapper = mount(DualPaneFileManager)
    await flushPromises()
    await wrapper.get('[data-testid="bookmark-current-left"]').trigger('click')
    expect(wrapper.get('[data-testid="bookmark-current-left"]').attributes('aria-pressed')).toBe('true')
    wrapper.unmount()
    wrapper = mount(DualPaneFileManager)
    await flushPromises()
    await wrapper.get('[data-testid="file-manager-path-input-right"]').trigger('focus')
    await wrapper.get('[data-testid="saved-location-right"]').trigger('click')
    await flushPromises()
    expect((wrapper.get('[data-testid="file-manager-path-input-right"]').element as HTMLInputElement).value).toBe('C:\\left')
    expect((wrapper.get('[data-testid="file-manager-path-input-left"]').element as HTMLInputElement).value).toBe('C:\\left')
    wrapper.unmount()
  })

  it('ignores an old directory response after I navigate to another location', async () => {
    const original = invoke.getMockImplementation()!
    let finish!: (entries: unknown[]) => void
    invoke.mockImplementation((command, payload) => command === 'list_files' && payload.path === 'C:/slow'
      ? new Promise(resolve => { finish = resolve }) : original(command, payload))
    const wrapper = mount(DualPaneFileManager)
    await flushPromises()
    const input = wrapper.get('[data-testid="file-manager-path-input-left"]')
    await input.setValue('C:/slow')
    await input.trigger('keydown', { key: 'Enter' })
    await input.setValue('C:/new')
    await input.trigger('keydown', { key: 'Enter' })
    await flushPromises()
    finish([{ path: 'C:/slow/old.txt', name: 'old.txt', is_dir: false, size: 1 }])
    await flushPromises()
    expect((input.element as HTMLInputElement).value).toBe('C:/new')
    expect(wrapper.text()).not.toContain('old.txt')
    wrapper.unmount()
  })

  it('lets me remove an unavailable bookmark without deleting files or changing the other pane', async () => {
    localStorage.setItem('long-saved-locations-v1', JSON.stringify({ bookmarks: ['Z:/missing'], recent: [] }))
    const original = invoke.getMockImplementation()!
    invoke.mockImplementation((command, payload) => command === 'list_files' && payload.path === 'Z:/missing'
      ? Promise.reject(new Error('目录不存在')) : original(command, payload))
    const wrapper = mount(DualPaneFileManager)
    await flushPromises()
    await wrapper.get('[data-testid="file-manager-path-input-left"]').trigger('focus')
    await wrapper.get('[data-testid="saved-location-left"]').trigger('click')
    await flushPromises()
    expect(wrapper.findAll('.file-pane')[0].text()).toContain('目录不存在')
    expect((wrapper.get('[data-testid="file-manager-path-input-right"]').element as HTMLInputElement).value).toBe('D:\\')
    await wrapper.get('[data-testid="remove-unavailable-bookmark-left"]').trigger('click')
    expect(JSON.parse(localStorage.getItem('long-saved-locations-v1')!).bookmarks).toEqual([])
    expect(invoke.mock.calls.some(([command]) => /delete|recycle/.test(command))).toBe(false)
    wrapper.unmount()
  })

  beforeEach(() => {
    localStorage.clear()
    invoke.mockReset()
    invoke.mockImplementation((command: string, payload?: any) => {
      if (command === 'file_manager_locations') return Promise.resolve([
        { name: '主目录', path: 'C:\\left', kind: 'home' },
        { name: 'D 盘', path: 'D:\\', kind: 'drive' },
      ])
      if (command === 'list_files') return Promise.resolve(payload.path === 'C:\\left' ? leftEntries : payload.path === 'D:\\' ? rightEntries : [])
      if (command === 'file_manager_copy') return Promise.resolve({ processed: 1, bytes: 2048 })
      return Promise.resolve(null)
    })
  })

  it('opens as a real dual-pane disk browser instead of an archive dropzone', async () => {
    const wrapper = mount(DualPaneFileManager, { attachTo: document.body })
    await flushPromises()
    expect(wrapper.findAll('.file-pane')).toHaveLength(2)
    expect((wrapper.get('[data-testid="file-manager-path-input-left"]').element as HTMLInputElement).value).toBe('C:\\left')
    expect((wrapper.get('[data-testid="file-manager-path-input-right"]').element as HTMLInputElement).value).toBe('D:\\')
    expect(wrapper.text()).toContain('photos.zip')
    expect(wrapper.text()).not.toContain('把压缩包拖到这里')
    wrapper.unmount()
  })

  it('copies the selected real path to the opposite pane and reports actual bytes', async () => {
    const wrapper = mount(DualPaneFileManager)
    await flushPromises()
    const archive = wrapper.findAll('.file-row').find(row => row.text().includes('photos.zip'))!
    await archive.trigger('click')
    await wrapper.findAll('.transfer-bar button')[0].trigger('click')
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('file_manager_copy', { sources: ['C:\\left\\photos.zip'], destination: 'D:\\' })
    expect(wrapper.get('[data-testid="file-manager-notice"]').text()).toContain('2.0 KB')
  })

  it('routes archive browsing and cross-pane extraction from the context menu', async () => {
    const wrapper = mount(DualPaneFileManager)
    await flushPromises()
    const archive = wrapper.findAll('.file-row').find(row => row.text().includes('photos.zip'))!
    await archive.trigger('contextmenu', { clientX: 20, clientY: 20 })
    await flushPromises()
    const extractButton = Array.from(document.body.querySelectorAll('.file-context button')).find(button => button.textContent?.includes('解压到另一栏')) as HTMLButtonElement
    extractButton.click()
    await flushPromises()
    expect(wrapper.emitted('extract')?.[0]).toEqual([['C:\\left\\photos.zip'], 'D:\\'])
    await archive.trigger('dblclick'); expect(wrapper.emitted('openArchive')?.[0]).toEqual(['C:\\left\\photos.zip'])
  })

  it('offers an explicit selection mode and clears it when exiting', async () => {
    const wrapper = mount(DualPaneFileManager)
    await flushPromises()
    await wrapper.get('[data-testid="file-manager-selection-mode-left"]').trigger('click')
    const leftRows = wrapper.findAll('.file-pane')[0].findAll('.file-row')
    await leftRows[0].trigger('click')
    await leftRows[1].trigger('click')
    expect(wrapper.findAll('.file-pane')[0].text()).toContain('已选择 2 项')
    await wrapper.get('[data-testid="file-manager-selection-mode-left"]').trigger('click')
    expect(wrapper.findAll('.file-pane')[0].text()).toContain('单击选择')
  })

  it('navigates with path breadcrumbs and opens the current folder in the other pane from blank space', async () => {
    const wrapper = mount(DualPaneFileManager, { attachTo: document.body })
    await flushPromises()
    const crumbs = wrapper.get('[data-testid="file-manager-breadcrumbs-left"]').findAll('button')
    expect(crumbs.map(crumb => crumb.text())).toEqual(['C:', 'left'])
    await crumbs[0].trigger('click')
    expect(invoke).toHaveBeenCalledWith('list_files', { path: 'C:\\' })

    await wrapper.findAll('.file-list')[1].trigger('contextmenu', { clientX: 30, clientY: 30 })
    await flushPromises()
    const sameFolder = document.body.querySelector('[data-testid="file-manager-open-same-other"]') as HTMLButtonElement
    expect(sameFolder).toBeTruthy()
    sameFolder.click()
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('list_files', { path: 'D:\\' })
    wrapper.unmount()
  })

  it('uses a styled editable location picker and navigates pasted paths with Enter', async () => {
    const wrapper = mount(DualPaneFileManager)
    await flushPromises()
    const input = wrapper.get('[data-testid="file-manager-path-input-left"]')
    expect(input.attributes('title')).toContain('粘贴路径')

    await input.trigger('focus')
    expect(wrapper.findAll('.location-menu')).toHaveLength(1)
    expect(wrapper.get('.location-menu').text()).toContain('主目录')
    expect(wrapper.get('.location-menu').text()).toContain('D 盘')

    await input.setValue('"E:\\Copied Path"')
    await input.trigger('keydown', { key: 'Enter' })
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('list_files', { path: 'E:\\Copied Path' })
    expect((input.element as HTMLInputElement).value).toBe('E:\\Copied Path')
    expect(wrapper.find('.location-menu').exists()).toBe(false)
  })

  it('uses directional move icons for left and right context menus', async () => {
    const wrapper = mount(DualPaneFileManager, { attachTo: document.body })
    await flushPromises()
    await wrapper.findAll('.file-pane')[0].findAll('.file-row')[0].trigger('contextmenu', { clientX: 30, clientY: 30 })
    let move = Array.from(document.body.querySelectorAll('.file-context button')).find(button => button.textContent?.includes('移动到另一栏'))!
    expect(move.querySelector('i')?.classList.contains('pi-arrow-right')).toBe(true)
    await wrapper.findAll('.file-pane')[1].findAll('.file-row')[0].trigger('contextmenu', { clientX: 30, clientY: 30 })
    move = Array.from(document.body.querySelectorAll('.file-context button')).find(button => button.textContent?.includes('移动到另一栏'))!
    expect(move.querySelector('i')?.classList.contains('pi-arrow-left')).toBe(true)
    wrapper.unmount()
  })

  it.each([
    ['folder', 'Folder', 'C:\\left\\Folder'],
    ['archive', 'photos.zip', 'C:\\left\\photos.zip'],
    ['file', 'notes.txt', 'C:\\left\\notes.txt'],
  ])('opens a %s exact path in the Windows file manager from the context menu', async (_kind, name, path) => {
    const wrapper = mount(DualPaneFileManager, { attachTo: document.body })
    await flushPromises()
    const row = wrapper.findAll('.file-pane')[0].findAll('.file-row').find(candidate => candidate.text().includes(name))!
    await row.trigger('contextmenu', { clientX: 30, clientY: 30 })
    const systemOpen = document.body.querySelector('[data-testid="file-manager-open-system"]') as HTMLButtonElement
    expect(systemOpen).toBeTruthy()
    expect(systemOpen.textContent).toContain(name === 'Folder' ? '在文件管理器中打开' : '在文件管理器中定位')
    systemOpen.click()
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('open_in_explorer', { path })
    wrapper.unmount()
  })

  it('turns protected-folder property failures into a stable friendly prompt', async () => {
    let propertyAttempts = 0
    invoke.mockImplementation((command: string, payload?: any) => {
      if (command === 'file_manager_locations') return Promise.resolve([
        { name: '主目录', path: 'C:\\left', kind: 'home' },
        { name: 'D 盘', path: 'D:\\', kind: 'drive' },
      ])
      if (command === 'list_files') return Promise.resolve(payload.path === 'C:\\left' ? leftEntries : rightEntries)
      if (command === 'file_manager_properties') {
        propertyAttempts += 1
        if (propertyAttempts === 1) return Promise.reject(new Error('REPARSE_POINT_DENIED: raw internal backend detail'))
        return Promise.resolve({
          path: payload.path, name: 'notes.txt', isDir: false, bytes: 128,
          files: 1, directories: 0, modifiedUnixMs: 1,
        })
      }
      return Promise.resolve(null)
    })
    const wrapper = mount(DualPaneFileManager, { attachTo: document.body })
    await flushPromises()
    const folder = wrapper.findAll('.file-pane')[0].findAll('.file-row').find(candidate => candidate.text().includes('Folder'))!
    await folder.trigger('contextmenu', { clientX: 30, clientY: 30 })
    const properties = Array.from(document.body.querySelectorAll('.file-context button')).find(button => button.textContent?.includes('属性')) as HTMLButtonElement
    properties.click()
    await flushPromises()
    const notice = wrapper.get('[data-testid="file-manager-notice"]').text()
    expect(notice).toContain('系统保护或特殊文件夹')
    expect(notice).toContain('Windows 文件管理器')
    expect(notice).not.toContain('REPARSE_POINT_DENIED')
    expect(notice).not.toContain('raw internal backend detail')

    const file = wrapper.findAll('.file-pane')[0].findAll('.file-row').find(candidate => candidate.text().includes('notes.txt'))!
    await file.trigger('contextmenu', { clientX: 30, clientY: 30 })
    const nextProperties = Array.from(document.body.querySelectorAll('.file-context button')).find(button => button.textContent?.includes('属性')) as HTMLButtonElement
    nextProperties.click()
    await flushPromises()
    expect(document.body.querySelector('.properties-dialog')).toBeTruthy()
    expect(wrapper.find('[data-testid="file-manager-notice"]').exists()).toBe(false)
    wrapper.unmount()
  })
  it('keeps both panes and the opposite location/selection when entering and leaving an archive', async () => {
    const wrapper=mount(DualPaneFileManager); await flushPromises()
    await wrapper.get('[data-file-pane="right"] .file-row').trigger('click')
    wrapper.get('[data-file-pane="left"] > .file-pane-content .file-list').element.scrollTop=120
    const row=wrapper.findAll('[data-file-pane="left"] .file-row').find(row=>row.text().includes('photos.zip'))!
    await row.trigger('click'); await row.trigger('dblclick'); await flushPromises()
    expect(wrapper.findAll('.file-pane')).toHaveLength(2)
    expect(wrapper.get('[data-file-pane="left"] [data-testid="pane-archive-stub"]').text()).toContain('photos.zip')
    expect((wrapper.get('[data-testid="file-manager-path-input-right"]').element as HTMLInputElement).value).toBe('D:\\')
    expect(wrapper.get('[data-file-pane="right"] .file-row').classes()).toContain('selected')
    await wrapper.get('[data-testid="pane-leave-stub"]').trigger('click'); await flushPromises()
    expect(wrapper.get('[data-testid="pane-archive-stub"]').isVisible()).toBe(false)
    expect(wrapper.get('[data-file-pane="left"] > .file-pane-content .file-list').element.scrollTop).toBe(120)
    expect(wrapper.findAll('[data-file-pane="left"] .file-row').find(row=>row.text().includes('photos.zip'))!.classes()).toContain('selected')
    wrapper.unmount()
  })
  it('targets an external request to the active pane without replacing the other archive', async () => {
    const wrapper=mount(DualPaneFileManager,{props:{archiveRequest:{id:1,path:'C:\\left\\photos.zip'}}});await flushPromises()
    await wrapper.get('[data-file-pane="right"]').trigger('pointerdown')
    await wrapper.setProps({archiveRequest:{id:2,path:'D:\\second.zip',options:{focusEntryPath:'docs/a.txt',entryAction:'preview'}}});await flushPromises()
    expect(wrapper.findAll('[data-testid="pane-archive-stub"]')).toHaveLength(2)
    expect(wrapper.get('[data-file-pane="left"]').text()).toContain('photos.zip')
    expect(wrapper.get('[data-file-pane="right"]').text()).toContain('second.zip')
    wrapper.unmount()
  })
  it('opens common local text in a preview while keeping the file panes mounted', async () => {
    const original=invoke.getMockImplementation()!
    invoke.mockImplementation((command,args)=>command==='file_manager_preview_text'?Promise.resolve({content:'说明内容',encoding:'UTF-8',lineCount:1,totalSize:12,byteSize:12,truncated:false}):original(command,args))
    const wrapper=mount(DualPaneFileManager,{global:{stubs:{Teleport:true}}});await flushPromises()
    const row=wrapper.findAll('.file-row').find(row=>row.text().includes('notes.txt'))!
    await row.trigger('dblclick');await flushPromises()
    expect(wrapper.get('[data-testid="local-text-preview"]').text()).toBe('说明内容')
    expect(wrapper.findAll('.file-pane')).toHaveLength(2)
    expect(invoke).toHaveBeenCalledWith('file_manager_preview_text',{path:'C:\\left\\notes.txt'})
    expect(invoke.mock.calls.some(([command])=>command==='open_in_explorer')).toBe(false)
    await wrapper.get('[data-testid="local-preview-close"]').trigger('click');await flushPromises()
    expect(wrapper.find('[data-testid="local-file-preview"]').exists()).toBe(false)
    wrapper.unmount()
  })

})
