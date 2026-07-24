import { afterEach, describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import Modal from '../Modal.vue'

describe('Modal', () => {
  afterEach(() => {
    document.body.innerHTML = ''
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
