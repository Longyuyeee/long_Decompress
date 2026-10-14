import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import FileIntegrityView from '../FileIntegrityView.vue'
import { useAppStore } from '@/stores/app'
import { useUIStore } from '@/stores/ui'
import { useTaskStore } from '@/stores/task'

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
  it('opens rescue from a failed task and queues the encrypted rebuilt archive with its original output choices', async () => {
    const pinia = createPinia(); setActivePinia(pinia)
    const tasks = useTaskStore()
    tasks.addTask({ id:'rescue-original', name:'broken.zip', type:'decompression', sourceFiles:['C:/broken.zip'], outputPath:'C:/chosen', extractToSubfolder:false })
    tasks.failTask('rescue-original','[archive-inspection:damaged] 目录丢失')
    const persisted = vi.spyOn(tasks,'waitForHistoryPersistence').mockResolvedValue(true)
    mocks.diagnoseArchive.mockResolvedValue({filePath:'C:/broken.zip',fileSize:8000,actualFormat:'ZIP',status:'directory_lost',encrypted:true,splitArchive:false,volumesFound:1,missingVolumes:[],totalFiles:1,totalDirectories:0,totalUncompressedSize:100,totalUncompressedSizeKnown:false,integrityTested:false,canRepair:true,recoverability:'local_entries',issues:[],evidence:[]})
    mocks.repairZip.mockResolvedValue({outputPath:'C:/rescued.zip',recoveredFiles:1,recoveredDirectories:0,skippedEntries:[],partial:true,verified:true,directoryRebuilt:true,outputEncrypted:true})
    mocks.save.mockResolvedValue('C:/rescued.zip')
    useAppStore().openArchiveForDiagnosis({path:'C:/broken.zip',taskId:'rescue-original',password:'correct secret'})
    const wrapper=mount(FileIntegrityView,{global:{plugins:[pinia]}})
    await flushPromises()
    expect(useAppStore().pendingArchiveDiagnosis).toBeNull()
    expect(mocks.diagnoseArchive).toHaveBeenCalledWith(expect.any(String),'C:/broken.zip','correct secret')
    expect(wrapper.get('[data-testid="recovery-task-context"]').text()).toContain('失败的解压任务')
    expect(wrapper.get('[data-testid="diagnostic-report"]').text()).toContain('已找到文件 / 目录')
    await wrapper.findAll('button').find(button=>button.text().includes('选择位置并修复'))!.trigger('click'); await flushPromises()
    await wrapper.get('.archive-password').setValue('later edit')
    await wrapper.get('[data-testid="queue-recovered-archive"]').trigger('click'); await flushPromises()
    expect(tasks.tasks[1]).toMatchObject({status:'pending',sourceFiles:['C:/rescued.zip'],outputPath:'C:/chosen',extractToSubfolder:false,password:'correct secret',retryOfTaskId:'rescue-original'})
    await wrapper.get('[data-testid="queue-recovered-archive"]').trigger('click'); await flushPromises()
    expect(tasks.tasks).toHaveLength(2)
    expect(persisted).toHaveBeenCalledWith('rescue-original')
    wrapper.unmount()
  })

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
      recoveredDirectories: 0, skippedEntries: ['bad.txt: CRC mismatch'], verified: true, partial: true, directoryRebuilt: true,
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
      'local-secret',
    )
    expect(wrapper.get('[data-testid="repair-result"]').text()).toContain('恢复 1 个文件')
    expect(wrapper.get('[data-testid="repair-result"]').text()).toContain('部分恢复')
    expect(wrapper.get('[data-testid="repair-result"]').text()).toContain('原包总条目数无法确认')
    expect(useUIStore().toasts).toContainEqual(expect.objectContaining({ type: 'warning', message: expect.stringContaining('部分恢复') }))
    expect(wrapper.text()).toContain('原压缩包不会被覆盖或删除')
  })

  it('rescues encrypted entries with the entered password and shows the new package encryption', async () => {
    const pinia = createPinia(); setActivePinia(pinia)
    mocks.open.mockResolvedValue('C:/下载/资料.zip.002')
    mocks.save.mockResolvedValue('C:/下载/资料.repaired.zip')
    mocks.diagnoseArchive.mockResolvedValue({
      actualFormat: 'ZIP', fileSize: 1000, status: 'ambiguous_data', encrypted: true,
      splitArchive: true, volumesFound: 3, totalFiles: 3, totalDirectories: 1,
      totalUncompressedSize: 3000, canRepair: true, issues: [], evidence: [],
    })
    mocks.repairZip.mockResolvedValue({outputPath:'C:/下载/资料.repaired.zip', recoveredFiles:2,
      recoveredDirectories:1, skippedEntries:['坏文件: 内容校验失败'], partial:true,
      verified:true, directoryRebuilt:false, outputEncrypted:true})
    const wrapper = mount(FileIntegrityView, {global:{plugins:[pinia]}})
    await wrapper.get('[data-testid="archive-diagnostic-mode"]').trigger('click')
    await wrapper.get('.archive-select').trigger('click'); await flushPromises()
    await wrapper.get('.archive-password').setValue('救援密码')
    await wrapper.get('.archive-primary').trigger('click'); await flushPromises()
    expect(wrapper.get('[data-testid="repair-password-note"]').text()).toContain('AES-256')
    await wrapper.findAll('button').find(button => button.text().includes('选择位置并修复'))?.trigger('click')
    await flushPromises()
    expect(mocks.save).toHaveBeenCalledWith(expect.objectContaining({defaultPath:'C:/下载/资料.repaired.zip'}))
    expect(mocks.repairZip).toHaveBeenCalledWith(expect.stringMatching(/^repair-/),
      'C:/下载/资料.zip.002','C:/下载/资料.repaired.zip','救援密码')
    expect(wrapper.get('[data-testid="repair-output-encryption"]').text()).toContain('密码与恢复时填写的一致')
    expect(wrapper.get('[data-testid="repair-result"]').text()).not.toContain('救援密码')
    await wrapper.findAll('button').find(button => button.text().includes('定位新包'))?.trigger('click')
    expect(mocks.invoke).toHaveBeenCalledWith('open_in_explorer',{path:'C:/下载/资料.repaired.zip'})
    await wrapper.get('.archive-password').setValue('纠正后的密码'); await flushPromises()
    expect(wrapper.find('[data-testid="diagnostic-report"]').exists()).toBe(false)
  })

  it('shows an ambiguous encrypted-content report without offering repair or silently retrying', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    mocks.open.mockResolvedValue('C:\\下载\\资料.zip.003')
    const returnedReport = {
      filePath: 'C:\\下载\\资料.zip.003', fileSize: 8192, actualFormat: 'ZIP',
      status: 'ambiguous_data', encrypted: true, splitArchive: true, volumesFound: 3,
      missingVolumes: [], totalFiles: 2, totalDirectories: 0, totalUncompressedSize: 1024,
      integrityTested: true, canRepair: false, recoverability: 'diagnostic_only',
      issues: [{ code: 'ambiguous_data', severity: 'error', title: '密码或加密内容异常',
        detail: '无法区分密码错误与加密内容损坏；反复尝试密码不能修复损坏' }], evidence: [],
    }
    mocks.diagnoseArchive.mockResolvedValue(returnedReport)
    const wrapper = mount(FileIntegrityView, { global: { plugins: [pinia] } })
    await wrapper.get('[data-testid="archive-diagnostic-mode"]').trigger('click')
    await wrapper.get('.archive-select').trigger('click')
    await flushPromises()
    await wrapper.get('.archive-password').setValue('local-secret')
    await wrapper.get('.archive-primary').trigger('click')
    await flushPromises()
    const report = wrapper.get('[data-testid="diagnostic-report"]')
    expect(report.get('h2').text()).toBe('密码或加密内容异常')
    expect(report.text()).toContain('反复尝试密码不能修复损坏')
    expect(report.text()).not.toContain('ambiguous_data')
    expect(wrapper.text()).not.toContain('选择位置并修复')
    expect(mocks.diagnoseArchive).toHaveBeenCalledTimes(1)
    expect(mocks.repairZip).not.toHaveBeenCalled()

    let finish!: (report: any) => void
    mocks.diagnoseArchive.mockImplementationOnce(() => new Promise(resolve => { finish = resolve }))
    await wrapper.get('.archive-primary').trigger('click')
    expect(wrapper.find('[data-testid="diagnostic-report"]').exists()).toBe(false)
    expect(wrapper.get('.archive-select').attributes('disabled')).toBeDefined()
    finish(returnedReport)
    await flushPromises()
    expect(wrapper.get('.archive-select').attributes('disabled')).toBeUndefined()
  })
})
