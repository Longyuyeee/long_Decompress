<script setup lang="ts">
import { computed } from 'vue'
import { open, save } from '@tauri-apps/api/dialog'
import Modal from '@/components/ui/Modal.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
import ArchiveOutputSummary from './ArchiveOutputSummary.vue'
import ConversionSettingsFields from './ConversionSettingsFields.vue'
import ConversionBatchConfig from './ConversionBatchConfig.vue'
import BatchResultActions from '@/components/tasks/BatchResultActions.vue'
import { useArchiveConversionStore } from '@/stores/archiveConversion'
import { resultLocation } from '@/utils/resultLocation'
import { formatProgressPercent } from '@/utils/progress'
const store = useArchiveConversionStore()
const queuedSelection = () => import.meta.env.VITE_DESKTOP_E2E === '1'
  ? window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection() : undefined
const results = computed(() => store.items.flatMap(item => {
  const task = store.taskFor(item), location = task ? resultLocation(task) : null
  return location ? [location] : []
}))
const status = (id: string) => {
  const item = store.items.find(item => item.id === id)!
  const task = store.taskFor(item)
  return !task ? '待处理' : ({ running: '正在转换', completed: '已完成', failed: '失败', cancelled: '已取消', cancelling: '正在取消' } as Record<string, string>)[task.status] || task.status
}
async function chooseSource(replaceId?: string) {
  try {
    const queued = queuedSelection()
    const selected = queued === undefined ? await open({ multiple: !replaceId, title: replaceId ? '重新选择来源' : '选择压缩包（可多选，分卷请选择首卷）' }) : queued
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
function toggle(id: string) {
  const item = store.items.find(item => item.id === id)!
  item.expanded = !item.expanded
  if (!store.busy) store.activeId = id
}
</script>
<template>
  <Modal v-model:visible="store.visible" title="压缩包转换" icon="pi pi-sync" size="xl">
    <div data-testid="archive-conversion" class="min-w-0 space-y-4">
      <div class="flex flex-wrap items-center gap-2 text-xs text-muted">
        <span class="mr-auto">转换格式 · 修改密码 · 调整分卷</span>
        <HelpHint label="转换说明">保留原压缩包并生成新包。分卷请选择首卷；新包密码留空表示不加密。</HelpHint>
        <button type="button" class="conversion-button" :disabled="store.busy" data-testid="conversion-choose-source" @click="chooseSource()"><i class="pi pi-plus" aria-hidden="true"></i>添加压缩包</button>
      </div>
      <ConversionBatchConfig />
      <BatchResultActions :locations="results" />
      <p v-if="store.error" role="alert" class="rounded-xl bg-red-500/5 px-3 py-2 text-xs text-red-500 break-words">{{ store.error }}</p>
      <div v-if="!store.items.length" class="rounded-2xl border border-dashed border-primary/30 bg-primary/5 py-10 text-center">
        <i class="pi pi-file-export mb-3 text-3xl text-primary" aria-hidden="true"></i>
        <p class="text-sm font-semibold">一次处理多个压缩包</p>
        <p class="mt-2 px-4 text-xs text-muted">添加文件后，点击条目编辑；多选后统一修改配置。</p>
        <button type="button" class="conversion-button mt-4 bg-primary text-white" @click="chooseSource()">选择压缩包</button>
      </div>
      <div v-else class="min-w-0 overflow-hidden rounded-2xl border border-subtle">
        <div class="flex items-center gap-3 border-b border-subtle bg-input/40 px-4 py-2.5 text-xs text-muted">
          <input type="checkbox" :checked="store.pending.length > 0 && store.selected.length === store.pending.length" :disabled="store.busy || !store.pending.length" aria-label="选择全部待转换任务" @change="store.pending.forEach(item => item.selected = ($event.target as HTMLInputElement).checked)" />
          <span class="mr-auto">共 {{ store.items.length }} 项 · 待处理 {{ store.pending.length }} 项</span>
          <button type="button" class="hover:text-primary" :disabled="store.busy || !results.length" @click="store.items.filter(item => store.taskFor(item)?.status === 'completed').forEach(item => store.remove(item.id))">清除已完成</button>
        </div>
        <section v-for="item in store.items" :key="item.id" class="min-w-0 border-b border-subtle last:border-b-0" data-testid="conversion-item">
          <div class="flex min-w-0 items-center gap-3 px-4 py-3 hover:bg-primary/5">
            <input v-model="item.selected" type="checkbox" :disabled="store.busy || !!item.taskId" :aria-label="'选择 ' + item.sourcePath.split(/[\\/]/).pop()" data-testid="conversion-checkbox" />
            <button type="button" class="min-w-0 flex-1 text-left" data-testid="conversion-row" :aria-expanded="item.expanded" @click="toggle(item.id)">
              <div class="flex min-w-0 items-center gap-2"><i class="pi pi-box text-primary" aria-hidden="true"></i><span class="truncate text-sm font-semibold" :title="item.sourcePath">{{ item.sourcePath.split(/[\\/]/).pop() }}</span><i class="pi ml-auto shrink-0 text-xs text-muted" :class="item.expanded ? 'pi-chevron-up' : 'pi-chevron-down'" aria-hidden="true"></i></div>
              <p class="mt-1 truncate text-xs text-muted" :title="store.outputFor(item)">→ {{ store.outputFor(item) }}</p>
            </button>
            <span class="shrink-0 rounded-lg bg-input/60 px-2 py-1 text-xs" :class="item.error || item.sourceIssue ? 'text-red-500' : store.taskFor(item)?.status === 'completed' ? 'text-emerald-600' : 'text-muted'">{{ item.sourceIssue ? '来源待调整' : status(item.id) }}</span>
            <button type="button" class="shrink-0 text-muted hover:text-red-500 disabled:opacity-30" :disabled="store.busy" :aria-label="'移除 ' + item.sourcePath.split(/[\\/]/).pop()" @click="store.remove(item.id)"><i class="pi pi-times" aria-hidden="true"></i></button>
          </div>
          <div v-if="item.expanded" class="min-w-0 space-y-4 border-t border-subtle bg-input/20 p-4 sm:px-6" data-testid="conversion-details">
            <p class="select-text break-words text-xs text-muted [overflow-wrap:anywhere]">{{ item.sourcePath }}</p>
            <p v-if="item.error || item.sourceIssue" role="alert" class="break-words text-xs text-red-500">{{ item.error || item.sourceIssue }}</p>
            <template v-if="!item.taskId">
              <div v-if="item.sourceIssue" class="flex items-center gap-2"><button type="button" class="conversion-button" @click="chooseSource(item.id)">重新选择来源</button></div>
              <div v-if="item.passwordRequired" class="flex flex-wrap items-center gap-2 rounded-lg bg-amber-500/5 p-2 text-xs text-amber-600"><span class="mr-auto">密码未保存在草稿中，请重新填写；不再需要时可留空确认。</span><button type="button" class="conversion-button" @click="item.passwordRequired = false">确认当前密码设置</button></div>
              <label class="block min-w-0 text-sm">新包保存位置<div class="mt-1 flex min-w-0 gap-2"><input :value="store.outputFor(item)" data-testid="conversion-output" class="conversion-input min-w-0 flex-1" :disabled="store.busy" @input="store.updateItem(item.id, { outputPath: ($event.target as HTMLInputElement).value })" /><button type="button" class="conversion-button" :disabled="store.busy" @click="chooseOutput(item.id)">另存为</button></div></label>
              <ConversionSettingsFields :settings="store.effectiveSettings(item)" :disabled="store.busy" @update="store.updateItem(item.id, $event)" />
            </template>
            <template v-else>
              <div v-if="store.taskFor(item)?.status === 'completed' && store.taskFor(item)?.archiveOutput" data-testid="conversion-result" class="space-y-2"><p class="text-sm font-semibold text-emerald-600">转换完成 · 原压缩包已保留</p><ArchiveOutputSummary :output="store.taskFor(item)!.archiveOutput!" /></div>
              <p v-else-if="store.taskFor(item)?.status === 'cancelled'" class="text-xs text-amber-600">转换已取消 · 未开始的条目仍保留</p>
              <div v-else-if="store.taskFor(item) && !['failed', 'completed', 'cancelled'].includes(store.taskFor(item)!.status)" class="space-y-2 text-xs text-primary"><span>{{ store.phase }} · {{ formatProgressPercent(store.taskFor(item)!.progress) }}%</span><progress class="w-full" :value="store.taskFor(item)!.progress" max="100" /></div>
              <p v-if="store.taskFor(item)?.historySaveError" class="text-xs text-red-500">{{ store.taskFor(item)?.historySaveError }}</p>
              <details class="text-xs text-muted"><summary class="cursor-pointer">执行日志</summary><div class="mt-2 max-h-40 space-y-1 overflow-y-auto"><p v-for="(log, index) in store.taskFor(item)?.logs" :key="index" class="break-words">{{ log.message }}</p></div></details>
              <button v-if="['failed', 'cancelled'].includes(store.taskFor(item)?.status || '')" type="button" class="conversion-button" :disabled="store.busy" @click="item.taskId = undefined; item.error = undefined; item.expanded = true; item.selected = true">调整后再次转换</button>
            </template>
          </div>
        </section>
      </div>
    </div>
    <template #footer>
      <button type="button" class="conversion-button" @click="store.visible = false">{{ store.busy ? '后台继续' : '关闭' }}</button>
      <button v-if="store.busy" type="button" class="conversion-button" data-testid="conversion-cancel" :disabled="store.stopRequested" @click="store.cancel">{{ store.stopRequested ? '正在停止…' : '停止本批次' }}</button>
      <button v-else type="button" class="conversion-button bg-primary text-white" data-testid="conversion-start" :disabled="!store.selected.length" @click="store.start">开始转换{{ store.selected.length ? '（' + store.selected.length + ' 项）' : '' }}</button>
    </template>
  </Modal>
</template>
<style>
.conversion-input { border:1px solid var(--border-subtle); border-radius:.65rem; padding:.6rem .7rem; background:var(--bg-input); font-size:.8rem; }
.conversion-button { display:inline-flex; flex-shrink:0; align-items:center; justify-content:center; gap:.45rem; border:1px solid var(--border-subtle); border-radius:.65rem; padding:.5rem .75rem; font-size:.75rem; font-weight:600; transition:background .15s; }
.conversion-button:disabled { opacity:.4; cursor:default; }
.conversion-button:focus-visible, .conversion-input:focus-visible { outline:2px solid var(--dynamic-accent); outline-offset:2px; }
</style>
