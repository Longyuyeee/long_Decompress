<script setup lang="ts">
import TaskScopeBar from '@/components/tasks/TaskScopeBar.vue'
import { matchesTaskScope, type TaskScope } from '@/utils/taskScope'
import TaskFailureHint from '@/components/tasks/TaskFailureHint.vue'
import { computed, ref, nextTick, watch } from 'vue'
import ZpaqUpdateFields from './ZpaqUpdateFields.vue'
import ZpaqSnapshotPicker from '@/components/archive/ZpaqSnapshotPicker.vue'
import { useTauriCommands } from '@/composables/useTauriCommands'
import { snapshotDate, type ZpaqHistory, type ZpaqSnapshot } from '@/types/zpaqSnapshot'
import { open, save } from '@tauri-apps/api/dialog'
import Modal from '@/components/ui/Modal.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
import ArchiveOutputSummary from './ArchiveOutputSummary.vue'
import ConversionSettingsFields from './ConversionSettingsFields.vue'
import ConversionBatchConfig from './ConversionBatchConfig.vue'
import BatchResultActions from '@/components/tasks/BatchResultActions.vue'
import { useArchiveConversionStore } from '@/stores/archiveConversion'
import { resultLocation } from '@/utils/resultLocation'
import { archiveInspectionMessageForDisplay } from '@/utils/taskFailure'
import { formatProgressPercent } from '@/utils/progress'
const store = useArchiveConversionStore()
const commands = useTauriCommands()
const scope = ref<TaskScope>('all')
const statuses = computed(() => store.items.map(item => store.taskFor(item)?.status))
const visibleItems = computed(() => store.items.filter(item => matchesTaskScope(scope.value, store.taskFor(item)?.status)))
function changeScope(value: TaskScope) {
  if (scope.value !== value) store.pending.forEach(item => item.selected = false)
  scope.value = value
}
watch(() => store.visible, visible => { if (visible) scope.value = 'all' })
watch(() => store.pending.map(item => item.id), (ids, previous) => { if (scope.value !== 'all' && ids.some(id => !previous.includes(id))) scope.value = 'pending' })
const histories = ref<Record<string, ZpaqHistory>>({})
const historyBusy = ref('')
async function loadSnapshots(id: string) {
  const item = store.items.find(item => item.id === id)
  if (!item || store.busy || store.preparing || historyBusy.value) return
  historyBusy.value = id
  try {
    const settings = store.effectiveSettings(item)
    const metadata = await commands.browseArchive(item.sourcePath, settings.sourcePassword, undefined, undefined, settings.sourceZpaqSnapshot)
    if (!metadata.zpaqHistory) throw new Error('此归档没有可选择的日志快照；流式 ZPAQ 仅有当前内容。')
    histories.value[id] = metadata.zpaqHistory
    store.updateItem(id, { sourceZpaqSnapshot: metadata.zpaqHistory.selected })
  } catch (error) { store.error = String(error) }
  finally { historyBusy.value = '' }
}
function chooseSnapshot(id: string, snapshot: ZpaqSnapshot) {
  store.updateItem(id, { sourceZpaqSnapshot: snapshot })
  if (histories.value[id]) histories.value[id] = { ...histories.value[id], selected: snapshot }
}
// A temporarily empty edit must stay empty while typing. outputFor supplies
// an automatic filename, so binding it directly would refill a cleared input.
const outputEdit = ref<{ id: string; value: string } | null>(null)
function editOutput(id: string, event: Event) {
  const value = (event.target as HTMLInputElement).value
  outputEdit.value = { id, value }
  store.updateItem(id, { outputPath: value })
}
const queuedSelection = () => import.meta.env.VITE_DESKTOP_E2E === '1'
  ? window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection() : undefined
