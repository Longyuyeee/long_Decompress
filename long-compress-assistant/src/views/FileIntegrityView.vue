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
  <div class="integrity-view flex flex-col h-full p-responsive p-8 transition-colors duration-700 overflow-x-hidden">
    <!-- 顶部标题栏 -->
    <header class="shrink-0 mb-8">
      <h1 class="text-4xl font-black text-content tracking-tighter mb-2">{{ appStore.t('integrity.title') }}</h1>
      <p class="text-muted text-sm font-bold uppercase tracking-[0.3em] ml-1">{{ appStore.t('integrity.subtitle') }}</p>
    </header>

    <!-- 主内容区 -->
    <div class="integrity-scroll flex-1 overflow-y-auto overflow-x-hidden custom-scrollbar pr-2 pb-20">
      <div class="max-w-5xl space-y-8">
        <!-- 模式切换 -->
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
          <button
            @click="activeMode = 'calculate'; clearResults()"
            class="aero-card p-8 text-left transition-all hover:scale-[1.02]"
            :class="activeMode === 'calculate' ? 'ring-2 ring-primary' : ''"
          >
            <div class="text-4xl mb-3">🔢</div>
            <div class="text-sm font-black text-content uppercase tracking-widest">{{ appStore.t('integrity.mode.calculate') }}</div>
            <div class="text-xs text-muted mt-2 uppercase tracking-tighter">计算文件的校验和</div>
          </button>
          <button
            @click="activeMode = 'verify'; clearResults()"
            class="aero-card p-8 text-left transition-all hover:scale-[1.02]"
            :class="activeMode === 'verify' ? 'ring-2 ring-primary' : ''"
          >
            <div class="text-4xl mb-3">✓</div>
            <div class="text-sm font-black text-content uppercase tracking-widest">{{ appStore.t('integrity.mode.verify') }}</div>
            <div class="text-xs text-muted mt-2 uppercase tracking-tighter">验证校验文件的正确性</div>
          </button>
          <button
            data-testid="archive-diagnostic-mode"
            @click="activeMode = 'archive'; clearResults()"
            class="aero-card p-8 text-left transition-all hover:scale-[1.02]"
            :class="activeMode === 'archive' ? 'ring-2 ring-primary' : ''"
          >
            <div class="text-4xl mb-3">🩺</div>
            <div class="text-sm font-black text-content uppercase tracking-widest">归档诊断</div>
            <div class="text-xs text-muted mt-2 uppercase tracking-tighter">识别损坏、缺卷与可恢复性</div>
          </button>
        </div>

        <!-- 计算模式 -->
        <div v-if="activeMode === 'calculate'" class="space-y-8">
          <!-- 选择算法 -->
          <section class="aero-card p-10">
            <h2 class="text-sm font-black text-content uppercase tracking-[0.3em] mb-6">
              {{ appStore.t('integrity.algorithm') }}
            </h2>
            <div class="grid grid-cols-3 gap-4">
              <button
                v-for="algo in algorithms"
                :key="algo.value"
                @click="selectedAlgorithm = algo.value as any"
                class="p-6 rounded-2xl border-2 transition-all text-left hover:scale-[1.02]"
                :class="selectedAlgorithm === algo.value
                  ? 'bg-primary/10 border-primary shadow-lg'
                  : 'bg-input/30 border-subtle hover:border-primary/50'"
              >
                <div class="text-sm font-black text-content uppercase tracking-widest">{{ algo.label }}</div>
                <div class="text-xs text-muted mt-2 uppercase tracking-tighter">{{ algo.description }}</div>
              </button>
            </div>
          </section>

          <!-- 选择文件 -->
          <section class="aero-card p-10">
            <h2 class="text-sm font-black text-content uppercase tracking-[0.3em] mb-6">
              {{ appStore.t('integrity.files') }}
            </h2>
            <button
              @click="selectFiles"
              class="w-full px-10 py-12 rounded-2xl border-2 border-dashed border-primary/30 hover:border-primary hover:bg-primary/5 transition-all text-center group"
            >
              <div class="text-5xl mb-4 group-hover:scale-110 transition-transform">📁</div>
              <div class="text-sm font-black text-content uppercase tracking-widest">
                {{ selectedFiles.length > 0
                  ? `已选择 ${selectedFiles.length} 个文件`
                  : appStore.t('integrity.select_files') }}
              </div>
            </button>

            <!-- 操作按钮 -->
            <div class="flex gap-4 mt-6">
              <button
                @click="calculateChecksums"
                :disabled="selectedFiles.length === 0 || isCalculating"
                class="flex-1 px-8 py-4 rounded-2xl bg-primary text-white font-black uppercase tracking-widest text-sm hover:bg-primary/90 disabled:opacity-80 disabled:cursor-not-allowed transition-all shadow-lg hover:shadow-xl disabled:shadow-none"
              >
                {{ isCalculating ? appStore.t('integrity.calculating') : appStore.t('integrity.calculate') }}
              </button>
              <button
                v-if="checksumResults.length > 0"
                @click="exportChecksumFile"
                class="px-8 py-4 rounded-2xl bg-input/30 border-2 border-subtle text-content font-black uppercase tracking-widest text-sm hover:border-primary transition-all"
              >
                {{ appStore.t('integrity.export') }}
              </button>
            </div>
          </section>

          <!-- 结果列表 -->
          <section v-if="checksumResults.length > 0" class="aero-card p-10">
            <h2 class="text-sm font-black text-content uppercase tracking-[0.3em] mb-6">
              {{ appStore.t('integrity.results') }}
            </h2>
            <!-- 添加滚动容器，限制高度 -->
            <div class="max-h-[400px] overflow-y-auto custom-scrollbar space-y-3">
              <div
                v-for="result in checksumResults"
                :key="result.path"
                class="p-6 rounded-2xl bg-input/30 border border-subtle hover:border-primary/50 transition-all"
              >
                <div class="flex items-start justify-between gap-4">
                  <div class="flex-1 min-w-0">
                    <!-- 使用 break-all 允许长文件名换行 -->
                    <div class="text-sm font-black text-content break-all uppercase tracking-widest">{{ result.fileName }}</div>
                    <div class="mt-3 flex items-start gap-3">
                      <span class="text-xs text-muted uppercase tracking-widest font-bold shrink-0">{{ result.algorithm }}:</span>
                      <!-- 校验和允许换行 -->
                      <code class="text-xs font-mono text-primary break-all">{{ result.checksum || '计算中...' }}</code>
                    </div>
                  </div>
                  <div class="flex items-center gap-3 shrink-0">
                    <span
                      v-if="result.status === 'success'"
                      class="text-green-500 text-2xl"
                    >✓</span>
                    <span
                      v-else-if="result.status === 'error'"
                      class="text-red-500 text-2xl"
                    >✗</span>
                    <button
                      v-if="result.status === 'success'"
                      @click="copyChecksum(result.checksum)"
                      class="px-4 py-2 rounded-xl bg-primary/10 hover:bg-primary/20 text-primary text-sm font-black uppercase tracking-widest transition-all"
                    >
                      {{ appStore.t('integrity.copy') }}
                    </button>
                  </div>
                </div>
                <div v-if="result.error" class="mt-3 text-xs text-red-500 uppercase tracking-tighter break-all">
                  {{ result.error }}
                </div>
              </div>
            </div>
          </section>
        </div>

        <!-- 验证模式 -->
        <div v-else-if="activeMode === 'verify'" class="space-y-8">
          <section class="aero-card p-10">
            <button
              @click="selectChecksumFile"
              :disabled="isCalculating"
              class="w-full px-10 py-16 rounded-2xl border-2 border-dashed border-primary/30 hover:border-primary hover:bg-primary/5 transition-all text-center group"
            >
              <div class="text-6xl mb-6 group-hover:scale-110 transition-transform">✓</div>
              <div class="text-base font-black text-content uppercase tracking-widest">
                {{ appStore.t('integrity.select_checksum') }}
              </div>
              <div class="text-xs text-muted mt-3 uppercase tracking-tighter">
                支持 .md5, .sha256, .sfv 格式
              </div>
            </button>
          </section>

          <!-- 验证结果 -->
          <section v-if="verifyResult" class="aero-card p-10 border-2"
            :class="verifyResult.valid
              ? 'bg-green-500/5 border-green-500'
              : 'bg-red-500/5 border-red-500'"
          >
            <div class="flex items-center gap-6">
              <span class="text-6xl shrink-0">{{ verifyResult.valid ? '✓' : '✗' }}</span>
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

        <div v-else class="min-w-0 space-y-6 overflow-x-hidden" data-testid="archive-diagnostic-panel">
          <section class="aero-card p-6 md:p-10 min-w-0 overflow-hidden">
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

          <section v-if="diagnosticReport" class="aero-card p-6 md:p-10 min-w-0 overflow-hidden" data-testid="diagnostic-report">
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

          <section v-if="repairResult" class="aero-card p-6 md:p-10 border border-green-500/30" data-testid="repair-result">
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
@media (max-width: 850px) { .diagnostic-metrics { grid-template-columns: repeat(2, minmax(0, 1fr)); } .archive-password { flex-basis: 100%; } }
</style>
