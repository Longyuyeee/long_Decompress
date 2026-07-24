import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import PasswordEntryModal from '../PasswordEntryModal.vue'

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))

describe('PasswordEntryModal', () => {
  beforeEach(() => {
    mocks.invoke.mockReset()
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'load_app_settings') return '{}'
      return undefined
    })
  })

  it('uses clear password-oriented wording and starts with an empty name', () => {
    const wrapper = mount(PasswordEntryModal, {
      props: { visible: true },
      global: {
        plugins: [createPinia()],
        stubs: {
          Modal: {
            props: ['title'],
            template: '<section><h2>{{ title }}</h2><slot /></section>',
          },
        },
      },
    })

    expect(wrapper.text()).toContain('创建新密码')
    expect(wrapper.text()).toContain('密码名称')
    expect(wrapper.text()).toContain('密码正文')
    expect(wrapper.find('input[type="text"]').element.value).toBe('')
    expect(wrapper.find('[aria-label="显示密码"]').exists()).toBe(true)
  })
})
