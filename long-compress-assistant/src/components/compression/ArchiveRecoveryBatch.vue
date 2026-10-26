<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import { open } from '@tauri-apps/api/dialog'
import { invoke } from '@tauri-apps/api/tauri'
import Modal from '@/components/ui/Modal.vue'
import ArchiveRecoveryDetails from './ArchiveRecoveryDetails.vue'
import { useArchiveRecoveryBatch, type ArchiveRecoveryRow } from '@/composables/useArchiveRecoveryBatch'
import { useUIStore } from '@/stores/ui'
import ZipPasswordEncodingField from '@/components/ui/ZipPasswordEncodingField.vue'
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import HelpHint from '@/components/ui/HelpHint.vue'
import { useAppStore } from '@/stores/app'
import { ZIP_PASSWORD_ENCODING_OPTIONS, isZipPasswordEncoding } from '@/types/zipPassword'
import { supportsZipPasswordEncoding } from '@/utils/zipPasswordSource'
import { detectArchiveGroup } from '@/utils/archiveSource'
const props = defineProps<{ paths: string[] }>()
const emit = defineEmits<{busy:[value:boolean]; inspect:[row:ArchiveRecoveryRow]; recoveryVolumes:[row:ArchiveRecoveryRow]}>()
const batch = useArchiveRecoveryBatch()
const {rows,busy,preparing,selected,repairable,queueable,completed,rootDirectory:root,saveError} = batch
const ui = useUIStore()
const app = useAppStore()
const sharedEncoding = ref('keep'), applying = ref(false)
let disposed = false
const error = ref(''), configuring = ref(false), changePassword = ref(false), sharedPassword = ref(''), changeExtract = ref(false), extractDirectory = ref(''), repairDirectory = ref('')
watch(configuring, visible => { if (!visible) sharedPassword.value = '' })
const allSelected = computed(() => !!rows.value.length && rows.value.every(row => row.selected))
const summary = computed(() => ({healthy:rows.value.filter(row => !row.needsRecheck && row.report?.status === 'healthy').length,missing:rows.value.filter(row => !row.needsRecheck && (row.report?.status === 'missing_volume' || !!row.report?.missingVolumes.length)).length,password:rows.value.filter(row => !row.needsRecheck && ['password_required','wrong_password','ambiguous_data'].includes(row.report?.status || '')).length,repairable:repairable.value.length}))
watch(() => props.paths, async paths => { error.value = ''; try { await batch.load(paths) } catch (failure) { error.value = String(failure) } }, {immediate:true})
watch(() => busy.value || preparing.value, value => emit('busy', value), {immediate:true})
onUnmounted(() => { disposed = true; emit('busy', false); sharedPassword.value = '' })
const labels: Record<string,string> = {partial_recovery:'缺卷，可部分救援',healthy:'正常',missing_volume:'缺少分卷',password_required:'需要密码',wrong_password:'请核对密码',ambiguous_data:'密码或内容异常',directory_lost:'目录丢失，可救援',crc_error:'内容校验失败',damaged:'归档损坏',truncated:'归档被截断',unsupported_feature:'暂不支持此特性',structure_only:'仅检查结构'}
const status = (row:ArchiveRecoveryRow) => row.queuedId ? '已加入解压队列' : row.state === 'checking' ? '检查中…' : row.state === 'repairing' ? '修复中…' : row.state === 'cancelled' ? '已停止，可重试' : row.error ? '处理未完成' : row.result ? row.result.partial ? '已部分恢复' : '已修复' : row.needsRecheck ? '已恢复配置，请重新检查' : (row.report?.recoverability==='lzop_prefix'?'可部分取回完整成员':labels[row.report?.status || '']) || (row.report ? '请查看详情' : '待检查')
labels.password_encoding = '原包编码不兼容'
function toggleAll() { const value = !allSelected.value; rows.value.forEach(row => { row.selected = value }) }
async function locate(path:string) { try { await invoke('open_in_explorer',{path}) } catch { ui.showToast('error','无法定位结果，请确认文件仍在原位置') } }
async function chooseDirectory() {
  const queued = import.meta.env.VITE_DESKTOP_E2E === '1' ? window.__LONG_DECOMPRESS_DESKTOP_E2E__?.takeDesktopDialogSelection() : undefined
  const directory = queued === undefined ? await open({directory:true,multiple:false,title:'选择批量恢复文件保存目录'}) : queued
  return typeof directory === 'string' ? directory : ''
}
async function repair() {
  error.value = ''
  try {
    if (repairable.value.some(row => !row.outputDirectory) && !root.value) { root.value = await chooseDirectory(); if (!root.value) return }
    await batch.repair(root.value)
  } catch (failure) { error.value = String(failure) }
}
function configure() { if (applying.value) return; changePassword.value = changeExtract.value = false; sharedPassword.value = repairDirectory.value = extractDirectory.value = ''; sharedEncoding.value = 'keep'; configuring.value = true }
async function apply() {
  if (applying.value || busy.value || preparing.value) return
  const chosen = [...selected.value]
  const patch = {...(changePassword.value ? {password:sharedPassword.value} : {}),...(repairDirectory.value.trim() ? {outputDirectory:repairDirectory.value.trim()} : {}),...(changeExtract.value ? {extractDirectory:extractDirectory.value.trim()} : {})}
  const encoding = sharedEncoding.value
  let skipped = 0
  applying.value = true
  try {
    for (const row of chosen) {
      let supports = false
      let actualFormat: string | null | undefined
      if (encoding !== 'keep') {
        try {
          const group = await detectArchiveGroup(row.path)
          supports = !!group?.archive_format && supportsZipPasswordEncoding(row.path, group.archive_format)
          actualFormat = group?.archive_format
        } catch { /* Unknown formats do not receive ZIP-only settings. */ }
        if (!supports) skipped++
      }
      if (disposed || busy.value || preparing.value || !rows.value.includes(row)) return
      if (actualFormat) row.sourceFormat = actualFormat
      batch.update(row, {...patch,...(supports && (encoding === 'auto' || isZipPasswordEncoding(encoding)) ? {zipPasswordEncoding:encoding === 'auto' ? undefined : encoding} : {})})
    }
    configuring.value = false; sharedPassword.value = ''
    if (skipped) ui.showToast('info',`${skipped} 项不适用或格式未确认，未修改原包编码`)
  } finally { applying.value = false }
}
function browseResult(row:ArchiveRecoveryRow) {
  if (!row.result) return
  app.openArchiveInBrowser(row.result.outputPath, {password:row.resultPassword,zipPasswordEncoding:row.resultZipPasswordEncoding})
}
async function queue() { try { const count = await batch.queue(); if (count) ui.showToast('success',`已加入 ${count} 项待处理任务，请核对后开始解压`) } catch (failure) { error.value = String(failure) } }
function updatePassword(row:ArchiveRecoveryRow, event:Event) { batch.update(row, {password:(event.target as HTMLInputElement).value}) }
</script>
<template>
  <section class="rounded-2xl border border-subtle bg-card p-4 sm:p-5" data-testid="archive-recovery-batch">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div><h2 class="font-black text-content">批量检查与修复 <span class="ml-2 text-sm font-normal text-muted">{{ preparing ? '正在整理卷组…' : `${rows.length} 组压缩包` }}</span></h2><p class="mt-1 text-xs leading-relaxed text-muted">同组分卷合并检查一次；填写密码或修复后，可继续处理剩余任务。</p></div>
      <span v-if="rows.length" class="text-xs text-muted">已检查 {{ completed }} / {{ rows.length }}</span>
    </div>
    <div class="my-4 grid grid-cols-2 gap-2 sm:grid-cols-4">
      <div v-for="card in [{label:'正常',value:summary.healthy},{label:'缺少分卷',value:summary.missing},{label:'需核对密码',value:summary.password},{label:'可尝试修复',value:summary.repairable}]" :key="card.label" class="rounded-xl bg-input/60 px-3 py-2"><p class="text-xs text-muted">{{ card.label }}</p><strong class="mt-1 block text-lg text-content">{{ card.value }}</strong></div>
    </div>
    <div class="mb-4 flex flex-wrap items-center gap-2">
      <button class="batch-action bg-primary text-white" :disabled="busy || preparing || !selected.length" data-testid="batch-diagnose" @click="batch.check()"><i class="pi pi-search"></i>检查所选</button>
      <button v-if="busy" class="batch-action border border-red-500/30 text-red-500" data-testid="batch-stop" @click="batch.stop()">停止处理</button>
      <template v-else>
        <button class="batch-action border border-subtle" :disabled="preparing || !selected.length" data-testid="batch-recovery-config" @click="configure">修改所选配置</button>
        <button class="batch-action border border-primary/30 text-primary" :disabled="!repairable.length" data-testid="batch-repair" @click="repair">修复所选（{{ repairable.length }}）</button>
        <button class="batch-action border border-primary/30 text-primary" :disabled="!queueable.length" data-testid="batch-queue-recovered" @click="queue">加入恢复结果（{{ queueable.length }}）</button>
      </template>
    </div>
    <p v-if="error || saveError" role="alert" class="mb-3 break-words rounded-xl bg-red-500/10 p-3 text-xs text-red-500">{{ error || saveError }}</p>
    <div v-if="rows.length" class="mb-2 flex items-center gap-2 text-xs text-muted"><label class="flex items-center gap-2"><input type="checkbox" :checked="allSelected" :disabled="busy" aria-label="选择全部压缩包" @change="toggleAll" />已选 {{ selected.length }} 组</label><button type="button" class="ml-auto text-primary" :disabled="busy" @click="rows.forEach(row => row.selected = row.report?.status !== 'healthy' && !row.result)">仅选待处理</button></div>
    <div class="space-y-2">
      <article v-for="row in rows" :key="row.id" class="overflow-hidden rounded-xl border border-subtle" :data-source="row.path" data-testid="archive-batch-row">
        <div class="flex min-w-0 items-center gap-3 px-3 py-3">
          <input v-model="row.selected" type="checkbox" :disabled="busy" :aria-label="`选择 ${row.path.split(/[\\/]/).pop()}`" />
          <button class="flex min-w-0 flex-1 flex-wrap items-center gap-x-3 gap-y-1 text-left" :aria-expanded="row.expanded" data-testid="archive-batch-expand" @click="row.expanded = !row.expanded">
            <span class="min-w-0 flex-1 basis-40 break-all text-sm font-semibold text-content">{{ row.path.split(/[\\/]/).pop() }}<span v-if="row.selectedMembers > 1" class="ml-2 text-xs font-normal text-muted">已合并 {{ row.selectedMembers }} 个所选卷</span></span>
            <span class="text-xs" :class="row.result?.partial || row.report?.status === 'partial_recovery' ? 'text-amber-600' : row.result || row.report?.status === 'healthy' ? 'text-green-600' : row.error ? 'text-red-500' : 'text-primary'" data-testid="archive-batch-status">{{ status(row) }}</span><i class="pi text-xs text-muted" :class="row.expanded ? 'pi-chevron-up' : 'pi-chevron-down'"></i>
          </button>
        </div>
        <div v-if="row.expanded" class="space-y-3 border-t border-subtle bg-input/20 p-3 sm:p-4" data-testid="archive-batch-detail">
          <p class="break-all text-xs text-muted">{{ row.path }}</p>
          <button v-if="row.result" class="batch-action border border-subtle" data-testid="browse-batch-repaired-archive" :disabled="busy" @click="browseResult(row)">浏览恢复结果</button>
          <p v-if="row.error" role="alert" class="break-words text-xs text-red-500">{{ row.error }}</p>
          <p v-if="row.needsRecheck" class="text-xs text-amber-600">上次检查结果仅供参考，请重新检查当前文件。</p>
          <ul v-if="row.report?.issues.length" class="space-y-2 text-xs leading-relaxed text-muted"><li v-for="(issue,index) in row.report.issues" :key="index"><strong class="text-content">{{ issue.title }}</strong>：{{ issue.detail }}</li></ul>
          <div class="grid min-w-0 gap-3 sm:grid-cols-2">
            <label class="min-w-0 space-y-1 text-xs text-muted">原包密码<input :value="row.password" type="password" autocomplete="off" :disabled="busy" placeholder="修改后重新检查" class="batch-input" data-testid="archive-batch-password" @input="updatePassword(row, $event)" /></label>
            <ZipPasswordEncodingField v-if="supportsZipPasswordEncoding(row.path, row.sourceFormat) || (!row.sourceFormat && row.zipPasswordEncoding !== undefined)" :model-value="row.zipPasswordEncoding" :source-path="row.path" :source-format="row.sourceFormat" :disabled="busy" test-id="recovery-row-zip-password-encoding" @update:model-value="batch.update(row, {zipPasswordEncoding:$event})" />
            <label class="min-w-0 space-y-1 text-xs text-muted">恢复文件保存目录<input v-model="row.outputDirectory" :disabled="busy" placeholder="修复时可统一选择" class="batch-input" /></label>
          </div>
          <div v-if="row.result" class="space-y-2 rounded-xl p-3 text-xs leading-relaxed" :class="row.result.partial ? 'bg-amber-500/5' : 'bg-green-500/5'" data-testid="archive-batch-result"><p class="font-bold" :class="row.result.partial ? 'text-amber-600' : 'text-green-600'">已恢复 {{ row.result.recoveredFiles }} 个文件{{ row.result.partial ? ' · 部分恢复，请核对内容' : '' }}</p><p class="mt-1 break-all text-muted">{{ row.result.outputPath }}</p><label v-if="row.resultPasswordRequired && !row.queuedId" class="block text-muted">恢复结果密码<input v-model="row.resultPassword" type="password" autocomplete="off" placeholder="重启后需重新填写，不保存密码" :disabled="busy" class="batch-input mt-1" data-testid="archive-batch-result-password" /></label><ArchiveRecoveryDetails :recovered-entries="row.result.recoveredEntries" :skipped-entries="row.result.skippedEntries" :entry-results="row.result.entryResults" /><div class="flex flex-wrap gap-3"><button class="font-semibold text-primary" @click="locate(row.result.outputPath)">定位恢复结果</button><button v-if="!row.queuedId" class="text-muted" :disabled="busy" @click="batch.forgetResult(row)">重新修复原包</button></div></div>
          <div class="flex flex-wrap gap-2"><button class="batch-action border border-subtle" :disabled="busy" @click="batch.check([row])">重新检查此项</button><button v-if="row.report" class="batch-action border border-subtle" :disabled="busy || row.needsRecheck" @click="emit('inspect', row)">查看完整诊断</button><button v-if="!row.needsRecheck && row.report?.actualFormat === 'RAR' && row.report.status === 'healthy' && row.report.volumesFound > 1" class="batch-action border border-primary/30 text-primary" :disabled="busy" @click="emit('recoveryVolumes', row)">创建恢复卷</button></div>
        </div>
      </article>
    </div>
    <p v-if="root" class="mt-3 break-all text-xs text-muted">本次恢复结果保存到 {{ root }}，每组使用独立目录；原包保留。</p>
  </section>
  <Modal v-model:visible="configuring" title="修改所选检查与恢复配置" :description="`已选 ${selected.length} 组压缩包`" icon="pi pi-sliders-h" size="md" :close-on-backdrop="false" :show-close-button="!applying" :close-on-escape="!applying">
    <fieldset :disabled="applying" class="space-y-4 min-w-0" data-testid="archive-batch-config-dialog">
      <p class="text-xs leading-relaxed text-muted">只修改指定项目；其他设置保留各自原值。修改密码后需要重新检查。同名归档统一解压时，会自动分开到子目录。</p>
      <label class="block text-sm"><span class="flex items-center gap-2"><input v-model="changePassword" type="checkbox" />填写共用原包密码</span><input v-if="changePassword" v-model="sharedPassword" type="password" autocomplete="off" placeholder="留空则清除原包密码" class="batch-input mt-2" data-testid="archive-batch-shared-password" /></label>
      <div class="min-w-0 space-y-2 text-xs text-muted">原包读取编码 <HelpHint label="批量检查密码编码说明">仅应用到检测确认的 ZIP/ZIPX、ALZ 或 EGG。EGG 默认 UTF-8，只影响密码；ALZ 默认 CP949，同时影响文件名和密码。修改后须重新检查，已恢复结果保留生成时的编码。</HelpHint>
        <ThemedSelect v-model="sharedEncoding" :disabled="applying" :options="[{value:'keep',label:'保持各自设置'},{value:'auto',label:'使用原包格式默认值'},...ZIP_PASSWORD_ENCODING_OPTIONS]" aria-label="批量检查原包读取编码" test-id="recovery-batch-zip-password-encoding" floating />
      </div>
      <label class="block space-y-2 text-xs text-muted">恢复文件保存目录<div class="flex min-w-0 gap-2"><input v-model="repairDirectory" placeholder="留空则保持各自目录" class="batch-input" data-testid="archive-batch-output" /><button class="batch-action shrink-0 border border-subtle" @click="chooseDirectory().then(value => {if(value) repairDirectory = value})">选择</button></div></label>
      <label class="block text-sm"><span class="flex items-center gap-2"><input v-model="changeExtract" type="checkbox" />修改再次解压的输出目录</span><input v-if="changeExtract" v-model="extractDirectory" placeholder="留空则使用各自原包目录" class="batch-input mt-2" /></label>
    </fieldset>
    <template #footer><button class="batch-action border border-subtle" :disabled="applying" @click="configuring = false; sharedPassword = ''">取消</button><button class="batch-action bg-primary text-white" :disabled="applying" data-testid="archive-batch-config-apply" @click="apply">{{ applying ? '正在应用…' : '应用到所选' }}</button></template>
  </Modal>
</template>
<style scoped>
.batch-action { @apply inline-flex items-center justify-center gap-2 rounded-xl px-3 py-2 text-xs font-semibold transition-colors disabled:cursor-not-allowed disabled:opacity-40; }
.batch-input { @apply block w-full min-w-0 rounded-xl border border-subtle bg-input px-3 py-2.5 text-sm text-content outline-none focus:border-primary; }
</style>
