<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { open } from '@tauri-apps/api/dialog'
import { invoke } from '@tauri-apps/api/tauri'
import ExtractionRecoveryPanel from './ExtractionRecoveryPanel.vue'
import ExtractionOutputPreview from './ExtractionOutputPreview.vue'
import { useTaskStore } from '@/stores/task'
import { extractionRetryPolicy, validateExtractionRetry } from '@/utils/extractionRetry'
import { extractErrorMessage, generateId } from '@/utils'
import { useAppStore } from '@/stores/app'
import { parentDirectory } from '@/utils/resultLocation'
import ZipPasswordEncodingField from '@/components/ui/ZipPasswordEncodingField.vue'
import { supportsZipPasswordEncoding } from '@/utils/zipPasswordSource'
import { archiveSourceFormat } from '@/utils/archiveSource'

const props = defineProps<{ taskIds: string[]; suppliedPassword?: string; inline?: boolean }>()
const emit = defineEmits<{ close: []; created: [ids: string[]] }>()
const store = useTaskStore()
const appStore = useAppStore()
const rows = ref(props.taskIds.flatMap(id => {
  const task = store.tasks.find(item => item.id === id)
  if (!task) return []
  const policy = extractionRetryPolicy(task)
  const draft = store.tasks.find(item => item.id === task.retryDraftId)
  const needsPassword = task.passwordRequired && policy.category === 'unknown'
  return [{ id, name: task.name, reason: task.error || (task.status === 'cancelled' ? '原任务已取消，未完成解压。' : '原任务未提供具体原因'), policy,
    summary: task.status === 'cancelled' ? '已取消，可调整后创建新任务' : needsPassword ? '需要确认解压密码' : policy.guidance?.title || '解压未完成',
    nextStep: needsPassword ? '在下方填写正确密码，再添加到队列。' : policy.guidance?.steps[0],
    selected: !policy.blocked && !task.retryDraftId,
    source: draft?.sourceFiles[0] || task.sourceFiles[0] || '', output: draft?.outputPath || task.outputPath,
    // Batch corrections do not silently reuse the credential that just failed.
    // The single-row password action can explicitly hand off the user's input.
    password: props.taskIds.length === 1 ? props.suppliedPassword || '' : '',
    extractToSubfolder: task.extractToSubfolder ?? true,
    zipPasswordEncoding: draft ? draft.zipPasswordEncoding : task.zipPasswordEncoding,
    zpaqRetrieval:draft?.zpaqRetrieval||task.zpaqRetrieval,
    zpaqSnapshot: draft?.zpaqSnapshot || task.zpaqSnapshot, zpaqCatalogId: draft?.zpaqCatalogId || task.zpaqCatalogId, selectedEntries: draft?.selectedEntries || task.selectedEntries,
    readerSelection:draft?.readerSelection || task.readerSelection,librarySourceId:task.librarySourceId,libraryRevision:task.libraryRevision,libraryKeepStructure:task.libraryKeepStructure,libraryConflictPolicy:task.libraryConflictPolicy,
    originalSource: task.sourceFiles[0], sourceFormat: task.format,
    createdId: task.retryDraftId || '', error: '', checking: false, missing: [] as string[] }]
}))
const busy = ref(false)
const stopped = ref(false)
const pickerBusy = ref(false)
const pickerError = ref('')
const sharedOutput = ref('')
const newIds = ref<string[]>([])
let disposed = false
let activeDiagnosticId: string | null = null
const selectedCount = computed(() => rows.value.filter(row => row.selected && !row.policy.blocked && !row.createdId).length)
const recoveryStep = (row: typeof rows.value[number]) => row.policy.category === 'ambiguous-data'
  && (row.source === row.originalSource ? row.sourceFormat || archiveSourceFormat(row.source) : archiveSourceFormat(row.source))?.toLowerCase() === 'egg'
  ? '请核对密码及下方 EGG 密码编码后创建新任务；原失败记录保留。'
  : row.nextStep
const cancelChecks = () => {
  stopped.value = true
  if (activeDiagnosticId) void invoke('cancel_archive_diagnosis', { diagnosticId: activeDiagnosticId }).catch(() => {})
}
onUnmounted(() => { disposed = true; cancelChecks(); rows.value.forEach(row => { row.password = '' }) })