const results = computed(() => store.items.flatMap(item => {
  const task = store.taskFor(item), location = task ? resultLocation(task) : null
  return location ? [location] : []
}))
const status = (id: string) => {
  const item = store.items.find(item => item.id === id)!
  const task = store.taskFor(item)
  return !task ? '待处理' : ({ running: item.intent === 'zpaq-update' ? '正在更新' : '正在转换', paused: '已暂停', completed: '已完成', failed: '失败', cancelled: '已取消', cancelling: '正在取消' } as Record<string, string>)[task.status] || task.status
}
async function chooseSource(replaceId?: string) {
  try {
    const queued = queuedSelection()
    const selected = queued === undefined ? await open({ multiple: !replaceId, title: replaceId ? '重新选择来源' : '选择压缩包（可多选，也可选择任意分卷）' }) : queued
    const paths = typeof selected === 'string' ? [selected] : Array.isArray(selected) ? selected : []
    if (replaceId && paths[0]) store.replaceSource(replaceId, paths[0])
    else store.addSources(paths, store.contextIntent)
  } catch (error) { store.error = String(error) }
}
async function chooseOutput(id: string) {
  try {
    const item = store.items.find(item => item.id === id)!
    const queued = queuedSelection()
    const selected = queued === undefined ? await save({ title: '新包保存位置', defaultPath: store.outputFor(item) }) : queued
    if (typeof selected === 'string') store.updateItem(id, { outputPath: selected })
  } catch (error) { store.error = String(error) }
}
async function prepareRetry(id: string) {
  const next=await store.prepareRetry(id)
  if (!next) return
  scope.value = 'pending'
  await nextTick()
  Array.from(document.querySelectorAll<HTMLElement>('[data-conversion-id]')).find(row=>row.dataset.conversionId===next)?.scrollIntoView?.({block:'start',behavior:'smooth'})
}
function toggle(id: string) {
  const item = store.items.find(item => item.id === id)!
  item.expanded = !item.expanded
  if (!store.busy) store.activeId = id
}
</script>
<template>
  <Modal v-model:visible="store.visible" :title="store.contextIntent === 'zpaq-update' ? 'ZPAQ 备份更新' : '压缩包转换'" icon="pi pi-sync" size="xl">
    <div data-testid="archive-conversion" class="min-w-0 space-y-4">
      <div class="flex flex-wrap items-center gap-2 text-xs text-muted">
        <span class="mr-auto">{{ store.contextIntent === 'zpaq-update' ? '添加与更新内容 · 保留历史 · 另存新包' : '转换格式 · 修改密码 · 调整分卷' }}</span>
        <HelpHint v-if="store.contextIntent !== 'zpaq-update'" label="转换说明">保留原压缩包并生成新包。同一组分卷只添加一个任务，可选择任意卷；新包密码留空表示不加密。</HelpHint>
        <button type="button" class="conversion-button" :disabled="store.busy || store.preparing" data-testid="conversion-choose-source" @click="chooseSource()"><i class="pi pi-plus" aria-hidden="true"></i>{{ store.contextIntent === 'zpaq-update' ? '添加 ZPAQ 备份' : '添加压缩包' }}</button>
      </div>
      <ConversionBatchConfig />
      <TaskScopeBar v-if="store.items.length" :model-value="scope" :statuses="statuses" @update:model-value="changeScope" />
      <BatchResultActions v-if="scope === 'all' || scope === 'completed'" :locations="results" />
      <p v-if="store.error && !store.items.some(item => store.taskFor(item)?.status === 'failed' && item.error === store.error)" role="alert" class="rounded-xl bg-red-500/5 px-3 py-2 text-xs text-red-500 break-words">{{ archiveInspectionMessageForDisplay(store.error) }}</p>
      <div v-if="!store.items.length" class="rounded-2xl border border-dashed border-primary/30 bg-primary/5 py-10 text-center">
        <i class="pi pi-file-export mb-3 text-3xl text-primary" aria-hidden="true"></i>
        <p class="text-sm font-semibold">{{ store.contextIntent === 'zpaq-update' ? '让已有备份继续保存新内容' : '一次处理多个压缩包' }}</p>
        <p class="mt-2 px-4 text-xs text-muted">添加原包后，点击条目展开编辑；多选后在弹窗统一修改配置。</p>
        <button type="button" class="conversion-button mt-4 bg-primary text-white" @click="chooseSource()">选择压缩包</button>
      </div>
      <div v-else class="min-w-0 overflow-hidden rounded-2xl border border-subtle">
        <div class="flex items-center gap-3 border-b border-subtle bg-input/40 px-4 py-2.5 text-xs text-muted">
          <input type="checkbox" :checked="store.pending.length > 0 && store.selected.length === store.pending.length" :disabled="store.busy || store.preparing || !store.pending.length || !['all', 'pending'].includes(scope)" aria-label="选择全部待转换任务" @change="store.pending.forEach(item => item.selected = ($event.target as HTMLInputElement).checked)" />
          <span class="mr-auto">共 {{ store.items.length }} 项 · 待处理 {{ store.pending.length }} 项</span>
          <button type="button" class="hover:text-primary" :disabled="store.busy || store.preparing || !results.length" @click="store.items.filter(item => store.taskFor(item)?.status === 'completed').forEach(item => store.remove(item.id))">清除已完成</button>
        </div>
        <div v-if="!visibleItems.length" data-testid="task-scope-empty" class="p-6 text-center text-sm text-muted"><p>当前分类没有任务。</p><button type="button" class="mt-3 text-primary" @click="changeScope('all')">返回全部任务</button></div>
        <section v-for="item in visibleItems" :key="item.id" class="min-w-0 border-b border-subtle last:border-b-0" data-testid="conversion-item" :data-conversion-id="item.id">
          <div class="flex min-w-0 items-center gap-3 px-4 py-3 hover:bg-primary/5">
            <input v-model="item.selected" type="checkbox" :disabled="store.busy || store.preparing || !!item.taskId" :aria-label="'选择 ' + item.sourcePath.split(/[\\/]/).pop()" data-testid="conversion-checkbox" />
            <button type="button" class="min-w-0 flex-1 text-left" data-testid="conversion-row" :aria-expanded="item.expanded" @click="toggle(item.id)">
              <div class="flex min-w-0 items-center gap-2"><i class="pi pi-box text-primary" aria-hidden="true"></i><span v-if="item.intent === 'zpaq-update'" class="shrink-0 rounded-md bg-primary/10 px-1.5 py-0.5 text-xs text-primary">增量更新</span><span class="min-w-0 break-words text-sm font-semibold [overflow-wrap:anywhere] line-clamp-2" :title="item.sourcePath">{{ item.sourcePath.split(/[\\/]/).pop() }}</span><i class="pi ml-auto shrink-0 text-xs text-muted" :class="item.expanded ? 'pi-chevron-up' : 'pi-chevron-down'" aria-hidden="true"></i></div>
              <p class="mt-1 truncate text-xs text-muted" :title="store.outputFor(item)">→ {{ store.outputFor(item) }}</p>
            </button>
            <span class="shrink-0 rounded-lg bg-input/60 px-2 py-1 text-xs" :class="item.error || item.sourceIssue ? 'text-red-500' : store.taskFor(item)?.status === 'completed' ? 'text-emerald-600' : 'text-muted'">{{ item.sourceIssue ? '来源待调整' : status(item.id) }}</span>
            <button type="button" class="shrink-0 text-muted hover:text-red-500 disabled:opacity-30" :disabled="store.busy || store.preparing" :aria-label="'移除 ' + item.sourcePath.split(/[\\/]/).pop()" @click="store.remove(item.id)"><i class="pi pi-times" aria-hidden="true"></i></button>
          </div>
          <div v-if="item.expanded" class="min-w-0 space-y-4 border-t border-subtle bg-input/20 p-4 sm:px-6" data-testid="conversion-details">
            <p v-if="item.retryOfTaskId && !item.taskId" data-testid="conversion-retry-notice" class="rounded-lg border border-primary/20 bg-primary/5 p-2 text-xs text-primary">从原任务创建的新条目，请确认后启动；原失败记录和已有结果保留。</p>
            <p class="select-text break-words text-xs text-muted [overflow-wrap:anywhere]">{{ item.sourcePath }}</p>
            <p v-if="item.sourceIssue || (!item.taskId && item.error)" role="alert" class="break-words text-xs text-red-500">{{ archiveInspectionMessageForDisplay(item.error || item.sourceIssue || '') }}</p>
            <template v-if="!item.taskId">
              <div v-if="item.sourceIssue" class="flex items-center gap-2"><button type="button" class="conversion-button" @click="chooseSource(item.id)">重新选择来源</button></div>
              <p v-if="item.passwordRequired" class="rounded-lg bg-amber-500/5 p-2 text-xs text-amber-600">密码未保存在草稿中，请在对应输入框重新填写；不再需要时可选择下方的免密码选项。</p>
              <label class="block min-w-0 text-sm">新包保存位置<div class="mt-1 flex min-w-0 gap-2"><input :value="outputEdit?.id === item.id ? outputEdit.value : store.outputFor(item)" data-testid="conversion-output" class="conversion-input min-w-0 flex-1" :disabled="store.busy || store.preparing" @input="editOutput(item.id, $event)" @blur="outputEdit = null" /><button type="button" class="conversion-button" :disabled="store.busy || store.preparing" @click="chooseOutput(item.id)">另存为</button></div></label>
              <div v-if="item.intent !== 'zpaq-update' && (item.sourceFormat === 'zpaq' || store.effectiveSettings(item).sourceZpaqSnapshot)" class="min-w-0 space-y-2" data-testid="conversion-zpaq-history">
                <ZpaqSnapshotPicker v-if="histories[item.id]" :history="{ ...histories[item.id], selected: store.effectiveSettings(item).sourceZpaqSnapshot || histories[item.id].selected }" :disabled="store.busy || store.preparing || !!historyBusy" @change="chooseSnapshot(item.id, $event)" />
                <div class="flex min-w-0 flex-wrap items-center gap-2 text-xs text-muted">
                  <span v-if="store.effectiveSettings(item).sourceZpaqSnapshot">读取快照 {{ store.effectiveSettings(item).sourceZpaqSnapshot!.version }} · {{ snapshotDate(store.effectiveSettings(item).sourceZpaqSnapshot!.date) }}</span><span v-else>默认读取最新内容，可选择历史快照后转换。</span>
                  <button type="button" class="conversion-button" data-testid="conversion-load-snapshots" :disabled="store.busy || store.preparing || !!historyBusy" @click="loadSnapshots(item.id)">{{ historyBusy === item.id ? '读取中…' : '读取历史快照' }}</button>
                </div>
              </div>
              <ZpaqUpdateFields v-if="item.intent === 'zpaq-update'" :settings="store.effectiveSettings(item)" :base-bytes="item.sourceBytes" :disabled="store.busy || store.preparing" :source-password-required="store.passwordRequirements(item).source" @update="store.updateItem(item.id, $event)" />
              <ConversionSettingsFields v-else :settings="store.effectiveSettings(item)" :source-path="item.sourcePath" :source-format="item.sourceFormat" :disabled="store.busy || store.preparing"
                :source-password-required="store.passwordRequirements(item).source" :target-password-required="store.passwordRequirements(item).target"
                @update="store.updateItem(item.id, $event)" />
            </template>
            <template v-else>
              <TaskFailureHint :task="store.taskFor(item)" />
              <div v-if="store.taskFor(item)?.status === 'completed' && store.taskFor(item)?.archiveOutput" data-testid="conversion-result" class="space-y-2"><p class="text-sm font-semibold text-emerald-600">{{ item.intent === 'zpaq-update' ? '更新完成 · 原备份和历史已保留' : '转换完成 · 原压缩包已保留' }}</p><ArchiveOutputSummary :output="store.taskFor(item)!.archiveOutput!" @checksum="store.visible = false" /></div>
              <p v-else-if="store.taskFor(item)?.status === 'cancelled'" class="text-xs text-amber-600">任务已取消 · 原包和未开始的条目仍保留</p>
              <div v-else-if="store.taskFor(item) && !['failed', 'completed', 'cancelled'].includes(store.taskFor(item)!.status)" class="space-y-2 text-xs text-primary"><span>{{ store.phase }} · 当前阶段 {{ formatProgressPercent(store.taskFor(item)!.progress) }}%</span><progress class="w-full" :value="store.taskFor(item)!.progress" max="100" /></div>
              <p v-if="store.taskFor(item)?.historySaveError" class="text-xs text-red-500">{{ store.taskFor(item)?.historySaveError }}</p>
              <details class="text-xs text-muted"><summary class="cursor-pointer">执行日志</summary><div class="mt-2 max-h-40 space-y-1 overflow-y-auto"><p v-for="(log, index) in store.taskFor(item)?.logs" :key="index" class="break-words">{{ log.message }}</p></div></details>
              <button v-if="['failed', 'cancelled'].includes(store.taskFor(item)?.status || '')" type="button" class="conversion-button" :disabled="store.busy || store.preparing" data-testid="conversion-prepare-retry" @click="prepareRetry(item.id)">{{ store.pending.some(pending => pending.retryOfTaskId === item.taskId) ? '查看已创建的待处理项' : item.intent === 'zpaq-update' ? '调整配置并创建更新草稿' : '调整配置并创建转换草稿' }}</button>
            </template>
          </div>
        </section>
      </div>
    </div>
    <template #footer>
      <button type="button" class="conversion-button" @click="store.visible = false">{{ store.busy ? '后台继续' : '关闭' }}</button>
      <button v-if="store.busy && store.activeId && store.items.find(item => item.id === store.activeId)?.intent === 'zpaq-update'" type="button" class="conversion-button" data-testid="zpaq-update-pause" @click="store.togglePause()">{{ store.task?.status === 'paused' ? '继续更新' : '暂停更新' }}</button>
      <button v-if="store.busy" type="button" class="conversion-button" data-testid="conversion-cancel" :disabled="store.stopRequested" @click="store.cancel">{{ store.stopRequested ? '正在停止…' : '停止本批次' }}</button>
      <button v-else type="button" class="conversion-button bg-primary text-white" data-testid="conversion-start" :disabled="!store.selected.length || store.preparing || !!historyBusy" @click="scope = 'all'; store.start()">{{ store.preparing ? '正在读取卷组…' : store.selected.some(item => item.intent === 'zpaq-update') ? '开始所选任务' : '开始转换' }}{{ store.selected.length ? '（' + store.selected.length + ' 项）' : '' }}</button>
    </template>
  </Modal>
</template>
<style>
.conversion-input { border:1px solid var(--border-subtle); border-radius:.65rem; padding:.6rem .7rem; background:var(--bg-input); font-size:.8rem; }
.conversion-button { display:inline-flex; flex-shrink:0; align-items:center; justify-content:center; gap:.45rem; border:1px solid var(--border-subtle); border-radius:.65rem; padding:.5rem .75rem; font-size:.75rem; font-weight:600; transition:background .15s; }
.conversion-button:disabled { opacity:.4; cursor:default; }
.conversion-button:focus-visible, .conversion-input:focus-visible { outline:2px solid var(--dynamic-accent); outline-offset:2px; }
</style>
