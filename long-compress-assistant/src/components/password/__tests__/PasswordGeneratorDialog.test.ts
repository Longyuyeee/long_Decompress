import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import PasswordGeneratorDialog from '../PasswordGeneratorDialog.vue'

const invoke = vi.fn()
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: (...args: unknown[]) => invoke(...args) }))

describe('PasswordGeneratorDialog', () => {
  beforeEach(() => {
    invoke.mockReset()
    invoke.mockImplementation(async (command: string) => {
      if (command === 'load_app_settings') return '{}'
      if (command === 'generate_password') return 'Compact!Password123'
      if (command === 'generate_memorable_password') return 'river-cloud-maple'
      if (command === 'generate_pin') return '482913'
      return undefined
    })
  })

  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('uses the bounded shared modal with a compact three-column mode layout', async () => {
    const wrapper = mount(PasswordGeneratorDialog, {
      props: { isOpen: true },
      attachTo: document.body,
      global: {
        plugins: [createPinia()],
        mocks: { $t: (_key: string, fallback: string) => fallback },
        stubs: { Transition: false },
      },
    })
    await flushPromises()

    const dialog = document.body.querySelector('[role="dialog"]') as HTMLElement
    expect(dialog).toBeTruthy()
    const panel = dialog.querySelector('[tabindex="-1"]') as HTMLElement
    expect(panel.classList.contains('max-h-[min(78vh,42rem)]')).toBe(true)
    expect(panel.classList.contains('overflow-hidden')).toBe(true)
    expect(dialog.querySelector('.custom-scrollbar')).toBeTruthy()
    expect(dialog.querySelectorAll('.mode-grid button')).toHaveLength(3)
    expect(dialog.querySelector('[data-testid="password-generator-result"]')?.textContent).toContain('Compact!Password123')
    expect((dialog.querySelector('[data-testid="password-use"]') as HTMLButtonElement).disabled).toBe(false)
    wrapper.unmount()
  })

  it('switches compact option panels and returns the generated password', async () => {
    const wrapper = mount(PasswordGeneratorDialog, {
      props: { isOpen: true },
      attachTo: document.body,
      global: {
        plugins: [createPinia()],
        mocks: { $t: (_key: string, fallback: string) => fallback },
        stubs: { Transition: false },
      },
    })
    await flushPromises()

    const dialog = document.body.querySelector('[role="dialog"]') as HTMLElement
    ;(dialog.querySelector('[data-testid="password-mode-pin"]') as HTMLButtonElement).click()
    ;(dialog.querySelector('[data-testid="password-generate"]') as HTMLButtonElement).click()
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('generate_pin', { length: 6 })
    expect(dialog.textContent).toContain('482913')

    ;(dialog.querySelector('[data-testid="password-use"]') as HTMLButtonElement).click()
    await flushPromises()
    expect(wrapper.emitted('select')?.at(-1)).toEqual(['482913'])
    expect(wrapper.emitted('close')).toBeTruthy()
    wrapper.unmount()
  })
})
