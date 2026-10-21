<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { open, save } from '@tauri-apps/api/dialog'
import Modal from '@/components/ui/Modal.vue'
import ZipOrganizeDialog from './ZipOrganizeDialog.vue'
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import ArchiveOutputSummary from '@/components/compression/ArchiveOutputSummary.vue'
import TaskFailureHint from '@/components/tasks/TaskFailureHint.vue'
import TaskExecutionOverview from '@/components/tasks/TaskExecutionOverview.vue'
import { useZipEditStore } from '@/stores/zipEdit'
import { archiveInspectionMessageForDisplay } from '@/utils/taskFailure'
import type { ZipStructureRow } from '@/types/zipEdit'
import { formatFileSize } from '@/utils'
const props = defineProps<{ sourcePath: string; directory: string; selectedFile?: string; selectedPaths?: string[]; disabled?: boolean }>()
const emit = defineEmits<{ 'open-result': [path: string] }>()
const store = useZipEditStore(), review = ref(false), discardVisible = ref(false), organizer = ref(false)
const organizePaths = ref<string[]>([]), organizeMode = ref<'rename'|'delete'>('rename'), reviewQuery = ref(''), reviewKind = ref('all')
const session = computed(() => store.sessionFor(props.sourcePath))
const level = computed({ get: () => String(session.value?.level ?? 6), set: value => { if (session.value) session.value.level = Number(value) } })
const task = computed(() => store.taskFor(session.value))
const resolved = computed(() => session.value ? store.plan(session.value) : { rows: [], structureRows: [], changes: [], originalEdits: [], nodes: [], unresolved: 0, changeCount: 0, hasChanges: false })
const locked = computed(() => props.disabled || session.value?.busy || store.opening || store.preparing)
const reviewRows = computed(() => [
  ...resolved.value.rows.map(input => ({ id: input.draft.id, order: input.draft.order ?? 0, input, structure: undefined as typeof resolved.value.structureRows[number] | undefined })),
  ...resolved.value.structureRows.map(structure => ({ id: structure.operation.id, order: structure.operation.order, input: undefined as typeof resolved.value.rows[number] | undefined, structure })),
].sort((a,b) => a.order-b.order).filter(item => {
  const kind = item.structure?.operation.kind || 'input', query=reviewQuery.value.trim().toLowerCase()
  return (reviewKind.value==='all'||kind===reviewKind.value) && (!query||(item.input ? item.input.target+' '+item.input.draft.sourcePath : item.structure!.affected.map(row=>row.before+' '+row.after).join(' ')).toLowerCase().includes(query))
}))
const displayedRows = computed(() => reviewRows.value.slice((page.value - 1) * 100, page.value * 100))
const page = ref(1), pageCount = computed(() => Math.max(1, Math.ceil(reviewRows.value.length / 100)))
watch([reviewQuery,reviewKind],() => { page.value=1 })
watch(pageCount, count => { page.value = Math.min(page.value, count) })
watch(() => props.sourcePath, () => { review.value = false; organizer.value = false; discardVisible.value = false; page.value = 1 })
const queuedSelection = () => import.meta.env.VITE_DESKTOP_E2E === '1' ? window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection() : undefined
async function chooseInputs(folder = false, replacement?: string, targetDirectory?: string) {
  if (locked.value) return
  const source = props.sourcePath, directory = targetDirectory ?? props.directory
  if (!await store.begin(source)) return
  try {
    const queued = queuedSelection()
    const picked = queued === undefined ? await open({ directory: folder, multiple: !replacement, title: replacement ? '选择替换此条目的文件' : folder ? '添加文件夹到当前包内目录' : '添加文件到当前包内目录' }) : queued
    if (props.sourcePath !== source) return
    const paths = typeof picked === 'string' ? [picked] : Array.isArray(picked) ? picked : []
    if (paths.length) { await store.add(source, paths, directory, replacement, targetDirectory !== undefined); review.value = true }
  } catch (error) { if (session.value) session.value.error = String(error) }
}
async function addDropped(paths: string[]) {
  if (locked.value || !session.value?.editing) return
  await store.add(props.sourcePath, paths, props.directory); review.value = true
}
async function chooseOutput() {
  const current = session.value; if (!current || current.busy) return
  try { const queued = queuedSelection(); const path = queued === undefined ? await save({ title: '另存编辑后的 ZIP', defaultPath: current.outputPath, filters: [{ name: 'ZIP 压缩包', extensions: ['zip'] }] }) : queued
    if (typeof path === 'string' && session.value === current) current.outputPath = path
  } catch (error) { current.error = String(error) }
}
function discard() {
  if (session.value && (session.value.drafts.length || session.value.operations.length)) discardVisible.value = true
  else store.discard(props.sourcePath)
}
const affectedRows = (row: ZipStructureRow) => row.affected.filter(item => !reviewQuery.value.trim() || (item.before+' '+item.after).toLowerCase().includes(reviewQuery.value.trim().toLowerCase())).slice(0,100)
const labels = { add: '新增', replace: '替换', merge: '合并目录', skip: '跳过', pending: '需要选择', superseded: '由后续项替换' }
const phase = computed(() => task.value?.status === 'completed' ? '另存完成' : task.value?.stage === 'Pre-checking' ? '读取原包' : task.value?.stage === 'Verifying' ? '确认新包内容' : task.value?.stage === 'Publishing' ? '保存新包' : '写入 ZIP 内容')
async function organize(paths: string[] = [], mode: 'rename'|'delete' = 'rename') {
  if (locked.value || !await store.begin(props.sourcePath)) return
  review.value=false; organizePaths.value=paths; organizeMode.value=mode; organizer.value=true
}
defineExpose({ organize, replace: (path: string) => chooseInputs(false, path), addDropped, addFiles: () => chooseInputs(false), addFolder: () => chooseInputs(true) })
</script>
<template>
  <section class="zip-edit-bar shrink-0 min-w-0 rounded-xl border border-primary/20 bg-primary/5 p-3" data-testid="zip-edit-bar">
    <div class="flex min-w-0 flex-wrap items-center gap-2">
      <div class="mr-auto min-w-0"><strong class="text-sm text-content"><i class="pi pi-file-edit mr-2 text-primary" aria-hidden="true"></i>{{ session?.editing ? 'ZIP 内容编辑' : task?.status === 'completed' ? '新 ZIP 已保存' : '整理 ZIP 内容，另存新包' }}</strong><p v-if="session?.editing" class="mt-1 break-words text-xs text-muted [overflow-wrap:anywhere]">添加到：{{ directory || '包内根目录' }} · 待保存 {{ resolved.changeCount }} 项{{ resolved.unresolved ? ` · ${resolved.unresolved} 项待处理` : '' }} · 原包保留</p></div>
      <template v-if="session?.editing">
        <button class="edit-button" data-testid="zip-edit-add-files" :disabled="locked" @click="chooseInputs(false)">添加文件</button>
        <button class="edit-button" data-testid="zip-edit-add-folder" :disabled="locked" @click="chooseInputs(true)">添加文件夹</button>
        <button v-if="selectedFile" class="edit-button" data-testid="zip-edit-replace-selected" :disabled="locked" @click="chooseInputs(false, selectedFile)">替换所选文件</button>
        <button class="edit-button text-primary" data-testid="zip-edit-organize" :disabled="locked" @click="organize(selectedPaths || [])">整理{{ selectedPaths?.length ? `所选（${selectedPaths.length}）` : '内容' }}</button>
        <button class="edit-button bg-primary text-white" data-testid="zip-edit-review" @click="review = true">{{ session.busy ? '查看进度' : task?.status === 'failed' ? '调整并继续' : '复核并另存' }}{{ resolved.changeCount ? `（${resolved.changeCount}）` : '' }}</button>
        <button class="edit-button text-muted" :disabled="locked" @click="discard">取消编辑</button>
      </template>
      <template v-else>
        <button v-if="task?.archiveOutput" class="edit-button" data-testid="zip-edit-open-result" @click="emit('open-result', task.archiveOutput.primaryPath)">浏览新包</button>
        <button class="edit-button text-primary" data-testid="zip-edit-begin" :disabled="locked" @click="store.begin(sourcePath)">{{ store.opening ? '读取编辑信息…' : '编辑内容' }}</button>
        <button v-if="task?.archiveOutput" class="edit-button" @click="review = true">查看结果</button>
      </template>
    </div>
    <p v-if="session?.editing" class="mt-2 text-xs leading-relaxed text-muted">下方浏览原包；“整理内容”查看待保存结构，可删除、重命名或批量移动。另存后再浏览新包。</p>
    <p v-if="store.openingError?.source === sourcePath" role="alert" class="mt-2 break-words text-xs leading-relaxed text-amber-600 [overflow-wrap:anywhere]">{{ store.openingError.message }}</p>
    <p v-if="session?.error" role="alert" class="mt-2 break-words text-xs text-red-500 [overflow-wrap:anywhere]">{{ session.error }}</p>
    <p v-else-if="session?.editing && task?.status === 'failed'" class="mt-2 text-xs text-amber-600">保存未完成，修改清单仍保留。可打开“调整并继续”处理原因后另存。</p>
    <p v-else-if="session?.editing && task?.status === 'cancelled'" class="mt-2 text-xs text-muted">已停止，原包与修改清单保留，确认后可重新另存。</p>
  </section>
  <ZipOrganizeDialog v-model:visible="organizer" :source-path="sourcePath" :initial-paths="organizePaths" :initial-mode="organizeMode" @review="review=true" @add="(directory,folder) => { organizer=false; chooseInputs(folder,undefined,directory) }" />
  <Modal v-model:visible="review" title="复核修改，另存新 ZIP" icon="pi pi-file-edit" size="xl" cancel-text="关闭编辑清单">
    <div v-if="session" class="grid min-w-0 gap-4" :class="session.editing ? 'lg:grid-cols-2' : ''" data-testid="zip-edit-review-content">
      <section v-if="session.editing" class="min-w-0 space-y-3" data-testid="zip-edit-change-list">
        <div class="flex flex-wrap items-center gap-2"><strong class="mr-auto text-sm">待保存修改 · {{ resolved.changeCount }} 项</strong><span v-if="resolved.unresolved" class="text-xs text-amber-600">{{ resolved.unresolved }} 项需处理</span></div>
        <p class="break-words text-xs leading-relaxed text-muted [overflow-wrap:anywhere]">原包：{{ sourcePath }}。按顺序应用以下修改；未修改内容保留原方法与条目时间，同名文件由你决定。</p>
        <div class="flex min-w-0 flex-wrap gap-2"><input v-model="reviewQuery" class="edit-input min-w-0 flex-1" placeholder="搜索修改前后路径或来源" data-testid="zip-edit-review-search" /><select v-model="reviewKind" class="edit-input" aria-label="筛选修改类型"><option value="all">全部修改</option><option value="input">添加 / 替换</option><option value="rename">改名 / 移动</option><option value="delete">删除</option></select><button class="edit-button text-primary" :disabled="locked" data-testid="zip-edit-structure-preview" @click="organize()">查看整理结构</button></div>
        <p v-if="!reviewRows.length" class="rounded-xl border border-dashed border-subtle p-4 text-xs text-muted">没有匹配的修改。可以调整筛选，或回到浏览/整理窗口继续编辑。</p>
        <ol class="max-h-80 space-y-2 overflow-y-auto custom-scrollbar">
          <li v-for="entry in displayedRows" :key="entry.id" class="min-w-0 space-y-2 rounded-xl border border-subtle bg-input/30 p-3" data-testid="zip-edit-change" :data-edit-id="entry.id">
            <template v-if="entry.input">
            <div class="flex min-w-0 items-start gap-2"><span class="shrink-0 rounded-md bg-primary/10 px-2 py-1 text-xs text-primary">{{ labels[entry.input.status] }}</span><strong class="min-w-0 flex-1 break-words text-xs leading-relaxed [overflow-wrap:anywhere]">{{ entry.input.target }}</strong><button class="shrink-0 text-xs text-muted hover:text-red-500" :disabled="session.busy" data-testid="zip-edit-undo" @click="store.remove(sourcePath, entry.input.draft.id)">撤销</button></div>
            <p class="select-text break-words text-xs leading-relaxed text-muted [overflow-wrap:anywhere]">{{ entry.input.draft.sourcePath }} · {{ entry.input.draft.isDir ? '目录' : formatFileSize(entry.input.draft.size) }}</p>
            <p v-if="entry.input.issue" class="text-xs text-amber-600">{{ entry.input.issue }}</p>
            <div v-if="entry.input.collision" class="flex flex-wrap gap-1.5">
              <button v-if="!entry.input.draft.isDir && entry.input.collision === 'file'" class="edit-choice" :aria-pressed="entry.input.draft.choice === 'replace'" :disabled="session.busy" data-testid="zip-edit-choice-replace" @click="store.choose(sourcePath, entry.input.draft.id, 'replace')">替换</button>
              <button class="edit-choice" :aria-pressed="entry.input.draft.choice === 'skip'" :disabled="session.busy" data-testid="zip-edit-choice-skip" @click="store.choose(sourcePath, entry.input.draft.id, 'skip')">跳过</button>
              <button class="edit-choice" :aria-pressed="entry.input.draft.choice === 'keep-both'" :disabled="session.busy" data-testid="zip-edit-choice-keep-both" @click="store.choose(sourcePath, entry.input.draft.id, 'keep-both')">保留两份</button>
            </div>
            </template>
            <template v-else-if="entry.structure">
              <div class="flex min-w-0 flex-wrap items-center gap-2"><span class="rounded-md bg-primary/10 px-2 py-1 text-xs text-primary">{{ entry.structure.operation.kind==='delete'?'删除':'改名 / 移动' }}</span><span class="mr-auto text-xs text-muted">影响 {{ entry.structure.affected.length }} 项</span><button class="text-xs text-muted hover:text-red-500" :disabled="session.busy" data-testid="zip-edit-undo-operation" @click="store.undoOperation(sourcePath,entry.id)">撤销</button></div>
              <p v-if="entry.structure.issue" class="break-words text-xs text-amber-600 [overflow-wrap:anywhere]">{{ entry.structure.issue }}</p>
              <details :open="entry.structure.affected.length<=3" class="min-w-0"><summary class="cursor-pointer text-xs text-muted">修改前后路径（{{ entry.structure.affected.length }}）</summary><ul class="mt-2 max-h-48 space-y-2 overflow-y-auto custom-scrollbar"><li v-for="(item,index) in affectedRows(entry.structure)" :key="index" class="text-xs leading-relaxed"><span class="block break-words text-muted [overflow-wrap:anywhere]">{{ item.before }}</span><span class="block break-words text-primary [overflow-wrap:anywhere]">→ {{ item.after??'从新包移除' }}</span></li></ul><p v-if="entry.structure.affected.length>100" class="mt-2 text-xs text-muted">最多显示 100 个匹配子项，可通过上方搜索定位其他路径；实际处理全部 {{ entry.structure.affected.length }} 项。</p></details>
            </template>
          </li>
        </ol>
        <div v-if="pageCount > 1" class="flex flex-wrap justify-between gap-2 text-xs"><button :disabled="page <= 1" @click="page--">上一页</button><span>{{ page }} / {{ pageCount }}</span><button :disabled="page >= pageCount" @click="page++">下一页</button></div>
      </section>
      <section class="min-w-0 space-y-3" data-testid="zip-edit-save-panel">
        <p v-if="!session.editing" class="break-words text-xs leading-relaxed text-muted [overflow-wrap:anywhere]">已生成新 ZIP，原包保留：{{ sourcePath }}</p>
        <fieldset v-if="session.editing" :disabled="session.busy" class="min-w-0 space-y-3 rounded-xl border border-subtle bg-input/20 p-3">
          <label class="block text-xs text-muted">新 ZIP 保存位置<div class="mt-2 flex min-w-0 gap-2"><input v-model="session.outputPath" data-testid="zip-edit-output" class="edit-input min-w-0 flex-1" /><button class="edit-button" data-testid="zip-edit-choose-output" @click="chooseOutput">另存为</button></div></label>
          <div class="grid min-w-0 gap-3 sm:grid-cols-2"><label class="min-w-0 text-xs text-muted">新增/替换方法<ThemedSelect v-model="session.method" :options="[{value:'deflate',label:'Deflate · 兼容 ZIP'},{value:'store',label:'Store · 仅存储'}]" test-id="zip-edit-method" floating /></label><label class="min-w-0 text-xs text-muted">压缩强度<ThemedSelect v-model="level" :disabled="session.method === 'store'" :options="Array.from({length:10},(_,n)=>({value:String(n),label:String(n)}))" test-id="zip-edit-level" floating /></label></div>
          <p class="text-xs leading-relaxed text-muted">另存独立 ZIP，不加密、不分卷；原包保留，未修改内容沿原方法复制。修改只在点击下方“另存新 ZIP”后生效。</p>
        </fieldset>
        <p v-if="session.error" role="alert" class="break-words rounded-xl bg-red-500/5 p-3 text-xs text-red-500 [overflow-wrap:anywhere]">{{ session.error }}</p>
        <p v-if="task?.status === 'failed'" role="alert" class="break-words rounded-xl bg-red-500/5 p-3 text-xs text-red-500 [overflow-wrap:anywhere]">{{ archiveInspectionMessageForDisplay(task.error || '') }}</p>
        <TaskFailureHint :task="task"><button class="edit-button" :disabled="locked" data-testid="zip-edit-refresh" @click="store.refresh(sourcePath)">重新读取原包，保留修改清单</button></TaskFailureHint>
        <p v-if="task?.status === 'cancelled'" class="rounded-xl bg-input/30 p-3 text-xs text-muted">已停止，修改清单保留。确认配置后可以重新另存。</p>
        <TaskExecutionOverview v-if="task" :stage="phase" :status="task.status" :progress="task.progress" :current-file="task.currentFile" />
        <ArchiveOutputSummary v-if="task?.status === 'completed' && task.archiveOutput" :output="task.archiveOutput" @checksum="review = false" />
        <button v-if="task?.status === 'completed' && task.archiveOutput" class="edit-button text-primary" data-testid="zip-edit-browse-result" @click="review = false; emit('open-result', task.archiveOutput.primaryPath)">浏览新 ZIP，继续预览或提取</button>
        <details v-if="task?.logs.length" class="min-w-0 text-xs text-muted"><summary class="cursor-pointer">执行日志（{{ task.logs.length }}）</summary><ol class="mt-2 max-h-40 space-y-2 overflow-y-auto custom-scrollbar"><li v-for="(log,index) in task.logs" :key="index" class="break-words leading-relaxed [overflow-wrap:anywhere]">{{ log.message }}</li></ol></details>
      </section>
    </div>
    <template #footer>
      <button class="edit-button" @click="review = false">{{ session?.busy ? '后台继续' : '返回浏览' }}</button>
      <button v-if="session?.busy" class="edit-button text-red-500" data-testid="zip-edit-stop" :disabled="task?.status === 'cancelling'" @click="store.cancel(sourcePath)">{{ task?.status === 'cancelling' ? '正在停止…' : '停止另存' }}</button>
      <button v-else-if="session?.editing" class="edit-button bg-primary text-white" data-testid="zip-edit-save" :disabled="!resolved.hasChanges || !!resolved.unresolved || locked" @click="store.save(sourcePath)">另存新 ZIP（{{ resolved.changeCount }}）</button>
    </template>
  </Modal>
  <Modal v-model:visible="discardVisible" title="放弃本次修改？" description="原包和已保存的新包不会改变" cancel-text="继续编辑" confirm-text="放弃修改" @confirm="store.discard(sourcePath); discardVisible = false">
    <p class="text-sm leading-relaxed text-muted">本次 {{ (session?.drafts.length || 0) + (session?.operations.length || 0) }} 项待保存修改将被撤销。</p>
  </Modal>
</template>
<style scoped>
.edit-button,.edit-choice { display:inline-flex; align-items:center; justify-content:center; gap:.4rem; min-height:2rem; border:1px solid var(--border-subtle); border-radius:.65rem; padding:.45rem .7rem; font-size:.75rem; transition:background .15s,color .15s; }
.edit-choice { min-height:1.7rem; padding:.3rem .6rem; }
.edit-choice[aria-pressed=true] { color:var(--dynamic-accent); border-color:var(--dynamic-accent); background:color-mix(in srgb,var(--dynamic-accent) 10%,transparent); }
.edit-input { border:1px solid var(--border-subtle); border-radius:.65rem; background:var(--bg-input); padding:.55rem .65rem; font-size:.75rem; }
button:disabled { opacity:.45; cursor:default; }
button:focus-visible,input:focus-visible { outline:2px solid var(--dynamic-accent); outline-offset:2px; }
</style>
