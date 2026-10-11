import { afterEach, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import HelpHint from '../HelpHint.vue'

afterEach(() => { document.body.innerHTML = '' })

it('解释按需显示，Escape 返回触发按钮，外部点击关闭', async () => {
  const wrapper = mount(HelpHint, { attachTo: document.body, props: { label: '目录说明' }, slots: { default: '详细解释' } })
  const trigger = wrapper.get('button')
  expect(document.querySelector('[role="note"]')).toBeNull()
  await trigger.trigger('click')
  expect(document.querySelector('[role="note"]')?.textContent).toBe('详细解释')
  expect(trigger.attributes('aria-expanded')).toBe('true')
  document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
  await wrapper.vm.$nextTick()
  expect(document.activeElement).toBe(trigger.element)
  expect(document.querySelector('[role="note"]')).toBeNull()
  await trigger.trigger('click')
  document.body.dispatchEvent(new Event('pointerdown', { bubbles: true }))
  await wrapper.vm.$nextTick()
  expect(trigger.attributes('aria-expanded')).toBe('false')
  wrapper.unmount()
})

it('点击说明不冒泡到外层控件，卸载移除浮层', async () => {
  const wrapper = mount(HelpHint, { attachTo: document.body, props: { label: '说明' }, slots: { default: '内容' } })
  const parentClick = vi.fn()
  document.body.addEventListener('click', parentClick)
  await wrapper.get('button').trigger('click')
  expect(document.querySelector('[role="note"]')).not.toBeNull()
  document.querySelector('[role="note"]')!.dispatchEvent(new MouseEvent('click', { bubbles: true }))
  expect(parentClick).not.toHaveBeenCalled()
  document.body.removeEventListener('click', parentClick)
  wrapper.unmount()
  expect(document.querySelector('[role="note"]')).toBeNull()
})
