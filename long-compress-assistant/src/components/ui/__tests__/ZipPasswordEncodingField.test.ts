import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import ZipPasswordEncodingField from '../ZipPasswordEncodingField.vue'
import ThemedSelect from '../ThemedSelect.vue'

describe('ZIP input encoding field', () => {
  it('uses an accurate EGG key-only label and UTF-8 default', () => {
    const wrapper=mount(ZipPasswordEncodingField,{props:{sourcePath:'native.vol3.egg',modelValue:'cp936'}})
    const select=wrapper.getComponent(ThemedSelect)
    expect(select.props('ariaLabel')).toBe('EGG 密码编码')
    expect(select.props('modelValue')).toBe('cp936')
    expect(select.props('options')[0]).toEqual({value:'auto',label:'UTF-8（默认）'})
    select.vm.$emit('update:modelValue','auto')
    expect(wrapper.emitted('update:modelValue')).toEqual([[undefined]])
    wrapper.unmount()
  })
  it('labels the ALZ default accurately and keeps the explicit producer codepage', () => {
    const wrapper=mount(ZipPasswordEncodingField,{props:{sourcePath:'native.a01',modelValue:'cp936'}})
    const select=wrapper.getComponent(ThemedSelect)
    expect(select.props('ariaLabel')).toBe('ALZ 文件名与密码编码')
    expect(select.props('modelValue')).toBe('cp936')
    expect(select.props('options')[0]).toEqual({value:'auto',label:'韩文（CP949，默认）'})
    select.vm.$emit('update:modelValue','auto')
    expect(wrapper.emitted('update:modelValue')).toEqual([[undefined]])
    wrapper.unmount()
  })
  it('offers only explicit output encodings with UTF-8 default and a separate target label', () => {
    const wrapper=mount(ZipPasswordEncodingField,{props:{mode:'output',label:'新包 ZIP 密码编码'}})
    const select=wrapper.getComponent(ThemedSelect)
    expect(select.props('modelValue')).toBe('utf8')
    expect(select.props('options').map(o=>o.value)).toEqual(['utf8','windows_local','cp932','cp936','cp950'])
    expect(select.props('ariaLabel')).toBe('新包 ZIP 密码编码')
    select.vm.$emit('update:modelValue','auto');expect(wrapper.emitted('update:modelValue')).toBeUndefined()
    select.vm.$emit('update:modelValue','cp950');expect(wrapper.emitted('update:modelValue')).toEqual([['cp950']])
    wrapper.unmount()
  })
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
