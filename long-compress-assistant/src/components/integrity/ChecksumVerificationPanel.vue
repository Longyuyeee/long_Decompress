<script setup lang="ts">
import HelpHint from '@/components/ui/HelpHint.vue'
import { computed, onUnmounted, ref } from 'vue'
import ChecksumProgressCard from './ChecksumProgressCard.vue'
import { useChecksumOperation } from '@/composables/useChecksumOperation'
import { open } from '@tauri-apps/api/dialog'
import { useAppStore } from '@/stores/app'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { parentDirectory } from '@/utils/resultLocation'
import { isDecompressArchivePath, isPotentialSplitArchivePath, isPotentialSfxPath } from '@/utils/compressionFormat'
import { checksumPendingList, checksumProblem, checksumUnfinished, checksumStatusLabels, mergeChecksumVerification } from '@/utils/checksumVerification'
import { uniqueContextPaths } from '@/utils/contextActions'
import type { ChecksumVerificationReport } from '@/types/checksum'

const stagedManifests = ref<string[]>([]), stagedPath = ref('')
function stage(paths: string[]) {
  stagedManifests.value = uniqueContextPaths([...stagedManifests.value, ...paths])
  if (!stagedPath.value) stagedPath.value = stagedManifests.value[0] || ''
}
const emit = defineEmits<{ busy: [value: boolean]; diagnose: [path: string]; tasks: [] }>()
const app = useAppStore(), commands = useTauriCommands()
const { progress, stopping, run, stop } = useChecksumOperation()
const report = ref<ChecksumVerificationReport | null>(null), busy = ref(false), error = ref(''), notice = ref('')
const selected = ref<number[]>([]), expanded = ref<number[]>([]), filter = ref<'all' | 'problems' | 'unchecked'>('all')
let active = true
onUnmounted(() => { active = false })
const entries = computed(() => report.value?.entries || [])
const problems = computed(() => entries.value.filter(checksumProblem))
const pending = computed(() => checksumPendingList(entries.value))
const matched = computed(() => entries.value.filter(row => row.status === 'matched').length)
const unchecked = computed(() => entries.value.filter(row => checksumUnfinished(row)).length)
const visible = computed(() => entries.value.filter(row => filter.value === 'all' || (filter.value === 'problems' ? checksumProblem(row) : checksumUnfinished(row))))
const heading = computed(() => report.value?.valid ? '清单项目均匹配' : problems.value.length ? '还有文件需要处理' : unchecked.value ? '还有项目未检查' : '清单中没有可检查的文件')
const toggle = (list: number[], line: number) => list.includes(line) ? list.filter(value => value !== line) : [...list, line]
const archiveCandidate = (path: string) => isDecompressArchivePath(path) || isPotentialSplitArchivePath(path) || isPotentialSfxPath(path)

