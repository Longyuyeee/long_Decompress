import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import MainLayout from '../MainLayout.vue'
import { createPinia, setActivePinia } from 'pinia'

// Mock PrimeVue图标
vi.mock('primevue/icons', () => ({
  Box: { template: '<i class="pi pi-box"></i>' },
  Home: { template: '<i class="pi pi-home"></i>' },
  FileImport: { template: '<i class="pi pi-file-import"></i>' },
  Cog: { template: '<i class="pi pi-cog"></i>' },
  Search: { template: '<i class="pi pi-search"></i>' },
  Sun: { template: '<i class="pi pi-sun"></i>' },
  Moon: { template: '<i class="pi pi-moon"></i>' },
  Bell: { template: '<i class="pi pi-bell"></i>' },
  User: { template: '<i class="pi pi-user"></i>' },
  ChevronDown: { template: '<i class="pi pi-chevron-down"></i>' },
  Times: { template: '<i class="pi pi-times"></i>' },
  InfoCircle: { template: '<i class="pi pi-info-circle"></i>' },
  CheckCircle: { template: '<i class="pi pi-check-circle"></i>' },
  ExclamationTriangle: { template: '<i class="pi pi-exclamation-triangle"></i>' },
  TimesCircle: { template: '<i class="pi pi-times-circle"></i>' },
  SignOut: { template: '<i class="pi pi-sign-out"></i>' },
}))

// Mock store
const mockAppStore = {
  currentTheme: 'light',
  updateSettings: vi.fn(),
}

vi.mock('@/stores/app', () => ({
  useAppStore: () => mockAppStore,
}))

// Mock utils
vi.mock('@/utils', () => ({
  formatRelativeTime: (date: Date) => {
    const diff = Date.now() - date.getTime()
    if (diff < 60000) return '刚刚'
    if (diff < 3600000) return `${Math.floor(diff / 60000)}分钟前`
    if (diff < 86400000) return `${Math.floor(diff / 3600000)}小时前`
    return `${Math.floor(diff / 86400000)}天前`
  },
}))

