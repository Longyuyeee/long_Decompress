import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import ThemeToggle from '../ThemeToggle.vue'
import { useUIStore } from '@/stores'
import { createPinia, setActivePinia } from 'pinia'

// 模拟localStorage
const localStorageMock = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn()
}

Object.defineProperty(window, 'localStorage', {
  value: localStorageMock
})

// 模拟matchMedia
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn()
  }))
})

describe('ThemeToggle组件', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('渲染按钮变体（默认）', () => {
    const wrapper = mount(ThemeToggle)

    expect(wrapper.exists()).toBe(true)
    expect(wrapper.find('.theme-toggle-button').exists()).toBe(true)
    expect(wrapper.find('.theme-toggle-button-icon').exists()).toBe(true)
    expect(wrapper.find('.theme-toggle-button-label').exists()).toBe(true)
  })

  it('渲染图标变体', () => {
    const wrapper = mount(ThemeToggle, {
      props: {
        variant: 'icon'
      }
    })

    expect(wrapper.find('.theme-toggle-icon').exists()).toBe(true)
    expect(wrapper.find('.theme-toggle-icon-inner').exists()).toBe(true)
  })

  it('渲染开关变�?, () => {
    const wrapper = mount(ThemeToggle, {
      props: {
        variant: 'switch'
      }
    })

    expect(wrapper.find('.theme-toggle-switch').exists()).toBe(true)
    expect(wrapper.find('.theme-toggle-switch-button').exists()).toBe(true)
    expect(wrapper.find('.theme-toggle-switch-thumb').exists()).toBe(true)
  })

  it('显示正确的主题标�?, () => {
    // 模拟浅色主题
    localStorageMock.getItem.mockReturnValue('false')
    const wrapper = mount(ThemeToggle)

    expect(wrapper.text()).toContain('浅色')
  })

  it('显示深色主题标签', () => {
    // 模拟深色主题
    const uiStore = useUIStore()
    uiStore.setDarkMode(true)
    localStorageMock.getItem.mockReturnValue('true')

    const wrapper = mount(ThemeToggle)

    expect(wrapper.text()).toContain('深色')
  })

  it('显示自动主题标签', () => {
    // 模拟自动主题
    localStorageMock.getItem.mockReturnValue(null)

    const wrapper = mount(ThemeToggle)

    expect(wrapper.text()).toContain('自动')
  })

  it('切换主题', async () => {
    const uiStore = useUIStore()
    const toggleDarkModeSpy = vi.spyOn(uiStore, 'toggleDarkMode')

    const wrapper = mount(ThemeToggle)

    const button = wrapper.find('.theme-toggle-button')
    await button.trigger('click')

    expect(toggleDarkModeSpy).toHaveBeenCalled()
  })

  it('触发toggle事件', async () => {
    const wrapper = mount(ThemeToggle)

    const button = wrapper.find('.theme-toggle-button')
    await button.trigger('click')

    expect(wrapper.emitted('toggle')).toBeTruthy()
  })

  it('显示主题选择菜单', async () => {
    const wrapper = mount(ThemeToggle)

    const button = wrapper.find('.theme-toggle-button')
    await button.trigger('click')

    expect(wrapper.find('.theme-toggle-menu').exists()).toBe(true)
    expect(wrapper.findAll('.theme-toggle-menu-option')).toHaveLength(3)
  })

  it('选择主题选项', async () => {
    const wrapper = mount(ThemeToggle)

    // 打开菜单
    const button = wrapper.find('.theme-toggle-button')
    await button.trigger('click')

    // 选择浅色主题
    const lightOption = wrapper.findAll('.theme-toggle-menu-option')[0]
    await lightOption.trigger('click')

    expect(wrapper.emitted('change')).toBeTruthy()
    expect(wrapper.emitted('change')?.[0]?.[0]).toBe('light')
  })

  it('关闭主题选择菜单', async () => {
    const wrapper = mount(ThemeToggle)

    // 打开菜单
    const button = wrapper.find('.theme-toggle-button')
    await button.trigger('click')

    expect(wrapper.find('.theme-toggle-menu').exists()).toBe(true)

    // 点击关闭按钮
    const closeButton = wrapper.find('.theme-toggle-menu-close')
    await closeButton.trigger('click')

    expect(wrapper.find('.theme-toggle-menu').exists()).toBe(false)
  })

  it('支持不同尺寸', () => {
    const sizes = ['sm', 'md', 'lg'] as const

    sizes.forEach(size => {
      const wrapper = mount(ThemeToggle, {
        props: { size }
      })

      expect(wrapper.classes()).toContain(`theme-toggle-${size}`)
    })
  })

  it('支持隐藏标签', () => {
    const wrapper = mount(ThemeToggle, {
      props: {
        showLabel: false
      }
    })

    expect(wrapper.find('.theme-toggle-button-label').exists()).toBe(false)
  })

  it('支持隐藏图标', () => {
    const wrapper = mount(ThemeToggle, {
      props: {
        showIcon: false
      }
    })

    expect(wrapper.find('.theme-toggle-button-icon').exists()).toBe(false)
  })

  describe('图标变体', () => {
    it('切换图标主题', async () => {
      const uiStore = useUIStore()
      const toggleDarkModeSpy = vi.spyOn(uiStore, 'toggleDarkMode')

      const wrapper = mount(ThemeToggle, {
        props: {
          variant: 'icon'
        }
      })

      const button = wrapper.find('.theme-toggle-icon')
      await button.trigger('click')

      expect(toggleDarkModeSpy).toHaveBeenCalled()
      expect(wrapper.emitted('toggle')).toBeTruthy()
    })

    it('显示激活状�?, () => {
      const uiStore = useUIStore()
      uiStore.setDarkMode(true)

      const wrapper = mount(ThemeToggle, {
        props: {
          variant: 'icon'
        }
      })

      expect(wrapper.find('.theme-toggle-icon').classes()).toContain('theme-toggle-icon-active')
    })
  })

  describe('开关变�?, () => {
    it('切换开关主�?, async () => {
      const uiStore = useUIStore()
      const toggleDarkModeSpy = vi.spyOn(uiStore, 'toggleDarkMode')

      const wrapper = mount(ThemeToggle, {
        props: {
          variant: 'switch'
        }
      })

      const button = wrapper.find('.theme-toggle-switch-button')
      await button.trigger('click')

      expect(toggleDarkModeSpy).toHaveBeenCalled()
      expect(wrapper.emitted('toggle')).toBeTruthy()
    })

    it('显示深色状�?, () => {
      const uiStore = useUIStore()
      uiStore.setDarkMode(true)

      const wrapper = mount(ThemeToggle, {
        props: {
          variant: 'switch'
        }
      })

      expect(wrapper.find('.theme-toggle-switch-button').classes()).toContain('theme-toggle-switch-button-dark')
      expect(wrapper.find('.theme-toggle-switch-thumb').classes()).toContain('theme-toggle-switch-thumb-dark')
    })
  })

  describe('可访问�?, () => {
    it('按钮有正确的ARIA标签', () => {
      const wrapper = mount(ThemeToggle)

      const button = wrapper.find('.theme-toggle-button')
      expect(button.attributes('aria-label')).toBe('切换主题')
    })

    it('按钮有正确的标题', () => {
      localStorageMock.getItem.mockReturnValue('false')
      const wrapper = mount(ThemeToggle)

      const button = wrapper.find('.theme-toggle-button')
      expect(button.attributes('title')).toContain('切换主题 (当前: 浅色)')
    })
  })

  describe('响应式设�?, () => {
    it('小尺寸样�?, () => {
      const wrapper = mount(ThemeToggle, {
        props: {
          size: 'sm'
        }
      })

      expect(wrapper.find('.theme-toggle-button').classes()).toContain('px-3')
      expect(wrapper.find('.theme-toggle-button').classes()).toContain('py-1.5')
      expect(wrapper.find('.theme-toggle-button').classes()).toContain('text-sm')
    })

    it('大尺寸样�?, () => {
      const wrapper = mount(ThemeToggle, {
        props: {
          size: 'lg'
        }
      })

      expect(wrapper.find('.theme-toggle-button').classes()).toContain('px-6')
      expect(wrapper.find('.theme-toggle-button').classes()).toContain('py-3')
      expect(wrapper.find('.theme-toggle-button').classes()).toContain('text-lg')
    })
  })
})
