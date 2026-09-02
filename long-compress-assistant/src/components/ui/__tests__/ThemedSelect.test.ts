import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import ThemedSelect from '../ThemedSelect.vue'

describe('ThemedSelect', () => {
  it('uses a themed listbox and supports keyboard selection', async () => {
    const wrapper = mount(ThemedSelect, {
      props: {
        modelValue: 'keep',
        ariaLabel: '输出格式',
        options: [{ value: 'keep', label: '保持原格式' }, { value: 'webp', label: 'WebP' }],
      },
    })
    const trigger = wrapper.get('.select-trigger')
    await trigger.trigger('keydown', { key: 'ArrowDown' })
    expect(wrapper.get('[role="listbox"]').exists()).toBe(true)
    const options = wrapper.findAll('[role="option"]')
    await options[1].trigger('click')
    expect(wrapper.emitted('update:modelValue')).toEqual([['webp']])
    expect(wrapper.get('.select-trigger').attributes('aria-expanded')).toBe('false')
  })
})
