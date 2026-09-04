<script setup lang="ts">
import { ref } from 'vue'
import { useAppStore } from '@/stores/app'
import { useTauriCommands, type ArchiveDiagnosticReport, type ZipRepairResult } from '@/composables/useTauriCommands'
import { open, save } from '@tauri-apps/api/dialog'

const appStore = useAppStore()
const tauriCommands = useTauriCommands()

interface ChecksumResult {
  path: string
  fileName: string
  algorithm: string
  checksum: string
  status: 'success' | 'pending' | 'error'
  error?: string
}

const selectedFiles = ref<string[]>([])
const selectedAlgorithm = ref<'crc32' | 'md5' | 'sha256'>('sha256')
const checksumResults = ref<ChecksumResult[]>([])
const isCalculating = ref(false)
const activeMode = ref<'calculate' | 'verify' | 'archive'>('calculate')
const verifyResult = ref<{ valid: boolean; message: string } | null>(null)
const archivePath = ref('')
const archivePassword = ref('')
const diagnosticReport = ref<ArchiveDiagnosticReport | null>(null)
const repairResult = ref<ZipRepairResult | null>(null)
const diagnosticId = ref('')
const isDiagnosing = ref(false)
const isRepairing = ref(false)
const repairId = ref('')

const algorithms = [
  { value: 'crc32', label: 'CRC32', description: '快速校验' },
  { value: 'md5', label: 'MD5', description: '中等强度' },
  { value: 'sha256', label: 'SHA256', description: '高强度（推荐）' },
]

const selectFiles = async () => {
  try {
    const files = await open({
      multiple: true,
      title: appStore.t('integrity.select_files', '选择文件')
    })
    if (files) {
      selectedFiles.value = Array.isArray(files) ? files : [files]
      checksumResults.value = []
      verifyResult.value = null
    }
  } catch (error) {
    appStore.setError(appStore.t('common.error'))
  }
}

const calculateChecksums = async () => {
  if (selectedFiles.value.length === 0) {
    appStore.setError(appStore.t('integrity.no_files', '请先选择文件'))
    return
  }

  isCalculating.value = true
  checksumResults.value = selectedFiles.value.map(path => ({
    path,
    fileName: path.split(/[\\/]/).pop() || path,
    algorithm: selectedAlgorithm.value.toUpperCase(),
    checksum: '',
    status: 'pending'
  }))

  for (let i = 0; i < selectedFiles.value.length; i++) {
    const path = selectedFiles.value[i]
    try {
      const checksum = await tauriCommands.invoke<string>('calculate_checksum', {
        path,
        algorithm: selectedAlgorithm.value
      })
      checksumResults.value[i].checksum = checksum
      checksumResults.value[i].status = 'success'
    } catch (error: any) {
      checksumResults.value[i].status = 'error'
      checksumResults.value[i].error = String(error)
    }
  }

  isCalculating.value = false
  const successCount = checksumResults.value.filter(result => result.status === 'success').length
  if (successCount > 0) {
    appStore.setSuccess(appStore.t('integrity.calc_complete', '校验和计算完成'))
  } else {
    appStore.setError(appStore.t('integrity.verify_error', '校验出错'))
  }
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
  if (checksumResults.value.length === 0) {
    appStore.setError(appStore.t('integrity.no_results', '没有可导出的结果'))
    return
  }

  try {
    const firstFile = selectedFiles.value[0]
    const dir = firstFile.substring(0, Math.max(firstFile.lastIndexOf('/'), firstFile.lastIndexOf('\\')))
    const ext = selectedAlgorithm.value === 'crc32' ? '.sfv' : `.${selectedAlgorithm.value}`

    const savePath = await save({
      title: appStore.t('integrity.export', '导出校验文件'),
      defaultPath: `${dir}/checksums${ext}`,
      filters: [{
        name: `${selectedAlgorithm.value.toUpperCase()} Checksum File`,
        extensions: [ext.slice(1)]
      }]
    }) as string | null

    if (savePath) {
      await tauriCommands.invoke('export_checksum_file', {
        path: savePath,
        results: checksumResults.value.filter(r => r.status === 'success').map(r => ({
          file_name: r.fileName,
          checksum: r.checksum
        })),
        algorithm: selectedAlgorithm.value
      })
      appStore.setSuccess(appStore.t('integrity.exported', '校验文件已导出'))
    }
  } catch (error) {
    appStore.setError(appStore.t('integrity.export_failed', '导出失败'))
  }
}