async function verify(path: string, lines?: number[]) {
  if (busy.value || (lines && !lines.length)) return
  busy.value = true; emit('busy', true); error.value = ''; notice.value = ''
  try {
    const previous = report.value
    const baseline = lines && previous ? { ...previous, entries: previous.entries.filter(row => !lines.includes(row.line)) } : null
    const accept = (next: ChecksumVerificationReport) => {
      if (active) report.value = lines ? mergeChecksumVerification(baseline, next) : next
    }
    const next = await run<ChecksumVerificationReport>('verify_checksum_file', {
      checksumPath: path, ...(lines ? { entryLines: lines, expectedManifestId: previous?.manifestId } : {}),
    }, event => {
      if (event.report) accept(event.report)
      if (active && event.entry && report.value) {
        const index = report.value.entries.findIndex(row => row.line === event.entry!.line)
        if (index >= 0) report.value.entries[index] = event.entry
      }
    })
    if (!active) return
    accept(next)
    const changed = !!lines && previous?.manifestId !== next.manifestId
    if (changed) { selected.value = []; expanded.value = []; filter.value = 'all'; notice.value = '校验清单已变化，旧结果已作废；请点击继续未完成项，检查新清单。' }
    else if (next.cancelled) notice.value = '已停止读取。完成项已保留，点击继续未完成项；中断的文件会从头读取。'
    else if (lines) notice.value = '所选项目已复查，其他项目保留上次结果，未重复读取。'
    else { selected.value = []; expanded.value = []; filter.value = 'all' }
  } catch (cause) {
    if (active) error.value = `本次未能完成检查：${String(cause)}。下方保留的是上次结果。`
  } finally { if (active) { busy.value = false; emit('busy', false) } }
}
async function choose() {
  try {
    const queued = import.meta.env.VITE_DESKTOP_E2E === '1'
      ? window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection() : undefined
    const file = queued !== undefined ? queued : await open({ multiple: false, title: '选择校验清单', filters: [{ name: '校验清单', extensions: ['md5', 'sha256', 'sfv'] }] })
    if (typeof file === 'string') await verify(file)
  } catch (cause) { error.value = String(cause) }
}
async function copy(value: string) {
  try { await navigator.clipboard.writeText(value); app.setSuccess('已复制到剪贴板') }
  catch { app.setError('复制失败，请重试') }
}
async function locate(path: string) {
  try { await commands.invoke('open_in_explorer', { path: parentDirectory(path) }) }
  catch (cause) { app.setError(`打开目录失败：${String(cause)}`) }
}
defineExpose({ verify, stage, hasWorkspace: () => !!report.value || stagedManifests.value.length > 0 })
</script>

