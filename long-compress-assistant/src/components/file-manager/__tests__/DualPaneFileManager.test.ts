import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import DualPaneFileManager from '../DualPaneFileManager.vue'

const invoke = vi.fn()
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: (...args: any[]) => invoke(...args) }))

const leftEntries = [
  { path: 'C:\\left\\Folder', name: 'Folder', size: 0, is_dir: true, extension: null },
  { path: 'C:\\left\\photos.zip', name: 'photos.zip', size: 2048, is_dir: false, extension: 'zip' },
]

describe('DualPaneFileManager', () => {
  beforeEach(() => {
    invoke.mockReset()
    invoke.mockImplementation((command: string, payload?: any) => {
      if (command === 'file_manager_locations') return Promise.resolve([
        { name: '主目录', path: 'C:\\left', kind: 'home' },
        { name: 'D 盘', path: 'D:\\', kind: 'drive' },
      ])
      if (command === 'list_files') return Promise.resolve(payload.path === 'C:\\left' ? leftEntries : [])
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
})
