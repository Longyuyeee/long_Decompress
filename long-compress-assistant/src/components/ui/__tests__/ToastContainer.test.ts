import { beforeEach, describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import ToastContainer from '../ToastContainer.vue'
import { useUIStore } from '@/stores/ui'

describe('ToastContainer', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('lets pointer input pass through the toast body while keeping close interactive', async () => {
    const uiStore = useUIStore()
    uiStore.toasts.push({
      id: 'toast-1',
      type: 'success',
      message: '完成',
      duration: 3000,
    })

    const wrapper = mount(ToastContainer)
    expect(wrapper.get('.toast-stack').classes()).toContain('z-[250]')
    expect(wrapper.get('.toast-stack').classes()).toContain('pointer-events-none')
    expect(wrapper.get('.toast-item').classes()).toContain('pointer-events-none')
    expect(wrapper.get('.toast-item').classes()).not.toContain('pointer-events-auto')

    const closeButton = wrapper.get('button[aria-label="关闭成功提示"]')
    expect(closeButton.classes()).toContain('pointer-events-auto')
    await closeButton.trigger('click')
    expect(uiStore.toasts).toHaveLength(0)
    uiStore.toasts.push({ id: 'error', type: 'error', message: '提交失败', duration: 0 })
    await wrapper.vm.$nextTick()
    expect(wrapper.get('.toast-stack').classes()).toContain('z-[9999]')
    await wrapper.get('button[aria-label="关闭错误提示"]').trigger('click')
    expect(wrapper.get('.toast-stack').classes()).toContain('z-[250]')
  })
})
