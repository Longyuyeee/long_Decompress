import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import GlassCard from '../GlassCard.vue'

describe('GlassCard Component', () => {
  let wrapper: any

  beforeEach(() => {
    wrapper = mount(GlassCard, {
      slots: {
        default: '<div>Test Content</div>',
      },
    })
  })

  it('renders correctly with default props', () => {
    expect(wrapper.exists()).toBe(true)
    expect(wrapper.find('.glass-card').exists()).toBe(true)
    expect(wrapper.text()).toContain('Test Content')
  })

  it('applies default classes', () => {
    const card = wrapper.find('.glass-card')

    expect(card.classes()).toContain('rounded-xl')
    expect(card.classes()).toContain('p-6')
    expect(card.classes()).toContain('transition-all')
    expect(card.classes()).toContain('duration-300')
    expect(card.classes()).toContain('backdrop-blur-md')
    expect(card.classes()).toContain('border')
    expect(card.classes()).toContain('shadow-lg')
    expect(card.classes()).toContain('hover:bg-white/15')
    expect(card.classes()).toContain('dark:hover:bg-black/15')
    expect(card.classes()).toContain('hover:shadow-xl')
  })

  it('applies custom className prop', async () => {
    await wrapper.setProps({
      className: 'custom-class another-class',
    })

    const card = wrapper.find('.glass-card')
    expect(card.classes()).toContain('custom-class')
    expect(card.classes()).toContain('another-class')
  })

  it('handles hoverable prop', async () => {
    // 默认是hoverable
    let card = wrapper.find('.glass-card')
    expect(card.classes()).toContain('hover:bg-white/15')
    expect(card.classes()).toContain('hover:shadow-xl')

    // 设置为非hoverable
    await wrapper.setProps({ hoverable: false })
    card = wrapper.find('.glass-card')
    expect(card.classes()).not.toContain('hover:bg-white/15')
    expect(card.classes()).not.toContain('hover:shadow-xl')
  })

  it('handles compact prop', async () => {
    // 默认不是compact
    let card = wrapper.find('.glass-card')
    expect(card.classes()).toContain('p-6')
    expect(card.classes()).not.toContain('p-4')

    // 设置为compact
    await wrapper.setProps({ compact: true })
    card = wrapper.find('.glass-card')
    expect(card.classes()).not.toContain('p-6')
    expect(card.classes()).toContain('p-4')
  })

  it('applies custom background color', async () => {
    await wrapper.setProps({
      backgroundColor: 'rgba(255, 0, 0, 0.5)',
    })

    const card = wrapper.find('.glass-card')
    expect(card.attributes('style')).toContain('background-color: rgba(255, 0, 0, 0.5)')
  })

  it('applies custom border color', async () => {
    await wrapper.setProps({
      borderColor: '#ff0000',
    })

    const card = wrapper.find('.glass-card')
    expect(card.attributes('style')).toContain('border-color: #ff0000')
  })

  it('applies custom blur level', async () => {
    await wrapper.setProps({
      blur: 'lg',
    })

    const card = wrapper.find('.glass-card')
    expect(card.classes()).not.toContain('backdrop-blur-md')
    expect(card.classes()).toContain('backdrop-blur-lg')
  })

  it('uses default background color when not provided', () => {
    const card = wrapper.find('.glass-card')
    expect(card.attributes('style')).toContain('background-color: rgba(255, 255, 255, 0.1)')
  })

  it('uses default border color when not provided', () => {
    const card = wrapper.find('.glass-card')
    expect(card.attributes('style')).toContain('border-color: rgba(255, 255, 255, 0.2)')
  })

  it('combines multiple custom styles', async () => {
    await wrapper.setProps({
      backgroundColor: 'rgba(0, 255, 0, 0.3)',
      borderColor: '#00ff00',
      blur: 'sm',
      compact: true,
      hoverable: false,
      className: 'test-class',
    })

    const card = wrapper.find('.glass-card')

    // 检查样�?
    expect(card.attributes('style')).toContain('background-color: rgba(0, 255, 0, 0.3)')
    expect(card.attributes('style')).toContain('border-color: #00ff00')

    // 检查类�?
    expect(card.classes()).toContain('backdrop-blur-sm')
    expect(card.classes()).not.toContain('backdrop-blur-md')
    expect(card.classes()).toContain('p-4')
    expect(card.classes()).not.toContain('p-6')
    expect(card.classes()).not.toContain('hover:bg-white/15')
    expect(card.classes()).not.toContain('hover:shadow-xl')
    expect(card.classes()).toContain('test-class')
  })

  it('renders slot content correctly', () => {
    const customWrapper = mount(GlassCard, {
      slots: {
        default: `
          <div>
            <h2 class="text-xl font-bold">Card Title</h2>
            <p class="text-gray-600">Card description goes here</p>
            <button class="mt-4 px-4 py-2 bg-blue-500 text-white rounded">Action</button>
          </div>
        `,
      },
    })

    expect(customWrapper.text()).toContain('Card Title')
    expect(customWrapper.text()).toContain('Card description goes here')
    expect(customWrapper.find('button').exists()).toBe(true)
    expect(customWrapper.find('button').text()).toBe('Action')
  })

  it('handles empty slot content', () => {
    const emptyWrapper = mount(GlassCard, {
      slots: {
        default: '',
      },
    })

    expect(emptyWrapper.find('.glass-card').exists()).toBe(true)
    expect(emptyWrapper.text()).toBe('')
  })

  it('maintains glass effect styling', () => {
    const card = wrapper.find('.glass-card')

    // 检查玻璃效果相关类�?
    expect(card.classes()).toContain('backdrop-blur-md')
    expect(card.classes()).toContain('border')

    // 检查样式中的透明背景
    expect(card.attributes('style')).toContain('rgba(255, 255, 255, 0.1)')
  })

  it('handles dark mode hover classes', () => {
    const card = wrapper.find('.glass-card')
    expect(card.classes()).toContain('dark:hover:bg-black/15')
  })

  it('applies transition classes', () => {
    const card = wrapper.find('.glass-card')
    expect(card.classes()).toContain('transition-all')
    expect(card.classes()).toContain('duration-300')
  })

  it('applies shadow classes', () => {
    const card = wrapper.find('.glass-card')
    expect(card.classes()).toContain('shadow-lg')
    expect(card.classes()).toContain('hover:shadow-xl')
  })
})
