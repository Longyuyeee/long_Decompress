import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import GlassButton from '../GlassButton.vue'

describe('GlassButton组件', () => {
  it('渲染正确', () => {
    const wrapper = mount(GlassButton, {
      slots: {
        default: '按钮文本'
      }
    })

    expect(wrapper.exists()).toBe(true)
    expect(wrapper.text()).toContain('按钮文本')
    expect(wrapper.find('button').exists()).toBe(true)
    expect(wrapper.classes()).toContain('glass-button')
  })

  it('使用默认属�?, () => {
    const wrapper = mount(GlassButton)

    const button = wrapper.find('button')
    expect(button.attributes('type')).toBe('button')
    expect(button.classes()).toContain('rounded-lg')
    expect(button.classes()).toContain('px-4')
    expect(button.classes()).toContain('py-2')
    expect(button.classes()).toContain('backdrop-blur-md')
    expect(button.classes()).toContain('border')
  })

  it('接受文本属�?, () => {
    const wrapper = mount(GlassButton, {
      props: {
        text: '提交按钮'
      }
    })

    expect(wrapper.text()).toBe('提交按钮')
  })

  it('接受插槽内容', () => {
    const wrapper = mount(GlassButton, {
      slots: {
        default: '<span>插槽内容</span>'
      }
    })

    expect(wrapper.text()).toContain('插槽内容')
    expect(wrapper.find('span').exists()).toBe(true)
  })

  it('处理点击事件', async () => {
    const wrapper = mount(GlassButton)

    await wrapper.find('button').trigger('click')

    expect(wrapper.emitted('click')).toBeTruthy()
    expect(wrapper.emitted('click')?.length).toBe(1)
  })

  it('禁用时不触发点击事件', async () => {
    const wrapper = mount(GlassButton, {
      props: {
        disabled: true
      }
    })

    await wrapper.find('button').trigger('click')

    expect(wrapper.emitted('click')).toBeFalsy()
  })

  it('加载时不触发点击事件', async () => {
    const wrapper = mount(GlassButton, {
      props: {
        loading: true
      }
    })

    await wrapper.find('button').trigger('click')

    expect(wrapper.emitted('click')).toBeFalsy()
  })

  describe('变体类型', () => {
    it('使用默认变体', () => {
      const wrapper = mount(GlassButton)
      expect(wrapper.classes()).toContain('glass-button')
    })

    it('使用主要变体', () => {
      const wrapper = mount(GlassButton, {
        props: {
          variant: 'primary'
        }
      })

      expect(wrapper.classes()).toContain('glass-button-primary')
      expect(wrapper.classes()).not.toContain('glass-button-secondary')
      expect(wrapper.classes()).not.toContain('glass-button-danger')
    })

    it('使用次要变体', () => {
      const wrapper = mount(GlassButton, {
        props: {
          variant: 'secondary'
        }
      })

      expect(wrapper.classes()).toContain('glass-button-secondary')
      expect(wrapper.classes()).not.toContain('glass-button-primary')
      expect(wrapper.classes()).not.toContain('glass-button-danger')
    })

    it('使用危险变体', () => {
      const wrapper = mount(GlassButton, {
        props: {
          variant: 'danger'
        }
      })

      expect(wrapper.classes()).toContain('glass-button-danger')
      expect(wrapper.classes()).not.toContain('glass-button-primary')
      expect(wrapper.classes()).not.toContain('glass-button-secondary')
    })
  })

  describe('尺寸', () => {
    it('使用小尺�?, () => {
      const wrapper = mount(GlassButton, {
        props: {
          size: 'sm'
        }
      })

      expect(wrapper.classes()).toContain('px-3')
      expect(wrapper.classes()).toContain('py-1.5')
      expect(wrapper.classes()).toContain('text-sm')
      expect(wrapper.classes()).not.toContain('px-4')
      expect(wrapper.classes()).not.toContain('py-2')
    })

    it('使用中尺寸（默认�?, () => {
      const wrapper = mount(GlassButton)

      expect(wrapper.classes()).toContain('px-4')
      expect(wrapper.classes()).toContain('py-2')
      expect(wrapper.classes()).not.toContain('px-3')
      expect(wrapper.classes()).not.toContain('py-1.5')
      expect(wrapper.classes()).not.toContain('text-sm')
    })

    it('使用大尺�?, () => {
      const wrapper = mount(GlassButton, {
        props: {
          size: 'lg'
        }
      })

      expect(wrapper.classes()).toContain('px-6')
      expect(wrapper.classes()).toContain('py-3')
      expect(wrapper.classes()).toContain('text-lg')
      expect(wrapper.classes()).not.toContain('px-4')
      expect(wrapper.classes()).not.toContain('py-2')
    })

    it('使用超大尺寸', () => {
      const wrapper = mount(GlassButton, {
        props: {
          size: 'xl'
        }
      })

      expect(wrapper.classes()).toContain('px-8')
      expect(wrapper.classes()).toContain('py-4')
      expect(wrapper.classes()).toContain('text-xl')
      expect(wrapper.classes()).not.toContain('px-4')
      expect(wrapper.classes()).not.toContain('py-2')
    })
  })

  describe('按钮类型', () => {
    it('使用button类型（默认）', () => {
      const wrapper = mount(GlassButton)
      expect(wrapper.find('button').attributes('type')).toBe('button')
    })

    it('使用submit类型', () => {
      const wrapper = mount(GlassButton, {
        props: {
          type: 'submit'
        }
      })

      expect(wrapper.find('button').attributes('type')).toBe('submit')
    })

    it('使用reset类型', () => {
      const wrapper = mount(GlassButton, {
        props: {
          type: 'reset'
        }
      })

      expect(wrapper.find('button').attributes('type')).toBe('reset')
    })
  })

  describe('图标', () => {
    it('显示图标', () => {
      const wrapper = mount(GlassButton, {
        props: {
          icon: 'check',
          text: '确认'
        }
      })

      expect(wrapper.find('i').exists()).toBe(true)
      expect(wrapper.find('i').classes()).toContain('pi')
      expect(wrapper.find('i').classes()).toContain('pi-check')
      expect(wrapper.text()).toContain('确认')
    })

    it('只有图标没有文本', () => {
      const wrapper = mount(GlassButton, {
        props: {
          icon: 'check'
        }
      })

      expect(wrapper.find('i').exists()).toBe(true)
      expect(wrapper.find('i').classes()).toContain('pi')
      expect(wrapper.find('i').classes()).toContain('pi-check')
      expect(wrapper.text()).toBe('')
    })
  })

  describe('加载状�?, () => {
    it('显示加载状�?, () => {
      const wrapper = mount(GlassButton, {
        props: {
          loading: true,
          loadingText: '加载�?..'
        }
      })

      expect(wrapper.find('i').exists()).toBe(true)
      expect(wrapper.find('i').classes()).toContain('pi-spinner')
      expect(wrapper.find('i').classes()).toContain('pi-spin')
      expect(wrapper.text()).toContain('加载�?..')
      expect(wrapper.classes()).toContain('opacity-70')
      expect(wrapper.classes()).toContain('cursor-wait')
    })

    it('加载状态没有文�?, () => {
      const wrapper = mount(GlassButton, {
        props: {
          loading: true
        }
      })

      expect(wrapper.find('i').exists()).toBe(true)
      expect(wrapper.find('i').classes()).toContain('pi-spinner')
      expect(wrapper.text()).toBe('')
    })
  })

  describe('禁用状�?, () => {
    it('显示禁用状�?, () => {
      const wrapper = mount(GlassButton, {
        props: {
          disabled: true
        }
      })

      expect(wrapper.find('button').attributes('disabled')).toBeDefined()
      expect(wrapper.classes()).toContain('opacity-50')
      expect(wrapper.classes()).toContain('cursor-not-allowed')
      expect(wrapper.classes()).not.toContain('hover:scale-105')
    })

    it('禁用时没有悬停效�?, () => {
      const wrapper = mount(GlassButton, {
        props: {
          disabled: true
        }
      })

      expect(wrapper.classes()).not.toContain('hover:scale-105')
      expect(wrapper.classes()).not.toContain('active:scale-95')
    })
  })

  describe('全宽模式', () => {
    it('启用全宽模式', () => {
      const wrapper = mount(GlassButton, {
        props: {
          fullWidth: true
        }
      })

      expect(wrapper.classes()).toContain('w-full')
    })

    it('禁用全宽模式（默认）', () => {
      const wrapper = mount(GlassButton)

      expect(wrapper.classes()).not.toContain('w-full')
    })
  })

  describe('自定义类�?, () => {
    it('接受自定义类�?, () => {
      const wrapper = mount(GlassButton, {
        props: {
          className: 'custom-class another-class'
        }
      })

      expect(wrapper.classes()).toContain('custom-class')
      expect(wrapper.classes()).toContain('another-class')
    })
  })

  describe('组合测试', () => {
    it('组合多个属�?, () => {
      const wrapper = mount(GlassButton, {
        props: {
          variant: 'primary',
          size: 'lg',
          disabled: true,
          fullWidth: true,
          className: 'my-custom-class',
          text: '组合测试按钮'
        }
      })

      expect(wrapper.classes()).toContain('glass-button-primary')
      expect(wrapper.classes()).toContain('px-6')
      expect(wrapper.classes()).toContain('py-3')
      expect(wrapper.classes()).toContain('text-lg')
      expect(wrapper.classes()).toContain('opacity-50')
      expect(wrapper.classes()).toContain('cursor-not-allowed')
      expect(wrapper.classes()).toContain('w-full')
      expect(wrapper.classes()).toContain('my-custom-class')
      expect(wrapper.text()).toBe('组合测试按钮')
    })

    it('加载状态组�?, () => {
      const wrapper = mount(GlassButton, {
        props: {
          loading: true,
          loadingText: '正在处理...',
          variant: 'danger',
          size: 'sm'
        }
      })

      expect(wrapper.find('i').classes()).toContain('pi-spinner')
      expect(wrapper.text()).toContain('正在处理...')
      expect(wrapper.classes()).toContain('glass-button-danger')
      expect(wrapper.classes()).toContain('px-3')
      expect(wrapper.classes()).toContain('py-1.5')
      expect(wrapper.classes()).toContain('text-sm')
      expect(wrapper.classes()).toContain('opacity-70')
      expect(wrapper.classes()).toContain('cursor-wait')
    })
  })

  describe('可访问�?, () => {
    it('禁用按钮有正确的ARIA属�?, () => {
      const wrapper = mount(GlassButton, {
        props: {
          disabled: true,
          text: '禁用按钮'
        }
      })

      const button = wrapper.find('button')
      expect(button.attributes('disabled')).toBeDefined()
      expect(button.attributes('aria-disabled')).toBe('true')
    })

    it('加载按钮有正确的ARIA属�?, () => {
      const wrapper = mount(GlassButton, {
        props: {
          loading: true,
          loadingText: '加载�?
        }
      })

      const button = wrapper.find('button')
      expect(button.attributes('disabled')).toBeDefined()
      expect(button.attributes('aria-busy')).toBe('true')
      expect(button.attributes('aria-label')).toBe('加载�?)
    })
  })

  describe('样式�?, () => {
    it('有过渡效果类', () => {
      const wrapper = mount(GlassButton)

      expect(wrapper.classes()).toContain('transition-all')
      expect(wrapper.classes()).toContain('duration-200')
    })

    it('有布局�?, () => {
      const wrapper = mount(GlassButton)

      expect(wrapper.classes()).toContain('flex')
      expect(wrapper.classes()).toContain('items-center')
      expect(wrapper.classes()).toContain('justify-center')
      expect(wrapper.classes()).toContain('space-x-2')
    })

    it('有字体类', () => {
      const wrapper = mount(GlassButton)

      expect(wrapper.classes()).toContain('font-medium')
    })
  })
})