onMounted(async () => {
  for (const row of rows.value) {
    if (!['missing-volume', 'incomplete'].includes(row.policy.category || '')) continue
    const source = row.source
    try {
      const split = await invoke<{ missing_parts?: string[] } | null>('detect_split_archive', { path: source })
      if (!disposed && row.source === source) row.missing = split?.missing_parts?.map(path => path.split(/[\\/]/).pop() || path) || []
    } catch { /* The user can reselect a moved or unreadable source below. */ }
  }
})

async function openSourceFolder(row: typeof rows.value[number]) {
  try { await invoke('open_folder_in_explorer', { path: parentDirectory(row.source) }) }
  catch (error) { row.error = extractErrorMessage(error) }
}
function rescueSource(row: typeof rows.value[number]) {
  if (busy.value || pickerBusy.value || !row.source.trim()) return
  appStore.openArchiveForDiagnosis({ path: row.source.trim(), taskId: row.id, password: row.password, zpaqSnapshot: row.zpaqSnapshot, zipPasswordEncoding: row.zipPasswordEncoding, outputPath: row.output.trim(), extractToSubfolder: row.extractToSubfolder })
  row.password = ''
  close()
}

async function choosePath(directory: boolean, row?: typeof rows.value[number]) {
  pickerBusy.value = true
  pickerError.value = ''
  try {
    const selected = await open({ directory, multiple: false, title: directory ? '选择本次输出目录' : '选择归档或首个分卷' })
    if (disposed || typeof selected !== 'string') return
    if (row) { if (directory) row.output = selected; else { row.source = selected; row.missing = [] } }
    else sharedOutput.value = selected
  } catch (error) { pickerError.value = extractErrorMessage(error) }
  finally { pickerBusy.value = false }
}
function applyOutput() {
  for (const row of rows.value) if (row.selected && !row.createdId && !row.policy.blocked) row.output = sharedOutput.value
}

async function createDrafts() {
  if (busy.value || pickerBusy.value || !selectedCount.value) return
  busy.value = true
  stopped.value = false
  // Snapshot the user-confirmed choices. Inputs are disabled until checks settle.
  const chosen = rows.value.filter(row => row.selected && !row.createdId && !row.policy.blocked)
  try {
    for (const row of chosen) {
      if (disposed || stopped.value) break
      row.error = ''
      row.checking = true
      try {
        const parent = store.tasks.find(task => task.id === row.id)
        if (!parent || !['failed','cancelled'].includes(parent.status)) throw new Error('原失败任务已不在队列，请到历史任务查看。')
        if (parent.retryDraftId) { row.createdId = parent.retryDraftId; continue }
        const policy = extractionRetryPolicy(parent)
        if (policy.blocked) throw new Error(policy.blocked)
        const saved = parent.historySaveError
          ? await store.retryHistoryPersistence(parent.id) : await store.waitForHistoryPersistence(parent.id)
        if (!saved) throw new Error('原失败记录尚未保存成功，请先重试保存历史；本项未创建。')
        if (disposed || stopped.value) break
        row.missing = []
        activeDiagnosticId = generateId()
        const result = await validateExtractionRetry(row, policy.requireIntegrity, activeDiagnosticId, () => disposed || stopped.value)
        if (disposed || stopped.value) break
        // The original can be cleared or retried by another entry while IPC is pending.
        if (!store.tasks.includes(parent) || !['failed','cancelled'].includes(parent.status)) throw new Error('原任务状态已变化，请重新打开处理窗口。')
        if (parent.retryDraftId) { row.createdId = parent.retryDraftId; continue }
        const id = store.addTask({
          id: generateId(), name: row.zpaqRetrieval?parent.name:result.name, type: 'decompression', workloadKind: 'archive',
          retryOfTaskId: parent.id, sourceFiles: [result.path], outputPath: result.outputPath,
          configurationMode: 'individual', extractToSubfolder: row.extractToSubfolder,
          recycleSourceAfterExtract: false, password: row.password || undefined,
          zipPasswordEncoding: result.zipPasswordEncoding, format: result.format, zpaqSnapshot: row.zpaqSnapshot, zpaqCatalogId: row.zpaqCatalogId, selectedEntries: row.selectedEntries,
          zpaqRetrieval:row.zpaqRetrieval,
          readerSelection:row.readerSelection,librarySourceId:row.librarySourceId,libraryRevision:row.libraryRevision,libraryKeepStructure:row.libraryKeepStructure,libraryConflictPolicy:row.libraryConflictPolicy,
          ...(row.zpaqCatalogId ? {executionConfig:{schemaVersion:1,kind:'archive-extraction',outputDirectory:result.outputPath,extractToSubfolder:row.extractToSubfolder,zpaqSnapshot:row.zpaqSnapshot,zpaqCatalogId:row.zpaqCatalogId,selectedEntries:[...(row.selectedEntries || [])]}} : {}),
        })
        parent.retryDraftId = id
        row.source = result.path
        row.output = result.outputPath
        row.createdId = id
        row.password = ''
        newIds.value.push(id)
      } catch (error) {
        if (!disposed) row.error = extractErrorMessage(error) || '无法创建，请核对来源和目录。'
      } finally { row.checking = false; activeDiagnosticId = null }
    }
  } finally {
    busy.value = false
    if (props.inline && newIds.value.length) emit('created', [...newIds.value])
  }
}
function close() {
  if (busy.value || pickerBusy.value) return
  if (newIds.value.length) emit('created', [...newIds.value])
  emit('close')
}
</script>