// 创建测试路由
const routes = [
  { path: '/', name: 'Home', component: { template: '<div>Home</div>' } },
  { path: '/decompress', name: 'Decompress', component: { template: '<div>Decompress</div>' } },
  { path: '/settings', name: 'Settings', component: { template: '<div>Settings</div>' } },
  { path: '/about', name: 'About', component: { template: '<div>About</div>' } },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

describe('MainLayout Component', () => {
  let wrapper: any

  beforeEach(async () => {
    setActivePinia(createPinia())

    // 重置mock
    vi.clearAllMocks()
    mockAppStore.currentTheme = 'light'
    mockAppStore.updateSettings.mockClear()

    // 挂载组件
    wrapper = mount(MainLayout, {
      global: {
        plugins: [router],
      },
    })

    // 等待路由初始?
    await router.push('/')
    await flushPromises()
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  it('renders correctly with all main elements', () => {
    expect(wrapper.exists()).toBe(true)

    // 检查主要元?
    expect(wrapper.find('header').exists()).toBe(true)
    expect(wrapper.find('nav').exists()).toBe(true)
    expect(wrapper.find('input[type="text"]').exists()).toBe(true)
    expect(wrapper.find('button[title*="主题"]').exists()).toBe(true)
    expect(wrapper.find('button .pi-bell').exists()).toBe(true)
    expect(wrapper.find('button .pi-user').exists()).toBe(true)
  })

  it('displays correct app title and description', () => {
    expect(wrapper.text()).toContain('胧压缩·方便助?)'
    expect(wrapper.text()).toContain('智能文件解压工具')
  })

  it('renders navigation items correctly', () => {
    const navItems = wrapper.vm.navItems
    expect(navItems).toHaveLength(3)

    expect(navItems[0]).toEqual({ to: '/', label: '首页', icon: 'pi-home' })
    expect(navItems[1]).toEqual({ to: '/decompress', label: '解压', icon: 'pi-file-import' })
    expect(navItems[2]).toEqual({ to: '/settings', label: '设置', icon: 'pi-cog' })
  })

  it('highlights active navigation item', async () => {
    // 导航到首?
    await router.push('/')
    await flushPromises()

    const homeLink = wrapper.find('nav a[href="/"]')
    expect(homeLink.classes()).toContain('bg-primary/10')
    expect(homeLink.classes()).toContain('text-primary')

    // 导航到解压页?
    await router.push('/decompress')
    await flushPromises()

    const decompressLink = wrapper.find('nav a[href="/decompress"]')
    expect(decompressLink.classes()).toContain('bg-primary/10')
    expect(decompressLink.classes()).toContain('text-primary')
  })

  it('toggles theme when theme button is clicked', async () => {
    const themeButton = wrapper.find('button[title*="主题"]')

    // 初始状?
    expect(mockAppStore.currentTheme).toBe('light')
    expect(themeButton.attributes('title')).toBe('切换到暗色主?)'
    expect(themeButton.find('.pi-moon').exists()).toBe(true)

    // 点击切换主题
    await themeButton.trigger('click')

    expect(mockAppStore.updateSettings).toHaveBeenCalledWith({ theme: 'dark' })

    // 模拟store更新
    mockAppStore.currentTheme = 'dark'
    await wrapper.vm.$nextTick()

    expect(themeButton.attributes('title')).toBe('切换到亮色主?)'
    expect(themeButton.find('.pi-sun').exists()).toBe(true)
  })

  it('toggles user menu when user button is clicked', async () => {
    const userButton = wrapper.find('button .pi-user').parent()

    expect(wrapper.vm.showUserMenu).toBe(false)

    await userButton.trigger('click')
    expect(wrapper.vm.showUserMenu).toBe(true)

    await userButton.trigger('click')
    expect(wrapper.vm.showUserMenu).toBe(false)
  })

  it('toggles notifications panel when bell button is clicked', async () => {
    const bellButton = wrapper.find('button .pi-bell').parent()

    expect(wrapper.vm.showNotifications).toBe(false)

    await bellButton.trigger('click')
    expect(wrapper.vm.showNotifications).toBe(true)

    await bellButton.trigger('click')
    expect(wrapper.vm.showNotifications).toBe(false)
  })

  it('shows notification count badge', () => {
    const bellButton = wrapper.find('button .pi-bell').parent()
    const badge = bellButton.find('span.bg-red-500')

    expect(badge.exists()).toBe(true)
    expect(badge.text()).toBe('2') // 初始?个通知
  })

  it('displays user menu dropdown when showUserMenu is true', async () => {
    wrapper.vm.showUserMenu = true
    await wrapper.vm.$nextTick()

    const userMenu = wrapper.find('.absolute.right-0.mt-2')
    expect(userMenu.exists()).toBe(true)
    expect(userMenu.text()).toContain('用户')
    expect(userMenu.text()).toContain('管理?)'
    expect(userMenu.text()).toContain('设置')
    expect(userMenu.text()).toContain('关于')
    expect(userMenu.text()).toContain('退出登?)'
  })

  it('closes user menu when clicking outside', async () => {
    wrapper.vm.showUserMenu = true
    await wrapper.vm.$nextTick()

    // 模拟点击外部
    const clickEvent = new MouseEvent('click')
    wrapper.vm.closeMenus(clickEvent)

    expect(wrapper.vm.showUserMenu).toBe(false)
  })

  it('displays notifications panel when showNotifications is true', async () => {
    wrapper.vm.showNotifications = true
    await wrapper.vm.$nextTick()

    const notificationsPanel = wrapper.find('.fixed.inset-0')
    expect(notificationsPanel.exists()).toBe(true)
    expect(notificationsPanel.text()).toContain('通知')
    expect(notificationsPanel.text()).toContain('欢迎使用')
    expect(notificationsPanel.text()).toContain('解压完成')
    expect(notificationsPanel.text()).toContain('清除所有通知')
  })

  it('clears all notifications when clear button is clicked', async () => {
    wrapper.vm.showNotifications = true
    await wrapper.vm.$nextTick()

    const clearButton = wrapper.find('button:contains("清除所有通知")')
    await clearButton.trigger('click')

    expect(wrapper.vm.notifications).toHaveLength(0)
    expect(wrapper.vm.showNotifications).toBe(false)
  })

  it('formats notification time correctly', () => {
    const notifications = wrapper.vm.notifications

    // 第一个通知?小时?
    expect(wrapper.vm.formatRelativeTime(notifications[0].createdAt)).toContain('小时?)'

    // 第二个通知?0分钟?
    expect(wrapper.vm.formatRelativeTime(notifications[1].createdAt)).toContain('分钟?)'
  })

  it('handles logout function', async () => {
    const consoleSpy = vi.spyOn(console, 'log')

    wrapper.vm.showUserMenu = true
    await wrapper.vm.$nextTick()

    const logoutButton = wrapper.find('button:contains("退出登?)')\"
    await logoutButton.trigger('click')

    expect(consoleSpy).toHaveBeenCalledWith('退出登?)'
    expect(wrapper.vm.showUserMenu).toBe(false)
  })

  it('computes current page title based on route', async () => {
    // 首页
    await router.push('/')
    await flushPromises()
    expect(wrapper.vm.currentPageTitle).toBe('首页')

    // 解压页面
    await router.push('/decompress')
    await flushPromises()
    expect(wrapper.vm.currentPageTitle).toBe('文件解压')

    // 设置页面
    await router.push('/settings')
    await flushPromises()
    expect(wrapper.vm.currentPageTitle).toBe('设置')

    // 关于页面
    await router.push('/about')
    await flushPromises()
    expect(wrapper.vm.currentPageTitle).toBe('关于')
  })

  it('computes unread notifications count', () => {
    expect(wrapper.vm.unreadNotifications).toBe(2)

    // 清除通知?
    wrapper.vm.notifications = []
    expect(wrapper.vm.unreadNotifications).toBe(0)
  })

  it('computes user name and app version', () => {
    expect(wrapper.vm.userName).toBe('用户')
    expect(wrapper.vm.appVersion).toBe('1.0.0')
  })

  it('closes menus when clicking on another menu button', async () => {
    // 打开用户菜单
    wrapper.vm.showUserMenu = true
    await wrapper.vm.$nextTick()

    // 点击通知按钮
    const bellButton = wrapper.find('button .pi-bell').parent()
    await bellButton.trigger('click')

    expect(wrapper.vm.showUserMenu).toBe(false)
    expect(wrapper.vm.showNotifications).toBe(true)

    // 点击用户按钮
    const userButton = wrapper.find('button .pi-user').parent()
    await userButton.trigger('click')

    expect(wrapper.vm.showNotifications).toBe(false)
    expect(wrapper.vm.showUserMenu).toBe(true)
  })

  it('has search functionality', () => {
    const searchInput = wrapper.find('input[type="text"]')
    expect(searchInput.exists()).toBe(true)
    expect(searchInput.attributes('placeholder')).toBe('搜索文件或设?..')
  })

  it('applies correct theme classes', async () => {
    // 亮色主题
    mockAppStore.currentTheme = 'light'
    await wrapper.vm.$nextTick()

    const header = wrapper.find('header')
    expect(header.classes()).toContain('bg-white/80')

    // 暗色主题
    mockAppStore.currentTheme = 'dark'
    await wrapper.vm.$nextTick()

    expect(header.classes()).toContain('dark:bg-gray-900/80')
  })

  it('handles mobile navigation visibility', () => {
    // 桌面端导航应该显?
    const desktopNav = wrapper.find('nav.hidden.md\\:flex')
    expect(desktopNav.exists()).toBe(true)

    // 移动端导航应该隐藏（默认?
    const mobileNav = wrapper.find('.md\\:hidden')
    expect(mobileNav.exists()).toBe(true)
  })

  it('adds and removes click event listener', () => {
    const addEventListenerSpy = vi.spyOn(document, 'addEventListener')
    const removeEventListenerSpy = vi.spyOn(document, 'removeEventListener')

    // 重新挂载组件
    wrapper.unmount()

    const newWrapper = mount(MainLayout, {
      global: {
        plugins: [router],
      },
    })

    expect(addEventListenerSpy).toHaveBeenCalledWith('click', expect.any(Function))

    // 卸载组件
    newWrapper.unmount()

    expect(removeEventListenerSpy).toHaveBeenCalledWith('click', expect.any(Function))
  })

  it('shows notification types with correct styling', async () => {
    wrapper.vm.showNotifications = true
    await wrapper.vm.$nextTick()

    const notifications = wrapper.findAll('.p-3.rounded-lg.border')
    expect(notifications).toHaveLength(2)

    // 第一个是info类型
    expect(notifications[0].classes()).toContain('bg-blue-50')
    expect(notifications[0].classes()).toContain('border-blue-200')

    // 第二个是success类型
    expect(notifications[1].classes()).toContain('bg-green-50')
    expect(notifications[1].classes()).toContain('border-green-200')
  })

  it('closes notifications panel when clicking overlay', async () => {
    wrapper.vm.showNotifications = true
    await wrapper.vm.$nextTick()

    const overlay = wrapper.find('.absolute.inset-0.bg-black\\/50')
    await overlay.trigger('click')

    expect(wrapper.vm.showNotifications).toBe(false)
  })

  it('prevents notifications panel close when clicking inside', async () => {
    wrapper.vm.showNotifications = true
    await wrapper.vm.$nextTick()

    const panelContent = wrapper.find('.absolute.right-0.top-16')
    await panelContent.trigger('click')

    expect(wrapper.vm.showNotifications).toBe(true)
  })
})
