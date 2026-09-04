import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import CompressionSettingsPanel from '../CompressionSettingsPanel.vue'
import type { CompressionOptions } from '@/stores/compression'

const invoke = vi.fn()
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: (...args: unknown[]) => invoke(...args) }))

const vaultEntry = {
  id: 'vault-1',
  name: '常用归档密码',
  password: 'Archive!2026',
  notes: '项目文件',
  tags: ['archive'],
  category: 'Work',
  created_at: '2026-09-01T00:00:00Z',
  updated_at: '2026-09-01T00:00:00Z',
  last_used: null,
  favorite: true,
  use_count: 3,
  usage_history: {},
}

const settings = (overrides: Partial<CompressionOptions> = {}): CompressionOptions => ({
  format: 'zip',
  level: 6,
  password: '',
  filename: 'archive',
  splitArchive: false,
  splitSize: '1024',
  keepStructure: true,
  deleteAfter: false,
  verifyAfter: true,
  createSolidArchive: false,
  ...overrides,
})

const mountPanel = (modelValue: CompressionOptions, allowSplitArchive = true) => mount(CompressionSettingsPanel, {
  props: { modelValue, allowSplitArchive },
  global: {
    plugins: [createPinia()],
    stubs: {
      PasswordGeneratorDialog: true,
      ProfileSelector: true,
      ProfileManager: true,
      Teleport: true,
      Transition: false,
    },
  },
})

describe('CompressionSettingsPanel split archive settings', () => {
  beforeEach(() => {
    invoke.mockReset()
    invoke.mockImplementation(async (command: string) => {
      if (command === 'get_archive_engine_capabilities') {
        return {
          available: true,
          fullEngine: true,
          formats: [],
          browseExtensions: [],
          nestedExtensions: [],
          boundedPreviewFormats: [],
          imagePreviewExtensions: [],
          textPreviewExtensions: [],
          message: '',
        }
      }
      if (command === 'is_encrypted_password_service_unlocked') return true
      if (command === 'list_encrypted_passwords') return [vaultEntry]
      if (command === 'increment_encrypted_password_use_count') return { ...vaultEntry, use_count: 4 }
      return '{}'
    })
  })

  it('shows and edits split ZIP settings directly in each configuration panel', async () => {
    const wrapper = mountPanel(settings())
    await flushPromises()

    const section = wrapper.get('[data-testid="compression-split-settings"]')
    expect(section.text()).toContain('分卷压缩')
    expect(section.text()).toContain('关闭')

    await wrapper.get('[data-testid="compression-split-toggle"]').setValue(true)
    const size = wrapper.get('[data-testid="compression-split-size"]')
    expect((size.element as HTMLInputElement).value).toBe('1024')
    await size.setValue('512')

    const emitted = wrapper.emitted('update:modelValue')?.at(-1)?.[0] as CompressionOptions
    expect(emitted.splitArchive).toBe(true)
    expect(emitted.splitSize).toBe('512')
  })

  it('keeps the setting visible and explains why it is unavailable', async () => {
    const wrapper = mountPanel(settings(), false)
    await flushPromises()

    const toggle = wrapper.get('[data-testid="compression-split-toggle"]')
    expect(toggle.attributes('disabled')).toBeDefined()
    expect(wrapper.get('[data-testid="compression-split-settings"]').text()).toContain('目前仅支持普通文件')

    await wrapper.setProps({
      allowSplitArchive: true,
      modelValue: settings({ password: 'secret' }),
    })
    await flushPromises()
    expect(wrapper.get('[data-testid="compression-split-settings"]').text()).toContain('请先清除压缩密码')
  })

  it('opens the password vault list on focus and fills the selected password', async () => {
    const wrapper = mountPanel(settings())
    await flushPromises()

    await wrapper.get('[data-testid="compression-password-input"]').trigger('focus')
    const menu = wrapper.get('[data-testid="compression-password-vault-menu"]')
    expect(menu.text()).toContain('常用归档密码')
    expect(menu.text()).not.toContain('Archive!2026')

    await wrapper.get('[data-testid="compression-vault-password-vault-1"]').trigger('click')
    await flushPromises()
    const emitted = wrapper.emitted('update:modelValue')?.at(-1)?.[0] as CompressionOptions
    expect(emitted.password).toBe('Archive!2026')
    expect(invoke).toHaveBeenCalledWith('increment_encrypted_password_use_count', { id: 'vault-1' })
    await new Promise(resolve => window.setTimeout(resolve, 180))
    expect(wrapper.find('[data-testid="compression-password-vault-menu"]').exists()).toBe(false)
  })
})
