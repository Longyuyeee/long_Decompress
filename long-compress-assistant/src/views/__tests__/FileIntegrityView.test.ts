import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import FileIntegrityView from '../FileIntegrityView.vue'
import { useAppStore } from '@/stores/app'
import { useUIStore } from '@/stores/ui'
import { useTaskStore } from '@/stores/task'
import { ARCHIVE_RECOVERY_DRAFT_KEY } from '@/composables/useArchiveRecoveryBatch'

const mocks = vi.hoisted(() => ({
  open: vi.fn(),
  save: vi.fn(),
  invoke: vi.fn(),
  clipboardWrite: vi.fn(),
  diagnoseArchive: vi.fn(),
  cancelArchiveDiagnosis: vi.fn(),
  repairZip: vi.fn(),
  repairRar: vi.fn(),
  cancelZipRepair: vi.fn(),
  createRarRecoveryVolumes: vi.fn(),
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
    repairRar: mocks.repairRar,
    cancelZipRepair: mocks.cancelZipRepair,
    createRarRecoveryVolumes: mocks.createRarRecoveryVolumes,
  }),
}))

describe('FileIntegrityView', () => {
  it('restores a single repaired result without secrets, preserves its own encoding and requires re-entering its password', async () => {
    const pinia=createPinia(); setActivePinia(pinia)
    mocks.diagnoseArchive.mockResolvedValue({filePath:'C:/old.zip',actualFormat:'ZIP',status:'directory_lost',encrypted:true,issues:[],evidence:[],missingVolumes:[],totalFiles:1,totalDirectories:0,canRepair:true})
    mocks.repairZip.mockResolvedValue({outputPath:'D:/rescued.zip',format:'ZIP',outputEncrypted:true,recoveredFiles:1,recoveredDirectories:0,skippedEntries:[],verified:true})
    mocks.save.mockResolvedValue('D:/rescued.zip')
    useAppStore().openArchiveForDiagnosis({path:'C:/old.zip',password:'original secret',zipPasswordEncoding:'cp932',outputPath:'E:/chosen',extractToSubfolder:false})
    let wrapper=mount(FileIntegrityView,{global:{plugins:[pinia]}}); await flushPromises()
    await wrapper.get('[data-testid="start-archive-repair"]').trigger('click'); await flushPromises()
    wrapper.findComponent({name:'ZipPasswordEncodingField'}).vm.$emit('update:modelValue','cp950')
    await wrapper.get('.archive-password').setValue('changed source secret'); await flushPromises()
    expect(localStorage.getItem(ARCHIVE_RECOVERY_DRAFT_KEY)).not.toContain('secret')
    wrapper.unmount()
    const reopened=createPinia(); setActivePinia(reopened)
    wrapper=mount(FileIntegrityView,{global:{plugins:[reopened]}}); await flushPromises()
    expect(wrapper.find('[data-testid="repair-result"]').exists()).toBe(false)
    await wrapper.get('[data-testid="archive-recovery-restore"]').trigger('click'); await flushPromises()
    expect(wrapper.find('[data-testid="diagnostic-report"]').exists()).toBe(false)
    expect(wrapper.get('[data-testid="repair-output-encryption"]').text()).toContain('CP932')
    expect(wrapper.findComponent({name:'ZipPasswordEncodingField'}).props('modelValue')).toBe('cp950')
    expect(wrapper.get('[data-testid="single-repair-result-password"]').element).toHaveProperty('value','')
    await wrapper.get('[data-testid="queue-recovered-archive"]').trigger('click'); await flushPromises()
    expect(useTaskStore().tasks).toHaveLength(0)
    await wrapper.get('[data-testid="single-repair-result-password"]').setValue('original secret')
    await wrapper.get('[data-testid="browse-repaired-archive"]').trigger('click')
    expect(useAppStore().takeArchiveBrowserOptions()).toEqual({password:'original secret',zipPasswordEncoding:'cp932'})
    mocks.invoke.mockRejectedValueOnce(new Error('file moved'))
    await wrapper.get('[data-testid="queue-recovered-archive"]').trigger('click'); await flushPromises()
    expect(useTaskStore().tasks).toHaveLength(0)
    await wrapper.get('[data-testid="queue-recovered-archive"]').trigger('click'); await flushPromises()
    expect(useTaskStore().tasks[0]).toMatchObject({sourceFiles:['D:/rescued.zip'],zipPasswordEncoding:'cp932',password:'original secret',outputPath:'E:/chosen',extractToSubfolder:false})
    expect(localStorage.getItem(ARCHIVE_RECOVERY_DRAFT_KEY)).not.toContain('secret')
    wrapper.unmount()
  })
  it.each(['cp932','cp936','cp950'] as const)('keeps %s on rescued output even after editing the source password and encoding', async encoding => {
    const pinia=createPinia(); setActivePinia(pinia)
    mocks.diagnoseArchive.mockResolvedValue({filePath:'C:/old.zip',actualFormat:'ZIP',status:'directory_lost',encrypted:true,issues:[],evidence:[],missingVolumes:[],totalFiles:1,totalDirectories:0,canRepair:true})
    mocks.repairZip.mockResolvedValue({outputPath:'D:/rescued.zip',format:'ZIP',outputEncrypted:true,recoveredFiles:1,recoveredDirectories:0,skippedEntries:[],verified:true})
    mocks.save.mockResolvedValue('D:/rescued.zip')
    const app=useAppStore()
    app.openArchiveForDiagnosis({path:'C:/old.zip',password:'source password',zipPasswordEncoding:encoding})
    const wrapper=mount(FileIntegrityView,{global:{plugins:[pinia]}}); await flushPromises()
    expect(mocks.diagnoseArchive).toHaveBeenCalledWith(expect.any(String),'C:/old.zip','source password',encoding)
    await wrapper.get('[data-testid="start-archive-repair"]').trigger('click'); await flushPromises()
    expect(mocks.repairZip).toHaveBeenCalledWith(expect.any(String),'C:/old.zip','D:/rescued.zip','source password',encoding)
    wrapper.findComponent({name:'ZipPasswordEncodingField'}).vm.$emit('update:modelValue','utf8')
    await wrapper.get('.archive-password').setValue('later password'); await flushPromises()
    expect(wrapper.find('[data-testid="diagnostic-report"]').exists()).toBe(false)
    expect(wrapper.get('[data-testid="repair-output-encryption"]').text()).toContain(encoding.toUpperCase())
    await wrapper.get('[data-testid="queue-recovered-archive"]').trigger('click'); await flushPromises()
    expect(useTaskStore().tasks[0]).toMatchObject({zipPasswordEncoding:encoding,password:'source password',sourceFiles:['D:/rescued.zip']})
    await wrapper.get('[data-testid="browse-repaired-archive"]').trigger('click')
    expect(app.pendingArchiveBrowserPath).toBe('D:/rescued.zip')
    expect(app.takeArchiveBrowserOptions()).toEqual({password:'source password',zipPasswordEncoding:encoding})
    expect(app.takeArchiveBrowserOptions()).toEqual({})
    wrapper.unmount()
  })
  it('shows a password encoding conclusion rather than an untranslated status or damage', async () => {
    const pinia=createPinia(); setActivePinia(pinia)
    mocks.diagnoseArchive.mockResolvedValue({filePath:'C:/old.zip',actualFormat:'ZIP',status:'password_encoding',encrypted:true,issues:[],evidence:[],missingVolumes:[],totalFiles:0,totalDirectories:0,canRepair:false})
    useAppStore().openArchiveForDiagnosis({path:'C:/old.zip',password:'🔒',zipPasswordEncoding:'cp932'})
    const wrapper=mount(FileIntegrityView,{global:{plugins:[pinia]}}); await flushPromises()
    expect(wrapper.get('[data-testid="diagnostic-report"]').text()).toContain('ZIP 密码编码不兼容')
    expect(wrapper.find('[data-testid="start-archive-repair"]').exists()).toBe(false)
    wrapper.unmount()
  })
  it('offers missing-disk rescue and keeps recovery lists folded while queuing the original password', async () => {
    const pinia = createPinia(); setActivePinia(pinia)
    mocks.diagnoseArchive.mockResolvedValue({filePath:'C:/parts.zip',fileSize:8000,actualFormat:'ZIP',status:'partial_recovery',encrypted:true,splitArchive:true,volumesFound:3,missingVolumes:['parts.z02'],totalFiles:3,totalDirectories:1,totalUncompressedSize:100,integrityTested:false,canRepair:true,recoverability:'missing_volume_entries',issues:[{code:'missing_volume',severity:'warning',title:'缺少分卷',detail:'缺少 parts.z02，可尝试救回 3 个文件'}],evidence:[]})
    mocks.repairZip.mockResolvedValue({outputPath:'D:/rescued.zip',recoveredFiles:3,recoveredDirectories:1,recoveredEntries:['中文/文件0.txt','中文/文件3.txt','中文/文件4.txt'],skippedEntries:['中文/文件1.txt：涉及缺少的 parts.z02'],partial:true,verified:true,directoryRebuilt:false,outputEncrypted:true})
    mocks.save.mockResolvedValue('D:/rescued.zip')
    useAppStore().openArchiveForDiagnosis({path:'C:/parts.zip',password:'original password',outputPath:'D:/chosen',extractToSubfolder:false})
    const wrapper = mount(FileIntegrityView,{global:{plugins:[pinia]}}); await flushPromises()
    expect(wrapper.get('[data-testid="diagnostic-report"]').text()).toContain('缺少分卷，可尝试部分救援')
    expect(wrapper.get('[data-testid="diagnostic-report"]').text()).toContain('可尝试文件 / 目录')
    await wrapper.get('[data-testid="start-archive-repair"]').trigger('click'); await flushPromises()
    expect(wrapper.get('[data-testid="recovery-file-details"]').attributes('open')).toBeUndefined()
    expect(wrapper.get('[data-testid="recovery-skipped-details"]').text()).toContain('parts.z02')
    expect(wrapper.text()).not.toContain('目录信息已重建')
    await wrapper.get('[data-testid="queue-recovered-archive"]').trigger('click'); await flushPromises()
    expect(useTaskStore().tasks[0]).toMatchObject({sourceFiles:['D:/rescued.zip'],password:'original password',outputPath:'D:/chosen',extractToSubfolder:false})
    wrapper.unmount()
  })
  it('creates recovery volumes for a healthy RAR group in a compact dialog', async () => {
    const pinia=createPinia(); setActivePinia(pinia)
    mocks.diagnoseArchive.mockResolvedValue({filePath:'C:/parts.part1.rar',fileSize:8000,actualFormat:'RAR',status:'healthy',encrypted:true,splitArchive:true,volumesFound:3,missingVolumes:[],totalFiles:1,totalDirectories:0,totalUncompressedSize:100,integrityTested:true,canRepair:false,recoverability:'healthy',issues:[],evidence:[]})
    mocks.createRarRecoveryVolumes.mockResolvedValue({outputFiles:['D:/parts.part1.rev'],totalBytes:2000,dataVolumes:3})
    useAppStore().openArchiveForDiagnosis({path:'C:/parts.part1.rar',password:'original secret'})
    const wrapper=mount(FileIntegrityView,{global:{plugins:[pinia],stubs:{Teleport:true,Transition:false}}}); await flushPromises()
    expect(wrapper.find('[data-testid="create-recovery-volumes-dialog"]').exists()).toBe(false)
    await wrapper.get('[data-testid="open-create-recovery-volumes"]').trigger('click'); await flushPromises()
    await wrapper.get('[data-testid="create-recovery-directory"]').setValue('D:/')
    await wrapper.get('[data-testid="confirm-create-recovery"]').trigger('click'); await flushPromises()
    expect(mocks.createRarRecoveryVolumes).toHaveBeenCalledWith(expect.any(String),'C:/parts.part1.rar','D:/',1,'original secret')
    expect(wrapper.get('[data-testid="created-recovery-volumes"]').text()).toContain('3 个数据卷生成 1 个恢复卷')
    wrapper.unmount()
  })
  it('opens rescue from a failed task and queues the encrypted rebuilt archive with its original output choices', async () => {
    const pinia = createPinia(); setActivePinia(pinia)
    const tasks = useTaskStore()
    tasks.addTask({ id:'rescue-original', name:'broken.zip', type:'decompression', sourceFiles:['C:/broken.zip'], outputPath:'C:/chosen', extractToSubfolder:false })
    tasks.failTask('rescue-original','[archive-inspection:damaged] 目录丢失')
    const persisted = vi.spyOn(tasks,'waitForHistoryPersistence').mockResolvedValue(true)
    mocks.diagnoseArchive.mockResolvedValue({filePath:'C:/broken.zip',fileSize:8000,actualFormat:'ZIP',status:'directory_lost',encrypted:true,splitArchive:false,volumesFound:1,missingVolumes:[],totalFiles:1,totalDirectories:0,totalUncompressedSize:100,totalUncompressedSizeKnown:false,integrityTested:false,canRepair:true,recoverability:'local_entries',issues:[],evidence:[]})
    mocks.repairZip.mockResolvedValue({outputPath:'C:/rescued.zip',recoveredFiles:1,recoveredDirectories:0,skippedEntries:[],partial:true,verified:true,directoryRebuilt:true,outputEncrypted:true})
    mocks.save.mockResolvedValue('C:/rescued.zip')
    useAppStore().openArchiveForDiagnosis({path:'C:/broken.zip',taskId:'rescue-original',password:'correct secret',outputPath:'D:/edited-output',extractToSubfolder:true})
    const wrapper=mount(FileIntegrityView,{global:{plugins:[pinia]}})
    await flushPromises()
    expect(useAppStore().pendingArchiveDiagnosis).toBeNull()
    expect(mocks.diagnoseArchive).toHaveBeenCalledWith(expect.any(String),'C:/broken.zip','correct secret')
    expect(wrapper.get('[data-testid="recovery-task-context"]').text()).toContain('失败的解压任务')
    expect(wrapper.get('[data-testid="diagnostic-report"]').text()).toContain('已找到文件 / 目录')
    await wrapper.findAll('button').find(button=>button.text().includes('选择位置并修复'))!.trigger('click'); await flushPromises()
    await wrapper.get('.archive-password').setValue('later edit')
    await wrapper.get('[data-testid="queue-recovered-archive"]').trigger('click'); await flushPromises()
    expect(tasks.tasks[1]).toMatchObject({status:'pending',sourceFiles:['C:/rescued.zip'],outputPath:'D:/edited-output',extractToSubfolder:true,password:'correct secret',retryOfTaskId:'rescue-original'})
    await wrapper.get('[data-testid="queue-recovered-archive"]').trigger('click'); await flushPromises()
    expect(tasks.tasks).toHaveLength(2)
    expect(persisted).toHaveBeenCalledWith('rescue-original')
    wrapper.unmount()
  })

  it('repairs a RAR group to a new folder and queues its first volume with the original password', async () => {
    const pinia = createPinia(); setActivePinia(pinia)
    mocks.diagnoseArchive.mockResolvedValue({filePath:'C:/parts.part2.rar',fileSize:8000,actualFormat:'RAR',status:'missing_volume',encrypted:false,splitArchive:true,volumesFound:1,missingVolumes:['parts.part1.rar'],totalFiles:0,totalDirectories:0,totalUncompressedSize:0,integrityTested:false,canRepair:true,recoverability:'rar_recovery_volumes',issues:[],evidence:[]})
    mocks.repairRar.mockResolvedValue({format:'RAR',outputPath:'D:/saved/parts.repaired/parts.part1.rar',outputFiles:['part1','part2'],recoveredFiles:2,recoveredDirectories:0,skippedEntries:[],partial:false,verified:true,directoryRebuilt:false,outputEncrypted:true})
    mocks.open.mockResolvedValue('D:/saved')
    useAppStore().openArchiveForDiagnosis({path:'C:/parts.part2.rar',password:'original secret'})
    const wrapper=mount(FileIntegrityView,{global:{plugins:[pinia]}}); await flushPromises()
    expect(wrapper.text()).toContain('恢复卷补齐或修复 RAR')
    await wrapper.findAll('button').find(button=>button.text().includes('选择位置并修复'))!.trigger('click'); await flushPromises()
    expect(mocks.open).toHaveBeenCalledWith(expect.objectContaining({directory:true}))
    expect(mocks.repairRar).toHaveBeenCalledWith(expect.any(String),'C:/parts.part2.rar','D:/saved','original secret')
    expect(mocks.repairZip).not.toHaveBeenCalled()
    expect(wrapper.get('[data-testid="rar-repair-volumes"]').text()).toContain('2 个数据卷')
    expect(wrapper.get('[data-testid="repair-output-encryption"]').text()).toContain('保留原加密方式与密码')
    await wrapper.get('[data-testid="queue-recovered-archive"]').trigger('click'); await flushPromises()
    expect(useTaskStore().tasks[0]).toMatchObject({status:'pending',sourceFiles:['D:/saved/parts.repaired/parts.part1.rar'],password:'original secret'})
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
    mocks.repairRar.mockReset()
    mocks.cancelZipRepair.mockReset()
    mocks.createRarRecoveryVolumes.mockReset()
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
