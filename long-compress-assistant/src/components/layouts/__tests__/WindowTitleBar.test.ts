import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import WindowTitleBar from '../WindowTitleBar.vue'

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  minimize: vi.fn(),
  toggleMaximize: vi.fn(),
  close: vi.fn(),
}))

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/api/window', () => ({
  appWindow: {
    minimize: mocks.minimize,
    toggleMaximize: mocks.toggleMaximize,
    close: mocks.close,
  },
}))

describe('WindowTitleBar', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.invoke.mockResolvedValue(undefined)
    mocks.minimize.mockResolvedValue(undefined)
    mocks.toggleMaximize.mockResolvedValue(undefined)
    mocks.close.mockResolvedValue(undefined)
  })

  it('exposes accessible controls and forwards each native window action', async () => {
    const wrapper = mount(WindowTitleBar, {
      global: { plugins: [createPinia()] },
    })

    expect(wrapper.get('[data-tauri-drag-region]').text()).toContain('Long解压')

    const minimize = wrapper.get('button[aria-label="最小化"]')
    const maximize = wrapper.get('button[aria-label="最大化"]')
    const close = wrapper.get('button[aria-label="关闭"]')
    expect(minimize.attributes('title')).toBe('最小化')
    expect(maximize.attributes('title')).toBe('最大化')
    expect(close.attributes('title')).toBe('关闭')

    await minimize.trigger('click')
    await maximize.trigger('click')
    await close.trigger('click')

    expect(mocks.minimize).toHaveBeenCalledOnce()
    expect(mocks.toggleMaximize).toHaveBeenCalledOnce()
    expect(mocks.close).toHaveBeenCalledOnce()
    wrapper.unmount()
  })
})
