import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import PasswordVaultView from '../PasswordVaultView.vue'

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }))
const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {})

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@/composables/useTauriCommands', () => ({
  useTauriCommands: () => ({ exportPasswords: vi.fn(), importPasswords: vi.fn() }),
}))

const mountView = () => mount(PasswordVaultView, {
  global: {
    plugins: [createPinia()],
    stubs: { PasswordEntryModal: true, Transition: true },
  },
})

describe('PasswordVaultView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
    mocks.invoke.mockImplementation(async (command: string, args?: Record<string, string>) => {
      if (command === 'load_app_settings') return '{}'
      if (command === 'is_encrypted_password_service_unlocked') return false
      if (command === 'get_or_create_master_key') throw new Error('not initialized')
      if (command === 'unlock_encrypted_password_service') return args?.masterPassword === 'correct-password'
      if (command === 'list_encrypted_passwords' || command === 'list_password_groups') return []
      return undefined
    })
  })

  it('opens an accessible master-password dialog from the locked state', async () => {
    const wrapper = mountView()
    await flushPromises()

    expect(wrapper.find('input[type="text"]').attributes('disabled')).toBeDefined()
    await wrapper.findAll('button').find(button => button.text().includes('解锁保险箱'))!.trigger('click')

    expect(wrapper.find('[role="dialog"]').exists()).toBe(true)
    expect(wrapper.find('input[autocomplete="current-password"]').exists()).toBe(true)
  })

  it('unlocks the vault through the dialog form', async () => {
    const wrapper = mountView()
    await flushPromises()
    await wrapper.findAll('button').find(button => button.text().includes('解锁保险箱'))!.trigger('click')
    await wrapper.find('input[autocomplete="current-password"]').setValue('correct-password')
    await wrapper.find('form').trigger('submit')
    await flushPromises()

    expect(mocks.invoke).toHaveBeenCalledWith('unlock_encrypted_password_service', { masterPassword: 'correct-password' })
    expect(wrapper.find('[role="dialog"]').exists()).toBe(false)
  })
})

void consoleError