const selectChecksumFile = async () => {
  try {
    const file = await open({
      multiple: false,
      title: appStore.t('integrity.select_checksum', '选择校验文件'),
      filters: [{
        name: 'Checksum Files',
        extensions: ['md5', 'sha256', 'sfv']
      }]
    })
    if (file && typeof file === 'string') {
      await verifyChecksumFile(file)
    }
  } catch (error) {
    appStore.setError(appStore.t('common.error'))
  }
}

const verifyChecksumFile = async (checksumPath: string) => {
  isCalculating.value = true
  verifyResult.value = null

  try {
    const result = await tauriCommands.invoke<{ valid: boolean; message: string }>('verify_checksum_file', {
      checksumPath
    })
    verifyResult.value = result
    if (result.valid) {
      appStore.setSuccess(appStore.t('integrity.verify_success', '✓ 校验通过'))
    } else {
      appStore.setError(appStore.t('integrity.verify_failed', '✗ 校验失败'))
    }
  } catch (error: any) {
    verifyResult.value = {
      valid: false,
      message: String(error)
    }
    appStore.setError(appStore.t('integrity.verify_error', '校验出错'))
  } finally {
    isCalculating.value = false
  }
}

const clearResults = () => {
  selectedFiles.value = []
  checksumResults.value = []
  verifyResult.value = null
  archivePath.value = ''
  archivePassword.value = ''
  diagnosticReport.value = null
  repairResult.value = null
}

const selectArchive = async () => {
  const file = await open({ multiple: false, title: '选择要诊断的压缩包' })
  if (typeof file === 'string') {
    archivePath.value = file
    diagnosticReport.value = null
    repairResult.value = null
  }
}

