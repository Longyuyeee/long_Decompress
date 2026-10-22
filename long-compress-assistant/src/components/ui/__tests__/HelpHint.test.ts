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


it('悬停和键盘聚焦显示说明，移到说明上可继续阅读', async () => {
  vi.useFakeTimers()
  const wrapper = mount(HelpHint, { attachTo: document.body, props:{label:'输出说明'}, slots:{default:'保存到所选目录'} })
  const trigger=wrapper.get('button')
  await trigger.trigger('mouseenter')
  expect(document.querySelector('[role="note"]')?.textContent).toBe('保存到所选目录')
  await trigger.trigger('mouseleave')
  document.querySelector('[role="note"]')!.dispatchEvent(new MouseEvent('mouseenter'))
  await vi.advanceTimersByTimeAsync(200)
  expect(document.querySelector('[role="note"]')).not.toBeNull()
  document.querySelector('[role="note"]')!.dispatchEvent(new MouseEvent('mouseleave'))
  await vi.advanceTimersByTimeAsync(200)
  expect(document.querySelector('[role="note"]')).toBeNull()
  trigger.element.focus();await wrapper.vm.$nextTick()
  expect(document.querySelector('[role="note"]')).not.toBeNull()
  document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}));await wrapper.vm.$nextTick()
  expect(document.querySelector('[role="note"]')).toBeNull()
  wrapper.unmount();vi.useRealTimers()
})