<template>
  <div class="checksum-verification" data-testid="checksum-verification" :aria-busy="busy">
    <section v-if="stagedManifests.length" class="checksum-panel" data-testid="checksum-explorer-manifests">
      <h2 class="text-sm font-bold text-content">从资源管理器收到 {{ stagedManifests.length }} 份校验清单</h2>
      <span class="context-help"><HelpHint label="多份清单说明">选择一份后主动开始核对。其余清单保留在这里，打开入口不会自动读取文件。</HelpHint></span>
      <label class="checksum-staged-label">本次清单<select v-model="stagedPath" :disabled="busy" data-testid="checksum-staged-path"><option v-for="path in stagedManifests" :key="path" :value="path">{{ path.split(/[\\/]/).pop() }} · {{ path }}</option></select></label>
      <p class="checksum-note" :title="stagedPath" data-testid="checksum-staged-location">本次位置：{{ stagedPath }}</p>
      <div class="checksum-actions"><button :disabled="busy || !stagedPath" data-testid="checksum-staged-start" @click="verify(stagedPath)"><i class="pi pi-play"></i>开始核对这份清单</button></div>
    </section>
    <section class="checksum-panel">
      <button class="checksum-picker" :disabled="busy" data-testid="checksum-choose" @click="choose">
        <i class="pi pi-check-circle picker-icon"></i>
        <span><strong>{{ report ? '选择其他校验清单' : '选择校验清单' }}</strong><small>SFV · MD5 · SHA256　文件与清单放在同一目录</small></span>
        <i class="pi pi-folder-open"></i>
      </button>
      <ChecksumProgressCard v-if="busy" :progress="progress" :stopping="stopping" @stop="stop" />
      <p v-if="error" class="checksum-error" role="alert" data-testid="checksum-error">{{ error }}</p>
    </section>

    <section v-if="report" class="checksum-panel" data-testid="checksum-report">
      <header class="checksum-summary" :data-valid="report.valid && !error && !busy">
        <i :class="report.valid && !error && !busy ? 'pi pi-check-circle' : 'pi pi-info-circle'"></i>
        <div><h2 data-testid="checksum-heading">{{ busy ? '正在检查文件…' : error ? '本次检查未完成' : heading }}</h2><p class="checksum-path" :title="report.checksumPath">{{ report.checksumPath }}</p></div>
      </header>
      <div class="checksum-counts" data-testid="checksum-counts">
        <span><strong>{{ matched }}</strong> 已匹配</span><span><strong>{{ problems.length }}</strong> 待处理</span><span><strong>{{ unchecked }}</strong> 未检查</span>
      </div>
      <span class="context-help"><HelpHint label="校验结果说明">摘要匹配仅表示与这份清单一致。复查所选时，其他项目显示上次结果；如有改动请主动复查。</HelpHint></span>
      <p v-if="notice" class="checksum-notice" role="status" data-testid="checksum-notice">{{ notice }}</p>
      <div class="checksum-actions">
        <button v-if="unchecked" :disabled="busy" data-testid="checksum-continue" @click="verify(report.checksumPath, entries.filter(checksumUnfinished).map(row => row.line))"><i class="pi pi-play"></i>继续未完成项（{{ unchecked }}）</button>
        <button :disabled="busy || !problems.length" data-testid="checksum-recheck-problems" @click="verify(report.checksumPath, problems.map(row => row.line))"><i class="pi pi-refresh"></i>复查问题项</button>
        <button :disabled="busy || !selected.length" data-testid="checksum-recheck-selected" @click="verify(report.checksumPath, [...selected])">复查所选{{ selected.length ? `（${selected.length}）` : '' }}</button>
        <button :disabled="busy || !pending" data-testid="checksum-copy-pending" @click="copy(pending)"><i class="pi pi-copy"></i>复制待补列表</button>
        <button :disabled="busy" data-testid="checksum-recheck-all" @click="verify(report.checksumPath)">全部重新检查</button>
      </div>
      <p v-if="pending" class="checksum-note">待补列表包含缺失和内容不符的文件；无法读取的文件请先解除占用或恢复访问。</p>
      <div class="checksum-filters" aria-label="筛选校验结果">
        <button v-for="option in ([['all','全部'],['problems','待处理'],['unchecked','未完成']] as const)" :key="option[0]" :aria-pressed="filter === option[0]" @click="filter = option[0]">{{ option[1] }}</button>
      </div>
      <div v-if="!visible.length" class="checksum-empty">{{ entries.length ? '没有这类项目' : report.message }}</div>
      <article v-for="row in visible" :key="row.line" class="checksum-entry" :data-status="row.status" :data-line="row.line" data-testid="checksum-entry">
        <div class="checksum-entry-heading">
          <input type="checkbox" :checked="selected.includes(row.line)" :disabled="busy" :aria-label="`选择 ${row.fileName || `第 ${row.line} 行`}`" @change="selected = toggle(selected, row.line)" />
          <button class="checksum-row" data-testid="checksum-row" :aria-expanded="expanded.includes(row.line)" @click="expanded = toggle(expanded, row.line)">
            <span class="checksum-file"><strong>{{ row.fileName || `清单第 ${row.line} 行` }}</strong><small :title="row.reason">{{ row.reason }}</small></span>
            <span class="checksum-badge">{{ checksumStatusLabels[row.status] }}</span><i :class="expanded.includes(row.line) ? 'pi pi-chevron-up' : 'pi pi-chevron-down'"></i>
          </button>
        </div>
        <div v-if="expanded.includes(row.line)" class="checksum-details" data-testid="checksum-details">
          <p class="checksum-full-path">{{ row.path || '此清单行无法确定文件路径' }}</p>
          <p class="checksum-note checksum-reason">{{ row.reason }}</p>
          <p class="checksum-note">清单第 {{ row.line }} 行 · {{ row.algorithm?.toUpperCase() || '未知算法' }} · {{ row.checkedAt ? `检查于 ${new Date(row.checkedAt).toLocaleTimeString()}` : '尚未读取文件' }}</p>
          <div class="checksum-digests"><div><span>清单预期</span><code>{{ row.expectedChecksum || '无法识别' }}</code></div><div><span>实际摘要</span><code>{{ row.actualChecksum || '尚无可用摘要' }}</code></div></div>
          <div class="checksum-actions">
            <button :disabled="busy" :data-testid="`checksum-recheck-${row.line}`" @click="verify(report.checksumPath, [row.line])">仅复查此项</button>
            <button v-if="row.path" @click="copy(row.path)">复制路径</button>
            <button v-if="row.path" @click="locate(row.path)">打开所在目录</button>
            <button v-if="row.path && archiveCandidate(row.path) && row.status !== 'invalid_entry'" :disabled="busy" data-testid="checksum-diagnose" @click="emit('diagnose', row.path)">诊断此归档<i class="pi pi-arrow-right"></i></button>
          </div>
        </div>
      </article>
      <footer class="checksum-footer"><span>补齐来源后，可在解压中心通过失败任务纠正入口创建新任务。</span><button :disabled="busy" data-testid="checksum-go-tasks" @click="emit('tasks')">前往解压队列<i class="pi pi-arrow-right"></i></button></footer>
    </section>
  </div>