const diagnoseArchive = async () => {
  if (!archivePath.value || isDiagnosing.value) return
  isDiagnosing.value = true
  repairResult.value = null
  diagnosticId.value = `diagnostic-${Date.now()}-${Math.random().toString(36).slice(2)}`
  try {
    diagnosticReport.value = await tauriCommands.diagnoseArchive(
      diagnosticId.value, archivePath.value, archivePassword.value,
    )
  } catch (error) {
    if (!String(error).toLocaleLowerCase().includes('cancel')) {
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

const defaultRepairPath = () => archivePath.value.replace(/\.zip$/i, '') + '.repaired.zip'

const repairArchive = async () => {
  if (!diagnosticReport.value?.canRepair || isRepairing.value) return
  const outputPath = await save({
    title: '保存修复后的 ZIP',
    defaultPath: defaultRepairPath(),
    filters: [{ name: 'ZIP Archive', extensions: ['zip'] }],
  }) as string | null
  if (!outputPath) return
  isRepairing.value = true
  repairId.value = `repair-${Date.now()}-${Math.random().toString(36).slice(2)}`
  try {
    repairResult.value = await tauriCommands.repairZip(repairId.value, archivePath.value, outputPath)
    appStore.setSuccess('ZIP 已重建到新文件并通过完整性校验')
  } catch (error) {
    if (!String(error).toLocaleLowerCase().includes('cancel')) {
      appStore.setError(`ZIP 修复失败：${String(error)}`)
    }
  } finally {
    isRepairing.value = false
    repairId.value = ''
  }
}

const cancelRepair = async () => {
  if (!repairId.value) return
  try { await tauriCommands.cancelZipRepair(repairId.value) } catch { /* repair may have just completed */ }
}

const statusLabel = (status: string) => ({
  healthy: '完整性正常', password_required: '需要密码', wrong_password: '密码错误',
  missing_volume: '缺少分卷', crc_error: '内容校验失败', truncated: '归档被截断',
  damaged: '归档损坏', checking: '检查中',
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

const reportText = () => {
  const report = diagnosticReport.value
  if (!report) return ''
  return [
    'Long解压 · 归档诊断报告',
    `文件：${report.filePath}`,
    `格式：${report.actualFormat}`,
    `状态：${statusLabel(report.status)}`,
    `大小：${report.fileSize} bytes`,
    `加密：${report.encrypted ? '是' : '否'}`,
    `分卷：${report.splitArchive ? `是（发现 ${report.volumesFound} 卷）` : '否'}`,
    `条目：${report.totalFiles} 个文件 / ${report.totalDirectories} 个目录`,
    ...report.issues.map(issue => `[${issue.severity}] ${issue.title}：${issue.detail}`),
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
</script>

<template>
  <div class="integrity-view flex h-full flex-col overflow-x-hidden transition-colors duration-700">
    <!-- 顶部标题栏 -->
    <header class="integrity-header shrink-0">
      <div>
        <h1>{{ appStore.t('integrity.title') }}</h1>
        <p>{{ appStore.t('integrity.subtitle') }}</p>
      </div>
    </header>

    <!-- 主内容区 -->
    <div class="integrity-scroll custom-scrollbar min-h-0 flex-1 overflow-y-auto overflow-x-hidden">
      <div class="integrity-workspace">
        <!-- 模式切换 -->
        <nav class="integrity-mode-switch" aria-label="文件完整性工具">
          <button
            @click="activeMode = 'calculate'; clearResults()"
            :class="{ active: activeMode === 'calculate' }"
          >
            <span class="mode-icon"><i class="pi pi-hashtag"></i></span>
            <span><strong>{{ appStore.t('integrity.mode.calculate') }}</strong><small>生成文件指纹</small></span>
            <i class="pi pi-chevron-right mode-arrow"></i>
          </button>
          <button
            @click="activeMode = 'verify'; clearResults()"
            :class="{ active: activeMode === 'verify' }"
          >
            <span class="mode-icon"><i class="pi pi-check-circle"></i></span>
            <span><strong>{{ appStore.t('integrity.mode.verify') }}</strong><small>核对校验文件</small></span>
            <i class="pi pi-chevron-right mode-arrow"></i>
          </button>
          <button
            data-testid="archive-diagnostic-mode"
            @click="activeMode = 'archive'; clearResults()"
            :class="{ active: activeMode === 'archive' }"
          >
            <span class="mode-icon"><i class="pi pi-search"></i></span>
            <span><strong>归档诊断</strong><small>识别损坏与缺卷</small></span>
            <i class="pi pi-chevron-right mode-arrow"></i>
          </button>
        </nav>

        <!-- 计算模式 -->
        <div v-if="activeMode === 'calculate'" class="integrity-calculate-grid">
          <!-- 选择算法 -->
          <section class="integrity-panel algorithm-panel">
            <header class="panel-heading"><span><i class="pi pi-sliders-h"></i>{{ appStore.t('integrity.algorithm') }}</span><small>选择摘要算法</small></header>
            <div class="algorithm-options">
              <button
                v-for="algo in algorithms"
                :key="algo.value"
                @click="selectedAlgorithm = algo.value as any"
                :class="{ active: selectedAlgorithm === algo.value }"
              >
                <span><strong>{{ algo.label }}</strong><small>{{ algo.description }}</small></span>
                <i v-if="selectedAlgorithm === algo.value" class="pi pi-check"></i>
              </button>
            </div>
          </section>

          <!-- 选择文件 -->
          <section class="integrity-panel file-panel">
            <header class="panel-heading"><span><i class="pi pi-file"></i>{{ appStore.t('integrity.files') }}</span><small>{{ selectedFiles.length ? `${selectedFiles.length} 个文件待处理` : '支持批量选择' }}</small></header>
            <button
              @click="selectFiles"
              class="integrity-dropzone"
            >
              <span class="dropzone-icon"><i class="pi pi-folder-open"></i></span>
              <span>
                <strong>
                {{ selectedFiles.length > 0
                  ? `已选择 ${selectedFiles.length} 个文件`
                  : appStore.t('integrity.select_files') }}
                </strong>
                <small>{{ selectedFiles.length ? '再次选择可替换当前列表' : '从本地选择一个或多个文件' }}</small>
              </span>
              <i class="pi pi-plus"></i>
            </button>

            <!-- 操作按钮 -->
            <div class="integrity-actions">
              <button
                @click="calculateChecksums"
                :disabled="selectedFiles.length === 0 || isCalculating"
                class="integrity-primary"
              >
                <i :class="isCalculating ? 'pi pi-spinner pi-spin' : 'pi pi-play'"></i>
                {{ isCalculating ? appStore.t('integrity.calculating') : appStore.t('integrity.calculate') }}
              </button>
              <button
                v-if="checksumResults.length > 0"
                @click="exportChecksumFile"
                class="integrity-secondary"
              >
                <i class="pi pi-download"></i>
                {{ appStore.t('integrity.export') }}
              </button>
            </div>
          </section>

          <!-- 结果列表 -->
          <section v-if="checksumResults.length > 0" class="integrity-panel result-panel">
            <header class="panel-heading"><span><i class="pi pi-list"></i>{{ appStore.t('integrity.results') }}</span><small>{{ checksumResults.filter(item => item.status === 'success').length }} / {{ checksumResults.length }} 完成</small></header>
            <div class="checksum-results custom-scrollbar">
              <div
                v-for="result in checksumResults"
                :key="result.path"
                class="checksum-row"
              >
                <span class="checksum-status" :data-status="result.status"><i :class="result.status === 'success' ? 'pi pi-check' : result.status === 'error' ? 'pi pi-times' : 'pi pi-spinner pi-spin'"></i></span>
                <div class="checksum-main"><strong :title="result.fileName">{{ result.fileName }}</strong><code :title="result.checksum">{{ result.error || result.checksum || '计算中...' }}</code></div>
                <span class="checksum-algorithm">{{ result.algorithm }}</span>
                    <button
                      v-if="result.status === 'success'"
                      @click="copyChecksum(result.checksum)"
                  class="checksum-copy"
                  :title="appStore.t('integrity.copy')"
                    >
                  <i class="pi pi-copy"></i><span class="sr-only">{{ appStore.t('integrity.copy') }}</span>
                    </button>
              </div>
            </div>
          </section>
        </div>

        <!-- 验证模式 -->
        <div v-else-if="activeMode === 'verify'" class="integrity-single-column">
          <section class="integrity-panel">
            <button
              @click="selectChecksumFile"
              :disabled="isCalculating"
              class="integrity-verify-dropzone"
            >
              <span class="dropzone-icon"><i class="pi pi-check-circle"></i></span>
              <span><strong>{{ appStore.t('integrity.select_checksum') }}</strong><small>支持 .md5、.sha256、.sfv</small></span>
              <i class="pi pi-folder-open"></i>
            </button>
          </section>

          <!-- 验证结果 -->
          <section v-if="verifyResult" class="integrity-panel verify-result"
            :class="verifyResult.valid
              ? 'bg-green-500/5 border-green-500'
              : 'bg-red-500/5 border-red-500'"
          >
            <div class="flex items-center gap-4">
              <span class="verify-result-icon"><i :class="verifyResult.valid ? 'pi pi-check' : 'pi pi-times'"></i></span>
              <div class="flex-1">
                <div class="text-lg font-black uppercase tracking-widest"
                  :class="verifyResult.valid ? 'text-green-500' : 'text-red-500'"
                >
                  {{ verifyResult.valid ? appStore.t('integrity.verify_success') : appStore.t('integrity.verify_failed') }}
                </div>
                <div class="text-xs text-muted mt-2 uppercase tracking-tighter">{{ verifyResult.message }}</div>
              </div>
            </div>
          </section>
        </div>

        <div v-else class="integrity-single-column min-w-0 overflow-x-hidden" data-testid="archive-diagnostic-panel">
          <section class="integrity-panel min-w-0 overflow-hidden">
            <div class="flex flex-col md:flex-row gap-3">
              <button type="button" class="archive-select" @click="selectArchive">
                <i class="pi pi-folder-open"></i>
                <span class="min-w-0 truncate">{{ archivePath ? archivePath.split(/[\\/]/).pop() : '选择压缩包' }}</span>
              </button>
              <input
                v-model="archivePassword"
                type="password"
                autocomplete="off"
                class="archive-password"
                placeholder="密码（可选，不写入报告）"
                :disabled="isDiagnosing"
              />
            </div>
            <p v-if="archivePath" class="mt-3 text-xs text-muted break-all">{{ archivePath }}</p>
            <div class="mt-5 flex flex-wrap gap-3">
              <button v-if="!isDiagnosing" type="button" class="archive-primary" :disabled="!archivePath" @click="diagnoseArchive">
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

          <section v-if="diagnosticReport" class="integrity-panel min-w-0 overflow-hidden" data-testid="diagnostic-report">
            <div class="flex flex-wrap items-start justify-between gap-4">
              <div>
                <p class="text-xs font-black tracking-[.22em] text-muted">诊断结论</p>
                <h2 class="mt-2 text-2xl font-black" :class="diagnosticReport.status === 'healthy' ? 'text-green-500' : diagnosticReport.status === 'password_required' ? 'text-amber-500' : 'text-red-500'">
                  {{ statusLabel(diagnosticReport.status) }}
                </h2>
              </div>
              <span class="diagnostic-format">{{ diagnosticReport.actualFormat }}</span>
            </div>

            <div class="diagnostic-metrics mt-6">
              <div><span>归档大小</span><strong>{{ formatBytes(diagnosticReport.fileSize) }}</strong></div>
              <div><span>文件 / 目录</span><strong>{{ diagnosticReport.totalFiles }} / {{ diagnosticReport.totalDirectories }}</strong></div>
              <div><span>展开大小</span><strong>{{ formatBytes(diagnosticReport.totalUncompressedSize) }}</strong></div>
              <div><span>加密 / 分卷</span><strong>{{ diagnosticReport.encrypted ? '是' : '否' }} / {{ diagnosticReport.splitArchive ? diagnosticReport.volumesFound + ' 卷' : '否' }}</strong></div>
            </div>

            <div v-if="diagnosticReport.issues.length" class="mt-6 space-y-3">
              <article v-for="issue in diagnosticReport.issues" :key="issue.code" class="diagnostic-issue" :data-severity="issue.severity">
                <strong>{{ issue.title }}</strong><p>{{ issue.detail }}</p>
              </article>
            </div>
            <div class="mt-6 rounded-2xl bg-input/30 p-4 min-w-0">
              <p class="text-xs font-black text-content">诊断证据</p>
              <ul class="mt-2 space-y-1 text-xs text-muted break-all"><li v-for="item in diagnosticReport.evidence" :key="item">· {{ item }}</li></ul>
            </div>

            <div v-if="diagnosticReport.canRepair" class="mt-6 rounded-2xl border border-amber-500/30 bg-amber-500/5 p-5">
              <strong class="text-sm text-content">可尝试安全重建 ZIP</strong>
              <p class="mt-2 text-xs leading-5 text-muted">只把仍能完整读取的条目写入新文件，跳过损坏条目；原压缩包不会被覆盖或删除。</p>
              <button v-if="!isRepairing" type="button" class="archive-primary mt-4" @click="repairArchive">
                <i class="pi pi-wrench"></i>选择位置并修复
              </button>
              <button v-else type="button" class="archive-danger mt-4" @click="cancelRepair">
                <i class="pi pi-stop-circle"></i>取消修复
              </button>
            </div>
          </section>

          <section v-if="repairResult" class="integrity-panel border border-green-500/30" data-testid="repair-result">
            <h2 class="text-lg font-black text-green-500">修复文件已通过完整性校验</h2>
            <p class="mt-3 text-xs text-muted break-all">{{ repairResult.outputPath }}</p>
            <p class="mt-3 text-sm text-content">恢复 {{ repairResult.recoveredFiles }} 个文件、{{ repairResult.recoveredDirectories }} 个目录；跳过 {{ repairResult.skippedEntries.length }} 个损坏或不安全条目。</p>
            <ul v-if="repairResult.skippedEntries.length" class="mt-3 space-y-1 text-xs text-amber-500 break-all"><li v-for="entry in repairResult.skippedEntries" :key="entry">· {{ entry }}</li></ul>
          </section>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
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
.checksum-algorithm { border-radius: 999px; background: color-mix(in srgb,var(--dynamic-accent) 9%,transparent); padding: .2rem .42rem; color: var(--dynamic-accent); font-size: .5rem; font-weight: 900; }
.checksum-copy { display: grid; width: 1.75rem; height: 1.75rem; place-items: center; border-radius: .5rem; color: var(--text-muted); font-size: .55rem; }
.checksum-copy:hover { background: color-mix(in srgb,var(--dynamic-accent) 10%,transparent); color: var(--dynamic-accent); }
.integrity-single-column { display: grid; min-width: 0; gap: .85rem; }
.integrity-verify-dropzone { min-height: 5.4rem; padding-right: 1rem; }
.verify-result { border-width: 1px; }
.verify-result-icon { display: grid; width: 2.5rem; height: 2.5rem; flex: 0 0 2.5rem; place-items: center; border-radius: .7rem; background: var(--bg-input); font-size: 1rem; }
.archive-select, .archive-password { min-width: 0; min-height: 3rem; border: 1px solid var(--border-subtle); border-radius: .9rem; background: color-mix(in srgb, var(--bg-input) 72%, transparent); color: var(--text-content); }
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
@media (max-width: 850px) { .integrity-calculate-grid { grid-template-columns: 1fr; } .result-panel { grid-column: auto; } .diagnostic-metrics { grid-template-columns: repeat(2, minmax(0, 1fr)); } .archive-password { flex-basis: 100%; } }
@media (max-width: 620px) { .integrity-view { padding-inline: .75rem; } .integrity-mode-switch { grid-template-columns: 1fr; } .integrity-mode-switch button { min-height: 3rem; } .checksum-row { grid-template-columns: 1.7rem minmax(0,1fr) 1.8rem; } .checksum-algorithm { display: none; } }
</style>
