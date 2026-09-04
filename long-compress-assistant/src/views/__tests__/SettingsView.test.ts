import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import SettingsView from '../SettingsView.vue'

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))

describe('SettingsView archive engine diagnostics', () => {
  let autoStartEnabled = false

  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
    vi.clearAllMocks()
    autoStartEnabled = false
    Object.defineProperty(window.navigator, 'platform', {
      configurable: true,
      value: 'Win32',
    })
    mocks.invoke.mockImplementation((command: string, payload?: { enable?: boolean }) => {
      if (command === 'check_auto_start') return Promise.resolve(autoStartEnabled)
      if (command === 'set_auto_start') {
        autoStartEnabled = Boolean(payload?.enable)
        return Promise.resolve(autoStartEnabled)
      }
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
    expect(wrapper.text()).toContain('软件更新')
    expect(wrapper.text()).toContain('立即检查更新')
    expect(wrapper.text()).toContain('线上最新版本')
    expect(wrapper.text()).toContain('最近尝试检查')
    expect(wrapper.text()).toContain('最近成功检查')
    expect(wrapper.text()).toContain('尚未检查')
    expect(wrapper.text()).toContain('更新包安装前会验证 Tauri 数字签名')
    expect(wrapper.text()).toContain('保留互联网来源安全标记')
    expect(wrapper.text()).toContain('开机自动启动')
    expect(wrapper.text()).toContain('仅在你点击开启时注册')
    expect(mocks.invoke).toHaveBeenCalledWith('check_auto_start')
    expect(mocks.invoke).not.toHaveBeenCalledWith('set_auto_start', expect.anything())

    const refresh = wrapper.findAll('button').find(button => button.text().includes('重新检测'))
    expect(refresh).toBeTruthy()
    await refresh!.trigger('click')
    await flushPromises()

    expect(mocks.invoke.mock.calls.filter(([command]) => command === 'get_archive_engine_capabilities')).toHaveLength(2)
    expect(mocks.invoke.mock.calls.filter(([command]) => command === 'check_rar_compression_support')).toHaveLength(2)
  })

  it('registers auto-start only after the user toggles it and persists after verification', async () => {
    const wrapper = mount(SettingsView, {
      global: { plugins: [createPinia()] },
    })
    await flushPromises()
    mocks.invoke.mockClear()

    await wrapper.get('[data-testid="auto-start-switch"]').trigger('click')
    await flushPromises()

    expect(mocks.invoke.mock.calls.filter(([command]) => command === 'set_auto_start')).toEqual([
      ['set_auto_start', { enable: true }],
    ])
    expect(wrapper.get('[data-testid="auto-start-switch"]').attributes('aria-checked')).toBe('true')
    expect(JSON.parse(localStorage.getItem('app-settings') || '{}').autoStart).toBe(true)
  })

  it('exposes and persists archive compression and extraction defaults', async () => {
    const wrapper = mount(SettingsView, {
      global: { plugins: [createPinia()] },
    })
    await flushPromises()

    const defaults = wrapper.get('[data-testid="archive-default-settings"]')
    await defaults.get('select').setValue('7z')
    await defaults.get('input[type="range"]').setValue('8')
    await defaults.get('button[role="switch"]').trigger('click')
    await flushPromises()

    const saved = JSON.parse(localStorage.getItem('app-settings') || '{}')
    expect(saved).toMatchObject({
      defaultCompressionFormat: '7z',
      defaultCompressionLevel: 8,
      defaultExtractToSubfolder: true,
    })
  })

  it('keeps the previous setting when Windows registration fails', async () => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === 'check_auto_start') return Promise.resolve(false)
      if (command === 'set_auto_start') return Promise.reject(new Error('Defender blocked the write'))
      if (command === 'get_archive_engine_capabilities') return Promise.resolve({
        available: true,
        version: '26.02',
        fullEngine: true,
        message: 'Full engine ready',
        formats: [],
      })
      if (command === 'check_rar_compression_support') {
        return Promise.resolve({ available: true, message: 'RAR ready' })
      }
      return Promise.resolve(undefined)
    })
    const wrapper = mount(SettingsView, {
      global: { plugins: [createPinia()] },
    })
    await flushPromises()

    await wrapper.get('[data-testid="auto-start-switch"]').trigger('click')
    await flushPromises()

    expect(wrapper.get('[data-testid="auto-start-switch"]').attributes('aria-checked')).toBe('false')
    expect(JSON.parse(localStorage.getItem('app-settings') || '{}').autoStart).not.toBe(true)
  })
})