</template>

<style scoped>
.checksum-staged-label { display:grid; min-width:0; gap:.4rem; color:var(--text-muted); font-size:.7rem; }
.checksum-staged-label select { width:100%; min-width:0; padding:.65rem; border:1px solid var(--border-subtle); border-radius:.65rem; color:var(--text-content); background:var(--bg-input); text-overflow:ellipsis; }
.checksum-verification { display:grid; min-width:0; gap:.85rem; }
.checksum-panel { min-width:0; border:1px solid var(--border-subtle); border-radius:1rem; padding:.9rem; background:var(--bg-card); }
button { transition:background-color .16s ease,border-color .16s ease; }
button:focus-visible,input:focus-visible { outline:2px solid var(--dynamic-accent); outline-offset:3px; }
button:disabled { opacity:.45; cursor:not-allowed; }
.checksum-picker { display:flex; width:100%; min-width:0; align-items:center; gap:.8rem; padding:.85rem; border:1px dashed color-mix(in srgb,var(--dynamic-accent) 45%,var(--border-subtle)); border-radius:.8rem; background:color-mix(in srgb,var(--dynamic-accent) 4%,var(--bg-input)); text-align:left; color:var(--text-content); }
.checksum-picker span { display:grid; flex:1; min-width:0; gap:.2rem; }
.checksum-picker strong,.checksum-summary h2 { font-size:.82rem; font-weight:850; }
.checksum-picker small,.checksum-note,.checksum-path,.checksum-file small,.checksum-footer { color:var(--text-muted); font-size:.65rem; line-height:1.6; }
.picker-icon { color:var(--dynamic-accent); font-size:1.35rem; }
.checksum-summary { display:flex; align-items:flex-start; gap:.65rem; color:var(--dynamic-accent); }
.checksum-summary>i { margin-top:.15rem; font-size:1.15rem; }
.checksum-summary>div { min-width:0; }
.checksum-summary[data-valid="true"] { color:#059669; }
.checksum-path,.checksum-full-path { overflow-wrap:anywhere; }
.checksum-path { margin-top:.2rem; }
.checksum-counts { display:grid; grid-template-columns:repeat(3,minmax(0,1fr)); gap:.55rem; margin:.85rem 0 .5rem; }
.checksum-counts>span { min-width:0; border-radius:.7rem; padding:.7rem; background:var(--bg-input); color:var(--text-muted); font-size:.65rem; }
.checksum-counts strong { display:block; margin-bottom:.15rem; color:var(--text-content); font-size:1.1rem; }
.checksum-note { margin:.5rem 0; overflow-wrap:anywhere; }
.checksum-actions { display:flex; flex-wrap:wrap; align-items:center; gap:.45rem; margin-top:.65rem; }
.checksum-actions button,.checksum-footer button { display:inline-flex; align-items:center; gap:.35rem; border:1px solid var(--border-subtle); border-radius:.6rem; padding:.45rem .65rem; background:var(--bg-input); color:var(--text-content); font-size:.65rem; font-weight:750; }
.checksum-actions button:not(:disabled):hover,.checksum-footer button:hover { border-color:var(--dynamic-accent); color:var(--dynamic-accent); }
.checksum-filters { display:flex; flex-wrap:wrap; gap:.35rem; margin:.9rem 0 .6rem; }
.checksum-filters button { padding:.35rem .65rem; border:1px solid transparent; border-radius:999px; font-size:.65rem; color:var(--text-muted); }
.checksum-filters [aria-pressed="true"] { border-color:color-mix(in srgb,var(--dynamic-accent) 35%,transparent); background:color-mix(in srgb,var(--dynamic-accent) 9%,transparent); color:var(--dynamic-accent); }
.checksum-entry { min-width:0; margin-top:.45rem; border:1px solid var(--border-subtle); border-radius:.75rem; overflow:hidden; }
.checksum-entry-heading { display:flex; min-width:0; align-items:center; gap:.6rem; padding:0 .7rem; }
.checksum-entry-heading>input { flex:none; accent-color:var(--dynamic-accent); }
.checksum-row { display:grid; flex:1; min-width:0; grid-template-columns:minmax(0,1fr) auto .65rem; align-items:center; gap:.6rem; padding:.7rem 0; text-align:left; }
.checksum-row>i { font-size:.55rem; color:var(--text-muted); }
.checksum-file { display:grid; min-width:0; gap:.2rem; }
.checksum-file strong { overflow-wrap:anywhere; color:var(--text-content); font-size:.74rem; font-weight:750; }
.checksum-file small { overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.checksum-badge { flex:none; border-radius:999px; padding:.22rem .45rem; background:color-mix(in srgb,#d97706 10%,transparent); color:#d97706; font-size:.6rem; font-weight:800; white-space:nowrap; }
[data-status="matched"] .checksum-badge { color:#059669; background:color-mix(in srgb,#059669 10%,transparent); }
[data-status="not_checked"] .checksum-badge { color:var(--text-muted); background:var(--bg-input); }
[data-status="mismatch"] .checksum-badge,[data-status="unreadable"] .checksum-badge { color:#e65e58; background:color-mix(in srgb,#e65e58 10%,transparent); }
.checksum-details { min-width:0; padding:.8rem; border-top:1px solid var(--border-subtle); background:color-mix(in srgb,var(--bg-input) 45%,transparent); }
.checksum-full-path { color:var(--text-content); font-size:.7rem; line-height:1.6; }
.checksum-digests { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:.6rem; margin-top:.7rem; }
.checksum-digests>div { min-width:0; border:1px solid var(--border-subtle); border-radius:.6rem; padding:.6rem; background:var(--bg-card); }
.checksum-digests span { display:block; color:var(--text-muted); font-size:.6rem; }
.checksum-digests code { display:block; margin-top:.35rem; overflow-wrap:anywhere; color:var(--text-content); font-size:.65rem; line-height:1.65; }
.checksum-footer { display:flex; flex-wrap:wrap; align-items:center; justify-content:space-between; gap:.6rem; margin-top:1rem; padding-top:.8rem; border-top:1px solid var(--border-subtle); }
.checksum-footer>span { min-width:0; flex:1 1 13rem; }
.checksum-notice,.checksum-error { border-radius:.65rem; padding:.6rem .75rem; background:color-mix(in srgb,var(--dynamic-accent) 8%,transparent); color:var(--text-content); overflow-wrap:anywhere; font-size:.7rem; line-height:1.6; margin-top:.6rem; }
.checksum-error { background:color-mix(in srgb,#e65e58 8%,transparent); }
.checksum-empty { padding:1.2rem .5rem; color:var(--text-muted); font-size:.75rem; text-align:center; }
@media(max-width:620px) { .checksum-panel { padding:.7rem; } .checksum-picker { gap:.55rem; padding:.7rem; } .checksum-row { gap:.4rem; } .checksum-counts>span { padding:.55rem; } }
@media(prefers-reduced-motion:reduce) { button { transition:none; } }
</style>
