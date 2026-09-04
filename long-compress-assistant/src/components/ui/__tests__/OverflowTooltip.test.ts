import { afterEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import OverflowTooltip from '../OverflowTooltip.vue'

describe('OverflowTooltip', () => {
  afterEach(() => {
    vi.useRealTimers()
    document.body.innerHTML = ''
  })

  it('replaces the native title with a styled tooltip only when text is truncated', async () => {
    vi.useFakeTimers()
    const wrapper = mount(OverflowTooltip, {
      props: { text: 'digital-rural-platform.zip (2 个分卷)', delay: 10 },
      attachTo: document.body,
    })
    const anchor = wrapper.get('.overflow-tooltip-anchor')
    Object.defineProperties(anchor.element, {
      clientWidth: { configurable: true, value: 120 },
      scrollWidth: { configurable: true, value: 280 },
    })

    expect(anchor.attributes('title')).toBeUndefined()
    await anchor.trigger('mouseenter')
    await vi.advanceTimersByTimeAsync(10)

    const tooltip = document.body.querySelector('[role="tooltip"]')
    expect(tooltip?.textContent).toContain('digital-rural-platform.zip (2 个分卷)')
    expect(tooltip?.classList.contains('overflow-tooltip-popover')).toBe(true)

    await anchor.trigger('mouseleave')
    await vi.advanceTimersByTimeAsync(200)
    expect(document.body.querySelector('[role="tooltip"]')).toBeNull()
  })
})
