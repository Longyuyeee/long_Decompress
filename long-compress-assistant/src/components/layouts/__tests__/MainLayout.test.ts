import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createMemoryHistory, createRouter } from 'vue-router'
import MainLayout from '../MainLayout.vue'

const mocks = vi.hoisted(() => ({
  onFocusChanged: vi.fn(async () => vi.fn()),
  getVersion: vi.fn(async () => '1.1.8'),
  t: vi.fn((key: string) => key),
  isMaximized: vi.fn(async () => false),
  scaleFactor: vi.fn(async () => 1),
  outerPosition: vi.fn(async () => ({ x: 100, y: 80 })),
  outerSize: vi.fn(async () => ({ width: 920, height: 620 })),
  setPosition: vi.fn(async () => undefined),
  setSize: vi.fn(async () => undefined),
}))

vi.mock('@tauri-apps/api/app', () => ({ getVersion: mocks.getVersion }))

vi.mock('@tauri-apps/api/window', () => ({
  LogicalPosition: class { constructor(public x: number, public y: number) {} },
  LogicalSize: class { constructor(public width: number, public height: number) {} },
  appWindow: {
    onFocusChanged: mocks.onFocusChanged,
    isMaximized: mocks.isMaximized,
    scaleFactor: mocks.scaleFactor,
    outerPosition: mocks.outerPosition,
    outerSize: mocks.outerSize,
    setPosition: mocks.setPosition,
    setSize: mocks.setSize,
  }
}))

vi.mock('@/stores/app', () => ({
  useAppStore: () => ({
    t: mocks.t
  })
}))

vi.mock('@/components/layouts/WindowTitleBar.vue', () => ({
  default: { template: '<div data-test="titlebar" />' }
}))

vi.mock('@/components/ui/GlobalProgressBar.vue', () => ({
  default: { template: '<div data-test="global-progress" />' }
}))

vi.mock('@/components/ui/PerformanceMeter.vue', () => ({
  default: { template: '<div data-test="performance-meter" />' }
}))

const routes = [
  { path: '/', redirect: { name: 'Decompress' } },
  { path: '/decompress', name: 'Decompress', component: { template: '<div>Decompress</div>' } },
  { path: '/compress', name: 'Compress', component: { template: '<div>Compress</div>' } },
  { path: '/special-compression', name: 'SpecialCompression', component: { template: '<div>Special Compression</div>' } },
  { path: '/browser', name: 'ArchiveBrowser', component: { template: '<div>Browser</div>' } },
  { path: '/vault', name: 'Vault', component: { template: '<div>Vault</div>' } },
  { path: '/integrity', name: 'FileIntegrity', component: { template: '<div>Integrity</div>' } },
  { path: '/history', name: 'History', component: { template: '<div>History</div>' } },
  { path: '/settings', name: 'Settings', component: { template: '<div>Settings</div>' } }
]

const mountLayout = async (initialRoute = '/decompress') => {
  const router = createRouter({
    history: createMemoryHistory(),
    routes
  })

  router.push(initialRoute)
  await router.isReady()

  const wrapper = mount(MainLayout, {
    global: {
      plugins: [router]
    }
  })

  await flushPromises()
  return { wrapper, router }
}

describe('MainLayout', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders the main shell and child infrastructure', async () => {
    const { wrapper } = await mountLayout()

    expect(wrapper.find('.main-container').exists()).toBe(true)
    expect(wrapper.find('[data-test="titlebar"]').exists()).toBe(true)
    expect(wrapper.find('aside nav').exists()).toBe(true)
    expect(wrapper.find('main').exists()).toBe(true)
    expect(wrapper.find('[data-test="global-progress"]').exists()).toBe(true)
    expect(wrapper.findAll('[data-resize-edge]')).toHaveLength(8)
    expect(wrapper.text()).not.toContain('本地处理 · 隐私优先')
    expect(wrapper.get('[data-testid="sidebar-version-badge"]').text()).toBe('v1.1.8')
    expect(mocks.getVersion).toHaveBeenCalledOnce()
    expect(mocks.onFocusChanged).toHaveBeenCalledOnce()
  })

  it('resizes a frameless window from the custom south-east edge', async () => {
    const { wrapper } = await mountLayout()
    await wrapper.get('[data-resize-edge="se"]').trigger('pointerdown', {
      button: 0, pointerId: 1, screenX: 1000, screenY: 700,
    })
    await flushPromises()
    window.dispatchEvent(new MouseEvent('pointermove', { screenX: 1040, screenY: 730 }))
    await vi.waitFor(() => expect(mocks.setSize).toHaveBeenCalled())
    expect(mocks.setSize.mock.calls.at(-1)?.[0]).toMatchObject({ width: 960, height: 650 })
    window.dispatchEvent(new MouseEvent('pointerup'))
  })

  it('renders the eight product navigation entries with special compression beside the file browser', async () => {
    const { wrapper } = await mountLayout()

    expect(wrapper.find('.pi-folder-open').exists()).toBe(true)
    expect(wrapper.find('.pi-box').exists()).toBe(true)
    expect(wrapper.find('.pi-sparkles').exists()).toBe(true)
    expect(wrapper.find('.pi-folder').exists()).toBe(true)
    expect(wrapper.find('.pi-shield').exists()).toBe(true)
    expect(wrapper.find('.pi-verified').exists()).toBe(true)
    expect(wrapper.find('.pi-history').exists()).toBe(true)
    expect(wrapper.find('.pi-cog').exists()).toBe(true)
    expect(mocks.t).toHaveBeenCalledWith('nav.decompress')
    expect(mocks.t).toHaveBeenCalledWith('nav.compress')
    expect(mocks.t).toHaveBeenCalledWith('nav.special_compression')
    expect(wrapper.get('[data-testid="nav-SpecialCompression"]').attributes('aria-label')).toContain('Ctrl+Shift+S')
    expect(mocks.t).toHaveBeenCalledWith('nav.browser')
    expect(mocks.t).toHaveBeenCalledWith('nav.vault')
    expect(mocks.t).toHaveBeenCalledWith('nav.integrity')
    expect(mocks.t).toHaveBeenCalledWith('nav.history')
    expect(mocks.t).toHaveBeenCalledWith('nav.settings')
  })

  it('highlights the active route', async () => {
    const { wrapper } = await mountLayout('/compress')

    const navButtons = wrapper.findAll('aside nav > button')
    expect(navButtons).toHaveLength(8)
    expect(navButtons[6].attributes('data-testid')).toBe('nav-History')
    expect(navButtons[7].attributes('data-testid')).toBe('nav-Settings')
    expect(navButtons[1].classes()).toContain('bg-primary/20')
    expect(navButtons[0].classes()).toContain('hover:bg-primary/8')
    expect(navButtons[1].attributes('aria-current')).toBe('page')
  })

  it('navigates when a sidebar item is clicked', async () => {
    const { wrapper, router } = await mountLayout('/decompress')

    await wrapper.get('[data-testid="nav-Vault"]').trigger('click')
    await flushPromises()

    expect(router.currentRoute.value.name).toBe('Vault')
  })

  it('keeps rendering when the native focus listener is unavailable', async () => {
    const warning = vi.spyOn(console, 'warn').mockImplementation(() => {})
    mocks.onFocusChanged.mockRejectedValueOnce(new Error('not running in Tauri'))

    const { wrapper } = await mountLayout()

    expect(wrapper.find('main').exists()).toBe(true)
    expect(warning).toHaveBeenCalledWith(
      'Window focus listener is unavailable:',
      expect.any(Error)
    )
    warning.mockRestore()
  })
})
