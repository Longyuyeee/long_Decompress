<script setup lang="ts">
import { computed, ref } from 'vue'
import { open } from '@tauri-apps/api/dialog'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { usePdfOptimizationBatch } from '@/composables/usePdfOptimizationBatch'
import { useAppStore } from '@/stores/app'
import { useTaskStore, type Task } from '@/stores/task'
import type { PdfInputAnalysisReport } from '@/types/pdf'
import {
  buildPdfConfigurationDraft,
  isPdfCandidate,
  type PdfOptimizationMode,
} from '@/utils/pdfOptimizationWorkspace'

interface SelectedFile { name: string, path: string, size: number, isDirectory: boolean }
interface PdfWorkspaceItem {
  id: string
  path: string
  name: string
  status: 'analyzing' | 'password-required' | 'ready' | 'blocked' | 'failed'
  report: PdfInputAnalysisReport | null
  mode: PdfOptimizationMode
  password: string
  riskConfirmed: boolean
  frozen: boolean
  allowLargerOutput: boolean
  taskId: string | null
  error: string
}

const appStore = useAppStore()
const taskStore = useTaskStore()
const commands = useTauriCommands()
const pdfBatch = usePdfOptimizationBatch()
const items = ref<PdfWorkspaceItem[]>([])
const selectionError = ref('')
const outputDirectory = ref('')
const isRunning = ref(false)

