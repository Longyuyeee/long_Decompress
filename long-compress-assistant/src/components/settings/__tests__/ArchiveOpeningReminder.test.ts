import { beforeEach, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import ArchiveOpeningReminder from '../ArchiveOpeningReminder.vue'
import { useAppStore } from '@/stores/app'

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), push: vi.fn() }))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('vue-router', () => ({ useRouter: () => ({ push: mocks.push, isReady: () => Promise.resolve() }) }))
let consumed = false
let isDefault = false
beforeEach(() => {
  localStorage.clear()
  setActivePinia(createPinia())
  vi.clearAllMocks()
  consumed = false
  isDefault = false
  Object.defineProperty(navigator, 'platform', { configurable: true, value: 'Win32' })
  mocks.invoke.mockImplementation(async (command: string) => {
    if (command === 'load_app_settings') return '{}'
    if (command === 'is_context_menu_registered') return true
    if (command === 'archive_file_associations') return [{ isDefault }]
    if (command === 'take_archive_opening_reminder') {
      const first = !consumed
      consumed = true
      return first
    }
  })
})

it('waits for saved preferences, then offers settings without opening Windows settings or changing defaults', async () => {
  const app = useAppStore()
  await flushPromises()
  app.settingsLoaded = false
  const view = mount(ArchiveOpeningReminder)
  await flushPromises()
  expect(view.find('aside').exists()).toBe(false)
  app.settingsLoaded = true
  await flushPromises()
  expect(view.find('aside').exists()).toBe(true)
  expect(mocks.invoke).not.toHaveBeenCalledWith('configure_archive_file_associations', expect.anything())
  expect(mocks.invoke).not.toHaveBeenCalledWith('open_default_archive_apps')
  await view.get('[data-testid="archive-opening-reminder-configure"]').trigger('click')
  expect(mocks.push).toHaveBeenCalledWith({ path: '/settings', query: { section: 'behavior', focus: 'archive-opening' } })
  expect(view.find('aside').exists()).toBe(false)
  view.unmount()
})

it('dismissing counts as reminded; remounting and resetting preferences do not repeat the suggestion', async () => {
  const app = useAppStore()
  await flushPromises()
  const view = mount(ArchiveOpeningReminder)
  await flushPromises()
  await view.get('[aria-label="关闭默认打开提醒"]').trigger('click')
  view.unmount()
  app.resetSettings()
  const reopened = mount(ArchiveOpeningReminder)
  await flushPromises()
  expect(reopened.find('aside').exists()).toBe(false)
  reopened.unmount()
})

it('does not remind users who already chose Long解压 as a default', async () => {
  isDefault = true
  const view = mount(ArchiveOpeningReminder)
  await flushPromises()
  expect(view.find('aside').exists()).toBe(false)
  expect(consumed).toBe(true)
  view.unmount()
})

it('keeps startup usable when association inspection is unavailable', async () => {
  const implementation = mocks.invoke.getMockImplementation()!
  mocks.invoke.mockImplementation((command: string) => command === 'archive_file_associations' ? Promise.reject(new Error('unavailable')) : implementation(command))
  const view = mount(ArchiveOpeningReminder)
  await flushPromises()
  expect(view.find('aside').exists()).toBe(false)
  expect(consumed).toBe(false)
  view.unmount()
})
