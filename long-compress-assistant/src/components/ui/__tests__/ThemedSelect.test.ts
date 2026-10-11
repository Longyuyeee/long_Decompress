import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import ThemedSelect from '../ThemedSelect.vue'

describe('ThemedSelect', () => {
  it('floating options retain unsupported-format explanations and cannot be selected', async () => {
    const wrapper = mount(ThemedSelect, { attachTo: document.body, props: {
      modelValue: 'zip', floating: true, options: [{ value: 'zip', label: 'ZIP' }, { value: 'gz', label: 'GZ', detail: '仅支持单文件', disabled: true }],
    } })
    await wrapper.get('button').trigger('click')
    const disabled = document.querySelector<HTMLButtonElement>('[role="option"][aria-disabled="true"]')!
    expect(disabled.textContent).toContain('仅支持单文件')
    disabled.click()
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    disabled.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    await wrapper.vm.$nextTick()
    expect(document.activeElement).toBe(wrapper.get('.select-trigger').element)
    wrapper.unmount()
  })
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
