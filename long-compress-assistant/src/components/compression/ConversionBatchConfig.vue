<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { open } from '@tauri-apps/api/dialog'
import HelpHint from '@/components/ui/HelpHint.vue'
import Modal from '@/components/ui/Modal.vue'
import ThemedSelect from '@/components/ui/ThemedSelect.vue'
import SavedLocationPicker from '@/components/ui/SavedLocationPicker.vue'
import { conversionFormats, useArchiveConversionStore } from '@/stores/archiveConversion'
import ConversionSettingsFields from './ConversionSettingsFields.vue'
import { normalizeConversionSettings } from '@/utils/archiveConversionDraft'
import { supportsNativePassword, supportsArchiveSplit } from '@/utils/compressionFormat'
import { isZipPasswordEncoding, ZIP_PASSWORD_ENCODING_OPTIONS } from '@/types/zipPassword'
import { useUIStore } from '@/stores/ui'
const store = useArchiveConversionStore()
const visible = ref(false)
const updatesOnly = computed(() => store.selected.length > 0 && store.selected.every(item => item.intent === 'zpaq-update'))
const hasUpdates = computed(() => store.selected.some(item => item.intent === 'zpaq-update'))
const applying = ref(false)
const applyError = ref('')
const mode = ref<'selected' | 'defaults'>('selected')
const format = ref('keep'), level = ref('keep'), destination = ref('keep'), directory = ref('')
const changePassword = ref(false), password = ref(''), changeSourcePassword = ref(false), sourcePassword = ref('')
const changeSplit = ref(false), splitMiB = ref(0), hiddenNames = ref('keep'), recovery = ref('keep')
const recoveryVolumes = ref('keep'), zipMethod = ref('keep')
const wimMethod = ref('keep')
watch(format, () => { if (format.value === 'swm' && !['keep','store','xpress','lzx'].includes(wimMethod.value)) wimMethod.value = 'keep' })
const sourceEncoding = ref('keep'), targetEncoding = ref('keep')
const defaults = ref({ ...store.defaults })
const keep = { value: 'keep', label: '保持各自设置' }
const changed = computed(() => wimMethod.value !== 'keep' || sourceEncoding.value !== 'keep' || targetEncoding.value !== 'keep' || format.value !== 'keep' || zipMethod.value !== 'keep' || level.value !== 'keep' || destination.value !== 'keep' || changePassword.value || changeSourcePassword.value || changeSplit.value || hiddenNames.value !== 'keep' || recovery.value !== 'keep' || recoveryVolumes.value !== 'keep')
const valid = computed(() => (mode.value === 'defaults' || changed.value) && (destination.value !== 'custom' || !!directory.value.trim()) && (!changeSplit.value || Number.isFinite(splitMiB.value) && splitMiB.value >= 0))
function show(nextMode: 'selected' | 'defaults') {
  mode.value = nextMode; format.value = level.value = destination.value = hiddenNames.value = recovery.value = 'keep'
  recoveryVolumes.value = zipMethod.value = 'keep'
  wimMethod.value = 'keep'
  sourceEncoding.value = targetEncoding.value = 'keep'
  directory.value = password.value = sourcePassword.value = ''
  changePassword.value = changeSourcePassword.value = changeSplit.value = false; splitMiB.value = 0
  defaults.value = { ...store.defaults }; visible.value = true
  applyError.value = ''
}
async function chooseDirectory() {
  try {
    const path = await open({ directory: true, multiple: false, title: '选择新包输出目录' })
    if (visible.value && typeof path === 'string') { directory.value = path; destination.value = 'custom' }
  } catch { /* The editable path remains available. */ }
}
async function apply() {
  if (!valid.value || store.busy || store.preparing || applying.value) return
  applying.value = true
  applyError.value = ''
  try {
  if (mode.value === 'defaults') store.updateDefaults(defaults.value)
  else applyError.value = await store.applySelected({
    ...(sourceEncoding.value === 'auto' || isZipPasswordEncoding(sourceEncoding.value) ? {sourceZipPasswordEncoding:sourceEncoding.value === 'auto' ? undefined : sourceEncoding.value} : {}),
    ...(isZipPasswordEncoding(targetEncoding.value) ? {zipPasswordEncoding:targetEncoding.value} : {}),
    ...(format.value === 'keep' ? {} : { format: format.value }),
    ...(zipMethod.value === 'keep' || (format.value !== 'keep' && format.value !== 'zip') ? {} : { zipMethod: zipMethod.value as 'deflate' | 'zstd' | 'bzip2' | 'ppmd' | 'lzma' }),
    ...(wimMethod.value === 'keep' || (format.value !== 'keep' && !['wim','swm'].includes(format.value)) ? {} : { wimMethod: wimMethod.value as 'store' | 'xpress' | 'lzx' | 'lzms' | 'solid' }),
    ...(level.value === 'keep' ? {} : { level: Number(level.value) }),
    ...(destination.value === 'keep' ? {} : { outputDirectory: destination.value === 'source' ? '' : directory.value.trim() }),
    ...(changePassword.value ? { password: password.value } : {}),
    ...(changeSourcePassword.value ? { sourcePassword: sourcePassword.value } : {}),
    ...(changeSplit.value ? { splitMiB: splitMiB.value } : {}),
    ...(hiddenNames.value === 'keep' ? {} : { encryptFilenames: hiddenNames.value === 'yes' }),
    ...(recovery.value === 'keep' ? {} : { rarRecoveryPercent: Number(recovery.value) }),
    ...(recoveryVolumes.value === 'keep' ? {} : { rarRecoveryVolumes: Number(recoveryVolumes.value) }),
  })
  if (applyError.value) return
  if (mode.value === 'selected' && store.batchNotice) useUIStore().showToast('info', store.batchNotice)
  visible.value = false
  } finally { applying.value = false; if (!visible.value) password.value = sourcePassword.value = '' }
}
</script>
<template>
  <div class="flex flex-wrap items-center gap-2">
    <button v-if="store.contextIntent !== 'zpaq-update'" type="button" class="conversion-button text-muted" :disabled="store.busy || store.preparing" @click="show('defaults')"><i class="pi pi-sliders-h" aria-hidden="true"></i>默认配置</button>
    <div v-if="store.selected.length" class="flex flex-wrap items-center gap-2 rounded-xl border border-primary/25 bg-primary/5 px-3 py-1.5" data-testid="conversion-batch-bar">
      <span class="text-xs font-semibold text-primary">已选 {{ store.selected.length }} 项</span>
      <button type="button" class="conversion-button bg-primary text-white" :disabled="store.busy || store.preparing" data-testid="conversion-batch-config" @click="show('selected')">修改所选配置</button>
      <button type="button" class="conversion-button text-muted" :disabled="store.busy || store.preparing" @click="store.pending.forEach(item => item.selected = false)">取消选择</button>
    </div>
  </div>
  <Modal v-model:visible="visible" :title="mode === 'selected' ? '修改所选转换配置' : '转换默认配置'" icon="pi pi-sliders-h" layer="nested" size="lg" :show-close-button="!applying" :close-on-escape="!applying" :close-on-backdrop="!applying">
    <fieldset :disabled="applying" class="space-y-4 min-w-0" data-testid="conversion-batch-fields">
      <p v-if="applyError" role="alert" class="text-xs text-red-500">{{ applyError }}</p>
      <template v-if="mode === 'defaults'">
        <p class="text-xs text-muted">应用到尚未单独编辑的任务；已编辑条目保留自己的配置。</p>
        <ConversionSettingsFields :settings="defaults" @update="defaults = normalizeConversionSettings(defaults, $event)" />
        <label class="block text-sm">输出目录<input v-model="defaults.outputDirectory" class="conversion-input mt-1 w-full" placeholder="留空放在各自源文件旁" /></label>
      </template>
      <template v-else>
        <p v-if="hasUpdates" class="rounded-lg bg-primary/5 p-3 text-xs leading-relaxed text-muted">ZPAQ 更新任务沿用原格式和原密码，可统一强度、分卷、保存目录与原包密码；添加内容和归档内位置在各条目展开编辑。</p>
        <p class="text-xs text-muted">只修改指定参数；名称、目录、密码等未指定的配置保持各自原值。</p>
        <p v-if="format !== 'keep' && (!supportsNativePassword(format) || !supportsArchiveSplit(format))" class="text-xs text-amber-600">此格式不支持的密码或分卷选项会关闭，其余配置保留。</p>
        <div class="grid min-w-0 gap-4 sm:grid-cols-2">
          <div v-if="!updatesOnly" class="min-w-0 space-y-1.5 text-sm">原包读取编码 <HelpHint label="批量原包编码说明">只修改检测确认的 ZIP/ZIPX、ALZ 或 EGG 原包，清除后使用各格式默认值。EGG 默认 UTF-8，只影响密码；ALZ 默认 CP949，同时影响文件名与密码。与新包编码无关。</HelpHint><ThemedSelect v-model="sourceEncoding" :options="[keep,{value:'auto',label:'使用原包格式默认值'},...ZIP_PASSWORD_ENCODING_OPTIONS]" :disabled="applying" test-id="conversion-batch-source-encoding" floating /></div>
          <div v-if="!updatesOnly" class="min-w-0 space-y-1.5 text-sm">新包 ZIP 密码编码 <HelpHint label="批量新包编码说明">只修改目标为 ZIP/ZIPX 的任务。保持各自设置不会覆盖已有值；UTF-8 是新包默认值。</HelpHint><ThemedSelect v-model="targetEncoding" :options="[keep,...ZIP_PASSWORD_ENCODING_OPTIONS]" :disabled="applying" test-id="conversion-batch-target-encoding" floating /></div>
          <label v-if="!updatesOnly" class="min-w-0 text-sm">目标格式 <HelpHint v-if="format === 'zipx'" label="ZIPX 分卷说明">ZIPX 使用 XZ，支持数字卷和 .zx01/…/.zipx 标准卷；卷型沿用各自设置，可在任务中调整。</HelpHint><ThemedSelect v-model="format" :options="[keep, ...conversionFormats]" test-id="conversion-batch-format" floating /></label>
          <label v-if="!updatesOnly && (format === 'keep' || format === 'zip')" class="min-w-0 text-sm">ZIP 压缩方法<ThemedSelect v-model="zipMethod" :options="[keep, { value: 'deflate', label: '兼容 ZIP · Deflate' }, { value: 'zstd', label: 'Zstandard' }, { value: 'bzip2', label: 'BZip2' }, { value: 'ppmd', label: 'PPMd · 文本归档' }, { value: 'lzma', label: 'LZMA' }]" test-id="conversion-batch-zip-method" floating /><span class="text-xs text-muted">仅修改 ZIP 任务；ZIPX 使用 XZ。</span></label>
          <label v-if="!updatesOnly && (format === 'keep' || ['wim','swm'].includes(format))" class="min-w-0 text-sm">WIM 压缩方法<ThemedSelect v-model="wimMethod" :options="[keep,{value:'store',label:'仅存储 · 去重'},{value:'xpress',label:'XPRESS · 快速压缩'},{value:'lzx',label:'LZX · 体积优先'},{value:'lzms',label:'LZMS · 高压缩'},{value:'solid',label:'LZMS · 固实归档'}].filter(option => format !== 'swm' || ['keep','store','xpress','lzx'].includes(option.value))" test-id="conversion-batch-wim-method" floating /><span class="text-xs text-muted">仅修改适用的 WIM/SWM 任务；不改变原包密码和分卷。</span></label>
          <label class="min-w-0 text-sm">压缩强度<ThemedSelect v-model="level" :disabled="['wim','esd','swm'].includes(format)" :options="[keep, ...Array.from({ length: 10 }, (_, n) => ({ value: String(n), label: String(n) }))]" test-id="conversion-batch-level" floating /></label>
          <label v-if="!updatesOnly" class="min-w-0 text-sm">RAR 恢复保护<ThemedSelect v-model="recovery" :options="[keep, {value:'0',label:'关闭'}, {value:'3',label:'3%'}, {value:'5',label:'5%'}, {value:'10',label:'10%'}]" test-id="conversion-batch-rar-recovery" floating /></label>
          <label v-if="!updatesOnly" class="min-w-0 text-sm">RAR 分包恢复卷<ThemedSelect v-model="recoveryVolumes" :options="[keep, ...[0,1,2,3,5,10].map(value => ({value:String(value),label:value ? `${value} 个 .rev 恢复卷` : '不生成恢复卷'}))]" test-id="conversion-batch-rar-recovery-volumes" floating /></label>
          <label v-if="!updatesOnly" class="min-w-0 text-sm">文件名加密<ThemedSelect v-model="hiddenNames" :options="[keep, { value: 'yes', label: '启用（7Z/RAR，需要新包密码）' }, { value: 'no', label: '关闭' }]" floating /></label>
          <div class="space-y-2 text-sm"><label class="flex items-center gap-2"><input v-model="changeSplit" type="checkbox" />修改分卷大小</label><input v-if="changeSplit" v-model.number="splitMiB" type="number" min="0" class="conversion-input w-full" placeholder="0 表示不分卷" /></div>
          <div class="space-y-2 text-sm"><label class="flex items-center gap-2"><input v-model="changeSourcePassword" type="checkbox" />填写共用原包密码</label><input v-if="changeSourcePassword" v-model="sourcePassword" type="password" autocomplete="off" class="conversion-input w-full" placeholder="原包未加密可留空" /></div>
          <div v-if="!updatesOnly" class="space-y-2 text-sm"><label class="flex items-center gap-2"><input v-model="changePassword" type="checkbox" />修改新包密码</label><input v-if="changePassword" v-model="password" type="password" autocomplete="new-password" class="conversion-input w-full" placeholder="留空则新包不加密" /></div>
        </div>
        <div class="space-y-2 text-sm">
          <label>输出目录<ThemedSelect v-model="destination" :options="[{ value: 'keep', label: '保持各自目录' }, { value: 'source', label: '各自源文件目录' }, { value: 'custom', label: '统一输出目录' }]" test-id="conversion-batch-destination" floating /></label>
          <div v-if="destination === 'custom'" class="flex min-w-0 gap-2"><input v-model="directory" class="conversion-input min-w-0 flex-1" placeholder="新包保存目录" /><button type="button" class="conversion-button" @click="chooseDirectory">选择目录</button><SavedLocationPicker @select="directory = $event" /></div>
        </div>
      </template>
    </fieldset>
    <template #footer>
      <button type="button" class="conversion-button" :disabled="applying" @click="visible = false">取消</button>
      <button type="button" class="conversion-button bg-primary text-white" data-testid="conversion-batch-apply" :disabled="!valid || store.busy || store.preparing || applying" @click="apply">{{ applying ? '正在应用…' : mode === 'selected' ? `应用到 ${store.selected.length} 项` : '保存默认配置' }}</button>
    </template>
  </Modal>
</template>
