import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import ArchiveOpeningSettings from '../ArchiveOpeningSettings.vue'

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
const formats = ['zip','7z','rar','zipx','tar','gz','bz2','xz','zst','tgz','tbz2','txz','lz','lzo','lz4','br','cab','arj','alz','egg','zpaq']
let defaults: string[]
let view: VueWrapper
const states = () => formats.map(extension => ({ extension: '.' + extension, registered: true, isDefault: defaults.includes(extension) }))
beforeEach(() => {
  localStorage.clear()
  setActivePinia(createPinia())
  Object.defineProperty(navigator, 'platform', { configurable: true, value: 'Win32' })
  defaults = ['zip', 'rar']
  mocks.invoke.mockReset()
  mocks.invoke.mockImplementation(async (command: string) => {
    if (command === 'load_app_settings') return '{}'
    if (command === 'archive_file_associations') return states()
  })
  view = mount(ArchiveOpeningSettings, { global: { stubs: {
    Modal: { props: ['visible'], template: '<div v-if="visible" role="dialog"><slot/><footer><slot name="footer"/></footer></div>' },
    HelpHint: true, ThemedSelect: true,
  } } })
})
afterEach(() => view.unmount())
const open = async () => { await flushPromises(); await view.get('[data-testid="archive-association-configure"]').trigger('click'); await flushPromises() }
const save = async () => { await view.get('[data-testid="archive-association-save"]').trigger('click'); await flushPromises() }

it('starts from effective defaults, enables default checkboxes, and keeps one management entry', async () => {
  await open()
  expect(view.findAll('input:checked')).toHaveLength(2)
  expect(view.get('input[value=".rar"]').attributes('disabled')).toBeUndefined()
  expect(view.text()).not.toContain('更改 / 取消默认')
  await view.get('[data-testid="archive-association-select-all"]').trigger('click')
  expect(view.findAll('input:checked')).toHaveLength(21)
  await view.get('[data-testid="archive-association-clear-all"]').trigger('click')
  expect(view.findAll('input:checked')).toHaveLength(0)
})

it('submits all 21 formats and reports partial system confirmation without closing the dialog', async () => {
  await open()
  await view.get('[data-testid="archive-association-select-all"]').trigger('click')
  await save()
  expect(mocks.invoke).toHaveBeenCalledWith('configure_archive_file_associations', { extensions: formats.map(extension => '.' + extension) })
  expect(mocks.invoke).toHaveBeenCalledWith('open_default_archive_apps')
  expect(view.get('[role="dialog"]').exists()).toBe(true)
  expect(view.get('[data-testid="archive-association-notice"]').text()).toContain('19 种')
  defaults = [...formats]
  window.dispatchEvent(new Event('focus'))
  await flushPromises()
  expect(view.text()).toContain('已全部生效')
  expect(view.findAll('input:checked')).toHaveLength(21)
  await save()
  expect(view.find('[role="dialog"]').exists()).toBe(false)
  expect(mocks.invoke.mock.calls.filter(([command]) => command === 'open_default_archive_apps')).toHaveLength(1)
})

it('accepts an empty selection, retains pending cancellation, and refreshes after Windows replaces it', async () => {
  await open()
  await view.get('[data-testid="archive-association-clear-all"]').trigger('click')
  await save()
  expect(mocks.invoke).toHaveBeenCalledWith('configure_archive_file_associations', { extensions: [] })
  expect(view.text()).toContain('待更换')
  expect(view.findAll('input:checked')).toHaveLength(0)
  defaults = []
  window.dispatchEvent(new Event('focus'))
  await flushPromises()
  expect(view.text()).not.toContain('待更换')
  await save()
  expect(view.find('[role="dialog"]').exists()).toBe(false)
  expect(view.text()).toContain('尚未设为默认打开应用')
})

it('keeps failures visible and retries the system launch without writing registrations twice', async () => {
  const invoke = mocks.invoke.getMockImplementation()!
  mocks.invoke.mockImplementation(async (command: string, ...args: unknown[]) => {
    if (command === 'open_default_archive_apps') throw new Error('无法打开 Windows 默认应用设置')
    return invoke(command, ...args)
  })
  await open()
  await view.get('[data-testid="archive-association-select-all"]').trigger('click')
  await save()
  expect(view.get('[role="alert"]').text()).toContain('无法打开 Windows 默认应用设置')
  expect(view.findAll('input:checked')).toHaveLength(21)
  await save()
  expect(mocks.invoke.mock.calls.filter(([command]) => command === 'configure_archive_file_associations')).toHaveLength(1)
  expect(mocks.invoke.mock.calls.filter(([command]) => command === 'open_default_archive_apps')).toHaveLength(2)
})

it('does not open system settings when the requested state already matches', async () => {
  await open()
  await save()
  expect(view.find('[role="dialog"]').exists()).toBe(false)
  expect(mocks.invoke).not.toHaveBeenCalledWith('open_default_archive_apps')
})

it('does not offer an empty dialog as a successful inspection', async () => {
  const invoke = mocks.invoke.getMockImplementation()!
  mocks.invoke.mockImplementation((command: string, ...args: unknown[]) => command === 'archive_file_associations' ? Promise.resolve([]) : invoke(command, ...args))
  await open()
  expect(view.find('[role="dialog"]').exists()).toBe(false)
  expect(view.get('[role="alert"]').text()).toContain('无法读取')
})
