import { afterEach, describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { defineComponent, h, KeepAlive, nextTick, ref } from 'vue'
import Modal from '../Modal.vue'

describe('Modal', () => {
  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('removes a cached tool dialog from the active page and restores it only when the tool returns', async () => {
    const selected = ref(true)
    const Tool = defineComponent({ setup: () => () => h(Modal, { visible: true, title: '待确认配置' }) })
    const Other = defineComponent({ setup: () => () => h('button', '其他工具') })
    const Parent = defineComponent({ setup: () => () => h(KeepAlive, null, { default: () => h(selected.value ? Tool : Other) }) })
    const wrapper = mount(Parent, { attachTo: document.body })
    await nextTick(); await nextTick()
    expect(document.querySelector('[role="dialog"]')).not.toBeNull()
    selected.value = false; await nextTick(); await nextTick()
    expect(document.querySelector('[role="dialog"]')).toBeNull()
    selected.value = true; await nextTick(); await nextTick()
    expect(document.querySelector('[role="dialog"]')?.textContent).toContain('待确认配置')
    wrapper.unmount()
  })

  it('focuses an initially visible dialog, traps Tab, and closes with Escape', async () => {
    const opener = document.createElement('button')
    document.body.appendChild(opener)
    opener.focus()

    const wrapper = mount(Modal, {
      attachTo: document.body,
      props: {
        visible: true,
        title: 'Confirm operation',
        showFooter: true,
      },
      slots: {
        default: '<button id="first-action">First</button><button id="last-action">Last</button>',
      },
    })
    await nextTick()
    await nextTick()

    const dialogContent = document.querySelector<HTMLElement>('[role="dialog"] [tabindex="-1"]')!
    expect(document.activeElement).toBe(dialogContent)

    const first = document.querySelector<HTMLElement>('[aria-label="Close dialog"]')!
    const last = document.querySelector<HTMLElement>('#last-action')!
    last.focus()
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true }))
    expect(document.activeElement).toBe(first)

    first.focus()
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', shiftKey: true, bubbles: true }))
    expect(document.activeElement).toBe(last)

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    expect(wrapper.emitted('update:visible')?.at(-1)).toEqual([false])
    wrapper.unmount()
  })
})
