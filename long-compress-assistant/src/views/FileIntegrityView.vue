<script setup lang="ts">
import HelpHint from '@/components/ui/HelpHint.vue'
import ZpaqBlockDetails from '@/components/archive/ZpaqBlockDetails.vue'
import {zpaqReportLines} from '@/types/zpaqBlocks'
import {prepareZpaqSource,type ZpaqGroup} from '@/utils/zpaqMultipart'
import type { ZpaqSnapshot } from '@/types/zpaqSnapshot'
import { useChecksumOperation } from '@/composables/useChecksumOperation'
import ChecksumProgressCard from '@/components/integrity/ChecksumProgressCard.vue'
import type { ChecksumCalculationEntry } from '@/types/checksum'
import ChecksumVerificationPanel from '@/components/integrity/ChecksumVerificationPanel.vue'
import ArchiveRecoveryDetails from '@/components/compression/ArchiveRecoveryDetails.vue'
import { computed, inject, nextTick, onUnmounted, ref, watch } from 'vue'
import { TOOLS_CENTER_EMBEDDED } from '@/utils/navigation'
import { useRouter } from 'vue-router'
import { useArchiveRecoveryStore } from '@/composables/useArchiveRecoveryBatch'
import ArchiveRecoveryBatch from '@/components/compression/ArchiveRecoveryBatch.vue'
import RarRecoveryVolumesDialog from '@/components/compression/RarRecoveryVolumesDialog.vue'
import type { ArchiveRecoveryRow } from '@/composables/useArchiveRecoveryBatch'
import { useAppStore } from '@/stores/app'
import { useUIStore } from '@/stores/ui'
import { useTauriCommands, type ArchiveDiagnosticReport, type ZipRepairResult } from '@/composables/useTauriCommands'
import { open, save } from '@tauri-apps/api/dialog'
import { useTaskStore } from '@/stores/task'
import { generateId } from '@/utils'
import { archiveInspectionMessageForDisplay } from '@/utils/taskFailure'
import { parentDirectory } from '@/utils/resultLocation'
import { uniqueContextPaths } from '@/utils/contextActions'
import { checksumBaseDirectory } from '@/utils/checksumSources'
import ZipPasswordEncodingField from '@/components/ui/ZipPasswordEncodingField.vue'
import { zipPasswordEncodingArguments, zipPasswordEncodingLabel, type ZipPasswordEncoding } from '@/types/zipPassword'
import { supportsZipPasswordEncoding, zipPasswordEncodingForSource } from '@/utils/zipPasswordSource'

const appStore = useAppStore()
const embedded = inject(TOOLS_CENTER_EMBEDDED, false)
const router = useRouter()
const uiStore = useUIStore()
const tauriCommands = useTauriCommands()
const taskStore = useTaskStore()
const recoveryTaskId = ref('')
const recoveredDraftId = ref('')
const recoveredPassword = ref('')
const recoveredPasswordRequired = ref(false)
const recoveredZipPasswordEncoding = ref<ZipPasswordEncoding>()
const archiveZipPasswordEncoding = ref<ZipPasswordEncoding>()
const sourceFormat = ref<string>()
const archiveSnapshot = ref<ZpaqSnapshot>()
const sourceEncoding = computed(() => sourceFormat.value
  ? zipPasswordEncodingForSource(archivePath.value, archiveZipPasswordEncoding.value, sourceFormat.value) : archiveZipPasswordEncoding.value)
const recoveryOptions = ref<{ outputPath?: string; extractToSubfolder?: boolean }>({})

const { progress: checksumProgress, stopping: stoppingChecksum, run: runChecksum, stop: stopChecksum } = useChecksumOperation()
const selectedFiles = ref<string[]>([])
const selectedAlgorithm = ref<'crc32' | 'md5' | 'sha256'>('sha256')
const checksumResults = ref<ChecksumCalculationEntry[]>([])
const isCalculating = ref(false)
const isExporting = ref(false), exportError = ref(''), exportedManifest = ref(''), exportedCount = ref(0)
const sourceLabel = ref(''), expandedCalculation = ref<string[]>([]), showSources = ref(false)
const verificationPanel = ref<InstanceType<typeof ChecksumVerificationPanel> | null>(null)
function resetExport() { exportError.value = ''; exportedManifest.value = ''; exportedCount.value = 0 }
const calculationUnfinished = computed(() => checksumResults.value.filter(row => ['pending', 'interrupted'].includes(row.status)))
const successfulChecksums = computed(() => checksumResults.value.filter(row => row.status === 'success'))
watch(selectedAlgorithm, () => { checksumResults.value = []; resetExport() })
const recoveryWorkspace = useArchiveRecoveryStore()
const activeMode = ref<'calculate' | 'verify' | 'archive'>(recoveryWorkspace.rows.length ? 'archive' : 'calculate')
const isVerifying = ref(false)
const archivePath = ref(recoveryWorkspace.sourcePaths[0] || '')
const nativeZpaqSource=ref<ZpaqGroup|null>(null)
const archiveDisplayPath=computed(()=>nativeZpaqSource.value?.members[0]?.path||archivePath.value)
let sourceDisplaySequence=0
watch(archivePath,async path=>{const sequence=++sourceDisplaySequence;nativeZpaqSource.value=null;if(!/\.zpaq$/i.test(path))return;try{const value=await tauriCommands.invoke<ZpaqGroup|null>('zpaq_multipart_details',{path});if(sequence===sourceDisplaySequence)nativeZpaqSource.value=value}catch{/* Keep the actual diagnosis responsible for source errors. */}},{immediate:true})
const diagnosticSources = ref<string[]>([...recoveryWorkspace.sourcePaths])
const batchBusy = computed(() => recoveryWorkspace.busy || recoveryWorkspace.preparing)
const batchDetailOpen = ref(false), recoveryVolumesVisible = ref(false)
const recoveryVolumesSource = ref(''), recoveryVolumesPassword = ref('')
function showRecoveryVolumes(path: string, password: string) { recoveryVolumesSource.value = path; recoveryVolumesPassword.value = password; recoveryVolumesVisible.value = true }
watch(recoveryVolumesVisible, visible => { if (!visible) recoveryVolumesPassword.value = '' })
async function inspectBatchItem(row: ArchiveRecoveryRow) {
  if (row.needsRecheck) return
  archivePath.value = row.path; archivePassword.value = row.password; repairResult.value = null
  archiveZipPasswordEncoding.value = row.zipPasswordEncoding; sourceFormat.value = row.report?.actualFormat
  recoveryTaskId.value = ''; recoveryOptions.value = {}; recoveredDraftId.value = ''; recoveredPassword.value = ''
  batchDetailOpen.value = true
  await nextTick()
  diagnosticReport.value = row.report || null
}
const isRar = computed(() => diagnosticReport.value?.actualFormat === 'RAR')
const archivePassword = ref('')
const diagnosticReport = ref<ArchiveDiagnosticReport | null>(null)
const repairResult = ref<ZipRepairResult | null>(null)
const diagnosticId = ref('')
const isDiagnosing = ref(false)
const isRepairing = ref(false)
const repairId = ref('')
const isCancellingRepair = ref(false)

watch([archivePassword, archiveZipPasswordEncoding], () => { diagnosticReport.value = null })

const algorithms = [
  { value: 'crc32', label: 'CRC32', description: '快速校验' },
  { value: 'md5', label: 'MD5', description: '中等强度' },
  { value: 'sha256', label: 'SHA256', description: '高强度（推荐）' },
]

const selectFiles = async () => {
  if (isCalculating.value || isExporting.value) return
  try {
    const queued = import.meta.env.VITE_DESKTOP_E2E === '1' ? window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection() : undefined
    const files = queued !== undefined ? queued : await open({
      multiple: true,
      title: appStore.t('integrity.select_files', '选择文件')
    })
    if (files) {
      selectedFiles.value = Array.isArray(files) ? files : [files]
      checksumResults.value = []
      sourceLabel.value = ''; expandedCalculation.value = []; resetExport()
    }
  } catch (error) {
    appStore.setError(appStore.t('common.error'))
  }
}

