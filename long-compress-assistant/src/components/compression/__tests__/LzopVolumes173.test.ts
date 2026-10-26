import { it, expect, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import CompressionSettingsPanel from '../CompressionSettingsPanel.vue'
import ConversionSettingsFields from '../ConversionSettingsFields.vue'
import ConversionBatchConfig from '../ConversionBatchConfig.vue'
import BatchConfigBar from '@/components/tasks/BatchConfigBar.vue'
import { useArchiveConversionStore } from '@/stores/archiveConversion'
import { defaultConversionSettings, normalizeConversionSettings } from '@/utils/archiveConversionDraft'

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: vi.fn(async (command: string) => command === 'get_archive_engine_capabilities' ? { available: true, formats: [] } : command === 'load_app_settings' ? '{}' : []) }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => vi.fn()) }))
vi.mock('@tauri-apps/api/dialog', () => ({ open: vi.fn(), ask: vi.fn() }))
const globalOptions = () => ({ plugins: [createPinia()], stubs: { Teleport: true, Transition: false, ProfileSelector: true, ProfileManager: true, PasswordGeneratorDialog: true } })
const choose = async (wrapper: ReturnType<typeof mount>, id: string, label: string) => {
  await wrapper.get(`[data-testid="${id}"]`).trigger('click')
  await wrapper.findAll('[role="option"]').find(option => option.find('strong').text() === label)!.trigger('click')
  await flushPromises()
}

it.each(['lzo', 'tar.lzo'] as const)('lzop173 %s inline settings retain inherited splitting and hide unsupported controls', async format => {
  const settings = { format, level: 6, password: '', filename: '资料', splitArchive: true, splitSize: '1', keepStructure: true, deleteAfter: false, verifyAfter: true, createSolidArchive: false }
  const wrapper = mount(CompressionSettingsPanel, { props: { modelValue: settings, allowSingleFileFormats: false, allowRegularFileFormats: format === 'lzo' }, global: globalOptions() })
  await flushPromises()
  expect(wrapper.get('[data-testid="compression-split-toggle"]').attributes('disabled')).toBeUndefined()
  expect(wrapper.get('[data-testid="compression-split-size"]').element).toHaveProperty('value', '1')
  for (const id of ['compression-level-control', 'compression-password-input', 'compression-zip-split-mode']) expect(wrapper.find(`[data-testid="${id}"]`).exists()).toBe(false)
  expect(wrapper.emitted('update:modelValue')).toBeUndefined()
  wrapper.unmount()
})

it.each(['lzo', 'tar.lzo'])('lzop173 conversion to %s preserves source password and volume size, and uses numeric volumes', async format => {
  const settings = normalizeConversionSettings({ ...defaultConversionSettings(), password: 'old-target', sourcePassword: 'source-only', splitMiB: 0.0625, zipSplitMode: 'standard' }, { format })
  expect(settings).toMatchObject({ password: '', sourcePassword: 'source-only', splitMiB: 0.0625, zipSplitMode: 'numeric' })
  const wrapper = mount(ConversionSettingsFields, { props: { settings }, global: globalOptions() }); await flushPromises()
  expect(wrapper.get('[data-testid="conversion-split"]').element).toHaveProperty('value', '0.0625')
  expect(wrapper.get('[data-testid="conversion-split"]').attributes('disabled')).toBeUndefined()
  expect(wrapper.find('[data-testid="conversion-password"]').exists()).toBe(false)
  wrapper.unmount()
})

it('lzop173 mixed LZOP and TAR.LZO tasks share a volume size without changing their formats', async () => {
  const wrapper = mount(BatchConfigBar, { props: { count: 2, mode: 'compression', subjects: [{ name: 'files', format: 'lzo' }, { name: 'folder', format: 'tar.lzo' }] }, global: globalOptions() })
  expect(wrapper.find('[data-testid="batch-lzop-volume-settings"]').exists()).toBe(false)
  await wrapper.get('button[aria-haspopup="dialog"]').trigger('click')
  for (const id of ['batch-level','batch-zip-password-encoding','batch-wim-method','batch-sevenz-method','batch-zip-method','batch-rar-recovery']) expect(wrapper.find(`[data-testid="${id}"]`).exists()).toBe(false)
  await choose(wrapper, 'batch-lzop-volume-mode', '统一每卷大小')
  await wrapper.get('[data-testid="batch-lzop-volume-size"]').setValue('0')
  expect(wrapper.get('[data-testid="batch-config-apply"]').attributes('disabled')).toBeDefined()
  await wrapper.get('[data-testid="batch-lzop-volume-size"]').setValue('1')
  await wrapper.get('[data-testid="batch-config-apply"]').trigger('click')
  expect(wrapper.emitted('apply')?.[0]?.[0]).toEqual({ splitArchive: true, splitSize: '1' })
  wrapper.unmount()
})

it.each([['lzo', 'LZOP（多文件，无加密）'], ['tar.lzo', 'TAR.LZO（目录，无加密）']])('lzop173 batch can explicitly disable splitting for %s', async (format, label) => {
  const wrapper = mount(BatchConfigBar, { props: { count: 2, mode: 'compression' }, global: globalOptions() })
  await wrapper.get('button[aria-haspopup="dialog"]').trigger('click')
  await choose(wrapper, 'batch-format', label)
  await choose(wrapper, 'batch-lzop-volume-mode', '不分卷')
  await wrapper.get('[data-testid="batch-config-apply"]').trigger('click')
  expect(wrapper.emitted('apply')?.[0]?.[0]).toEqual({ format, splitArchive: false })
  wrapper.unmount()
})

it.each([['lzo', 'LZOP（多文件，无目录）'], ['tar.lzo', 'TAR.LZO（目录，无加密）']])('lzop173 conversion batch keeps an explicit size when changing target to %s', async (format, label) => {
  const pinia = createPinia(); setActivePinia(pinia); const store = useArchiveConversionStore(); store.addSources(['C:/source.zip'])
  const apply = vi.spyOn(store, 'applySelected').mockResolvedValue('')
  const wrapper = mount(ConversionBatchConfig, { global: { ...globalOptions(), plugins: [pinia] } }); await flushPromises()
  await wrapper.get('[data-testid="conversion-batch-config"]').trigger('click')
  await wrapper.get('[data-testid="conversion-batch-change-split"]').setValue(true)
  await wrapper.get('[data-testid="conversion-batch-split"]').setValue(0.5)
  await choose(wrapper, 'conversion-batch-format', label)
  expect(wrapper.get('[data-testid="conversion-batch-split"]').element).toHaveProperty('value', '0.5')
  expect(wrapper.find('[data-testid="conversion-batch-level"]').exists()).toBe(false)
  await wrapper.get('[data-testid="conversion-batch-apply"]').trigger('click'); await flushPromises()
  expect(apply).toHaveBeenCalledWith({ format, splitMiB: 0.5 })
  wrapper.unmount()
})
