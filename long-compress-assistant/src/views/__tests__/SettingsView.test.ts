import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import SettingsView from '../SettingsView.vue'

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))

describe('SettingsView archive engine diagnostics', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    mocks.invoke.mockImplementation((command: string) => {
      if (command === 'get_archive_engine_capabilities') {
        return Promise.resolve({
          available: true,
          version: '26.02',
          fullEngine: true,
          message: 'Full engine ready',
          formats: [
            { name: 'zip', extensions: ['zip', 'zipx'], canCreate: true },
            { name: 'wim', extensions: ['wim', 'esd'], canCreate: true },
            { name: 'APFS', extensions: ['apfs'], canCreate: false },
          ],
        })
      }
      if (command === 'check_rar_compression_support') {
        return Promise.resolve({ available: true, message: 'RAR ready' })
      }
      if (command === 'check_auto_start') return Promise.resolve(false)
      return Promise.resolve(undefined)
    })
  })

  it('shows live engine capabilities and can detect them again', async () => {
    const wrapper = mount(SettingsView, {
      global: { plugins: [createPinia()] },
    })
    await flushPromises()

    expect(wrapper.text()).toContain('7-Zip 26.02')
    expect(wrapper.text()).toContain('Full engine ready')
    expect(wrapper.text()).toContain('RAR')
    expect(wrapper.text()).toContain('WIM')
    expect(wrapper.text()).toContain('密码解压: ZIP · 7Z · RAR · TAR.AES')
    expect(wrapper.text()).toContain('TGZ.AES')
    expect(wrapper.text()).toContain('ZST.AES')

    const refresh = wrapper.findAll('button').find(button => button.text().includes('重新检测'))
    expect(refresh).toBeTruthy()
    await refresh!.trigger('click')
    await flushPromises()

    expect(mocks.invoke.mock.calls.filter(([command]) => command === 'get_archive_engine_capabilities')).toHaveLength(2)
    expect(mocks.invoke.mock.calls.filter(([command]) => command === 'check_rar_compression_support')).toHaveLength(2)
  })
})