let calculationActive = true
const calculateChecksums = async (resume = false) => {
  if (isCalculating.value || isExporting.value || !selectedFiles.value.length) return
  const paths = resume ? calculationUnfinished.value.map(row => row.path) : [...selectedFiles.value]
  if (!paths.length) return
  isCalculating.value = true
  resetExport()
  if (!resume) checksumResults.value = paths.map(path => ({ path, fileName: path.split(/[\\/]/).pop() || path, algorithm: selectedAlgorithm.value.toUpperCase(), checksum: '', status: 'pending' }))
  const accept = (rows: ChecksumCalculationEntry[]) => {
    if (!calculationActive) return
    const updates = new Map(rows.map(row => [row.path, row]))
    checksumResults.value = checksumResults.value.map(row => updates.get(row.path) || row)
  }
  try {
    accept(await runChecksum<ChecksumCalculationEntry[]>('calculate_checksum_batch', { paths, algorithm: selectedAlgorithm.value }, event => {
      if (event.calculationResults) accept(event.calculationResults)
    }))
    if (calculationActive) {
      const failed = checksumResults.value.filter(row => row.status === 'error').length
      if (failed) appStore.setError(`${failed} 个文件未能计算，请查看逐项原因；已完成的摘要已保留。`)
      else appStore.setSuccess(calculationUnfinished.value.length ? '已停止，已完成的摘要已保留，可继续未完成项' : '本次文件已处理，请查看逐项结果')
    }
  } catch (error) { if (calculationActive) appStore.setError(`本次计算未完成：${String(error)}`) }
  finally { if (calculationActive) isCalculating.value = false }
}

const copyChecksum = async (checksum: string) => {
  try {
    await navigator.clipboard.writeText(checksum)
    appStore.setSuccess(appStore.t('integrity.copied', '已复制到剪贴板'))
  } catch {
    appStore.setError(appStore.t('integrity.copy_failed', '复制失败'))
  }
}

const exportChecksumFile = async () => {
  if (isCalculating.value || isExporting.value) return
  if (!successfulChecksums.value.length) {
    appStore.setError(appStore.t('integrity.no_results', '没有可导出的结果'))
    return
  }

  isExporting.value = true; exportError.value = ''
  try {
    const rows = successfulChecksums.value.map(row => ({ path: row.path, checksum: row.checksum, status: row.status }))
    const dir = checksumBaseDirectory(selectedFiles.value)
    const ext = selectedAlgorithm.value === 'crc32' ? '.sfv' : `.${selectedAlgorithm.value}`

    const queued = import.meta.env.VITE_DESKTOP_E2E === '1' ? window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection() : undefined
    const savePath = queued !== undefined ? queued : await save({
      title: appStore.t('integrity.export', '导出校验文件'),
      defaultPath: `${dir ? dir.replace(/[\\/]$/, '') + '/' : ''}checksums${ext}`,
      filters: [{
        name: `${selectedAlgorithm.value.toUpperCase()} Checksum File`,
        extensions: [ext.slice(1)]
      }]
    })

    if (typeof savePath === 'string' && savePath && calculationActive) {
      await tauriCommands.invoke('export_checksum_file', {
        path: savePath,
        results: rows,
        algorithm: selectedAlgorithm.value
      })
      if (!calculationActive) return
      exportedManifest.value = savePath; exportedCount.value = rows.length
      appStore.setSuccess(appStore.t('integrity.exported', '校验文件已导出'))
    }
  } catch (error) {
    if (calculationActive) exportError.value = `导出未完成：${String(error)}`
  } finally { if (calculationActive) isExporting.value = false }
}

async function verifyExportedManifest() {
  if (!exportedManifest.value || isCalculating.value || isVerifying.value || isExporting.value) return
  activeMode.value = 'verify'
  await nextTick()
  await verificationPanel.value?.verify(exportedManifest.value)
}

function acceptChecksumSources() {
  const request = appStore.pendingChecksumSources
  if (!request || isCalculating.value || isExporting.value || isVerifying.value || isDiagnosing.value || isRepairing.value || batchBusy.value) return
  selectedFiles.value = [...request.paths]; sourceLabel.value = request.label
  checksumResults.value = []; expandedCalculation.value = []; showSources.value = false; resetExport()
  activeMode.value = 'calculate'; appStore.pendingChecksumSources = null
}

const integrityBusy = computed(() => isCalculating.value || isExporting.value || isVerifying.value || isDiagnosing.value || isRepairing.value || batchBusy.value)
const explorerRequest = computed(() => appStore.pendingExplorerChecksums[0])
async function acceptExplorerRequest(append = false) {
  if (integrityBusy.value || !explorerRequest.value) return
  const request = appStore.pendingExplorerChecksums.shift()!
  if (request.kind === 'verify') {
    activeMode.value = 'verify'
    await nextTick()
    verificationPanel.value?.stage(request.paths)
  } else {
    selectedFiles.value = uniqueContextPaths(append ? [...selectedFiles.value, ...request.paths] : request.paths)
    sourceLabel.value = '资源管理器所选文件'; checksumResults.value = []; expandedCalculation.value = []; showSources.value = false; resetExport()
    activeMode.value = 'calculate'
  }
}
watch(() => appStore.pendingExplorerChecksums.length, () => {
  if (explorerRequest.value && !integrityBusy.value && !selectedFiles.value.length && !appStore.pendingChecksumSources && activeMode.value === 'calculate' && !verificationPanel.value?.hasWorkspace()) void acceptExplorerRequest()
}, { immediate: true })

const clearResults = () => {
  archivePath.value = ''
  diagnosticSources.value = []
  batchDetailOpen.value = false
  archivePassword.value = ''
  archiveZipPasswordEncoding.value = undefined
  archiveSnapshot.value = undefined
  sourceFormat.value = undefined
  diagnosticReport.value = null
  repairResult.value = null
  recoveryTaskId.value = ''
  recoveryOptions.value = {}
  recoveredDraftId.value = ''
  recoveredPassword.value = ''
  recoveredZipPasswordEncoding.value = undefined
  recoveredPasswordRequired.value = false
}

function resumeRecoveryWorkspace() {
  recoveryWorkspace.restore()
  activeMode.value = 'archive'
  diagnosticSources.value = [...recoveryWorkspace.sourcePaths]
  archivePath.value = diagnosticSources.value[0] || ''
  batchDetailOpen.value = false
  restoreSingleResult()
}
function openArchiveMode() {
  archiveSnapshot.value = undefined
  activeMode.value = 'archive'
  diagnosticSources.value = [...recoveryWorkspace.sourcePaths]
  archivePath.value = diagnosticSources.value[0] || ''
}

const selectArchive = async () => {
  const queued = import.meta.env.VITE_DESKTOP_E2E === '1'
    ? window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection() : undefined
  const file = queued === undefined ? await open({ multiple: true, title: '选择要诊断的压缩包，可多选' }) : queued
  let paths = Array.isArray(file) ? file : typeof file === 'string' ? [file] : []
  try {paths=[...new Set(await Promise.all(paths.map(prepareZpaqSource)))]}
  catch(error){if(!String(error).includes('已取消'))appStore.setError(String(error));return}
  if (paths.length) archiveSnapshot.value = undefined
  if (paths.length) {
    if (paths.length === 1) recoveryWorkspace.clear()
    archivePath.value = paths[0]
    archivePassword.value = ''
    archiveZipPasswordEncoding.value = undefined
    sourceFormat.value = undefined
    diagnosticSources.value = paths
    batchDetailOpen.value = false
    diagnosticReport.value = null
    repairResult.value = null
    recoveryTaskId.value = ''
    recoveryOptions.value = {}
    recoveredDraftId.value = ''
    recoveredPassword.value = ''
  }
}

const diagnoseArchive = async () => {
  if (!archivePath.value || isDiagnosing.value || isRepairing.value) return
  isDiagnosing.value = true
  diagnosticReport.value = null
  repairResult.value = null
  diagnosticId.value = `diagnostic-${Date.now()}-${Math.random().toString(36).slice(2)}`
  try {
    const prepared=await prepareZpaqSource(archivePath.value)
    if(prepared!==archivePath.value){archivePath.value=prepared;diagnosticSources.value=[prepared]}
    diagnosticReport.value = archiveSnapshot.value
      ? await tauriCommands.diagnoseArchive(diagnosticId.value, archivePath.value, archivePassword.value, sourceEncoding.value, archiveSnapshot.value)
      : await tauriCommands.diagnoseArchive(diagnosticId.value, archivePath.value, archivePassword.value, ...zipPasswordEncodingArguments(sourceEncoding.value))
    sourceFormat.value = diagnosticReport.value.actualFormat
  } catch (error) {
    if (!/cancel|已取消/i.test(String(error))) {
      appStore.setError(`归档诊断失败：${String(error)}`)
    }
  } finally {
    isDiagnosing.value = false
    diagnosticId.value = ''
  }
}

const cancelDiagnosis = async () => {
  if (!diagnosticId.value) return
  try { await tauriCommands.cancelArchiveDiagnosis(diagnosticId.value) } catch { /* command may have just completed */ }
}

const defaultRepairPath = () => archivePath.value.replace(/(?:\.zx?\d+|\.\d{3,})$/i, '').replace(/\.zipx?$/i, '') + '.repaired.zip'

