<script setup lang="ts">
import { computed, ref } from 'vue'
import EnhancedFileDropzone from '@/components/ui/EnhancedFileDropzone.vue'
import { useTauriCommands } from '@/composables/useTauriCommands'
import type { PdfInputAnalysisReport } from '@/types/pdf'
import {
  buildPdfConfigurationDraft,
  isPdfCandidate,
  type PdfOptimizationMode,
} from '@/utils/pdfOptimizationWorkspace'

interface SelectedFile { name: string, path: string, size: number, isDirectory: boolean }
interface PdfWorkspaceItem {
  path: string
  name: string
  status: 'analyzing' | 'password-required' | 'ready' | 'blocked' | 'failed'
  report: PdfInputAnalysisReport | null
  mode: PdfOptimizationMode
  password: string
  riskConfirmed: boolean
  frozen: boolean
  error: string
}

const commands = useTauriCommands()
const items = ref<PdfWorkspaceItem[]>([])
const selectionError = ref('')

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
    else item.status = report.analysisComplete && report.blockingReasons.length === 0 && report.hasDigitalSignature !== true
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
      path: file.path, name: file.name || fileName(file.path), status: 'analyzing', report: null,
      mode: 'lossless-organization', password: '', riskConfirmed: false, frozen: false, error: '',
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
</script>

<template>
  <section class="pdf-workspace" data-testid="pdf-compression-workspace">
    <header class="workspace-header">
      <div>
        <span class="eyebrow">PDF · D-02.2</span>
        <h2>分析真实结构，先锁定安全配置</h2>
        <p>当前仅执行只读分析并保存页面内配置草稿；D-03 执行尚未接入，不会压缩、生成文件或创建任务。</p>
      </div>
      <div class="frozen-count"><strong>{{ readyCount }}</strong><span>已锁定配置</span></div>
    </header>

    <div class="boundary-banner" role="status">
      <i class="pi pi-shield"></i>
      <span>默认输出为新文件，禁止覆盖源文件；文件是否变小取决于原始结构，不保证压缩率。</span>
    </div>

    <EnhancedFileDropzone
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
          <button type="button" class="icon-button" data-testid="pdf-remove" aria-label="移除 PDF" @click="items = items.filter(candidate => candidate !== item)"><i class="pi pi-times"></i></button>
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
              <button v-if="item.frozen" type="button" class="secondary-action" @click="item.frozen = false">解除锁定</button>
              <button v-else type="button" class="primary-action" data-testid="pdf-freeze-configuration" :disabled="!draftFor(item)?.canFreeze" @click="freeze(item)">锁定配置（不执行）</button>
            </div>
          </template>
        </template>
        <p v-if="item.error" class="analysis-error" role="alert">{{ item.error }}</p>
      </article>
    </div>
  </section>
</template>

