import { afterEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import SmoothProgressValue from '../SmoothProgressValue.vue'

describe('SmoothProgressValue', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('eases a forward jump instead of replacing the visible number immediately', async () => {
    const frames: FrameRequestCallback[] = []
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
      frames.push(callback)
      return frames.length
    })
    vi.stubGlobal('cancelAnimationFrame', vi.fn())
    vi.stubGlobal('matchMedia', () => ({ matches: false }))
    vi.spyOn(performance, 'now').mockReturnValue(0)

    const wrapper = mount(SmoothProgressValue, { props: { value: 20.22 } })
    await wrapper.setProps({ value: 33.55 })
    await nextTick()

    expect(wrapper.text()).toBe('20.22%')
    frames.shift()?.(180)
    await nextTick()
    const intermediate = Number.parseFloat(wrapper.text())
    expect(intermediate).toBeGreaterThan(20.22)
    expect(intermediate).toBeLessThan(33.55)
    frames.shift()?.(360)
    await nextTick()
    expect(wrapper.text()).toBe('33.55%')
  })
})
