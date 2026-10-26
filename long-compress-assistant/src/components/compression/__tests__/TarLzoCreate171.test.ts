import { expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia } from 'pinia'
import ConversionSettingsFields from '../ConversionSettingsFields.vue'
import CompressionSettingsPanel from '../CompressionSettingsPanel.vue'
import { defaultConversionSettings, normalizeConversionSettings } from '@/utils/archiveConversionDraft'
import { FORMAT_CAPABILITIES, effectiveFormatForPassword } from '@/utils/compressionFormat'

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: vi.fn(async (command: string) => command === 'get_archive_engine_capabilities' ? { available: true, formats: [] } : command === 'load_app_settings' ? '{}' : command === 'is_context_menu_registered' ? true : []) }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => vi.fn()) }))

it('TAR.LZO keeps the source password but removes unsupported target controls and the single-file restriction', async () => {
  const settings = normalizeConversionSettings({ ...defaultConversionSettings(), format: '7z', sourcePassword: 'source-secret', password: 'old-target', splitMiB: 2, encryptFilenames: true, outputPath: 'D:/out.7z' }, { format: 'tar.lzo' })
  expect(settings).toMatchObject({ format: 'tar.lzo', sourcePassword: 'source-secret', password: '', splitMiB: 2, encryptFilenames: false, outputPath: 'D:/out.tar.lzo' })
  const wrapper = mount(ConversionSettingsFields, { props: { settings }, global: { plugins: [createPinia()], stubs: { Teleport: true } } })
  await flushPromises()
  expect(wrapper.get('[data-testid="conversion-source-password"]').element).toHaveProperty('value', 'source-secret')
  for (const id of ['conversion-password', 'conversion-level-control', 'conversion-hidden-names', 'conversion-single-file-hint']) expect(wrapper.find(`[data-testid="${id}"]`).exists()).toBe(false)
  expect(wrapper.emitted('update')).toBeUndefined()
  wrapper.unmount()
})

it('folder configuration offers TAR.LZO, hides fixed controls, and keeps inherited configuration unchanged on open', async () => {
  const settings = { format: 'tar.lzo' as const, level: 6, password: '', filename: '资料', splitArchive: false, splitSize: '1024', keepStructure: true, deleteAfter: false, verifyAfter: true, createSolidArchive: false }
  const wrapper = mount(CompressionSettingsPanel, { props: { modelValue: settings, allowSingleFileFormats: false }, global: { plugins: [createPinia()], stubs: { Teleport: true, ProfileSelector: true, ProfileManager: true, PasswordGeneratorDialog: true } } })
  await flushPromises()
  expect(wrapper.find('[data-testid="compression-level-control"]').exists()).toBe(false)
  expect(wrapper.find('[data-testid="compression-password-input"]').exists()).toBe(false)
  expect(wrapper.emitted('update:modelValue')).toBeUndefined()
  await wrapper.get('[data-testid="compression-format-select"]').trigger('click')
  const tar = wrapper.findAll('[role="option"]').find(option => option.text().includes('TAR.LZO'))!
  expect(tar.exists()).toBe(true)
  expect(tar.attributes('disabled')).toBeUndefined()
  expect(FORMAT_CAPABILITIES.find(item => item.format === 'tar.lzo')).toMatchObject({ canCompress: true, singleFileOnly: false, supportsSplit: true })
  expect(effectiveFormatForPassword('tar.lzo', '')).toBe('tar.lzo')
  wrapper.unmount()
})