const repairArchive = async () => {
  if (!diagnosticReport.value?.canRepair || isRepairing.value) return
  const report = diagnosticReport.value
  const source = archivePath.value, secret = archivePassword.value, encoding = sourceEncoding.value
  const queued = import.meta.env.VITE_DESKTOP_E2E === '1'
    ? window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection() : undefined
  const outputPath = queued === undefined ? isRar.value ? await open({ directory: true, multiple: false, title: '选择修复结果保存目录' }) : await save({
    title: '保存修复后的 ZIP',
    defaultPath: defaultRepairPath(),
    filters: [{ name: 'ZIP Archive', extensions: ['zip'] }],
  }) : queued
  if (typeof outputPath !== 'string' || !outputPath) return
  if (diagnosticReport.value !== report || archivePath.value !== source || archivePassword.value !== secret || sourceEncoding.value !== encoding) return
  isCancellingRepair.value = false
  isRepairing.value = true
  repairId.value = `repair-${Date.now()}-${Math.random().toString(36).slice(2)}`
  try {
    repairResult.value = null
    repairResult.value = isRar.value
      ? await tauriCommands.repairRar(repairId.value, archivePath.value, outputPath, archivePassword.value)
      : await tauriCommands.repairZip(repairId.value, source, outputPath, secret, ...zipPasswordEncodingArguments(encoding))
    recoveredPassword.value = repairResult.value.outputEncrypted ? archivePassword.value : ''
    recoveredPasswordRequired.value = false
    recoveredZipPasswordEncoding.value = repairResult.value.outputEncrypted && repairResult.value.format !== 'RAR' ? encoding ?? 'utf8' : undefined
    recoveredDraftId.value = ''
    rememberSingleResult()
    if (repairResult.value.partial || repairResult.value.skippedEntries.length) {
      uiStore.showToast('warning', '部分恢复：已恢复可读条目，请核对内容是否齐全', 6000)
    } else { appStore.setSuccess('修复结果已另存并通过完整性校验') }
  } catch (error) {
    if (!String(error).toLocaleLowerCase().includes('cancel')) {
      appStore.setError(`修复未完成：${String(error)}`)
    }
  } finally {
    isRepairing.value = false
    repairId.value = ''
    isCancellingRepair.value = false
  }
}

const cancelRepair = async () => {
  if (!repairId.value || isCancellingRepair.value) return
  isCancellingRepair.value = true
  try { await tauriCommands.cancelZipRepair(repairId.value) } catch { /* repair may have just completed */ }
}

const locateRepair = async () => {
  if (!repairResult.value) return
  try { await tauriCommands.invoke('open_in_explorer', { path: repairResult.value.outputPath }) }
  catch { appStore.setError('无法定位恢复文件，请确认文件仍在原位置') }
}

const browseRepair = () => {
  if (!repairResult.value) return
  appStore.openArchiveInBrowser(repairResult.value.outputPath, {
    password: recoveredPassword.value, zipPasswordEncoding: recoveredZipPasswordEncoding.value,
  })
}

const statusLabel = (status: string) => ({
  healthy: diagnosticReport.value?.zpaqBlocks?`快照 ${diagnosticReport.value.zpaqBlocks.snapshot.version} 检查通过`:'完整性正常', password_required: '需要密码', wrong_password: '密码错误',
  zpaq_affected:'发现受损数据块', interrupted_tail:'尾部更新中断',
  password_encoding: diagnosticReport.value?.actualFormat === 'ZIP' ? 'ZIP 密码编码不兼容' : '原包编码不兼容',
  directory_lost: '目录信息丢失，可尝试救援',
  partial_recovery: '缺少分卷，可尝试部分救援',
  missing_volume: '缺少分卷', crc_error: '内容校验失败', truncated: '归档被截断',
  damaged: '归档损坏', checking: '检查中',
  ambiguous_data: '密码或加密内容异常', unsupported_feature: '暂不支持此归档特性',
  permission_error: '无法读取归档', unknown_failure: '暂不能确定失败原因',
  source_unavailable: '来源文件不可用', io_error: '读取归档失败',
  structure_only: '仅完成结构诊断', verification_unavailable: '无法完成内容校验',
}[status] || status)