<style scoped>
.pdf-workspace { box-sizing: border-box; width: 100%; max-width: 100%; min-height: 0; overflow-x: hidden; overflow-y: auto; padding: 1rem; color: var(--text-content); }
.pdf-workspace > *, .draft-list, .draft-card { box-sizing: border-box; max-width: 100%; min-width: 0; }
.workspace-header { display: flex; align-items: center; justify-content: space-between; gap: 1rem; margin-bottom: .8rem; }
.eyebrow { color: var(--dynamic-accent); font-size: .68rem; font-weight: 900; letter-spacing: .12em; }
h2 { margin: .25rem 0; font-size: 1.15rem; font-weight: 900; } .workspace-header p { color: var(--text-muted); font-size: .72rem; line-height: 1.55; }
.frozen-count { min-width: 7rem; border: 1px solid var(--border-subtle); border-radius: 1rem; padding: .65rem; text-align: center; background: var(--bg-input); }
.frozen-count strong { display: block; font-size: 1.1rem; } .frozen-count span { color: var(--text-muted); font-size: .65rem; }
.boundary-banner { display: flex; gap: .55rem; align-items: center; margin-bottom: .8rem; border: 1px solid rgb(34 197 94 / .25); border-radius: .8rem; padding: .65rem .8rem; background: rgb(34 197 94 / .08); font-size: .7rem; line-height: 1.45; }
.selection-error,.analysis-error { margin-top: .5rem; color: #fb7185; font-size: .7rem; font-weight: 750; }
.draft-list { display: grid; gap: .8rem; margin-top: .9rem; }.draft-card { width: 100%; border: 1px solid var(--border-subtle); border-radius: 1.1rem; padding: .9rem; background: var(--bg-card); }
.card-title { display: flex; align-items: center; gap: .65rem; }.file-icon { display: grid; place-items: center; width: 2.2rem; height: 2.2rem; border-radius: .7rem; color: #fb7185; background: rgb(244 63 94 / .1); }
.file-heading { min-width: 0; flex: 1; }.file-heading h3 { font-size: .82rem; font-weight: 900; }.file-heading p { overflow: hidden; color: var(--text-muted); font-size: .62rem; text-overflow: ellipsis; white-space: nowrap; }
.status { border-radius: 99px; padding: .25rem .5rem; font-size: .62rem; font-weight: 850; background: var(--bg-input); }.status.ready { color: #4ade80; }.status.blocked,.status.password-required { color: #fbbf24; }.status.failed { color: #fb7185; }
.icon-button { color: var(--text-muted); padding: .35rem; }.loading { padding: 1rem; color: var(--text-muted); font-size: .72rem; text-align: center; }
.fact-grid { display: grid; grid-template-columns: repeat(7,minmax(0,1fr)); gap: .45rem; margin-top: .8rem; }.fact-grid div { min-width: 0; border-radius: .7rem; padding: .55rem; background: var(--bg-input); }.fact-grid span,.output-preview span { display: block; color: var(--text-muted); font-size: .58rem; }.fact-grid strong { font-size: .67rem; overflow-wrap: anywhere; }
.password-panel { display: grid; grid-template-columns: 1fr minmax(9rem,16rem) auto; gap: .55rem; align-items: center; margin-top: .7rem; border: 1px solid rgb(245 158 11 / .3); border-radius: .8rem; padding: .7rem; background: rgb(245 158 11 / .08); }.password-panel strong { font-size: .7rem; }.password-panel p { color: var(--text-muted); font-size: .6rem; }.password-panel input { min-width: 0; border: 1px solid var(--border-subtle); border-radius: .6rem; padding: .55rem; background: var(--bg-input); font-size: .7rem; }.password-panel button,.primary-action,.secondary-action { border-radius: .65rem; padding: .55rem .75rem; font-size: .68rem; font-weight: 850; }.password-panel button,.primary-action { background: var(--dynamic-accent); color: white; }.password-panel button:disabled,.primary-action:disabled { cursor: not-allowed; opacity: .4; }
.mode-grid { display: grid; grid-template-columns: 1fr 1fr; gap: .55rem; margin-top: .7rem; }.mode-grid button { display: grid; gap: .2rem; border: 1px solid var(--border-subtle); border-radius: .8rem; padding: .7rem; text-align: left; background: var(--bg-input); }.mode-grid button.selected { border-color: var(--dynamic-accent); box-shadow: inset 0 0 0 1px var(--dynamic-accent); }.mode-grid span { width: max-content; color: var(--dynamic-accent); font-size: .57rem; font-weight: 900; }.mode-grid .risk-label { color: #fb7185; }.mode-grid strong { font-size: .74rem; }.mode-grid small { color: var(--text-muted); font-size: .61rem; line-height: 1.45; }
.risk-confirmation { display: flex; gap: .5rem; align-items: flex-start; margin-top: .65rem; color: #fbbf24; font-size: .66rem; line-height: 1.5; }.risk-confirmation input { margin-top: .12rem; }
.output-preview { margin-top: .65rem; border-radius: .75rem; padding: .65rem; background: var(--bg-input); }.output-preview strong { display: block; margin: .15rem 0; color: var(--text-content); font-size: .7rem; overflow-wrap: anywhere; }.output-preview small { color: var(--text-muted); font-size: .59rem; }
.signature-warning { display: flex; gap: .55rem; margin-top: .65rem; border: 1px solid rgb(244 63 94 / .3); border-radius: .75rem; padding: .65rem; color: #fb7185; background: rgb(244 63 94 / .08); }.signature-warning strong { font-size: .7rem; }.signature-warning p { margin-top: .1rem; font-size: .61rem; line-height: 1.4; }.blocking-list { margin: .45rem 0 0 1rem; color: #fb7185; font-size: .6rem; overflow-wrap: anywhere; }.card-actions { display: flex; justify-content: flex-end; margin-top: .7rem; }.secondary-action { border: 1px solid var(--border-subtle); background: var(--bg-input); }
@media (max-width: 900px) { .fact-grid { grid-template-columns: repeat(4,minmax(0,1fr)); }.password-panel { grid-template-columns: 1fr; } }
@media (max-width: 620px) { .pdf-workspace { padding: .75rem; }.workspace-header { align-items: flex-start; }.frozen-count { min-width: 5rem; }.fact-grid { grid-template-columns: repeat(2,minmax(0,1fr)); }.mode-grid { grid-template-columns: 1fr; }.status { display: none; } }
</style>
