import { flushPromises, mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import CompressionSettingsPanel from '../CompressionSettingsPanel.vue'
import type { CompressionOptions } from '@/stores/compression'

const invoke = vi.fn()
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: (...args: unknown[]) => invoke(...args) }))

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
})
