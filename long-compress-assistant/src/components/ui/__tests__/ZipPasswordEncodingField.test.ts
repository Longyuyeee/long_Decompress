import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import ZipPasswordEncodingField from '../ZipPasswordEncodingField.vue'
import ThemedSelect from '../ThemedSelect.vue'

describe('ZIP input encoding field', () => {
  it('starts automatic and emits only user choices, including clearing an override', () => {
    const wrapper = mount(ZipPasswordEncodingField, { global: { stubs: { Teleport: true } } })
    const select = wrapper.getComponent(ThemedSelect)
    expect(select.props('modelValue')).toBe('auto')
    expect(select.props('options').map(option => option.value)).toEqual(['auto', 'utf8', 'windows_local', 'cp932', 'cp936', 'cp950'])
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    select.vm.$emit('update:modelValue', 'cp932')
    select.vm.$emit('update:modelValue', 'auto')
    select.vm.$emit('update:modelValue', 'cp1252')
    expect(wrapper.emitted('update:modelValue')).toEqual([['cp932'], [undefined]])
    wrapper.unmount()
  })
  it('shows the restored value and blocks changes while busy', async () => {
    const wrapper = mount(ZipPasswordEncodingField, { props: { modelValue: 'cp950', disabled: true } })
    expect(wrapper.get('button.select-trigger').text()).toContain('繁体中文')
    expect(wrapper.get('button.select-trigger').attributes('disabled')).toBeDefined()
    await wrapper.get('button.select-trigger').trigger('click')
    expect(wrapper.find('[role="listbox"]').exists()).toBe(false)
    wrapper.unmount()
  })
})