const formatBytes = (value: number) => {
  if (value < 1024) return `${value} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let size = value / 1024
  let index = 0
  while (size >= 1024 && index < units.length - 1) { size /= 1024; index++ }
  return `${size.toFixed(size >= 100 ? 0 : size >= 10 ? 1 : 2)} ${units[index]}`
}

const encryptionLabel = (report: ArchiveDiagnosticReport) => report.encrypted ? '是'
  : report.status === 'missing_volume' || (!report.integrityTested && !report.totalFiles && !report.canRepair) ? '未检查' : '否'

const reportText = () => {
  const report = diagnosticReport.value
  if (!report) return ''
  return [
    'Long解压 · 归档诊断报告',
    `文件：${archiveDisplayPath.value||report.filePath}`,
    `格式：${report.actualFormat}`,
    `状态：${statusLabel(report.status)}`,
    `大小：${report.fileSize} bytes`,
    `加密：${encryptionLabel(report)}`,
    `分卷：${report.splitArchive ? `是（发现 ${report.volumesFound} 卷）` : '否'}`,
    `条目：${report.totalFiles} 个文件 / ${report.totalDirectories} 个目录`,
    ...report.issues.map(issue => `[${issue.severity}] ${issue.title}：${issue.detail}`),
    ...(report.zpaqBlocks?zpaqReportLines(report.zpaqBlocks):[]),
    ...report.evidence.map(item => `证据：${item}`),
  ].join('\n')
}

const copyDiagnosticReport = async () => {
  try {
    await navigator.clipboard.writeText(reportText())
    appStore.setSuccess('诊断报告已复制')
  } catch {
    appStore.setError('复制诊断报告失败')
  }
}

watch(() => isDiagnosing.value || isRepairing.value || isCalculating.value || isExporting.value || isVerifying.value || batchBusy.value ? null : appStore.pendingArchiveDiagnosis, request => {
  if (!request || isDiagnosing.value || isRepairing.value || batchBusy.value) return
  appStore.pendingArchiveDiagnosis = null
  clearResults()
  activeMode.value = 'archive'
  archivePath.value = request.path
  diagnosticSources.value = request.paths || [request.path]
  if (diagnosticSources.value.length < 2) recoveryWorkspace.clear()
  archivePassword.value = request.password || ''
  archiveZipPasswordEncoding.value = request.zipPasswordEncoding
  archiveSnapshot.value = request.zpaqSnapshot
  recoveryTaskId.value = request.taskId || ''
  recoveryOptions.value = { outputPath: request.outputPath, extractToSubfolder: request.extractToSubfolder }
  if (diagnosticSources.value.length < 2) void diagnoseArchive()
}, { immediate: true })

watch(() => appStore.pendingChecksumSources, request => {
  // Never replace an existing workspace silently, even when its current pass ends.
  if (request && !selectedFiles.value.length && !isCalculating.value && !isVerifying.value && !isDiagnosing.value && !isRepairing.value && !batchBusy.value) acceptChecksumSources()
}, { immediate: true })

const queueRecoveredArchive = async () => {
  const result = repairResult.value
  if (!result || recoveredDraftId.value) return
  if (recoveredPasswordRequired.value && !recoveredPassword.value) {
    appStore.setError('请填写恢复结果密码后再加入解压队列')
    return
  }
  try {
    for (const path of result.outputFiles?.length ? result.outputFiles : [result.outputPath]) await tauriCommands.invoke('get_file_info', {path})
  } catch {
    appStore.setError('恢复结果已移动、删除或无法读取，请定位结果或重新检查并修复原包')
    return
  }
  const original = taskStore.tasks.find(task => task.id === recoveryTaskId.value)
  if (original && !await (original.historySaveError ? taskStore.retryHistoryPersistence(original.id) : taskStore.waitForHistoryPersistence(original.id))) {
    appStore.setError('原失败记录尚未保存，请先重试保存历史后再添加。')
    return
  }
  if (recoveredDraftId.value || result !== repairResult.value) return
  const id = generateId()
  taskStore.addTask({ id, name: result.outputPath.split(/[\\/]/).pop() || result.outputPath,
    type: 'decompression', workloadKind: 'archive', sourceFiles: [result.outputPath],
    outputPath: recoveryOptions.value.outputPath || original?.outputPath || parentDirectory(result.outputPath),
    extractToSubfolder: recoveryOptions.value.extractToSubfolder ?? original?.extractToSubfolder ?? true, configurationMode: 'individual',
    recycleSourceAfterExtract: false, password: recoveredPassword.value || undefined,
    zipPasswordEncoding: recoveredZipPasswordEncoding.value,
    retryOfTaskId: original?.id || recoveryTaskId.value || undefined })
  recoveredDraftId.value = id
  if (original) original.retryDraftId = id
  rememberSingleResult()
  appStore.setSuccess('已加入解压队列，原输出设置已保留；请核对后开始解压。')
}

function rememberSingleResult() {
  if (!repairResult.value || diagnosticSources.value.length !== 1) return
  const original = taskStore.tasks.find(task => task.id === recoveryTaskId.value)
  recoveryWorkspace.rememberSingle({path:archivePath.value, password:archivePassword.value,
    zipPasswordEncoding:archiveZipPasswordEncoding.value, sourceFormat:sourceFormat.value,
    report:diagnosticReport.value || undefined, result:repairResult.value,
    resultPassword:recoveredPassword.value, resultPasswordRequired:recoveredPasswordRequired.value,
    resultZipPasswordEncoding:recoveredZipPasswordEncoding.value, queuedId:recoveredDraftId.value || undefined,
    retryOfTaskId:recoveryTaskId.value || undefined,
    extractDirectory:recoveryOptions.value.outputPath || original?.outputPath || '',
    extractToSubfolder:recoveryOptions.value.extractToSubfolder ?? original?.extractToSubfolder ?? true})
}
function restoreSingleResult() {
  if (diagnosticSources.value.length !== 1 || recoveryWorkspace.rows.length !== 1) return
  const row = recoveryWorkspace.rows[0]!
  if (!row.result) return
  archiveSnapshot.value = undefined
  archivePath.value = row.path; archivePassword.value = row.password
  archiveZipPasswordEncoding.value = row.zipPasswordEncoding; sourceFormat.value = row.sourceFormat
  diagnosticReport.value = null; repairResult.value = row.result
  recoveredPassword.value = row.resultPassword || ''; recoveredPasswordRequired.value = !!row.resultPasswordRequired
  recoveredZipPasswordEncoding.value = row.resultZipPasswordEncoding
  recoveredDraftId.value = row.queuedId || ''; recoveryTaskId.value = row.retryOfTaskId || ''
  recoveryOptions.value = {outputPath:row.extractDirectory,extractToSubfolder:row.extractToSubfolder}
}
restoreSingleResult()
watch([archivePassword, archiveZipPasswordEncoding, recoveredPassword], rememberSingleResult, {flush:'post'})

onUnmounted(() => {
  calculationActive = false
  void cancelDiagnosis()
  void cancelRepair()
  archivePassword.value = ''
  recoveredPassword.value = ''
})
</script>

<template>
  <div class="integrity-view flex h-full flex-col overflow-x-hidden transition-colors duration-700">
    <!-- 顶部标题栏 -->
    <header class="integrity-header shrink-0">
      <div>
        <div class="flex items-center gap-2"><h1 v-if="!embedded">{{ appStore.t('integrity.title') }}</h1><HelpHint label="完整性工具说明">计算文件摘要、核对校验清单，或检查与修复压缩包。</HelpHint></div>
      </div>
            <nav class="integrity-mode-switch" aria-label="文件完整性工具">
          <button
            @click="activeMode = 'calculate'"
            :disabled="isDiagnosing || isRepairing || isCalculating || isExporting || isVerifying"
            :class="{ active: activeMode === 'calculate' }"
          >
            <span class="mode-icon"><i class="pi pi-hashtag"></i></span>
            <span><strong>{{ appStore.t('integrity.mode.calculate') }}</strong></span>
            <i class="pi pi-chevron-right mode-arrow"></i>
          </button>
          <button
            data-testid="checksum-verify-mode"
            @click="activeMode = 'verify'"
            :disabled="isDiagnosing || isRepairing || isCalculating || isExporting || isVerifying"
            :class="{ active: activeMode === 'verify' }"
          >
            <span class="mode-icon"><i class="pi pi-check-circle"></i></span>
            <span><strong>{{ appStore.t('integrity.mode.verify') }}</strong></span>
            <i class="pi pi-chevron-right mode-arrow"></i>
          </button>
          <button
            data-testid="archive-diagnostic-mode"
            @click="openArchiveMode"
            :disabled="isDiagnosing || isRepairing || isCalculating || isExporting || isVerifying"
            :class="{ active: activeMode === 'archive' }"
          >
            <span class="mode-icon"><i class="pi pi-search"></i></span>
            <span><strong>归档诊断</strong></span>
            <i class="pi pi-chevron-right mode-arrow"></i>
          </button>
        </nav></header>

    <!-- 主内容区 -->
    <div class="integrity-scroll custom-scrollbar min-h-0 flex-1 overflow-y-auto overflow-x-hidden">
      <div class="integrity-workspace">
        <div v-if="explorerRequest" class="checksum-handoff" data-testid="checksum-explorer-pending">
          <span>资源管理器送来 {{ explorerRequest.paths.length }} {{ explorerRequest.kind === 'verify' ? '份清单；当前结果会保留，使用后可逐份核对。' : '个文件；使用后替换计算列表，也可合并后重新计算。' }}</span>
          <button class="integrity-secondary" :disabled="integrityBusy" data-testid="checksum-explorer-use" @click="acceptExplorerRequest()">{{ explorerRequest.kind === 'verify' ? '使用这些清单' : '使用这组文件' }}</button>
          <button v-if="explorerRequest.kind === 'calculate' && selectedFiles.length" class="integrity-secondary" :disabled="integrityBusy" data-testid="checksum-explorer-append" @click="acceptExplorerRequest(true)">合并到当前列表</button>
          <button class="integrity-secondary" @click="appStore.pendingExplorerChecksums.shift()">忽略这次请求</button>
        </div>
        <div v-if="appStore.pendingChecksumSources" class="checksum-handoff" data-testid="checksum-pending-sources">
          <span>已收到 {{ appStore.pendingChecksumSources.paths.length }} 个结果文件。使用后会替换当前计算列表。</span>
          <button class="integrity-secondary" :disabled="isCalculating || isExporting || isVerifying || isDiagnosing || isRepairing || batchBusy" @click="acceptChecksumSources">使用这组文件</button>
          <button class="integrity-secondary" @click="appStore.pendingChecksumSources = null">保留当前列表</button>
        </div>
        <div v-if="recoveryWorkspace.recoveryCount || recoveryWorkspace.saveError" class="flex min-w-0 flex-wrap items-center gap-2 rounded-xl border border-primary/20 bg-primary/5 p-3 text-xs" data-testid="archive-recovery-resume">
          <span class="min-w-0 flex-1 text-content">{{ recoveryWorkspace.saveError || `上次还有 ${recoveryWorkspace.recoveryCount} 组检查修复记录` }}</span>
          <button v-if="recoveryWorkspace.recoveryCount" class="archive-secondary" :disabled="recoveryWorkspace.busy || recoveryWorkspace.preparing || isDiagnosing || isRepairing" data-testid="archive-recovery-restore" @click="resumeRecoveryWorkspace">恢复工作区</button>
          <button class="archive-secondary" :disabled="recoveryWorkspace.busy || recoveryWorkspace.preparing" data-testid="archive-recovery-discard" @click="recoveryWorkspace.discardPrevious">丢弃上次记录</button>
        </div>
        <!-- 模式切换 -->


        <!-- 计算模式 -->
        <div v-if="activeMode === 'calculate'" class="integrity-calculate-grid">
          <!-- 选择算法 -->
          <section class="integrity-panel algorithm-panel">
            <header class="panel-heading"><span><i class="pi pi-sliders-h"></i>{{ appStore.t('integrity.algorithm') }}</span><HelpHint label="摘要算法说明">MD5、SHA-1 用于兼容旧清单；新清单建议使用 SHA-256。</HelpHint></header>
            <div class="algorithm-options">
              <button
                v-for="algo in algorithms"
                :key="algo.value"
                @click="selectedAlgorithm = algo.value as any"
                :disabled="isCalculating || isExporting"
                :class="{ active: selectedAlgorithm === algo.value }"
              >
                <span><strong>{{ algo.label }}</strong></span>
                <i v-if="selectedAlgorithm === algo.value" class="pi pi-check"></i>
              </button>
            </div>
          </section>

          <!-- 选择文件 -->
          <section class="integrity-panel file-panel" :class="{'has-results':checksumResults.length>0}">
            <header v-if="!checksumResults.length" class="panel-heading"><span><i class="pi pi-file"></i>{{ appStore.t('integrity.files') }}</span><small>{{ selectedFiles.length ? `${selectedFiles.length} 个文件待处理` : '支持批量选择' }}</small></header>
            <button
              @click="selectFiles"
              :disabled="isCalculating || isExporting"
              data-testid="checksum-calculate-choose"
              class="integrity-dropzone"
            >
              <span class="dropzone-icon"><i class="pi pi-folder-open"></i></span>
              <span>
                <strong>
                {{ selectedFiles.length > 0
                  ? `已选择 ${selectedFiles.length} 个文件`
                  : appStore.t('integrity.select_files') }}
                </strong>

              </span>
              <i class="pi pi-plus"></i>
            </button>

            <div v-if="selectedFiles.length" class="checksum-source-summary" data-testid="checksum-source-summary">
              <p v-if="sourceLabel" class="break-words">来源：{{ sourceLabel }} · 已带入整组 {{ selectedFiles.length }} 个文件</p>
              <HelpHint label="计算与导出说明">点击开始计算后读取文件，导出仅使用已完成摘要。清单保存到文件的共同上级目录，分享时保留目录结构。</HelpHint>
              <button class="checksum-source-toggle" :aria-expanded="showSources" data-testid="checksum-source-toggle" @click="showSources = !showSources">{{ showSources ? '收起' : '查看' }}文件路径 <i :class="showSources ? 'pi pi-chevron-up' : 'pi pi-chevron-down'"></i></button>
              <ul v-if="showSources" class="checksum-source-paths custom-scrollbar"><li v-for="path in selectedFiles" :key="path">{{ path }}</li></ul>
            </div>
            <ChecksumProgressCard v-if="isCalculating" :progress="checksumProgress" :stopping="stoppingChecksum" @stop="stopChecksum" />
            <!-- 操作按钮 -->
            <div class="integrity-actions">
              <button v-if="calculationUnfinished.length" :disabled="isCalculating || isExporting" class="integrity-secondary" data-testid="checksum-calculate-continue" @click="calculateChecksums(true)">继续未完成项（{{ calculationUnfinished.length }}）</button>
              <button
                @click="calculateChecksums(false)"
                data-testid="checksum-calculate-start"
                :disabled="selectedFiles.length === 0 || isCalculating || isExporting"
                class="integrity-primary"
              >
                <i :class="isCalculating ? 'pi pi-spinner pi-spin' : 'pi pi-play'"></i>
                {{ isCalculating ? appStore.t('integrity.calculating') : appStore.t('integrity.calculate') }}
              </button>
              <button
                v-if="successfulChecksums.length"
                :disabled="isCalculating || isExporting"
                data-testid="checksum-export"
                @click="exportChecksumFile"
                class="integrity-secondary"
              >
                <i class="pi pi-download"></i>
                {{ isExporting ? '正在导出…' : `导出已完成项（${successfulChecksums.length}）` }}
              </button>
            </div>
            <p v-if="exportError" role="alert" class="checksum-export-error" data-testid="checksum-export-error">{{ exportError }}</p>
            <div v-if="exportedManifest" class="checksum-export-receipt" data-testid="checksum-export-receipt">
              <strong><i class="pi pi-check-circle"></i> 已保存 {{ exportedCount }} 项{{ exportedCount < selectedFiles.length ? `，其余 ${selectedFiles.length - exportedCount} 项未写入` : '' }}</strong>
              <code>{{ exportedManifest }}</code>

              <button class="integrity-secondary" data-testid="checksum-verify-exported" @click="verifyExportedManifest">核对这份清单</button>
            </div>
          </section>

          <!-- 结果列表 -->
          <section v-if="checksumResults.length > 0" class="integrity-panel result-panel">
            <header class="panel-heading"><span><i class="pi pi-list"></i>{{ appStore.t('integrity.results') }}</span><small>{{ checksumResults.filter(item => item.status === 'success').length }} / {{ checksumResults.length }} 完成</small></header>
            <div class="checksum-results custom-scrollbar">
              <div
                v-for="result in checksumResults"
                :key="result.path"
                :data-status="result.status" data-testid="checksum-calculation-entry"
                class="checksum-row"
              >
                <span class="checksum-status" :data-status="result.status"><i :class="result.status === 'success' ? 'pi pi-check' : result.status === 'error' ? 'pi pi-times' : 'pi pi-pause'"></i></span>
                <div class="checksum-main"><button class="checksum-calculation-toggle" :aria-expanded="expandedCalculation.includes(result.path)" data-testid="checksum-calculation-toggle" @click="expandedCalculation = expandedCalculation.includes(result.path) ? expandedCalculation.filter(path => path !== result.path) : [...expandedCalculation, result.path]"><strong :title="result.path">{{ result.fileName }}</strong><i :class="expandedCalculation.includes(result.path) ? 'pi pi-chevron-up' : 'pi pi-chevron-down'"></i></button><small :title="result.path">{{ result.path }}</small><code :title="result.checksum">{{ result.error || result.checksum || '尚未读取' }}</code></div>
                <span class="checksum-algorithm">{{ result.algorithm }}</span>
                    <button
                      v-if="result.status === 'success'"
                      @click="copyChecksum(result.checksum)"
                  class="checksum-copy"
                  :title="appStore.t('integrity.copy')"
                    >
                  <i class="pi pi-copy"></i><span class="sr-only">{{ appStore.t('integrity.copy') }}</span>
                    </button>
                <div v-if="expandedCalculation.includes(result.path)" class="checksum-calculation-details" data-testid="checksum-calculation-details"><span>来源路径</span><code>{{ result.path }}</code><span>{{ result.algorithm }} · {{ result.status === 'success' ? '已完成' : result.status === 'error' ? '读取失败' : result.status === 'interrupted' ? '已停止，可继续' : '尚未读取' }}</span><code>{{ result.error || result.checksum || '没有完整摘要，不会写入清单' }}</code></div>
              </div>
            </div>
          </section>
        </div>

        <div v-show="activeMode === 'verify'" class="integrity-single-column">
          <ChecksumVerificationPanel ref="verificationPanel" @busy="isVerifying = $event" @diagnose="appStore.openArchiveForDiagnosis({path: $event})" @tasks="router.push('/decompress')" />
        </div>

        <div v-if="activeMode === 'archive'" class="integrity-single-column min-w-0 overflow-x-hidden" data-testid="archive-diagnostic-panel">
          <div v-if="diagnosticSources.length > 1" class="flex flex-wrap items-center justify-between gap-2">
            <button v-if="batchDetailOpen" class="archive-secondary" @click="batchDetailOpen = false"><i class="pi pi-arrow-left"></i>返回批量列表</button>
            <button class="archive-secondary" :disabled="batchBusy || isDiagnosing || isRepairing" @click="selectArchive"><i class="pi pi-folder-open"></i>重新选择压缩包</button>
            <button class="archive-secondary" :disabled="batchBusy" data-testid="archive-recovery-clear" @click="recoveryWorkspace.clear(); clearResults()">清空本批记录</button>
          </div>
          <div v-if="diagnosticSources.length > 1" v-show="!batchDetailOpen"><ArchiveRecoveryBatch :paths="diagnosticSources" @inspect="inspectBatchItem" @recovery-volumes="showRecoveryVolumes($event.path, $event.password)" /></div>
          <div v-if="recoveryTaskId" class="repair-password-note" data-testid="recovery-task-context"><i class="pi pi-wrench"></i><span>正在处理失败的解压任务。救回新包后可直接加入解压队列，原输出设置会保留。</span></div>
          <section v-show="diagnosticSources.length < 2 || batchDetailOpen" class="integrity-panel min-w-0 overflow-hidden">
            <div class="archive-inputs">
              <button type="button" class="archive-select" data-testid="diagnostic-choose-archive" :disabled="isDiagnosing || isRepairing" @click="selectArchive">
                <i class="pi pi-folder-open"></i>
                <span class="min-w-0 truncate">{{ archiveDisplayPath ? archiveDisplayPath.split(/[\\/]/).pop() : '选择压缩包' }}<template v-if="nativeZpaqSource"> · {{nativeZpaqSource.members.length}} 个部分</template></span>
              </button>
              <input
                v-model="archivePassword"
                data-testid="diagnostic-password"
                type="password"
                autocomplete="off"
                class="archive-password"
                placeholder="密码（可选，不写入报告）"
                :disabled="isDiagnosing || isRepairing"
              />
            </div>
            <ZipPasswordEncodingField v-if="archivePath && (supportsZipPasswordEncoding(archivePath, sourceFormat) || (!sourceFormat && archiveZipPasswordEncoding !== undefined))" v-model="archiveZipPasswordEncoding" :source-path="archivePath" :source-format="sourceFormat" :disabled="isDiagnosing || isRepairing" class="mt-3 max-w-sm" test-id="diagnosis-zip-password-encoding" />
            <p v-if="archivePath" class="mt-3 text-xs text-muted break-all">{{ archiveDisplayPath }}</p>
            <div class="mt-5 flex flex-wrap gap-3">
              <button v-if="!isDiagnosing" data-testid="start-archive-diagnosis" type="button" class="archive-primary" :disabled="!archivePath || isRepairing" @click="diagnoseArchive">
                <i class="pi pi-search"></i>开始诊断
              </button>
              <button v-else type="button" class="archive-danger" @click="cancelDiagnosis">
                <i class="pi pi-stop-circle"></i>取消诊断
              </button>
              <button v-if="diagnosticReport" type="button" class="archive-secondary" @click="copyDiagnosticReport">
                <i class="pi pi-copy"></i>复制报告
              </button>
            </div>
          </section>

          <section v-if="diagnosticReport" v-show="diagnosticSources.length < 2 || batchDetailOpen" class="integrity-panel min-w-0 overflow-hidden" data-testid="diagnostic-report">
            <div class="flex flex-wrap items-start justify-between gap-4">
              <div>
                <p class="text-xs font-black tracking-[.22em] text-muted">诊断结论</p>
                <h2 class="mt-2 text-2xl font-black" :class="diagnosticReport.status === 'healthy' ? 'text-green-500' : ['password_required', 'partial_recovery', 'interrupted_tail', 'structure_only'].includes(diagnosticReport.status) ? 'text-amber-500' : 'text-red-500'">
                  {{ statusLabel(diagnosticReport.status) }}
                </h2>
              </div>
              <span class="diagnostic-format">{{ diagnosticReport.actualFormat }}</span>
            </div>

            <ZpaqBlockDetails v-if="diagnosticReport.zpaqBlocks" :report="diagnosticReport.zpaqBlocks" :source-path="archivePath" :password="archivePassword" :encrypted="diagnosticReport.encrypted" />
            <button v-if="diagnosticReport.actualFormat==='ZPAQ'" class="archive-secondary mt-3" data-testid="zpaq-diagnosis-browse" @click="appStore.openArchiveInBrowser(archivePath,{password:archivePassword,zpaqSnapshot:diagnosticReport.zpaqBlocks?.snapshot||archiveSnapshot})"><i class="pi pi-folder-open"/>{{diagnosticReport.status==='interrupted_tail'?'选择完整旧快照取回':'在浏览器选择快照'}}</button>
            <button v-if="isRar && diagnosticReport.status === 'healthy' && diagnosticReport.volumesFound > 1 && diagnosticReport.recoverability !== 'sfx_readonly'" class="archive-secondary mt-4" data-testid="open-create-recovery-volumes" @click="showRecoveryVolumes(archivePath, archivePassword)"><i class="pi pi-shield"></i>为这组分包创建恢复卷</button>
            <div class="diagnostic-metrics mt-6">
              <div><span>归档大小</span><strong>{{ formatBytes(diagnosticReport.fileSize) }}</strong></div>
              <div><span>{{ diagnosticReport.recoverability === 'missing_volume_entries' ? '可尝试文件 / 目录' : diagnosticReport.recoverability === 'local_entries' ? '已找到文件 / 目录' : '文件 / 目录' }}</span><strong>{{ diagnosticReport.integrityTested || diagnosticReport.totalFiles || diagnosticReport.totalDirectories ? `${diagnosticReport.totalFiles} / ${diagnosticReport.totalDirectories}` : '未能确认' }}</strong></div>
              <div><span>展开大小</span><strong>{{ diagnosticReport.totalUncompressedSizeKnown === false ? '大小未知' : formatBytes(diagnosticReport.totalUncompressedSize) }}</strong></div>
              <div><span>加密 / 分卷</span><strong>{{ encryptionLabel(diagnosticReport) }} / {{ diagnosticReport.splitArchive ? diagnosticReport.volumesFound + ' 卷' : '否' }}</strong></div>
            </div>

            <div v-if="diagnosticReport.issues.length" class="mt-6 space-y-3">
              <article v-for="issue in diagnosticReport.issues" :key="issue.code" class="diagnostic-issue" :data-severity="issue.severity">
                <strong>{{ issue.title }}</strong><p>{{ archiveInspectionMessageForDisplay(issue.detail) }}</p>
              </article>
            </div>
            <details class="mt-4 rounded-xl bg-input/30 p-3 min-w-0">
              <summary class="cursor-pointer text-xs font-bold text-muted">诊断依据</summary>
              <ul class="mt-2 space-y-1 text-xs text-muted break-all"><li v-for="item in diagnosticReport.evidence" :key="item">· {{ item }}</li></ul>
            </details>

            <div v-if="diagnosticReport.canRepair" class="mt-6 rounded-2xl border border-amber-500/30 bg-amber-500/5 p-5">
              <strong class="text-sm text-content">{{ isRar ? diagnosticReport.recoverability === 'rar_recovery_volumes' ? '可尝试使用恢复卷补齐或修复 RAR' : '可尝试修复 RAR' : diagnosticReport.encrypted ? '可尝试救回密码 ZIP 中的完整文件' : '可尝试恢复 ZIP 中的完整文件' }}</strong>
              <p class="mt-2 text-xs leading-5 text-muted">{{ isRar ? '在副本中使用本机 RAR 工具处理，完整校验通过后保存到新的 .repaired 文件夹；原件保持不变。恢复数据不足时可能无法修复。' : '只把仍能完整读取的条目写入新文件，跳过损坏条目；原压缩包不会被覆盖或删除。' }}</p>
              <p v-if="diagnosticReport.encrypted && !isRar" class="repair-password-note mt-3" data-testid="repair-password-note"><i class="pi pi-lock"></i><span>新包沿用填写的密码，使用 AES-256 加密（{{ zipPasswordEncodingLabel(sourceEncoding ?? 'utf8') }}）。</span></p>
              <button v-if="!isRepairing" data-testid="start-archive-repair" type="button" class="archive-primary mt-4" @click="repairArchive">
                <i class="pi pi-wrench"></i>选择位置并修复
              </button>
              <button v-else type="button" class="archive-danger mt-4" :disabled="isCancellingRepair" @click="cancelRepair">
                <i class="pi pi-stop-circle"></i>{{ isCancellingRepair ? '正在取消…' : '取消修复' }}
              </button>
            </div>
          </section>

          <section v-if="repairResult" v-show="diagnosticSources.length < 2 || batchDetailOpen" class="integrity-panel border" :class="repairResult.partial || repairResult.skippedEntries.length ? 'border-amber-500/30' : 'border-green-500/30'" data-testid="repair-result">
            <h2 class="text-lg font-black" :class="repairResult.partial || repairResult.skippedEntries.length ? 'text-amber-600' : 'text-green-500'">{{ repairResult.partial || repairResult.skippedEntries.length ? '部分恢复 · 新文件已通过校验' : '修复文件已通过完整性校验' }}</h2>
            <p class="mt-3 text-xs text-muted break-all">{{ repairResult.outputPath }}</p>
            <p class="mt-3 text-sm text-content" data-testid="repair-result-summary">恢复 {{ repairResult.recoveredFiles }} 个文件、{{ repairResult.recoveredDirectories }} 个目录；<template v-if="repairResult.entryResults?.length">未恢复 {{ repairResult.entryResults.filter(entry => entry.name && entry.status !== 'recovered').length }} 个已知文件<span v-if="repairResult.entryResults.some(entry => !entry.name)">，另有内容无法确认</span>。</template><template v-else>未恢复说明 {{ repairResult.skippedEntries.length }} 项。</template></p>
            <p class="repair-password-note mt-3" data-testid="repair-output-encryption"><i :class="repairResult.outputEncrypted ? 'pi pi-lock' : 'pi pi-folder'"></i><span>{{ repairResult.outputEncrypted ? repairResult.format === 'RAR' ? '修复后的 RAR 保留原加密方式与密码。' : `新包已使用 AES-256 加密，密码与恢复时填写的一致（${zipPasswordEncodingLabel(recoveredZipPasswordEncoding ?? 'utf8')}）。` : '新包未设置密码。' }}</span></p>
            <label v-if="recoveredPasswordRequired && !recoveredDraftId" class="mt-3 block text-xs text-muted">恢复结果密码<input v-model="recoveredPassword" type="password" autocomplete="off" placeholder="重启后需重新填写，不保存密码" class="archive-password mt-1 block w-full max-w-md" data-testid="single-repair-result-password" /></label>
            <p v-if="repairResult.outputFiles && repairResult.outputFiles.length > 1" class="mt-2 text-xs text-muted" data-testid="rar-repair-volumes">已保存 {{ repairResult.outputFiles.length }} 个数据卷，请整组保留；加入队列时会自动从首卷解压。</p>
            <p v-if="repairResult.directoryRebuilt" class="mt-2 text-xs text-amber-600">目录信息已重建，原包总条目数无法确认，请核对内容是否齐全。</p>
            <ArchiveRecoveryDetails class="mt-3" :recovered-entries="repairResult.recoveredEntries" :skipped-entries="repairResult.skippedEntries" :entry-results="repairResult.entryResults" />
            <div class="mt-4 flex flex-wrap gap-3"><button type="button" class="archive-secondary" @click="locateRepair"><i class="pi pi-folder-open"></i>定位新包</button><button type="button" class="archive-secondary" data-testid="browse-repaired-archive" @click="browseRepair"><i class="pi pi-box"></i>浏览新包</button><button type="button" class="archive-secondary" @click="copyChecksum(repairResult.outputPath)"><i class="pi pi-copy"></i>复制路径</button></div>
            <div class="mt-3 flex flex-wrap items-center gap-3"><button type="button" data-testid="queue-recovered-archive" class="archive-primary" :disabled="!!recoveredDraftId" @click="queueRecoveredArchive"><i class="pi pi-plus"></i>{{ recoveredDraftId ? '已加入解压队列' : '将新包加入解压队列' }}</button><a v-if="recoveredDraftId" href="#/decompress" class="text-sm font-bold text-primary">返回解压中心</a></div>
          </section>
        </div>
      </div>
    </div>
  </div>
  <RarRecoveryVolumesDialog v-model:visible="recoveryVolumesVisible" :path="recoveryVolumesSource" :password="recoveryVolumesPassword" />
</template>

<style scoped>
.repair-password-note { display: flex; align-items: flex-start; gap: .6rem; padding: .75rem .9rem; border-radius: .75rem; background: color-mix(in srgb, var(--dynamic-accent) 8%, transparent); color: var(--text-content); font-size: .75rem; line-height: 1.6; }
.repair-password-note i { flex: none; margin-top: .2rem; color: var(--dynamic-accent); }
.integrity-view { padding: 1rem 1.35rem 0; }
.integrity-header { display: flex; align-items: center; justify-content: space-between; padding: .15rem .15rem .8rem; }
.integrity-header h1 { color: var(--text-content); font-size: 1.55rem; font-weight: 950; letter-spacing: -.055em; line-height: 1.1; }
.integrity-header p { margin-top: .22rem; color: var(--text-muted); font-size: .62rem; font-weight: 700; letter-spacing: .04em; }
.integrity-scroll { padding: 0 .3rem 1.2rem 0; }
.integrity-workspace { display: grid; width: 100%; min-width: 0; gap: .85rem; }
.integrity-mode-switch { display: grid; grid-template-columns: repeat(3,minmax(0,1fr)); gap: .55rem; padding: .35rem; border: 1px solid var(--border-subtle); border-radius: .95rem; background: color-mix(in srgb,var(--bg-input) 48%,transparent); }
.integrity-mode-switch button { display: grid; min-width: 0; min-height: 3.5rem; grid-template-columns: 2rem minmax(0,1fr) .75rem; align-items: center; gap: .55rem; border: 1px solid transparent; border-radius: .72rem; padding: .45rem .65rem; color: var(--text-muted); text-align: left; transition: border-color .16s ease,background-color .16s ease,color .16s ease,transform .16s ease; }
.integrity-mode-switch button:hover { color: var(--text-content); transform: translateY(-1px); }
.integrity-mode-switch button.active { border-color: color-mix(in srgb,var(--dynamic-accent) 48%,var(--border-subtle)); background: color-mix(in srgb,var(--dynamic-accent) 9%,var(--bg-card)); color: var(--dynamic-accent); }
.mode-icon { display: grid; width: 2rem; height: 2rem; place-items: center; border-radius: .58rem; background: var(--bg-card); color: currentColor; font-size: .72rem; }
.integrity-mode-switch button>span:nth-child(2) { display: flex; min-width: 0; flex-direction: column; gap: .1rem; }
.integrity-mode-switch strong,.integrity-mode-switch small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.integrity-mode-switch strong { color: var(--text-content); font-size: .67rem; font-weight: 900; }
.integrity-mode-switch small { color: var(--text-muted); font-size: .52rem; font-weight: 650; }
.mode-arrow { font-size: .5rem; opacity: .45; }
.integrity-calculate-grid { display: grid; min-width: 0; grid-template-columns: minmax(12rem,.72fr) minmax(19rem,1.45fr); gap: .85rem; }
.integrity-panel { min-width: 0; border: 1px solid var(--border-subtle); border-radius: 1rem; background: color-mix(in srgb,var(--bg-card) 92%,transparent); padding: .85rem; box-shadow: 0 14px 34px -28px rgb(0 0 0 / .65); }
.panel-heading { display: flex; align-items: center; justify-content: space-between; gap: .75rem; margin-bottom: .65rem; }
.panel-heading span { display: flex; align-items: center; gap: .42rem; color: var(--text-content); font-size: .67rem; font-weight: 900; }
.panel-heading span i { color: var(--dynamic-accent); font-size: .65rem; }
.panel-heading small { color: var(--text-muted); font-size: .52rem; font-weight: 700; }
.algorithm-options { display: grid; gap: .38rem; }
.algorithm-options button { display: flex; min-height: 2.65rem; align-items: center; justify-content: space-between; gap: .5rem; border: 1px solid var(--border-subtle); border-radius: .68rem; background: var(--bg-input); padding: .42rem .62rem; color: var(--text-muted); text-align: left; transition: border-color .15s ease,background-color .15s ease; }
.algorithm-options button.active,.algorithm-options button:hover { border-color: color-mix(in srgb,var(--dynamic-accent) 55%,var(--border-subtle)); background: color-mix(in srgb,var(--dynamic-accent) 8%,var(--bg-input)); }
.algorithm-options button span { display: flex; min-width: 0; flex-direction: column; gap: .08rem; }
.algorithm-options strong { color: var(--text-content); font-size: .64rem; font-weight: 900; }
.algorithm-options small { color: var(--text-muted); font-size: .51rem; font-weight: 650; }
.algorithm-options button>i { color: var(--dynamic-accent); font-size: .52rem; }
.integrity-dropzone,.integrity-verify-dropzone { display: grid; width: 100%; min-width: 0; grid-template-columns: 2.25rem minmax(0,1fr) .8rem; align-items: center; gap: .65rem; border: 1px dashed color-mix(in srgb,var(--dynamic-accent) 38%,var(--border-subtle)); border-radius: .78rem; background: color-mix(in srgb,var(--dynamic-accent) 3%,var(--bg-input)); padding: .62rem .7rem; color: var(--text-muted); text-align: left; transition: border-color .15s ease,background-color .15s ease; }
.integrity-dropzone:hover,.integrity-verify-dropzone:hover { border-color: var(--dynamic-accent); background: color-mix(in srgb,var(--dynamic-accent) 7%,var(--bg-input)); }
.dropzone-icon { display: grid; width: 2.25rem; height: 2.25rem; place-items: center; border-radius: .62rem; background: color-mix(in srgb,var(--dynamic-accent) 12%,transparent); color: var(--dynamic-accent); font-size: .78rem; }
.integrity-dropzone>span:nth-child(2),.integrity-verify-dropzone>span:nth-child(2) { display: flex; min-width: 0; flex-direction: column; gap: .08rem; }
.integrity-dropzone strong,.integrity-verify-dropzone strong { overflow: hidden; color: var(--text-content); font-size: .66rem; font-weight: 900; text-overflow: ellipsis; white-space: nowrap; }
.integrity-dropzone small,.integrity-verify-dropzone small { color: var(--text-muted); font-size: .51rem; font-weight: 650; }
.integrity-dropzone>i,.integrity-verify-dropzone>i { color: var(--dynamic-accent); font-size: .58rem; }
.integrity-actions { display: flex; justify-content: flex-end; gap: .48rem; margin-top: .58rem; }
.integrity-primary,.integrity-secondary { display: inline-flex; min-height: 2.2rem; align-items: center; justify-content: center; gap: .38rem; border-radius: .65rem; padding: 0 .8rem; font-size: .6rem; font-weight: 900; }
.integrity-primary { background: var(--dynamic-accent); color: white; }
.integrity-primary:disabled { cursor: not-allowed; opacity: .42; }
.integrity-secondary { border: 1px solid var(--border-subtle); background: var(--bg-input); color: var(--text-content); }
.result-panel { grid-column: 1/-1; }
.checksum-results { display: grid; max-height: 16rem; gap: .35rem; overflow-y: auto; }
.checksum-row { display: grid; min-width: 0; grid-template-columns: 1.7rem minmax(0,1fr) auto 1.8rem; align-items: center; gap: .52rem; border: 1px solid var(--border-subtle); border-radius: .68rem; background: var(--bg-input); padding: .42rem .52rem; }
.checksum-status { display: grid; width: 1.6rem; height: 1.6rem; place-items: center; border-radius: .48rem; background: color-mix(in srgb,var(--dynamic-accent) 10%,transparent); color: var(--dynamic-accent); font-size: .55rem; }
.checksum-status[data-status="success"] { background: color-mix(in srgb,#22c55e 12%,transparent); color: #22c55e; }
.checksum-status[data-status="error"] { background: color-mix(in srgb,#ef4444 12%,transparent); color: #ef4444; }
.checksum-main { display: flex; min-width: 0; flex-direction: column; gap: .08rem; }
.checksum-main strong,.checksum-main code { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.checksum-main strong { color: var(--text-content); font-size: .61rem; font-weight: 850; }
.checksum-main code { color: var(--text-muted); font-size: .52rem; }
.checksum-main small { overflow:hidden; text-overflow:ellipsis; white-space:nowrap; color:var(--text-muted); font-size:.55rem; }
.checksum-calculation-toggle { display:flex; min-width:0; align-items:center; justify-content:space-between; gap:.5rem; text-align:left; color:var(--text-muted); }
.checksum-calculation-toggle strong { min-width:0; }
.checksum-calculation-toggle i { flex-shrink:0; font-size:.55rem; }
.checksum-calculation-details { grid-column:1/-1; display:grid; min-width:0; gap:.35rem; border-top:1px solid var(--border-subtle); padding:.65rem .25rem .25rem; font-size:.65rem; }
.checksum-calculation-details span { color:var(--text-muted); }
.checksum-calculation-details code { white-space:normal; overflow-wrap:anywhere; user-select:text; }
.checksum-source-summary { min-width:0; margin-top:.65rem; color:var(--text-muted); font-size:.65rem; line-height:1.7; }
.checksum-source-toggle { display:inline-flex; align-items:center; gap:.4rem; padding:.35rem 0; color:var(--dynamic-accent); }
.checksum-source-paths { max-height:8rem; overflow-y:auto; padding:.5rem .65rem; border:1px solid var(--border-subtle); border-radius:.6rem; background:var(--bg-input); }
.checksum-source-paths li { overflow-wrap:anywhere; user-select:text; padding:.25rem 0; }
.checksum-export-error { margin-top:.7rem; padding:.7rem; border:1px solid color-mix(in srgb,#ef4444 35%,var(--border-subtle)); border-radius:.65rem; overflow-wrap:anywhere; font-size:.7rem; line-height:1.7; }
.checksum-export-receipt { display:flex; min-width:0; flex-direction:column; align-items:flex-start; gap:.5rem; margin-top:.7rem; padding:.8rem; border:1px solid color-mix(in srgb,var(--dynamic-accent) 30%,var(--border-subtle)); border-radius:.75rem; background:color-mix(in srgb,var(--dynamic-accent) 5%,var(--bg-input)); font-size:.7rem; }
.checksum-export-receipt strong { color:var(--dynamic-accent); }
.checksum-export-receipt code { max-width:100%; overflow-wrap:anywhere; user-select:text; }
.checksum-export-receipt small { color:var(--text-muted); line-height:1.7; }
.checksum-handoff { display:flex; min-width:0; align-items:center; flex-wrap:wrap; gap:.5rem; border:1px solid var(--border-subtle); border-radius:.8rem; padding:.75rem; font-size:.7rem; }
.checksum-handoff span { min-width:0; flex:1 1 14rem; overflow-wrap:anywhere; }
.checksum-algorithm { border-radius: 999px; background: color-mix(in srgb,var(--dynamic-accent) 9%,transparent); padding: .2rem .42rem; color: var(--dynamic-accent); font-size: .5rem; font-weight: 900; }
.checksum-copy { display: grid; width: 1.75rem; height: 1.75rem; place-items: center; border-radius: .5rem; color: var(--text-muted); font-size: .55rem; }
.checksum-copy:hover { background: color-mix(in srgb,var(--dynamic-accent) 10%,transparent); color: var(--dynamic-accent); }
.algorithm-options button:disabled,.integrity-dropzone:disabled { opacity:.55; cursor:wait; }
.integrity-single-column { display: grid; min-width: 0; gap: .85rem; }
.integrity-verify-dropzone { min-height: 5.4rem; padding-right: 1rem; }
.verify-result { border-width: 1px; }
.verify-result-icon { display: grid; width: 2.5rem; height: 2.5rem; flex: 0 0 2.5rem; place-items: center; border-radius: .7rem; background: var(--bg-input); font-size: 1rem; }
.archive-select, .archive-password { min-width: 0; min-height: 3rem; border: 1px solid var(--border-subtle); border-radius: .9rem; background: color-mix(in srgb, var(--bg-input) 72%, transparent); color: var(--text-content); }
.archive-inputs { display: grid; grid-template-columns: minmax(0, 1fr) minmax(12rem, .85fr); gap: .75rem; }
.archive-select { flex: 1 1 18rem; display: flex; align-items: center; gap: .65rem; padding: 0 1rem; font-size: .78rem; font-weight: 900; text-align: left; }
.archive-password { flex: 0 1 18rem; padding: 0 1rem; font-size: .78rem; outline: none; }
.archive-password:focus, .archive-select:hover { border-color: var(--dynamic-accent); }
.archive-primary, .archive-secondary, .archive-danger { min-height: 2.6rem; display: inline-flex; align-items: center; justify-content: center; gap: .45rem; border-radius: .8rem; padding: 0 1rem; font-size: .72rem; font-weight: 900; }
.archive-primary { background: var(--dynamic-accent); color: white; }
.archive-secondary { border: 1px solid var(--border-subtle); background: var(--bg-input); color: var(--text-content); }
.archive-danger { background: color-mix(in srgb, #ef4444 78%, var(--bg-card)); color: white; }
.archive-primary:disabled { cursor: not-allowed; opacity: .45; }
.diagnostic-format { border-radius: 999px; padding: .4rem .7rem; background: color-mix(in srgb, var(--dynamic-accent) 12%, transparent); color: var(--dynamic-accent); font-size: .7rem; font-weight: 900; }
.diagnostic-metrics { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: .65rem; }
.diagnostic-metrics > div { min-width: 0; border-radius: .9rem; padding: .8rem; background: color-mix(in srgb, var(--bg-input) 65%, transparent); }
.diagnostic-metrics span, .diagnostic-metrics strong { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.diagnostic-metrics span { color: var(--text-muted); font-size: .64rem; font-weight: 800; }
.diagnostic-metrics strong { margin-top: .25rem; color: var(--text-content); font-size: .82rem; }
.diagnostic-issue { border-left: 3px solid #ef4444; border-radius: .7rem; padding: .75rem .9rem; background: color-mix(in srgb, #ef4444 8%, transparent); }
.diagnostic-issue[data-severity="warning"] { border-left-color: #f59e0b; background: color-mix(in srgb, #f59e0b 8%, transparent); }
.diagnostic-issue strong { color: var(--text-content); font-size: .78rem; }
.diagnostic-issue p { margin-top: .25rem; color: var(--text-muted); font-size: .7rem; line-height: 1.45; }
@media (max-width: 850px) { .integrity-calculate-grid { grid-template-columns: 1fr; } .result-panel { grid-column: auto; } .diagnostic-metrics { grid-template-columns: repeat(2, minmax(0, 1fr)); } .archive-inputs { grid-template-columns: minmax(0, 1fr); } }
@media (max-width: 620px) { .integrity-view { padding-inline: .75rem; } .integrity-mode-switch { grid-template-columns: 1fr; } .integrity-mode-switch button { min-height: 3rem; } .checksum-row { grid-template-columns: 1.7rem minmax(0,1fr) 1.8rem; } .checksum-algorithm { display: none; } }

.integrity-header{gap:10px;flex-wrap:wrap;padding-bottom:10px}.integrity-mode-switch{display:flex;gap:4px;padding:3px;flex-wrap:wrap}.integrity-mode-switch button{display:flex;gap:7px;min-height:32px;padding:5px 10px}.integrity-mode-switch button>span:first-child{width:auto;height:auto;background:transparent}.integrity-mode-switch strong{font-size:12px}.integrity-header h1{font-size:23px}.integrity-calculate-grid{grid-template-columns:minmax(0,1fr)}.algorithm-panel{display:flex;align-items:center;gap:12px;flex-wrap:wrap}.algorithm-panel .panel-heading{margin:0}.algorithm-options{display:flex;gap:6px;flex:1;flex-wrap:wrap}.algorithm-options button{min-height:30px;padding:5px 12px;gap:10px}.algorithm-options strong{font-size:12px}.integrity-dropzone{padding:8px 10px}.integrity-workspace{gap:10px}.integrity-source-summary{display:flex;align-items:center;gap:8px;flex-wrap:wrap}.result-panel{grid-column:1/-1}@media(max-width:620px){.integrity-mode-switch{display:flex;flex-wrap:wrap}.integrity-mode-switch button{min-height:32px}}

.file-panel.has-results{display:flex;align-items:center;gap:8px 12px;flex-wrap:wrap}.file-panel.has-results>.integrity-dropzone{width:auto;flex:1 1 165px;min-height:36px;padding:5px 8px;grid-template-columns:24px minmax(0,1fr) 14px}.file-panel.has-results .dropzone-icon{width:24px;height:24px;font-size:12px}.file-panel.has-results .checksum-source-summary{display:flex;align-items:center;gap:8px;flex-wrap:wrap;margin:0}.file-panel.has-results .checksum-source-summary>ul{flex-basis:100%}.file-panel.has-results>.integrity-actions{margin:0 0 0 auto;flex-wrap:wrap}.file-panel.has-results>.checksum-export-receipt,.file-panel.has-results>[role=alert]{flex-basis:100%}
</style>