const formatBytes = (bytes: number) => {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 ** 2).toFixed(2)} MB`
}

const displayError = (error: unknown) => error instanceof Error ? error.message : String(error)
const fileName = (path: string) => path.split(/[\\/]/).filter(Boolean).pop() || path
const draftFor = (item: PdfWorkspaceItem) => item.report
  ? buildPdfConfigurationDraft(item.report, item.mode, item.riskConfirmed)
  : null

const analyze = async (item: PdfWorkspaceItem, password?: string) => {
  item.status = 'analyzing'
  item.error = ''
  item.frozen = false
  try {
    const report = await commands.analyzePdfInput({ path: item.path, password: password || null })
    item.report = report
    item.password = ''
    if (report.passwordState === 'required') item.status = 'password-required'
    else item.status = report.analysisComplete && report.blockingReasons.length === 0 && report.hasDigitalSignature !== true && !report.encrypted
      ? 'ready'
      : 'blocked'
  } catch (error) {
    item.password = ''
    item.status = item.report?.passwordState === 'required' ? 'password-required' : 'failed'
    item.error = displayError(error)
  }
}

const onFilesSelected = async (files: SelectedFile[]) => {
  selectionError.value = ''
  const rejected = files.filter(file => !isPdfCandidate(file))
  if (rejected.length) selectionError.value = `已拒绝 ${rejected.length} 个非 PDF 或目录输入。`
  for (const file of files.filter(isPdfCandidate)) {
    if (items.value.some(item => item.path.toLowerCase() === file.path.toLowerCase())) continue
    const item: PdfWorkspaceItem = {
      id: globalThis.crypto?.randomUUID?.() || `pdf-item-${Date.now()}-${items.value.length}`,
      path: file.path, name: file.name || fileName(file.path), status: 'analyzing', report: null,
      mode: 'lossless-organization', password: '', riskConfirmed: false, frozen: false,
      allowLargerOutput: false, taskId: null, error: '',
    }
    items.value.push(item)
    await analyze(items.value[items.value.length - 1])
  }
}

const chooseMode = (item: PdfWorkspaceItem, mode: PdfOptimizationMode) => {
  if (item.frozen) return
  item.mode = mode
  item.riskConfirmed = false
}

const freeze = (item: PdfWorkspaceItem) => {
  if (draftFor(item)?.canFreeze) item.frozen = true
}

const readyCount = computed(() => items.value.filter(item => item.frozen).length)
const taskFor = (item: PdfWorkspaceItem): Task | undefined => item.taskId
  ? taskStore.tasks.find(task => task.id === item.taskId)
  : undefined
const canRetry = (item: PdfWorkspaceItem) => {
  const task = taskFor(item)
  return !task || task.status === 'failed' || task.status === 'cancelled'
}
const runnableItems = computed(() => items.value.filter(item => item.frozen && canRetry(item)))
const canStart = computed(() => !isRunning.value && runnableItems.value.length > 0)

const chooseOutputDirectory = async () => {
  try {
    const queued = import.meta.env.VITE_DESKTOP_E2E === '1'
      ? window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection()
      : undefined
    const selected = queued === undefined
      ? await open({ directory: true, multiple: false, title: '选择 PDF 输出目录' })
      : queued
    if (selected && !Array.isArray(selected)) outputDirectory.value = selected
  } catch (error) {
    appStore.setError(`无法选择 PDF 输出目录：${String(error)}`)
  }
}

const startPdfOptimization = async () => {
  const sources = runnableItems.value
  if (!canStart.value) return
  isRunning.value = true
  try {
    const results = await pdfBatch.runPdfBatch(
      sources.map(item => ({
        id: item.id,
        name: item.name,
        path: item.path,
        mode: item.mode,
        confirmedLossyImageChanges: item.riskConfirmed,
        allowLargerOutput: item.allowLargerOutput,
      })),
      outputDirectory.value || null,
      appStore.settings.preserveMarkOfWeb,
      (itemId, taskId) => {
        const item = items.value.find(candidate => candidate.id === itemId)
        if (item) item.taskId = taskId
      },
    )
    const published = results.filter(result => result.status === 'published').length
    const failed = results.filter(result => result.status === 'failed').length
    const cancelled = results.filter(result => result.status === 'cancelled').length
    const summary = `PDF 处理结束：${published} 个完成，${failed} 个失败，${cancelled} 个取消`
    if (failed || cancelled) appStore.setError(summary)
    else appStore.setSuccess(summary)
  } catch (error) {
    appStore.setError(`PDF 批量处理失败：${String(error)}`)
  } finally {
    isRunning.value = false
  }
}

const openPublishedPdf = async (item: PdfWorkspaceItem) => {
  const path = taskFor(item)?.outputPath
  if (!path) return
  try {
    await commands.openPdfOutputWithDefaultApplication(path)
    appStore.setSuccess('已将 PDF 交给系统默认阅读器')
  } catch (error) {
    appStore.setError(`无法使用默认阅读器打开 PDF：${String(error)}`)
  }
}
</script>

<template>
  <section class="pdf-workspace" data-testid="pdf-compression-workspace">
    <header class="workspace-toolbar">
      <div class="header-actions">
        <div class="frozen-count"><strong>{{ readyCount }}</strong><span>已锁定配置</span></div>
        <button v-if="isRunning" type="button" class="danger-action" data-testid="pdf-cancel-batch" @click="pdfBatch.cancelPdfBatch()">取消处理</button>
        <button v-else type="button" class="primary-action" data-testid="pdf-start-batch" :disabled="!canStart" @click="startPdfOptimization">开始批量优化</button>
      </div>
    </header>

    <div class="output-directory">
      <div><span>输出目录</span><strong :title="outputDirectory">{{ outputDirectory || '与源文件同目录' }}</strong></div>
      <button type="button" :disabled="isRunning" data-testid="pdf-output-directory" @click="chooseOutputDirectory"><i class="pi pi-folder-open"></i>选择目录</button>
    </div>

    <EnhancedFileDropzone
      :compact="items.length > 0"
      accept="pdf"
      hint="添加需要分析的 PDF"
      sub-hint="只读调用已校验的 qpdf；支持加密 PDF 密码验证"
      picker-title="选择 PDF"
      @files-selected="onFilesSelected"
    />
    <p v-if="selectionError" class="selection-error">{{ selectionError }}</p>

    <div v-if="items.length" class="draft-list">
      <article v-for="item in items" :key="item.path" class="draft-card" data-testid="pdf-draft-card">
        <div class="card-title">
          <div class="file-icon"><i class="pi pi-file-pdf"></i></div>
          <div class="file-heading"><h3>{{ item.name }}</h3><p :title="item.path">{{ item.path }}</p></div>
          <span class="status" :class="item.status">{{ item.frozen ? '配置已锁定' : item.status === 'analyzing' ? '分析中' : item.status === 'password-required' ? '需要密码' : item.status === 'ready' ? '可配置' : item.status === 'blocked' ? '仅可分析' : '分析失败' }}</span>
          <button type="button" class="icon-button" data-testid="pdf-remove" aria-label="移除 PDF" :disabled="isRunning" @click="items = items.filter(candidate => candidate !== item)"><i class="pi pi-times"></i></button>
        </div>

        <div v-if="item.status === 'analyzing'" class="loading"><i class="pi pi-spin pi-spinner"></i> 正在读取真实 PDF 结构…</div>

        <template v-if="item.report">
          <div class="fact-grid">
            <div><span>页数</span><strong>{{ item.report.pageCount ?? '待解锁' }}</strong></div>
            <div><span>文件大小</span><strong>{{ formatBytes(item.report.inputBytes) }}</strong></div>
            <div><span>加密</span><strong>{{ item.report.encrypted ? (item.report.passwordState === 'accepted' ? '是 · 密码已验证' : '是 · 待验证') : '否' }}</strong></div>
            <div><span>数字签名</span><strong>{{ item.report.hasDigitalSignature == null ? '待解锁' : item.report.hasDigitalSignature ? `有（${item.report.signatureFieldNames.length}）` : '无' }}</strong></div>
            <div><span>表单字段</span><strong>{{ item.report.hasFormFields == null ? '待解锁' : item.report.hasFormFields ? item.report.formFieldNames.length : 0 }}</strong></div>
            <div><span>附件</span><strong>{{ item.report.hasAttachments == null ? '待解锁' : item.report.hasAttachments ? item.report.attachmentNames.length : 0 }}</strong></div>
            <div><span>书签/大纲</span><strong>{{ item.report.outlineCount ?? '待解锁' }}</strong></div>
          </div>

          <div v-if="item.status === 'password-required'" class="password-panel">
            <div><strong>需要正确密码才能完成结构分析</strong><p>密码只通过标准输入传给 qpdf，不写入命令参数或配置草稿。</p></div>
            <input v-model="item.password" data-testid="pdf-password-input" type="password" autocomplete="off" placeholder="输入 PDF 密码" @keyup.enter="item.password && analyze(item, item.password)" />
            <button type="button" data-testid="pdf-password-analyze" :disabled="!item.password" @click="analyze(item, item.password)">验证并继续分析</button>
          </div>

          <template v-if="item.report.analysisComplete">
            <div class="mode-grid" :class="{ locked: item.frozen }">
              <button type="button" data-testid="pdf-mode-lossless" :class="{ selected: item.mode === 'lossless-organization' }" :disabled="item.frozen" @click="chooseMode(item, 'lossless-organization')">
                <span>推荐</span><strong>无损整理</strong><small>整理对象流与通用压缩，不改变可见页面内容。</small>
              </button>
              <button type="button" data-testid="pdf-mode-image" :class="{ selected: item.mode === 'compatible-image-optimization' }" :disabled="item.frozen" @click="chooseMode(item, 'compatible-image-optimization')">
                <span class="risk-label">有损</span><strong>兼容图片优化</strong><small>可能重新编码合格图片，像素和编码数据可能不可逆变化。</small>
              </button>
            </div>

            <label v-if="item.mode === 'compatible-image-optimization'" class="risk-confirmation">
              <input v-model="item.riskConfirmed" data-testid="pdf-risk-confirmation" type="checkbox" :disabled="item.frozen" />
              <span>我已理解图片优化是有损操作，未来执行时可能改变图片像素与编码数据。</span>
            </label>

            <label class="larger-output-confirmation">
              <input v-model="item.allowLargerOutput" data-testid="pdf-allow-larger-output" type="checkbox" :disabled="item.frozen || isRunning" />
              <span><strong>仍保留比原文件更大的结果</strong>（默认关闭）。开启后只有结构验证全部通过才会发布，但可能增加占用空间。</span>
            </label>

            <div v-if="draftFor(item)" class="output-preview" data-testid="pdf-output-preview">
              <span>建议的新文件</span><strong>{{ draftFor(item)?.proposedOutput }}</strong>
              <small>源文件不会被覆盖；输出大小不保证小于输入。</small>
            </div>

            <div v-if="item.report.hasDigitalSignature" class="signature-warning">
              <i class="pi pi-ban"></i><div><strong>含数字签名：当前仅可分析</strong><p>任何重写都可能使现有签名失效，因此本阶段禁止锁定执行配置。</p></div>
            </div>
            <ul v-if="draftFor(item)?.blockingReasons.length" class="blocking-list">
              <li v-for="reason in draftFor(item)?.blockingReasons" :key="reason">{{ reason }}</li>
            </ul>

            <div class="card-actions">
              <div v-if="taskFor(item)" class="execution-result">
                <span>{{ taskFor(item)!.stage || taskFor(item)!.status }}</span>
                <strong v-if="taskFor(item)!.outputBytes">{{ formatBytes(taskFor(item)!.outputBytes || 0) }} · {{ taskFor(item)!.metrics?.media?.pageCount ?? item.report.pageCount }} 页</strong>
                <button v-if="taskFor(item)!.status === 'completed'" type="button" data-testid="pdf-open-default-app" @click="openPublishedPdf(item)">默认阅读器打开</button>
              </div>
              <button v-if="item.frozen" type="button" class="secondary-action" :disabled="isRunning" @click="item.frozen = false">解除锁定</button>
              <button v-else type="button" class="primary-action" data-testid="pdf-freeze-configuration" :disabled="!draftFor(item)?.canFreeze || isRunning" @click="freeze(item)">锁定执行配置</button>
            </div>
          </template>
        </template>
        <p v-if="item.error" class="analysis-error" role="alert">{{ item.error }}</p>
      </article>
    </div>
  </section>
</template>

<style scoped>
.pdf-workspace { box-sizing: border-box; display:flex; width: 100%; max-width: 100%; min-width:0; min-height: 0; flex:1; flex-direction:column; overflow: hidden; padding: .1rem; color: var(--text-content); }
.pdf-workspace > *, .draft-list, .draft-card { box-sizing: border-box; max-width: 100%; min-width: 0; }
.workspace-toolbar { display: flex; align-items: center; justify-content: flex-end; gap: 1rem; margin-bottom: .65rem; }
.frozen-count { min-width: 7rem; border: 1px solid var(--border-subtle); border-radius: 1rem; padding: .65rem; text-align: center; background: var(--bg-input); }
.frozen-count strong { display: block; font-size: 1.1rem; } .frozen-count span { color: var(--text-muted); font-size: .65rem; }
.header-actions { display: flex; align-items: center; gap: .5rem; }.danger-action { border-radius: .65rem; padding: .55rem .75rem; color: white; background: #ef4444; font-size: .68rem; font-weight: 850; }
.header-actions button,.output-directory button,.mode-grid button,.card-actions button { transition: transform .18s ease, box-shadow .18s ease, border-color .18s ease, background-color .18s ease; }
.header-actions button:not(:disabled):hover,.output-directory button:not(:disabled):hover,.card-actions button:not(:disabled):hover { transform: translateY(-1px); box-shadow: 0 8px 18px -14px rgb(0 0 0 / .55); }
.header-actions button:not(:disabled):active,.output-directory button:not(:disabled):active,.card-actions button:not(:disabled):active { transform: scale(.98); }
.output-directory { display: flex; align-items: center; justify-content: space-between; gap: .75rem; margin-bottom: .8rem; border: 1px solid var(--border-subtle); border-radius: .75rem; padding: .65rem .75rem; background: var(--bg-input); }.output-directory div { min-width: 0; }.output-directory span,.output-directory strong { display: block; }.output-directory span { color: var(--text-muted); font-size: .58rem; }.output-directory strong { overflow: hidden; margin-top: .1rem; font-size: .65rem; text-overflow: ellipsis; white-space: nowrap; }.output-directory button,.execution-result button { flex: 0 0 auto; border: 1px solid var(--border-subtle); border-radius: .55rem; padding: .4rem .55rem; font-size: .6rem; font-weight: 800; }
.selection-error,.analysis-error { margin-top: .5rem; color: #fb7185; font-size: .7rem; font-weight: 750; }
.draft-list { display: grid; min-height:0; align-content:start; gap: .8rem; margin-top: .9rem; overflow-x:hidden;overflow-y:auto;padding-right:.25rem }.draft-card { width: 100%; border: 1px solid var(--border-subtle); border-radius: 1.1rem; padding: .9rem; background: var(--bg-card); }
.card-title { display: flex; align-items: center; gap: .65rem; }.file-icon { display: grid; place-items: center; width: 2.2rem; height: 2.2rem; border-radius: .7rem; color: #fb7185; background: rgb(244 63 94 / .1); }
.file-heading { min-width: 0; flex: 1; }.file-heading h3 { font-size: .82rem; font-weight: 900; }.file-heading p { overflow: hidden; color: var(--text-muted); font-size: .62rem; text-overflow: ellipsis; white-space: nowrap; }
.status { border-radius: 99px; padding: .25rem .5rem; font-size: .62rem; font-weight: 850; background: var(--bg-input); }.status.ready { color: #4ade80; }.status.blocked,.status.password-required { color: #fbbf24; }.status.failed { color: #fb7185; }
.icon-button { color: var(--text-muted); padding: .35rem; }.loading { padding: 1rem; color: var(--text-muted); font-size: .72rem; text-align: center; }
.fact-grid { display: grid; grid-template-columns: repeat(7,minmax(0,1fr)); gap: .45rem; margin-top: .8rem; }.fact-grid div { min-width: 0; border-radius: .7rem; padding: .55rem; background: var(--bg-input); }.fact-grid span,.output-preview span { display: block; color: var(--text-muted); font-size: .58rem; }.fact-grid strong { font-size: .67rem; overflow-wrap: anywhere; }
.password-panel { display: grid; grid-template-columns: 1fr minmax(9rem,16rem) auto; gap: .55rem; align-items: center; margin-top: .7rem; border: 1px solid rgb(245 158 11 / .3); border-radius: .8rem; padding: .7rem; background: rgb(245 158 11 / .08); }.password-panel strong { font-size: .7rem; }.password-panel p { color: var(--text-muted); font-size: .6rem; }.password-panel input { min-width: 0; border: 1px solid var(--border-subtle); border-radius: .6rem; padding: .55rem; background: var(--bg-input); font-size: .7rem; }.password-panel button,.primary-action,.secondary-action { border-radius: .65rem; padding: .55rem .75rem; font-size: .68rem; font-weight: 850; }.password-panel button,.primary-action { background: var(--dynamic-accent); color: white; }.password-panel button:disabled,.primary-action:disabled { cursor: not-allowed; opacity: .4; }
.mode-grid { display: grid; grid-template-columns: 1fr 1fr; gap: .55rem; margin-top: .7rem; }.mode-grid button { display: grid; gap: .2rem; border: 1px solid var(--border-subtle); border-radius: .8rem; padding: .7rem; text-align: left; background: var(--bg-input); }.mode-grid button.selected { border-color: var(--dynamic-accent); box-shadow: inset 0 0 0 1px var(--dynamic-accent); }.mode-grid span { width: max-content; color: var(--dynamic-accent); font-size: .57rem; font-weight: 900; }.mode-grid .risk-label { color: #fb7185; }.mode-grid strong { font-size: .74rem; }.mode-grid small { color: var(--text-muted); font-size: .61rem; line-height: 1.45; }
.risk-confirmation { display: flex; gap: .5rem; align-items: flex-start; margin-top: .65rem; color: #fbbf24; font-size: .66rem; line-height: 1.5; }.risk-confirmation input { margin-top: .12rem; }
.larger-output-confirmation { display: flex; gap: .5rem; align-items: flex-start; margin-top: .65rem; border: 1px solid rgb(245 158 11 / .28); border-radius: .7rem; padding: .6rem; color: #fbbf24; background: rgb(245 158 11 / .07); font-size: .62rem; line-height: 1.45; }.larger-output-confirmation input { margin-top: .12rem; }.larger-output-confirmation strong { color: var(--text-content); }
.output-preview { margin-top: .65rem; border-radius: .75rem; padding: .65rem; background: var(--bg-input); }.output-preview strong { display: block; margin: .15rem 0; color: var(--text-content); font-size: .7rem; overflow-wrap: anywhere; }.output-preview small { color: var(--text-muted); font-size: .59rem; }
.signature-warning { display: flex; gap: .55rem; margin-top: .65rem; border: 1px solid rgb(244 63 94 / .3); border-radius: .75rem; padding: .65rem; color: #fb7185; background: rgb(244 63 94 / .08); }.signature-warning strong { font-size: .7rem; }.signature-warning p { margin-top: .1rem; font-size: .61rem; line-height: 1.4; }.blocking-list { margin: .45rem 0 0 1rem; color: #fb7185; font-size: .6rem; overflow-wrap: anywhere; }.card-actions { display: flex; align-items: center; justify-content: flex-end; gap: .5rem; margin-top: .7rem; }.secondary-action { border: 1px solid var(--border-subtle); background: var(--bg-input); }.execution-result { display: flex; min-width: 0; flex: 1; align-items: center; gap: .5rem; color: var(--text-muted); font-size: .6rem; }.execution-result strong { color: var(--text-content); }
@media (max-width: 900px) { .fact-grid { grid-template-columns: repeat(4,minmax(0,1fr)); }.password-panel { grid-template-columns: 1fr; } }
@media (max-width: 620px) { .pdf-workspace { padding: .1rem; }.workspace-toolbar { margin-bottom: .45rem; }.header-actions { justify-content: flex-end; }.frozen-count { min-width: 5rem; }.fact-grid { grid-template-columns: repeat(2,minmax(0,1fr)); }.mode-grid { grid-template-columns: 1fr; }.status { display: none; }.card-actions { align-items: stretch; flex-direction: column; }.execution-result { flex-wrap: wrap; } }
@media (prefers-reduced-motion: reduce) { .header-actions button,.output-directory button,.mode-grid button,.card-actions button { transition: none; } }
</style>
