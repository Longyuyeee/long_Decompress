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
  diagnoseArchive: vi.fn(),
  cancelArchiveDiagnosis: vi.fn(),
  repairZip: vi.fn(),
  cancelZipRepair: vi.fn(),
}))

vi.mock('@tauri-apps/api/dialog', () => ({
  open: mocks.open,
  save: mocks.save,
}))
vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@/composables/useTauriCommands', () => ({
  useTauriCommands: () => ({
    invoke: mocks.invoke,
    diagnoseArchive: mocks.diagnoseArchive,
    cancelArchiveDiagnosis: mocks.cancelArchiveDiagnosis,
    repairZip: mocks.repairZip,
    cancelZipRepair: mocks.cancelZipRepair,
  }),
}))

describe('FileIntegrityView', () => {
  beforeEach(() => {
    localStorage.clear()
    mocks.open.mockReset()
    mocks.save.mockReset()
    mocks.invoke.mockReset()
    mocks.clipboardWrite.mockReset()
    mocks.diagnoseArchive.mockReset()
    mocks.cancelArchiveDiagnosis.mockReset()
    mocks.repairZip.mockReset()
    mocks.cancelZipRepair.mockReset()
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

  it('diagnoses a damaged archive, copies evidence, and repairs to a new verified file', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    mocks.open.mockResolvedValue('C:\\data\\damaged.zip')
    mocks.save.mockResolvedValue('C:\\data\\damaged.repaired.zip')
    mocks.diagnoseArchive.mockResolvedValue({
      filePath: 'C:\\data\\damaged.zip', fileSize: 4096, actualFormat: 'ZIP',
      status: 'crc_error', encrypted: false, splitArchive: false, volumesFound: 1,
      missingVolumes: [], totalFiles: 2, totalDirectories: 0, totalUncompressedSize: 1024,
      integrityTested: true, canRepair: true, recoverability: 'repairable',
      issues: [{ code: 'crc_error', severity: 'error', title: '内容校验失败', detail: '一个条目 CRC 错误' }],
      evidence: ['Integrity failure class: crc_error'],
    })
    mocks.repairZip.mockResolvedValue({
      outputPath: 'C:\\data\\damaged.repaired.zip', recoveredFiles: 1,
      recoveredDirectories: 0, skippedEntries: ['bad.txt: CRC mismatch'], verified: true,
    })
    mocks.clipboardWrite.mockResolvedValue(undefined)
    const wrapper = mount(FileIntegrityView, { global: { plugins: [pinia] } })

    await wrapper.get('[data-testid="archive-diagnostic-mode"]').trigger('click')
    await wrapper.findAll('button').find(button => button.text().includes('选择压缩包'))?.trigger('click')
    await flushPromises()
    await wrapper.get('input[type="password"]').setValue('local-secret')
    await wrapper.findAll('button').find(button => button.text().includes('开始诊断'))?.trigger('click')
    await flushPromises()

    expect(mocks.diagnoseArchive).toHaveBeenCalledWith(
      expect.stringMatching(/^diagnostic-/), 'C:\\data\\damaged.zip', 'local-secret',
    )
    expect(wrapper.get('[data-testid="diagnostic-report"]').text()).toContain('内容校验失败')
    await wrapper.findAll('button').find(button => button.text().includes('复制报告'))?.trigger('click')
    expect(mocks.clipboardWrite).toHaveBeenCalledWith(expect.stringContaining('Integrity failure class: crc_error'))
    expect(mocks.clipboardWrite.mock.calls[0][0]).not.toContain('local-secret')

    await wrapper.findAll('button').find(button => button.text().includes('选择位置并修复'))?.trigger('click')
    await flushPromises()
    expect(mocks.repairZip).toHaveBeenCalledWith(
      expect.stringMatching(/^repair-/), 'C:\\data\\damaged.zip', 'C:\\data\\damaged.repaired.zip',
    )
    expect(wrapper.get('[data-testid="repair-result"]').text()).toContain('恢复 1 个文件')
    expect(wrapper.text()).toContain('原压缩包不会被覆盖或删除')
  })
})
