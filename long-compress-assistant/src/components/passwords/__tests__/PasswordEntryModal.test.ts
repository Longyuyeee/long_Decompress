import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import {
  PasswordCategory,
  usePasswordStore,
  type PasswordEntry,
} from '@/stores/password'
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

  it('sends only archive-password fields while the backend preserves lifecycle metadata', async () => {
    const pinia = createPinia()
    const original: PasswordEntry = {
      id: 'entry-edit',
      name: '原密码',
      password: 'old-password',
      notes: '原备注',
      tags: ['work'],
      category: PasswordCategory.Work,
      created_at: '2025-01-01T00:00:00.000Z',
      updated_at: '2026-01-01T00:00:00.000Z',
      last_used: '2026-07-20T00:00:00.000Z',
      favorite: true,
      use_count: 9,
      usage_history: { '2026-07-20': 4 },
    }
    const store = usePasswordStore(pinia)
    store.entries = [original]
    mocks.invoke.mockImplementation(async (command: string, args?: Record<string, any>) => {
      if (command === 'load_app_settings') return '{}'
      if (command === 'update_encrypted_password') return args?.entry
      return undefined
    })

    const wrapper = mount(PasswordEntryModal, {
      props: { visible: true, entry: original },
      global: {
        plugins: [pinia],
        stubs: {
          Modal: {
            props: ['title'],
            template: '<section><h2>{{ title }}</h2><slot /></section>',
          },
        },
      },
    })

    await wrapper.get('input[type="password"]').setValue('Long!Secure#Password123')
    await wrapper.get('[data-testid="password-entry-save"]').trigger('click')
    await flushPromises()

    const updateCall = mocks.invoke.mock.calls.find(([command]) => command === 'update_encrypted_password')
    expect(updateCall).toBeTruthy()
    expect(updateCall?.[1]?.entry).toEqual({
      name: '原密码',
      password: 'Long!Secure#Password123',
      notes: '原备注',
      tags: ['work'],
      category: undefined,
      favorite: true,
    })
  })

  it('creates an archive password without traditional strength or login fields', async () => {
    const pinia = createPinia()
    mocks.invoke.mockImplementation(async (command: string, args?: Record<string, any>) => {
      if (command === 'load_app_settings') return '{}'
      if (command === 'add_encrypted_password') {
        return {
          ...args?.entry,
          id: 'entry-new',
          created_at: '2026-07-29T00:00:00.000Z',
          updated_at: '2026-07-29T00:00:00.000Z',
          last_used: null,
          favorite: false,
          use_count: 0,
          usage_history: {},
        }
      }
      return undefined
    })
    const wrapper = mount(PasswordEntryModal, {
      props: { visible: true },
      global: {
        plugins: [pinia],
        stubs: {
          Modal: {
            props: ['title'],
            template: '<section><h2>{{ title }}</h2><slot /></section>',
          },
        },
      },
    })

    await wrapper.get('input[type="text"]').setValue('新密码')
    await wrapper.get('input[type="password"]').setValue('Long!Secure#Password123')
    await wrapper.get('[data-testid="password-entry-save"]').trigger('click')
    await flushPromises()

    const addCall = mocks.invoke.mock.calls.find(([command]) => command === 'add_encrypted_password')
    expect(addCall?.[1]?.entry).toEqual({
      name: '新密码',
      password: 'Long!Secure#Password123',
      notes: '',
      category: PasswordCategory.Other,
      tags: [],
    })
    expect(addCall?.[1]?.entry).not.toHaveProperty('username')
    expect(addCall?.[1]?.entry).not.toHaveProperty('url')
    expect(addCall?.[1]?.entry).not.toHaveProperty('strength')
  })
})
