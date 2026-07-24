import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import FileIntegrityView from '../FileIntegrityView.vue'
import { useAppStore } from '@/stores/app'

const mocks = vi.hoisted(() => ({
  open: vi.fn(),
  save: vi.fn(),
  invoke: vi.fn(),
  clipboardWrite: vi.fn(),
}))

vi.mock('@tauri-apps/api/dialog', () => ({
  open: mocks.open,
  save: mocks.save,
}))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@/composables/useTauriCommands', () => ({
  useTauriCommands: () => ({ invoke: mocks.invoke }),
}))

describe('FileIntegrityView', () => {
  beforeEach(() => {
    localStorage.clear()
    mocks.open.mockReset()
    mocks.save.mockReset()
    mocks.invoke.mockReset()
    mocks.clipboardWrite.mockReset()
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: mocks.clipboardWrite },
    })
    mocks.invoke.mockImplementation(async (command: string, payload?: any) => {
      if (command === 'load_app_settings') return '{}'
      if (command === 'calculate_checksum') {
        if (payload.path.endsWith('broken.bin')) throw new Error('read failed')
        return 'abc123'
      }
      return undefined
    })
  })

  it('calculates mixed results, copies successful checksums, and exports them', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const appStore = useAppStore()
    mocks.open.mockResolvedValue(['C:\\data\\good.bin', 'C:\\data\\broken.bin'])
    mocks.save.mockResolvedValue('C:\\data\\checksums.sha256')
    mocks.clipboardWrite.mockResolvedValue(undefined)

    const wrapper = mount(FileIntegrityView, {
      global: { plugins: [pinia] },
    })

    await wrapper.findAll('button').find(
      button => button.text().includes(appStore.t('integrity.select_files')),
    )?.trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('已选择 2 个文件')

    await wrapper.findAll('button').find(
      button => button.text() === appStore.t('integrity.calculate'),
    )?.trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('good.bin')
    expect(wrapper.text()).toContain('broken.bin')
    expect(wrapper.text()).toContain('abc123')
    expect(wrapper.text()).toContain('read failed')
    expect(appStore.successMessage).toBe(appStore.t('integrity.calc_complete', '校验和计算完成'))

    await wrapper.findAll('button').find(
      button => button.text() === appStore.t('integrity.copy'),
    )?.trigger('click')
    expect(mocks.clipboardWrite).toHaveBeenCalledWith('abc123')

    await wrapper.findAll('button').find(
      button => button.text() === appStore.t('integrity.export'),
    )?.trigger('click')
    await flushPromises()
    expect(mocks.invoke).toHaveBeenCalledWith('export_checksum_file', {
      path: 'C:\\data\\checksums.sha256',
      results: [{ file_name: 'good.bin', checksum: 'abc123' }],
      algorithm: 'sha256',
    })
  })

  it('verifies a checksum file and surfaces an invalid result', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const appStore = useAppStore()
    mocks.open.mockResolvedValue('C:\\data\\checksums.md5')
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === 'load_app_settings') return '{}'
      if (command === 'verify_checksum_file') {
        return { valid: false, message: '1 file mismatch' }
      }
      return undefined
    })

    const wrapper = mount(FileIntegrityView, {
      global: { plugins: [pinia] },
    })
    await wrapper.findAll('button').find(
      button => button.text().includes(appStore.t('integrity.mode.verify')),
    )?.trigger('click')
    await wrapper.findAll('button').find(
      button => button.text().includes(appStore.t('integrity.select_checksum')),
    )?.trigger('click')
    await flushPromises()

    expect(mocks.invoke).toHaveBeenCalledWith('verify_checksum_file', {
      checksumPath: 'C:\\data\\checksums.md5',
    })
    expect(wrapper.text()).toContain('1 file mismatch')
    expect(wrapper.text()).toContain(appStore.t('integrity.verify_failed'))
    expect(appStore.error).toBe(appStore.t('integrity.verify_failed', '✗ 校验失败'))
  })
})