<template>
  <ExtractionRecoveryPanel :inline="inline" :busy="busy || pickerBusy" @close="close">
    <div data-testid="failed-extraction-retry" :data-inline="!!inline" class="failed-extraction-retry-workspace space-y-3 min-w-0" :class="{ 'is-inline': inline }">
      <p class="text-xs leading-relaxed text-muted">{{ inline ? '修改需要调整的设置，检查后加入待处理队列；原配置和失败记录保留。' : '选择要重试的文件，修改后添加到队列。返回队列时由你开始解压，原失败记录保留。' }}</p>
      <fieldset v-if="rows.length > 1" :disabled="busy || pickerBusy" class="flex flex-wrap items-center gap-2 text-sm">
        <button class="retry-button" @click="choosePath(true)">选择共用输出目录</button>
        <span class="min-w-0 break-all flex-1">{{ sharedOutput || '仅应用到勾选的未创建项' }}</span>
        <button class="retry-button" :disabled="!sharedOutput || !selectedCount" @click="applyOutput">应用到勾选项</button>
      </fieldset>
      <p v-if="pickerError" role="alert" class="text-red-500 text-sm break-words">{{ pickerError }}</p>
      <article v-for="row in rows" :key="row.id" :data-retry-source-id="row.id" class="retry-item border border-subtle rounded-xl p-3 min-w-0 space-y-2">
        <label v-if="!inline" class="flex items-start gap-2 font-bold text-sm break-all">
          <input v-model="row.selected" type="checkbox" :disabled="busy || pickerBusy || !!row.policy.blocked || !!row.createdId" class="mt-1" />
          {{ row.name }}
        </label>
        <p v-if="!row.createdId" class="text-sm text-red-500 break-words">{{ row.summary }}</p>
        <details class="text-xs text-muted">
          <summary class="cursor-pointer w-fit py-1 hover:text-content">查看原始错误</summary>
          <p class="mt-1 max-h-32 overflow-y-auto break-all whitespace-pre-wrap rounded-lg bg-input p-2 select-text">{{ row.reason }}</p>
        </details>
        <p v-if="row.policy.blocked" class="text-sm text-amber-500">{{ row.policy.blocked }}</p>
        <template v-else-if="!row.createdId">
          <div v-if="['missing-volume', 'incomplete'].includes(row.policy.category || '')" class="rounded-lg bg-amber-500/5 border border-amber-500/20 p-3 space-y-2">
            <p class="text-xs text-muted">把缺失分卷补到原目录，保持文件名，再点击下方检查。</p>
            <p v-if="row.missing.length" data-testid="retry-missing-volumes" class="text-xs font-mono text-amber-500 break-words">缺少：{{ row.missing.join('、') }}</p>
            <button class="retry-button" :disabled="busy || pickerBusy" @click="openSourceFolder(row)"><i class="pi pi-folder-open mr-1"></i>打开分卷目录</button>
          </div>
          <button v-if="['damaged', 'ambiguous-data', 'unknown', 'missing-volume', 'incomplete'].includes(row.policy.category || '')" data-testid="retry-rescue" class="retry-button text-primary" :disabled="busy || pickerBusy" @click="rescueSource(row)"><i class="pi pi-wrench mr-1"></i>检查并尝试救援</button>
          <p v-if="row.policy.requireIntegrity" data-testid="retry-guidance" class="text-xs text-muted">{{ row.zpaqRetrieval?'重新检查原快照和所选文件，其他受影响内容不会混入。':'请重新选择下载完整的文件。' }}添加前会检查完整性，大文件可能需要一些时间。</p>
          <p v-else-if="recoveryStep(row)" data-testid="retry-guidance" class="text-xs text-muted">{{ recoveryStep(row) }}</p>
          <fieldset :disabled="busy || pickerBusy || !row.selected" class="retry-fields grid grid-cols-1 sm:grid-cols-2 gap-3 min-w-0">
            <div class="min-w-0 space-y-2">
              <label class="block text-xs">来源文件
                <input v-model="row.source" data-testid="retry-source" class="retry-input" />
              </label>
              <button class="retry-button" @click="choosePath(false, row)">重新选择来源</button>
            </div>
            <div class="min-w-0 space-y-2">
              <label class="block text-xs">本次输出目录
                <input v-model="row.output" data-testid="retry-output" class="retry-input" />
              </label>
              <button class="retry-button" @click="choosePath(true, row)">选择输出目录</button>
            </div>
            <label class="block text-xs">本次密码（留空时按当前密码设置尝试）
              <input v-model="row.password" type="password" autocomplete="off" data-testid="retry-password" class="retry-input" />
            </label>
            <ZipPasswordEncodingField v-if="supportsZipPasswordEncoding(row.source, row.source === row.originalSource ? row.sourceFormat : undefined)" v-model="row.zipPasswordEncoding" :source-path="row.source" :source-format="row.source === row.originalSource ? row.sourceFormat : undefined" :disabled="busy || pickerBusy || !row.selected" test-id="retry-zip-password-encoding" />
            <label v-if="!row.zpaqRetrieval" class="flex items-center gap-2 text-xs"><input v-model="row.extractToSubfolder" data-testid="retry-subfolder" type="checkbox" />解压到同名子目录</label>
            <p v-if="row.zpaqRetrieval" class="text-xs text-primary sm:col-span-2">保留所选文件与快照，另存独立结果目录，原包与已有结果保留。</p>
            <ExtractionOutputPreview v-else class="sm:col-span-2" :source="row.source.trim()" :output="row.output.trim()" :subfolder="row.extractToSubfolder" @change-output="choosePath(true, row)" />
          </fieldset>
        </template>
        <p v-if="row.checking" role="status" class="text-sm text-primary">正在检查来源和目录…</p>
        <p v-if="row.createdId" role="status" class="text-sm text-emerald-600">已创建新任务，不会重复创建。可返回队列查看。</p>
        <dl v-if="row.createdId" class="grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1 text-xs text-muted">
          <dt>来源</dt><dd class="break-all select-text">{{ row.source }}</dd>
          <dt>输出目录</dt><dd class="break-all select-text">{{ row.output }}</dd>
        </dl>
        <p v-if="row.error" role="alert" class="text-sm text-red-500 break-words">{{ row.error }}</p>
      </article>
    </div>
    <template #footer>
      <div class="flex flex-wrap items-center justify-end gap-2 w-full text-sm">
        <span v-if="newIds.length" role="status" class="mr-auto">本次已创建 {{ newIds.length }} 项</span>
        <button v-if="busy" class="retry-button" :disabled="stopped" @click="cancelChecks">{{ stopped ? '正在停止…' : '停止检查' }}</button>
        <template v-else>
          <button v-if="!inline" class="retry-button" :disabled="pickerBusy" @click="close">{{ newIds.length ? '返回队列确认并启动' : '关闭' }}</button>
          <button data-testid="create-retry-drafts" class="retry-button text-primary" :disabled="!selectedCount || pickerBusy" @click="createDrafts">{{ inline ? '检查并加入待处理队列' : `确认并创建 ${selectedCount} 项待执行任务` }}</button>
        </template>
      </div>
    </template>
  </ExtractionRecoveryPanel>
</template>

<style scoped>
.retry-input { display:block; width:100%; min-width:0; margin-top:.25rem; padding:.5rem .65rem; border:1px solid var(--border-subtle); border-radius:.5rem; background:var(--bg-input); color:var(--text-base); }
.retry-button { padding:.45rem .65rem; border:1px solid var(--border-subtle); border-radius:.5rem; background:var(--bg-input); font-size:.75rem; }
.retry-button:disabled, fieldset:disabled { opacity:.6; }
.retry-button:not(:disabled):hover { border-color:var(--dynamic-accent); }
.is-inline .retry-item { border:0; padding:0; }
.is-inline .retry-fields { grid-template-columns:minmax(0,1fr); }
.is-inline .retry-fields > * { grid-column:1; }
/* Keep background task notices behind this focused correction window.
   Errors from the correction itself are displayed beside the affected file. */
:global(body:has(.failed-extraction-retry-workspace[data-inline="false"]) .toast-stack) { z-index:250; }
</style>
