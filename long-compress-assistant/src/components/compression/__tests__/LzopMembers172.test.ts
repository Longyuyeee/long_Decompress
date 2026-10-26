import { it, expect, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia } from 'pinia'
import CompressionSettingsPanel from '../CompressionSettingsPanel.vue'

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: vi.fn(async (command: string) => command === 'get_archive_engine_capabilities' ? { available: true, formats: [] } : command === 'load_app_settings' ? '{}' : []) }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => vi.fn()) }))

it.each([true, false])('lzop172 allows ordinary-file groups=%s while keeping directory formats and inherited settings', async allowRegularFileFormats => {
  const settings = { format: 'zip' as const, level: 6, password: '', filename: '', splitArchive: false, splitSize: '1024', keepStructure: true, deleteAfter: false, verifyAfter: true, createSolidArchive: false }
  const wrapper = mount(CompressionSettingsPanel, { props: { modelValue: settings, allowSingleFileFormats: false, allowRegularFileFormats }, global: { plugins: [createPinia()], stubs: { Teleport: true, ProfileSelector: true, ProfileManager: true, PasswordGeneratorDialog: true } } })
  await flushPromises()
  expect(wrapper.emitted('update:modelValue')).toBeUndefined()
  await wrapper.get('[data-testid="compression-format-select"]').trigger('click')
  const option = (label: string) => wrapper.findAll('[role="option"]').find(item => item.find('strong').text() === label)!
  expect(option('LZOP').attributes('aria-disabled') !== undefined).toBe(!allowRegularFileFormats)
  expect(option('TAR.LZO').attributes('aria-disabled')).toBeUndefined()
  expect(option('LZMA').attributes('aria-disabled')).toBeDefined()
  wrapper.unmount()
})
