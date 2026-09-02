import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import DualPaneFileManager from '../DualPaneFileManager.vue'

const invoke = vi.fn()
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: (...args: any[]) => invoke(...args) }))
vi.mock('@tauri-apps/api/dialog', () => ({ open: vi.fn() }))

const leftEntries = [
  { path: 'C:\\left\\Folder', name: 'Folder', size: 0, is_dir: true, extension: null },
  { path: 'C:\\left\\photos.zip', name: 'photos.zip', size: 2048, is_dir: false, extension: 'zip' },
  { path: 'C:\\left\\notes.txt', name: 'notes.txt', size: 128, is_dir: false, extension: 'txt' },
]
const rightEntries = [
  { path: 'D:\\Target', name: 'Target', size: 0, is_dir: true, extension: null },
]

describe('DualPaneFileManager', () => {
  beforeEach(() => {
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
    expect(wrapper.text()).toContain('C:\\left')
    expect(wrapper.text()).toContain('D:\\')
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
    await archive.trigger('dblclick')
    expect(wrapper.emitted('openArchive')?.[0]).toEqual(['C:\\left\\photos.zip'])
    await archive.trigger('contextmenu', { clientX: 20, clientY: 20 })
    await flushPromises()
    const extractButton = Array.from(document.body.querySelectorAll('.file-context button')).find(button => button.textContent?.includes('解压到另一栏')) as HTMLButtonElement
    extractButton.click()
    await flushPromises()
    expect(wrapper.emitted('extract')?.[0]).toEqual([['C:\\left\\photos.zip'], 'D:\\'])
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
    invoke.mockImplementation((command: string, payload?: any) => {
      if (command === 'file_manager_locations') return Promise.resolve([
        { name: '主目录', path: 'C:\\left', kind: 'home' },
        { name: 'D 盘', path: 'D:\\', kind: 'drive' },
      ])
      if (command === 'list_files') return Promise.resolve(payload.path === 'C:\\left' ? leftEntries : rightEntries)
      if (command === 'file_manager_properties') return Promise.reject(new Error('REPARSE_POINT_DENIED: raw internal backend detail'))
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
    wrapper.unmount()
  })
})
